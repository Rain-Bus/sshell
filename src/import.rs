use crate::config::{
    ConnectionProfile, ConnectionSource, ConnectionType, CredentialEntry, SshellConfig,
};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ImportCandidate {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub identity_file: Option<PathBuf>,
}

pub fn load_candidates(cfg: &SshellConfig) -> Result<Vec<ImportCandidate>> {
    let path = dirs::home_dir()
        .context("could not find home directory")?
        .join(".ssh")
        .join("config");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut out = Vec::new();
    let mut current: Option<ImportCandidate> = None;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let key = parts.next().unwrap_or("").to_ascii_lowercase();
        let value = parts.next().unwrap_or("").trim();
        if key == "host" {
            push_candidate(&mut out, current.take(), cfg);
            let name = value.split_whitespace().next().unwrap_or("").to_string();
            if name.is_empty() || name.contains('*') || name.contains('?') {
                current = None;
            } else {
                current = Some(ImportCandidate {
                    host: name.clone(),
                    name,
                    port: 22,
                    user: whoami::username(),
                    identity_file: None,
                });
            }
            continue;
        }
        let Some(candidate) = current.as_mut() else {
            continue;
        };
        match key.as_str() {
            "hostname" => candidate.host = value.to_string(),
            "port" => candidate.port = value.parse().unwrap_or(22),
            "user" => candidate.user = value.to_string(),
            "identityfile" => candidate.identity_file = Some(expand_ssh_path(value)),
            _ => {}
        }
    }
    push_candidate(&mut out, current.take(), cfg);
    Ok(out)
}

pub fn import_candidates(cfg: &mut SshellConfig, candidates: &[ImportCandidate]) -> Result<usize> {
    let mut count = 0;
    for item in candidates {
        if cfg.connections.contains_key(&item.name) {
            continue;
        }
        let auth_ref = imported_auth_ref(item);
        let mut tags = vec!["imported".to_string()];

        if let Some(path) = &item.identity_file {
            let key_content = fs::read_to_string(path).ok();
            if let Some(existing) = cfg.credentials.entries.get(&auth_ref) {
                if let Some(content) = key_content.as_deref()
                    && existing.value() != content
                {
                    bail!("credential {auth_ref} already exists with different content");
                }
                tags.push("key-reused".to_string());
            } else {
                match key_content {
                    Some(content) => {
                        cfg.credentials.entries.insert(
                            auth_ref.clone(),
                            CredentialEntry::private_key(content),
                        );
                        tags.push("key".to_string());
                    }
                    None => {
                        cfg.credentials.entries.insert(
                            auth_ref.clone(),
                            CredentialEntry::PrivateKey {
                                value: None,
                                path: None,
                            },
                        );
                        tags.push("key-missing".to_string());
                    }
                }
            }
        }

        cfg.connections.insert(
            item.name.clone(),
            ConnectionProfile {
                tags,
                local_tags: Vec::new(),
                source: ConnectionSource::Imported,
                added_order: cfg.next_added_order(),
                usage_count: 0,
                kind: ConnectionType::Ssh {
                    host: item.host.clone(),
                    port: item.port,
                    user: item.user.clone(),
                    auth_ref,
                    sync: true,
                },
            },
        );
        count += 1;
    }
    cfg.save()?;
    Ok(count)
}

fn push_candidate(
    out: &mut Vec<ImportCandidate>,
    candidate: Option<ImportCandidate>,
    cfg: &SshellConfig,
) {
    if let Some(candidate) = candidate
        && !cfg.connections.contains_key(&candidate.name)
    {
        out.push(candidate);
    }
}

fn expand_ssh_path(value: &str) -> PathBuf {
    crate::config::expand_user_path(value.trim_matches('"'))
}

fn imported_auth_ref(item: &ImportCandidate) -> String {
    item.identity_file
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{}-auth", item.name))
}
