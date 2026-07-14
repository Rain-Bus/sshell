use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

mod config_migrate;
mod config_shell;

const CONFIG_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshellConfig {
    pub version: u32,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub connections: IndexMap<String, ConnectionProfile>,
    #[serde(default)]
    pub credentials: CredentialStore,
    #[serde(default)]
    pub deleted: IndexMap<String, u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionSource {
    Manual,
    Imported,
    Scanned,
}

#[derive(Debug, Clone, Default)]
pub struct ShellScanConflict {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ShellCandidate {
    pub name: String,
    pub path: PathBuf,
    pub conflict: Option<ShellScanConflict>,
    #[cfg(not(unix))]
    pub wsl_distro: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SyncBackend {
    #[default]
    Gist,
    Webdav,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub backend: SyncBackend,
    pub gist_id: Option<String>,
    pub webdav_url: Option<String>,
    pub webdav_user: Option<String>,
    pub webdav_password: Option<String>,
    pub sync_password: Option<String>,
    #[serde(default)]
    pub sync_on_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub local_tags: Vec<String>,
    pub source: ConnectionSource,
    pub added_order: u64,
    pub usage_count: u64,
    pub modified_at: u64,
    #[serde(flatten)]
    pub kind: ConnectionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConnectionType {
    Ssh {
        host: String,
        #[serde(default = "default_ssh_port")]
        port: u16,
        user: String,
        auth_ref: String,
        #[serde(default = "default_ssh_sync")]
        sync: bool,
    },
    Shell {
        shell_name: String,
        #[serde(default)]
        auth_ref: Option<String>,
        #[serde(default = "default_shell")]
        command: String,
        #[serde(default)]
        sync_args: Vec<String>,
        #[serde(default)]
        local_args: Vec<String>,
        #[serde(default = "default_shell_sync")]
        sync: bool,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CredentialStore {
    #[serde(default)]
    pub entries: IndexMap<String, CredentialEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialEntry {
    Password {
        value: String,
    },
    PrivateKey {
        #[serde(alias = "data", default)]
        value: Option<String>,
        #[serde(default, skip_serializing)]
        path: Option<String>,
    },
}

impl Default for SshellConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            settings: Settings::default(),
            connections: IndexMap::new(),
            credentials: CredentialStore::default(),
            deleted: IndexMap::new(),
        }
    }
}

impl SshellConfig {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            let cfg = Self::default();
            cfg.save()?;
            return Ok(cfg);
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut cfg: Self =
            toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
        cfg.migrate_path_to_embedded();
        cfg.migrate_shell_prefix();
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let data = toml::to_string_pretty(self)?;
        let tmp_path = path.with_extension("toml.tmp");
        fs::write(&tmp_path, &data)
            .with_context(|| format!("failed to write {}", tmp_path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))
                .with_context(|| format!("failed to chmod {}", tmp_path.display()))?;
        }

        fs::rename(&tmp_path, &path).with_context(|| {
            format!(
                "failed to rename {} -> {}",
                tmp_path.display(),
                path.display()
            )
        })?;

        Ok(())
    }

    pub fn credential(&self, auth_ref: &str) -> Option<&CredentialEntry> {
        self.credentials.entries.get(auth_ref)
    }

    pub fn next_added_order(&self) -> u64 {
        self.connections
            .values()
            .map(|profile| profile.added_order)
            .max()
            .unwrap_or(0)
            + 1
    }

    /// Remove a credential if it is no longer referenced by any connection.
    /// `exclude` is an optional connection name to skip during the check (e.g. the one being renamed).
    pub fn prune_credential_if_unused(&mut self, auth_ref: &str, exclude: Option<&str>) {
        let still_used = self
            .connections
            .iter()
            .filter(|(name, _)| Some(name.as_str()) != exclude)
            .any(|(_, profile)| profile.auth_ref() == Some(auth_ref));
        if !still_used {
            self.credentials.entries.shift_remove(auth_ref);
        }
    }
}

impl CredentialEntry {
    pub fn password(value: String) -> Self {
        Self::Password { value }
    }

    pub fn private_key(value: String) -> Self {
        Self::PrivateKey {
            value: Some(value),
            path: None,
        }
    }

    pub fn value(&self) -> &str {
        match self {
            Self::Password { value } => value,
            Self::PrivateKey { value, .. } => value.as_deref().unwrap_or(""),
        }
    }

    pub fn has_value(&self) -> bool {
        match self {
            Self::Password { value } => !value.is_empty(),
            Self::PrivateKey { value, .. } => value.as_deref().is_some_and(|v| !v.is_empty()),
        }
    }
}

impl ConnectionProfile {
    pub fn sync(&self) -> bool {
        match &self.kind {
            ConnectionType::Ssh { sync, .. } => *sync,
            ConnectionType::Shell { sync, .. } => *sync,
        }
    }

    /// For Shell connections, returns the merged sync_args + local_args.
    /// Returns an empty vec for SSH connections.
    pub fn merged_shell_args(&self) -> Vec<String> {
        match &self.kind {
            ConnectionType::Shell {
                sync_args,
                local_args,
                ..
            } => {
                let mut out = sync_args.clone();
                out.extend(local_args.iter().cloned());
                out
            }
            ConnectionType::Ssh { .. } => Vec::new(),
        }
    }
}

pub fn expand_user_path(value: &str) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(value)
}

pub fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir().context("could not find user config directory")?;
    Ok(dir.join("sshell").join("config.toml"))
}

pub fn find_binary(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    let candidates = binary_candidates(name);
    std::env::split_paths(&path)
        .flat_map(|dir| candidates.iter().map(move |c| dir.join(c)))
        .find(|p| p.is_file())
        .map(|p| p.display().to_string())
}

#[cfg(unix)]
fn binary_candidates(name: &str) -> Vec<String> {
    vec![name.to_string()]
}

#[cfg(not(unix))]
fn binary_candidates(name: &str) -> Vec<String> {
    let mut out = vec![name.to_string()];
    if !name.contains('.') {
        if let Ok(ext) = std::env::var("PATHEXT") {
            for ext in ext.split(';') {
                out.push(format!("{name}{ext}"));
            }
        } else {
            for ext in &[".exe", ".cmd", ".bat"] {
                out.push(format!("{name}{ext}"));
            }
        }
    }
    out
}

fn default_ssh_port() -> u16 {
    22
}

fn default_shell() -> String {
    "bash".to_string()
}

fn default_shell_sync() -> bool {
    false
}

fn default_ssh_sync() -> bool {
    true
}

pub fn now_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
