use crate::config::SshellConfig;
use super::gist::{PullStrategy, build_sync_payload, merge_remote};
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, CONTENT_TYPE};

const FILE_NAME: &str = "sshell-config.toml";

pub fn push(cfg: &mut SshellConfig) -> Result<String> {
    let url = webdav_file_url(cfg)?;
    let user = cfg
        .settings
        .webdav_user
        .as_deref()
        .context("webdav_user not set")?;
    let password = cfg
        .settings
        .webdav_password
        .as_deref()
        .context("webdav_password not set")?;

    let payload = build_sync_payload(cfg, cfg.settings.sync_password.as_deref())?;
    let content = toml::to_string_pretty(&payload)?;

    let client = Client::new();
    let response = client
        .put(&url)
        .basic_auth(user, Some(password))
        .header(CONTENT_TYPE, "text/plain")
        .header(ACCEPT, "*/*")
        .body(content)
        .send()?;

    if !response.status().is_success() {
        bail!("sync push failed: {}", response.status());
    }

    Ok(url)
}

pub fn pull_with_strategy(cfg: &mut SshellConfig, strategy: PullStrategy) -> Result<usize> {
    let url = webdav_file_url(cfg)?;
    let user = cfg
        .settings
        .webdav_user
        .as_deref()
        .context("webdav_user not set")?;
    let password = cfg
        .settings
        .webdav_password
        .as_deref()
        .context("webdav_password not set")?;

    let client = Client::new();
    let response = client
        .get(&url)
        .basic_auth(user, Some(password))
        .header(ACCEPT, "*/*")
        .send()?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        bail!("sync pull failed: remote file not found");
    }
    if !response.status().is_success() {
        bail!("sync pull failed: {}", response.status());
    }

    let content = response.text()?;
    let remote: toml::Value =
        toml::from_str(&content).with_context(|| "failed to parse remote config")?;

    merge_remote(cfg, remote, strategy)
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
