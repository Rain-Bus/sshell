mod crypto;
pub mod gist;
pub mod webdav;

use crate::config::ConnectionType;
use crate::config::{ConnectionSource, CredentialStore, SshellConfig};
use anyhow::{Context, Result};

pub(crate) const GIST_TOKEN_REF: &str = "__gist_token";

#[derive(Debug, Clone, Copy)]
pub enum PullStrategy {
    Merge,
    Overwrite,
}

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

    payload.credentials.entries.retain(|name, _| {
        name != GIST_TOKEN_REF && synced_refs.iter().any(|r| r == name)
    });

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

    Ok(toml::Value::Table(table))
}

pub(crate) fn merge_remote(
    cfg: &mut SshellConfig,
    remote: toml::Value,
    strategy: PullStrategy,
) -> Result<usize> {
    let mut count = 0;

    if let Some(conns) = remote.get("connections").and_then(|v| v.as_table()) {
        for (name, profile_val) in conns {
            let should_insert = match strategy {
                PullStrategy::Merge => !cfg.connections.contains_key(name),
                PullStrategy::Overwrite => true,
            };
            if should_insert
                && let Ok(mut profile) = profile_val
                    .clone()
                    .try_into::<crate::config::ConnectionProfile>()
                && localize_shell_profile(cfg, name, &mut profile)
            {
                cfg.connections.insert(name.clone(), profile);
                count += 1;
            }
        }
    }

    let remote_credentials =
        if let Some(enc) = remote.get("credentials_encrypted").and_then(|v| v.as_str()) {
            let sync_password = cfg
                .settings
                .sync_password
                .as_deref()
                .context("sync_password not set; needed to decrypt remote credentials")?;
            Some(crypto::decrypt_credentials(enc, sync_password)?)
        } else if let Some(creds_val) = remote.get("credentials") {
            creds_val.clone().try_into::<CredentialStore>().ok()
        } else {
            None
        };

    if let Some(remote_creds) = remote_credentials {
        for (name, credential) in remote_creds.entries {
            if name == GIST_TOKEN_REF {
                continue;
            }
            let should_insert = match strategy {
                PullStrategy::Merge => !cfg.credentials.entries.contains_key(&name),
                PullStrategy::Overwrite => true,
            };
            if should_insert {
                cfg.credentials.entries.insert(name, credential);
                count += 1;
            }
        }
    }

    cfg.save()?;
    Ok(count)
}

pub(crate) fn to_toml_value<T: serde::Serialize>(val: &T) -> Result<toml::Value> {
    toml::Value::try_from(val).map_err(|e| anyhow::anyhow!("toml conversion failed: {e}"))
}
