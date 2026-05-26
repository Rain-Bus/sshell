use crate::config::{ConnectionType, CredentialEntry, SshellConfig};
use anyhow::{Context, Result, bail};
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

pub fn connect(name: &str, cfg: &SshellConfig) -> Result<()> {
    let profile = cfg
        .connections
        .get(name)
        .with_context(|| format!("connection {name} not found"))?;
    crate::ui::restore_terminal()?;
    match &profile.kind {
        ConnectionType::Ssh {
            host,
            port,
            user,
            auth_ref,
            ..
        } => connect_ssh(cfg, host, *port, user, auth_ref),
        ConnectionType::Shell {
            command,
            sync_args,
            local_args,
            ..
        } => {
            let mut merged_args = sync_args.clone();
            merged_args.extend(local_args.clone());
            exec_shell(command, &merged_args)
        }
    }
}

fn connect_ssh(
    cfg: &SshellConfig,
    host: &str,
    port: u16,
    user: &str,
    auth_ref: &str,
) -> Result<()> {
    if auth_ref.is_empty() {
        bail!("this connection has no credential; edit it and set password or private key");
    }
    match cfg.credential(auth_ref) {
        Some(CredentialEntry::Password { value, .. }) => {
            let args = vec![
                "ssh".to_string(),
                "-o".to_string(),
                "StrictHostKeyChecking=accept-new".to_string(),
                "-p".to_string(),
                port.to_string(),
                format!("{user}@{host}"),
            ];
            run_sshpass(&args, value)
        }
        Some(CredentialEntry::PrivateKey { value, .. }) => {
            let key_content = match value {
                Some(v) if !v.is_empty() => v,
                _ => bail!("private key credential {auth_ref} is empty"),
            };
            let mut key = NamedTempFile::new()?;
            key.write_all(key_content.as_bytes())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                key.as_file()
                    .set_permissions(std::fs::Permissions::from_mode(0o600))?;
            }
            let status = Command::new("ssh")
                .arg("-o")
                .arg("StrictHostKeyChecking=accept-new")
                .arg("-i")
                .arg(key.path())
                .arg("-p")
                .arg(port.to_string())
                .arg(format!("{user}@{host}"))
                .status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        None => bail!("credential {auth_ref} not found"),
    }
}

fn run_sshpass(args: &[String], password: &str) -> Result<()> {
    let status = Command::new("sshpass")
        .arg("-e")
        .args(args)
        .env("SSHPASS", password)
        .status()
        .context("failed to run sshpass — is it installed?")?;
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(unix)]
fn exec_shell(command: &str, args: &[String]) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let err = Command::new(command).args(args).exec();
    Err(err).with_context(|| format!("failed to exec {command}"))
}

#[cfg(not(unix))]
fn exec_shell(command: &str, args: &[String]) -> Result<()> {
    let status = Command::new(command)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {command}"))?;
    std::process::exit(status.code().unwrap_or(1));
}
