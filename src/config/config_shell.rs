use super::{ConnectionProfile, ConnectionSource, ConnectionType, ShellCandidate, ShellScanConflict};
use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

impl super::SshellConfig {
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
                .contains_key(&format!("${base_name}"))
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
        let key = format!("${}", candidate.name);
        if candidate.conflict.is_some() || self.connections.contains_key(&key) {
            bail!("shell name conflict: {}", candidate.name);
        }
        let command = candidate.path.to_string_lossy().to_string();
        if self.connections.values().any(|profile| {
            matches!(&profile.kind, ConnectionType::Shell { command: existing, .. } if existing == &command)
        }) {
            return Ok(());
        }
        self.connections.insert(
            key,
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
        if let Some(found) = super::find_binary(name) {
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
