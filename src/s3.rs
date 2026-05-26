use crate::config::SshellConfig;
use crate::gist::{PullStrategy, build_sync_payload, merge_remote};
use anyhow::{Context, Result, bail};
use hmac::{Hmac, Mac};
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HOST};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

const FILE_NAME: &str = "sshell-config.toml";
const SERVICE: &str = "s3";

pub fn push(cfg: &mut SshellConfig) -> Result<String> {
    let endpoint = cfg.settings.s3_endpoint.as_deref().context("s3_endpoint not set")?;
    let bucket = cfg.settings.s3_bucket.as_deref().context("s3_bucket not set")?;
    let access_key = cfg.settings.s3_access_key.as_deref().context("s3_access_key not set")?;
    let secret_key = cfg.settings.s3_secret_key.as_deref().context("s3_secret_key not set")?;
    let payload = build_sync_payload(cfg, cfg.settings.sync_password.as_deref())?;
    let body = toml::to_string_pretty(&payload)?;
    let body_bytes = body.as_bytes();

    let path = format!("/{bucket}/{FILE_NAME}");
    let host = endpoint_host(endpoint);

    let now = chrono_now();
    let region = region_from_endpoint(endpoint);

    let payload_hash = hex_hash(body_bytes);
    let (auth_header, amz_date) = sign(
        access_key,
        secret_key,
        &region,
        &SigningRequest {
            method: "PUT",
            host: &host,
            path: &path,
            query: &[],
            payload_hash: &payload_hash,
            timestamp: &now,
        },
    );

    let url = format!("https://{host}{path}");

    let client = Client::new();
    let response = client
        .put(&url)
        .header(HOST, host.clone())
        .header(CONTENT_TYPE, "application/octet-stream")
        .header("x-amz-content-sha256", &payload_hash)
        .header("x-amz-date", &amz_date)
        .header("Authorization", &auth_header)
        .body(body)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("sync push failed: {status} {body}");
    }

    Ok(url)
}

pub fn pull_with_strategy(cfg: &mut SshellConfig, strategy: PullStrategy) -> Result<usize> {
    let endpoint = cfg.settings.s3_endpoint.as_deref().context("s3_endpoint not set")?;
    let bucket = cfg.settings.s3_bucket.as_deref().context("s3_bucket not set")?;
    let access_key = cfg.settings.s3_access_key.as_deref().context("s3_access_key not set")?;
    let secret_key = cfg.settings.s3_secret_key.as_deref().context("s3_secret_key not set")?;

    let path = format!("/{bucket}/{FILE_NAME}");
    let host = endpoint_host(endpoint);

    let now = chrono_now();
    let region = region_from_endpoint(endpoint);

    let payload_hash = hex_hash(b"");
    let (auth_header, amz_date) = sign(
        access_key,
        secret_key,
        &region,
        &SigningRequest {
            method: "GET",
            host: &host,
            path: &path,
            query: &[],
            payload_hash: &payload_hash,
            timestamp: &now,
        },
    );

    let url = format!("https://{host}{path}");

    let client = Client::new();
    let response = client
        .get(&url)
        .header(HOST, host.clone())
        .header("x-amz-content-sha256", &payload_hash)
        .header("x-amz-date", &amz_date)
        .header("Authorization", &auth_header)
        .send()?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        bail!("sync pull failed: remote file not found");
    }
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("sync pull failed: {status} {body}");
    }

    let content = response.text()?;
    let remote: toml::Value =
        toml::from_str(&content).with_context(|| "failed to parse remote config")?;

    merge_remote(cfg, remote, strategy)
}

// ── AWS Signature V4 ───────────────────────────────────────────

struct SigningRequest<'a> {
    method: &'a str,
    host: &'a str,
    path: &'a str,
    query: &'a [(&'a str, &'a str)],
    payload_hash: &'a str,
    timestamp: &'a str,
}

fn sign(
    access_key: &str,
    secret_key: &str,
    region: &str,
    req: &SigningRequest<'_>,
) -> (String, String) {
    let date = &req.timestamp[..8];
    let amz_date = req.timestamp.to_string();

    let content_type_val = if req.method == "PUT" {
        "application/octet-stream"
    } else {
        ""
    };

    // Canonical headers (sorted by key)
    let mut headers: Vec<(&str, String)> = vec![
        ("content-type", content_type_val.to_string()),
        ("host", req.host.to_string()),
        ("x-amz-content-sha256", req.payload_hash.to_string()),
        ("x-amz-date", amz_date.clone()),
    ];
    headers.sort_by_key(|(k, _)| *k);

    let signed_headers: String = headers.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(";");
    let canonical_headers: String = headers
        .iter()
        .map(|(k, v)| format!("{k}:{v}\n"))
        .collect();

    let canonical_querystring = req.query
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let canonical_request = format!(
        "{method}\n{path}\n{qs}\n{headers}\n{signed}\n{hash}",
        method = req.method,
        path = req.path,
        qs = canonical_querystring,
        headers = canonical_headers,
        signed = signed_headers,
        hash = req.payload_hash,
    );

    let credential_scope = format!("{date}/{region}/{SERVICE}/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{timestamp}\n{scope}\n{hash}",
        timestamp = req.timestamp,
        scope = credential_scope,
        hash = hex_hash(canonical_request.as_bytes()),
    );

    let signing_key = derive_signing_key(secret_key, date, region);
    let signature = hex_hmac(&signing_key, string_to_sign.as_bytes());

    let auth = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );

    (auth, amz_date)
}

fn derive_signing_key(secret_key: &str, date: &str, region: &str) -> Vec<u8> {
    let k_date = hmac_bytes(format!("AWS4{secret_key}").as_bytes(), date.as_bytes());
    let k_region = hmac_bytes(&k_date, region.as_bytes());
    let k_service = hmac_bytes(&k_region, SERVICE.as_bytes());
    hmac_bytes(&k_service, b"aws4_request")
}

fn hmac_bytes(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key len");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn hex_hmac(key: &[u8], data: &[u8]) -> String {
    hex::encode(hmac_bytes(key, data))
}

fn hex_hash(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'~' | b'.' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn endpoint_host(endpoint: &str) -> String {
    let s = endpoint
        .strip_prefix("https://")
        .or_else(|| endpoint.strip_prefix("http://"))
        .unwrap_or(endpoint);
    s.trim_end_matches('/').to_string()
}

fn region_from_endpoint(endpoint: &str) -> String {
    if endpoint.contains("r2.cloudflarestorage.com") {
        return "auto".to_string();
    }
    let host = endpoint_host(endpoint);
    let parts: Vec<&str> = host.split('.').collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "s3" && i + 1 < parts.len() {
            return parts[i + 1].to_string();
        }
    }
    "us-east-1".to_string()
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Simple UTC time formatting without chrono dependency
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year/month/day from days since epoch
    let (year, month, day) = days_to_date(days);

    format!(
        "{year:04}{month:02}{day:02}T{hours:02}{minutes:02}{seconds:02}Z"
    )
}

fn days_to_date(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0u64;
    for (i, &md) in month_days.iter().enumerate() {
        if days < md {
            month = i as u64 + 1;
            break;
        }
        days -= md;
    }
    if month == 0 {
        month = 12;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}
