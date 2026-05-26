use crate::config::{CredentialEntry, SshellConfig};
use super::{GIST_TOKEN_REF, PullStrategy, build_sync_payload, merge_remote};
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde_json::json;

const FILE_NAME: &str = "sshell-config.toml";

pub fn push(cfg: &mut SshellConfig) -> Result<String> {
    let token = gist_token(cfg)?;

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
        bail!("sync push failed: {}", response.status());
    }
    let value: serde_json::Value = response.json()?;
    let id = value["id"]
        .as_str()
        .context("sync response did not include id")?
        .to_string();
    cfg.settings.gist_id = Some(id.clone());
    cfg.save()?;
    Ok(id)
}

pub fn pull_with_strategy(cfg: &mut SshellConfig, strategy: PullStrategy) -> Result<usize> {
    let token = gist_token(cfg)?;
    let id = cfg
        .settings
        .gist_id
        .clone()
        .context("gist id not configured")?;
    let client = Client::new();
    let response = client
        .get(format!("https://api.github.com/gists/{id}"))
        .bearer_auth(token)
        .header("User-Agent", "sshell")
        .send()?;
    if !response.status().is_success() {
        bail!("sync pull failed: {}", response.status());
    }
    let value: serde_json::Value = response.json()?;
    let content = value["files"][FILE_NAME]["content"]
        .as_str()
        .context("remote file not found")?;

    let remote: toml::Value =
        toml::from_str(content).with_context(|| "failed to parse remote config")?;

    merge_remote(cfg, remote, strategy)
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
