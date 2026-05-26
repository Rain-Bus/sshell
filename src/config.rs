use anyhow::{Context, Result, bail};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SyncBackend {
    #[default]
    Gist,
    Webdav,
    S3,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub backend: SyncBackend,
    pub gist_id: Option<String>,
    pub webdav_url: Option<String>,
    pub webdav_user: Option<String>,
    pub webdav_password: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    #[serde(default)]
    pub sync_usage_count: bool,
    pub sync_password: Option<String>,
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

    pub fn local_shell_candidates(&self) -> Vec<ShellCandidate> {
        let mut out = Vec::new();
        for path in local_shell_paths() {
            let Some(base_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let command = path.to_string_lossy().to_string();
            if self.connections.values().any(|profile| {
                matches!(&profile.kind, ConnectionType::Shell { command: existing, .. } if existing == &command)
            }) {
                continue;
            }
            let conflict = self
                .connections
                .contains_key(base_name)
                .then(|| ShellScanConflict {
                    name: base_name.to_string(),
                    path: path.clone(),
                });
            out.push(ShellCandidate {
                name: base_name.to_string(),
                path,
                conflict,
            });
        }
        out
    }

    pub fn local_shell_command(&self, shell_name: &str) -> Option<String> {
        self.connections
            .values()
            .find_map(|profile| {
                if let ConnectionType::Shell {
                    shell_name: existing_shell_name,
                    command,
                    ..
                } = &profile.kind
                    && existing_shell_name == shell_name
                {
                    Some(command.clone())
                } else {
                    None
                }
            })
            .or_else(|| {
                local_shell_paths()
                    .into_iter()
                    .find(|path| {
                        path.file_name().and_then(|value| value.to_str()) == Some(shell_name)
                    })
                    .map(|path| path.to_string_lossy().to_string())
            })
    }

    pub fn add_local_shell(&mut self, candidate: &ShellCandidate) -> Result<()> {
        if candidate.conflict.is_some() || self.connections.contains_key(&candidate.name) {
            bail!("shell name conflict: {}", candidate.name);
        }
        let command = candidate.path.to_string_lossy().to_string();
        if self.connections.values().any(|profile| {
            matches!(&profile.kind, ConnectionType::Shell { command: existing, .. } if existing == &command)
        }) {
            return Ok(());
        }
        self.connections.insert(
            candidate.name.clone(),
            ConnectionProfile {
                tags: Vec::new(),
                local_tags: vec!["local".to_string(), "scanned".to_string()],
                source: ConnectionSource::Scanned,
                added_order: self.next_added_order(),
                usage_count: 0,
                kind: ConnectionType::Shell {
                    shell_name: candidate.name.clone(),
                    auth_ref: None,
                    command,
                    sync_args: Vec::new(),
                    local_args: Vec::new(),
                    sync: false,
                },
            },
        );
        Ok(())
    }

    fn migrate_path_to_embedded(&mut self) {
        for entry in self.credentials.entries.values_mut() {
            let CredentialEntry::PrivateKey { value, path, .. } = entry else {
                continue;
            };
            if (value.is_none() || value.as_deref().is_some_and(|v| v.is_empty()))
                && let Some(p) = path.take()
            {
                let expanded = expand_user_path(&p);
                if let Ok(content) = fs::read_to_string(&expanded) {
                    *value = Some(content);
                }
            }
            *path = None;
        }
    }
}

#[cfg(unix)]
fn local_shell_paths() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Ok(raw) = fs::read_to_string("/etc/shells") {
        for line in raw.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let path = PathBuf::from(line);
            if is_executable_file(&path)
                && !out.iter().any(|existing| same_file_name(existing, &path))
            {
                out.push(path);
            }
        }
    }

    if out.is_empty() {
        for candidate in [
            "/bin/bash",
            "/bin/zsh",
            "/bin/sh",
            "/usr/bin/bash",
            "/usr/bin/zsh",
            "/usr/bin/sh",
        ] {
            let path = PathBuf::from(candidate);
            if is_executable_file(&path)
                && !out.iter().any(|existing| same_file_name(existing, &path))
            {
                out.push(path);
            }
        }
    }

    out
}

#[cfg(not(unix))]
fn local_shell_paths() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    for name in &["pwsh", "powershell", "cmd", "bash"] {
        if let Some(found) = find_binary(name) {
            let path = PathBuf::from(&found);
            if !out.iter().any(|existing| same_file_name(existing, &path)) {
                out.push(path);
            }
        }
    }

    let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    for path in [
        PathBuf::from(&system_root).join("System32").join("WindowsPowerShell").join("v1.0").join("powershell.exe"),
        PathBuf::from(&system_root).join("System32").join("cmd.exe"),
    ] {
        if path.is_file() && !out.iter().any(|existing| same_file_name(existing, &path)) {
            out.push(path);
        }
    }

    for path in [
        PathBuf::from(r"C:\Program Files\Git\bin\bash.exe"),
        PathBuf::from(r"C:\Program Files (x86)\Git\bin\bash.exe"),
    ] {
        if path.is_file() && !out.iter().any(|existing| same_file_name(existing, &path)) {
            out.push(path);
        }
    }

    out
}

fn same_file_name(a: &Path, b: &Path) -> bool {
    a.file_name() == b.file_name()
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file() && is_executable(path)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    let exts = [
        std::ffi::OsStr::new("exe"),
        std::ffi::OsStr::new("cmd"),
        std::ffi::OsStr::new("bat"),
        std::ffi::OsStr::new("ps1"),
    ];
    path.extension().is_some_and(|ext| exts.contains(&ext))
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
