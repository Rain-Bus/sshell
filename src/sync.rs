mod crypto;
pub mod gist;
pub mod webdav;

use crate::config::SyncBackend;

use crate::config::ConnectionType;
use crate::config::{ConnectionSource, CredentialStore, SshellConfig};
use anyhow::{Context, Result};
use indexmap::IndexMap;

pub(crate) const GIST_TOKEN_REF: &str = "__gist_token";

// ── Sync report ──────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SyncReport {
    pub pulled: usize,
    pub updated: usize,
    pub pushed: usize,
    pub deleted: usize,
    pub skipped: usize,
}

impl std::fmt::Display for SyncReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.pulled > 0 {
            parts.push(format!("{} pulled", self.pulled));
        }
        if self.updated > 0 {
            parts.push(format!("{} updated", self.updated));
        }
        if self.pushed > 0 {
            parts.push(format!("{} pushed", self.pushed));
        }
        if self.deleted > 0 {
            parts.push(format!("{} deleted", self.deleted));
        }
        if self.skipped > 0 {
            parts.push(format!("{} skipped", self.skipped));
        }
        if parts.is_empty() {
            write!(f, "already up to date")
        } else {
            write!(f, "sync: {}", parts.join(", "))
        }
    }
}

// ── Remote payload ───────────────────────────────────────────────

struct RemotePayload {
    connections: IndexMap<String, crate::config::ConnectionProfile>,
    credentials: CredentialStore,
    deleted: IndexMap<String, u64>,
}

fn parse_remote_payload(remote: toml::Value, sync_password: Option<&str>) -> Result<RemotePayload> {
    let mut connections = IndexMap::new();
    if let Some(conns) = remote.get("connections").and_then(|v| v.as_table()) {
        for (name, profile_val) in conns {
            if let Ok(profile) = profile_val
                .clone()
                .try_into::<crate::config::ConnectionProfile>()
            {
                connections.insert(name.clone(), profile);
            }
        }
    }

    let credentials =
        if let Some(enc) = remote.get("credentials_encrypted").and_then(|v| v.as_str()) {
            let pw = sync_password
                .context("sync_password not set; needed to decrypt remote credentials")?;
            crypto::decrypt_credentials(enc, pw)?
        } else if let Some(creds_val) = remote.get("credentials") {
            creds_val
                .clone()
                .try_into::<CredentialStore>()
                .unwrap_or_default()
        } else {
            CredentialStore::default()
        };

    let mut deleted = IndexMap::new();
    if let Some(del) = remote.get("deleted").and_then(|v| v.as_table()) {
        for (name, ts_val) in del {
            if let Some(ts) = ts_val.as_integer() {
                deleted.insert(name.clone(), ts as u64);
            }
        }
    }

    Ok(RemotePayload {
        connections,
        credentials,
        deleted,
    })
}

// ── Bidirectional merge ──────────────────────────────────────────

pub(crate) fn bidirectional_merge(
    cfg: &mut SshellConfig,
    remote: toml::Value,
) -> Result<SyncReport> {
    let mut report = SyncReport::default();
    let remote_payload = parse_remote_payload(remote, cfg.settings.sync_password.as_deref())?;

    // 1. Merge remote connections into local
    for (name, remote_profile) in &remote_payload.connections {
        match cfg.connections.get_mut(name) {
            None => {
                // Only remote has it → pull
                let mut p = remote_profile.clone();
                if localize_shell_profile(cfg, name, &mut p) {
                    cfg.connections.insert(name.clone(), p);
                    report.pulled += 1;
                } else {
                    report.skipped += 1;
                }
            }
            Some(local_profile) if remote_profile.modified_at > local_profile.modified_at => {
                // Remote is newer → update local (preserve local-only fields)
                let mut p = remote_profile.clone();
                p.local_tags = local_profile.local_tags.clone();
                p.usage_count = local_profile.usage_count;
                p.added_order = local_profile.added_order;
                p.source = local_profile.source;
                // Preserve local_args for shell connections
                if let (
                    ConnectionType::Shell { local_args, .. },
                    ConnectionType::Shell {
                        local_args: remote_local_args,
                        ..
                    },
                ) = (&local_profile.kind, &p.kind)
                {
                    // Keep existing local_args from the remote profile's
                    // localization step
                    let _ = (local_args, remote_local_args);
                }
                if let ConnectionType::Shell { .. } = &p.kind {
                    // Re-localize the shell profile for this machine
                    let preserved = (p.local_tags.clone(), p.usage_count, p.added_order, p.source);
                    if localize_shell_profile(cfg, name, &mut p) {
                        p.local_tags = preserved.0;
                        p.usage_count = preserved.1;
                        p.added_order = preserved.2;
                        p.source = preserved.3;
                        *cfg.connections.get_mut(name).unwrap() = p;
                        report.updated += 1;
                    } else {
                        report.skipped += 1;
                    }
                } else {
                    *cfg.connections.get_mut(name).unwrap() = p;
                    report.updated += 1;
                }
            }
            _ => {
                // Local is newer or equal → nothing to do
            }
        }
    }

    // 2. Count connections to push (local has newer or remote doesn't have)
    for (name, local_profile) in &cfg.connections {
        if !local_profile.sync() {
            continue;
        }
        match remote_payload.connections.get(name) {
            None => report.pushed += 1,
            Some(remote_p) if local_profile.modified_at > remote_p.modified_at => {
                report.pushed += 1
            }
            _ => {}
        }
    }

    // 3. Process remote tombstones
    for (name, tombstone_ts) in &remote_payload.deleted {
        if let Some(local_profile) = cfg.connections.get(name)
            && *tombstone_ts > local_profile.modified_at
        {
            // Remote deletion is newer → delete locally
            let removed = cfg.connections.shift_remove(name);
            if let Some(profile) = removed
                && let Some(auth_ref) = profile.auth_ref()
            {
                cfg.prune_credential_if_unused(auth_ref, None);
            }
            report.deleted += 1;
        }
        // else: local is newer or not present → local edit wins over deletion
    }

    // 4. Merge credentials: bring in remote credentials for pulled/updated connections
    for (name, remote_profile) in &remote_payload.connections {
        let local_is_newer = cfg
            .connections
            .get(name)
            .is_some_and(|lp| lp.modified_at >= remote_profile.modified_at);

        if !local_is_newer {
            // We accepted the remote version of this connection → bring its credential too
            if let Some(auth_ref) = remote_profile.auth_ref()
                && let Some(credential) = remote_payload.credentials.entries.get(auth_ref)
            {
                cfg.credentials
                    .entries
                    .insert(auth_ref.to_string(), credential.clone());
            }
        }
    }

    // 5. Prune local tombstones that both sides agree are gone
    cfg.deleted.retain(|name, _| {
        // Keep tombstone if remote still has this connection (need to propagate deletion)
        remote_payload.connections.contains_key(name)
    });

    // NOTE: caller is responsible for cfg.save() after successful upload
    Ok(report)
}

