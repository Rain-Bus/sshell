use crate::config::{ConnectionType, CredentialEntry, SshellConfig, SyncBackend, config_path, find_binary};
use crate::sync::{self, gist, s3, webdav};
use crate::{connection, import, ui};
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Debug, Parser)]
#[command(
    name = "sshell",
    version,
    about = "Personal SSH and shell connection manager"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Tui,
    Connect {
        name: String,
    },
    Import,
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
    Doctor {
        name: Option<String>,
    },
    ConfigPath,
}

#[derive(Debug, Subcommand)]
pub enum SyncCommand {
    Push,
    Pull {
        #[arg(long, value_enum, default_value_t = PullStrategy::Merge)]
        strategy: PullStrategy,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum PullStrategy {
    Merge,
    Overwrite,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Tui) {
        Command::Tui => ui::app::run(),
        Command::Connect { name } => {
            let cfg = SshellConfig::load()?;
            connection::connect(&name, &cfg)
        }
        Command::Import => {
            let mut cfg = SshellConfig::load()?;
            let candidates = import::load_candidates(&cfg)?;
            let count = import::import_candidates(&mut cfg, &candidates)?;
            println!("imported {count} connections");
            Ok(())
        }
        Command::Sync { command } => run_sync(command),
        Command::Doctor { name } => doctor(name),
        Command::ConfigPath => {
            println!("{}", config_path()?.display());
            Ok(())
        }
    }
}

fn run_sync(command: SyncCommand) -> Result<()> {
    let mut cfg = SshellConfig::load()?;
    let strat = |s: PullStrategy| match s {
        PullStrategy::Merge => sync::PullStrategy::Merge,
        PullStrategy::Overwrite => sync::PullStrategy::Overwrite,
    };
    match command {
        SyncCommand::Push => match cfg.settings.backend {
            SyncBackend::Gist => {
                let id = gist::push(&mut cfg)?;
                println!("pushed ({id})");
            }
            SyncBackend::Webdav => {
                webdav::push(&mut cfg)?;
                println!("pushed");
            }
            SyncBackend::S3 => {
                s3::push(&mut cfg)?;
                println!("pushed");
            }
        },
        SyncCommand::Pull { strategy } => {
            let count = match cfg.settings.backend {
                SyncBackend::Gist => gist::pull_with_strategy(&mut cfg, strat(strategy))?,
                SyncBackend::Webdav => webdav::pull_with_strategy(&mut cfg, strat(strategy))?,
                SyncBackend::S3 => s3::pull_with_strategy(&mut cfg, strat(strategy))?,
            };
            println!("pulled {count} items");
        }
    }
    Ok(())
}

fn doctor(name: Option<String>) -> Result<()> {
    let path = config_path()?;
    println!("config: {}", path.display());
    if path.exists() {
        check_config_permissions(&path)?;
    } else {
        println!("config status: missing, will be created on first run");
    }

    let cfg = SshellConfig::load()?;
    println!("connections: {}", cfg.connections.len());
    println!("credentials: {}", cfg.credentials.entries.len());
    let synced = cfg.connections.values().filter(|p| p.sync()).count();
    println!("synced connections: {synced}");
    println!(
        "sync backend: {}",
        match cfg.settings.backend {
            SyncBackend::Gist => "gist",
            SyncBackend::Webdav => "webdav",
            SyncBackend::S3 => "s3",
        }
    );
    println!(
        "ssh binary: {}",
        find_binary("ssh").unwrap_or_else(|| "not found".to_string())
    );
    println!(
        "sshpass binary: {}",
        find_binary("sshpass").unwrap_or_else(|| "not found".to_string())
    );

    if let Some(name) = name {
        let profile = cfg
            .connections
            .get(&name)
            .with_context(|| format!("connection {name} not found"))?;
        check_connection(&cfg, &name, profile)?;
    }
    Ok(())
}

fn check_connection(
    cfg: &SshellConfig,
    name: &str,
    profile: &crate::config::ConnectionProfile,
) -> Result<()> {
    println!("connection: {name}");
    match &profile.kind {
        ConnectionType::Ssh {
            host,
            port,
            user,
            auth_ref,
            sync,
        } => {
            println!("type: ssh");
            println!("target: {user}@{host}:{port}");
            println!("sync: {}", if *sync { "yes" } else { "no" });
            println!("credential: {auth_ref}");
            let credential = cfg
                .credential(auth_ref)
                .with_context(|| format!("credential {auth_ref} missing"))?;
            match credential {
                CredentialEntry::Password { .. } => println!("auth: password"),
                CredentialEntry::PrivateKey { value, .. } => {
                    if value.as_deref().is_some_and(|v| !v.is_empty()) {
                        println!(
                            "auth: embedded private key ({} bytes)",
                            value.as_deref().unwrap_or_default().len()
                        );
                    } else {
                        bail!("private key credential is empty");
                    }
                }
            }
            println!(
                "ssh command: ssh -o StrictHostKeyChecking=accept-new -p {port} {user}@{host}"
            );
        }
        ConnectionType::Shell {
            command,
            sync_args,
            local_args,
            ..
        } => {
            println!("type: shell");
            let mut merged_args = sync_args.clone();
            merged_args.extend(local_args.clone());
            println!("command: {command} {}", merged_args.join(" "));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn check_config_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = path.metadata()?.permissions().mode() & 0o777;
    println!("config permissions: {mode:o}");
    if mode != 0o600 {
        println!("warning: config should be 600; saving from sshell will fix it");
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_config_permissions(_path: &Path) -> Result<()> {
    println!("config permissions: not checked on this platform");
    Ok(())
}
