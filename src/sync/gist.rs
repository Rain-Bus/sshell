use super::{GIST_TOKEN_REF, SyncReport, bidirectional_merge, build_sync_payload, count_synced};
use crate::config::{CredentialEntry, SshellConfig};
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde_json::json;

const FILE_NAME: &str = "sshell-config.toml";

pub fn sync(cfg: &mut SshellConfig) -> Result<SyncReport> {
    let token = gist_token(cfg)?;

    // Step 1: Download remote if gist_id exists
    let remote_payload = if let Some(id) = &cfg.settings.gist_id {
        let client = Client::new();
        let response = client
            .get(format!("https://api.github.com/gists/{id}"))
            .bearer_auth(&token)
            .header("User-Agent", "sshell")
            .send()?;
        if response.status().is_success() {
            let value: serde_json::Value = response.json()?;
            if let Some(content) = value["files"][FILE_NAME]["content"].as_str() {
                Some(toml::from_str(content).with_context(|| "failed to parse remote config")?)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
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
    let body = json!({
        "description": "sshell sync",
        "public": false,
        "files": { FILE_NAME: { "content": content } }
    });

    let client = Client::new();
    let response = if let Some(id) = &cfg.settings.gist_id {
        client
            .patch(format!("https://api.github.com/gists/{id}"))
            .bearer_auth(&token)
            .header("User-Agent", "sshell")
            .json(&body)
            .send()?
    } else {
        client
            .post("https://api.github.com/gists")
            .bearer_auth(&token)
            .header("User-Agent", "sshell")
            .json(&body)
            .send()?
    };

    if !response.status().is_success() {
        // Rollback in-memory state
        *cfg = snapshot;
        bail!("sync upload failed: {}", response.status());
    }

    // Save gist_id if this was a first-time creation
    if cfg.settings.gist_id.is_none() {
        let value: serde_json::Value = response.json()?;
        if let Some(id) = value["id"].as_str() {
            cfg.settings.gist_id = Some(id.to_string());
        }
    }
    cfg.save()?;

    Ok(report)
}

fn gist_token(cfg: &SshellConfig) -> Result<String> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.trim().is_empty()
    {
        return Ok(token);
    }
    match cfg.credentials.entries.get(GIST_TOKEN_REF) {
        Some(CredentialEntry::Password { value, .. }) if !value.trim().is_empty() => {
            Ok(value.clone())
        }
        _ => bail!("set GITHUB_TOKEN or create password credential __gist_token"),
    }
}