// ── Helpers ──────────────────────────────────────────────────────

pub(crate) fn localize_shell_profile(
    cfg: &SshellConfig,
    name: &str,
    profile: &mut crate::config::ConnectionProfile,
) -> bool {
    let ConnectionType::Shell {
        shell_name,
        command,
        local_args,
        auth_ref,
        ..
    } = &mut profile.kind
    else {
        return true;
    };

    local_args.clear();
    *auth_ref = None;

    let Some(local_command) = cfg.local_shell_command(shell_name) else {
        return false;
    };
    *command = local_command;

    if let Some(local_profile) = cfg.connections.get(name) {
        profile.local_tags = local_profile.local_tags.clone();
        profile.added_order = local_profile.added_order;
        profile.usage_count = local_profile.usage_count;
        if let ConnectionType::Shell {
            local_args: existing_local_args,
            ..
        } = &local_profile.kind
        {
            *local_args = existing_local_args.clone();
        }
    }

    true
}

pub(crate) fn build_sync_payload(
    cfg: &SshellConfig,
    sync_password: Option<&str>,
) -> Result<toml::Value> {
    let mut payload = cfg.clone();

    let synced_refs: Vec<String> = payload
        .connections
        .iter()
        .filter(|(_, profile)| profile.sync())
        .filter_map(|(_, profile)| profile.auth_ref().map(String::from))
        .collect();

    payload
        .credentials
        .entries
        .retain(|name, _| name != GIST_TOKEN_REF && synced_refs.iter().any(|r| r == name));

    let encrypted = if !payload.credentials.entries.is_empty() {
        if let Some(pw) = sync_password {
            Some(crypto::encrypt_credentials(&payload.credentials, pw)?)
        } else {
            None
        }
    } else {
        None
    };

    let mut table = toml::map::Map::new();
    table.insert(
        "version".to_string(),
        toml::Value::Integer(payload.version as i64),
    );
    let mut conns = toml::map::Map::new();
    for (name, profile) in &mut payload.connections {
        profile.local_tags.clear();
        profile.usage_count = 0;
        match &mut profile.kind {
            ConnectionType::Shell {
                auth_ref,
                command,
                shell_name: _,
                local_args,
                sync,
                ..
            } => {
                if !*sync {
                    continue;
                }
                local_args.clear();
                *command = String::new();
                *auth_ref = None;
            }
            ConnectionType::Ssh { sync, .. } => {
                if !*sync {
                    continue;
                }
                profile.source = ConnectionSource::Manual;
            }
        }
        conns.insert(name.clone(), to_toml_value(&*profile)?);
    }
    table.insert("connections".to_string(), toml::Value::Table(conns));

    if let Some(enc) = encrypted {
        table.insert(
            "credentials_encrypted".to_string(),
            toml::Value::String(enc),
        );
    } else if !payload.credentials.entries.is_empty() {
        table.insert(
            "credentials".to_string(),
            to_toml_value(&payload.credentials)?,
        );
    }

    // Include tombstones
    if !payload.deleted.is_empty() {
        let mut del_map = toml::map::Map::new();
        for (name, ts) in &payload.deleted {
            del_map.insert(name.clone(), toml::Value::Integer(*ts as i64));
        }
        table.insert("deleted".to_string(), toml::Value::Table(del_map));
    }

    Ok(toml::Value::Table(table))
}

pub(crate) fn count_synced(cfg: &SshellConfig) -> usize {
    cfg.connections.iter().filter(|(_, p)| p.sync()).count()
}

pub(crate) fn to_toml_value<T: serde::Serialize>(val: &T) -> Result<toml::Value> {
    toml::Value::try_from(val).map_err(|e| anyhow::anyhow!("toml conversion failed: {e}"))
}

/// Run sync using the configured backend.
pub fn run_sync(cfg: &mut crate::config::SshellConfig) -> Result<SyncReport> {
    match cfg.settings.backend {
        SyncBackend::Gist => gist::sync(cfg),
        SyncBackend::Webdav => webdav::sync(cfg),
    }
}
