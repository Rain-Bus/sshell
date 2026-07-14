use super::{SyncReport, bidirectional_merge, build_sync_payload, count_synced};
use crate::config::SshellConfig;
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, CONTENT_TYPE};

const FILE_NAME: &str = "sshell-config.toml";

pub fn sync(cfg: &mut SshellConfig) -> Result<SyncReport> {
    let url = webdav_file_url(cfg)?;
    let user = cfg
        .settings
        .webdav_user
        .clone()
        .context("webdav_user not set")?;
    let password = cfg
        .settings
        .webdav_password
        .clone()
        .context("webdav_password not set")?;
    let client = Client::new();

    // Step 1: Download remote
    let remote_payload = {
        let response = client
            .get(&url)
            .basic_auth(&user, Some(&password))
            .header(ACCEPT, "*/*")
            .send()?;
        if response.status().is_success() {
            let content = response.text()?;
            Some(toml::from_str(&content).with_context(|| "failed to parse remote config")?)
        } else {
            None
        }
    };

    // Step 2: Snapshot before merge so we can rollback on upload failure
    let snapshot = cfg.clone();

    // Step 3: Bidirectional merge (modifies cfg in memory only)
    let report = if let Some(remote) = remote_payload {
        bidirectional_merge(cfg, remote)?
    } else {
        SyncReport {
            pushed: count_synced(cfg),
            ..Default::default()
        }
    };

    // Step 4: Upload merged payload
    let payload = build_sync_payload(cfg, cfg.settings.sync_password.as_deref())?;
    let content = toml::to_string_pretty(&payload)?;
    let response = client
        .put(&url)
        .basic_auth(&user, Some(&password))
        .header(CONTENT_TYPE, "text/plain")
        .header(ACCEPT, "*/*")
        .body(content)
        .send()?;
    if !response.status().is_success() {
        // Rollback in-memory state
        *cfg = snapshot;
        bail!("sync upload failed: {}", response.status());
    }

    cfg.save()?;
    Ok(report)
}

fn webdav_file_url(cfg: &SshellConfig) -> Result<String> {
    let base = cfg
        .settings
        .webdav_url
        .as_deref()
        .context("webdav_url not set")?;
    let base = base.trim_end_matches('/');
    Ok(format!("{base}/{FILE_NAME}"))
}
