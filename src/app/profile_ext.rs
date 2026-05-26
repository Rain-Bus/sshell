use crate::config::{ConnectionProfile, ConnectionType};
use std::fs;

pub fn display_name(key: &str) -> &str {
    key.strip_prefix('$').unwrap_or(key)
}

pub fn matches_search(name: &str, profile: &ConnectionProfile, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    if display_name(name).to_lowercase().contains(&query) {
        return true;
    }
    if profile
        .tags
        .iter()
        .chain(profile.local_tags.iter())
        .any(|t| t.to_lowercase().contains(&query))
    {
        return true;
    }
    match &profile.kind {
        ConnectionType::Ssh { host, user, .. } => {
            host.to_lowercase().contains(&query)
                || format!("{user}@{host}").to_lowercase().contains(&query)
        }
        ConnectionType::Shell { command, .. } => command.to_lowercase().contains(&query),
    }
}

pub fn smart_score(profile: &ConnectionProfile) -> u64 {
    profile.usage_count.saturating_mul(10_000) + profile.added_order
}

impl ConnectionProfile {
    pub fn auth_ref(&self) -> Option<&str> {
        match &self.kind {
            ConnectionType::Ssh { auth_ref, .. } => {
                if auth_ref.is_empty() {
                    None
                } else {
                    Some(auth_ref)
                }
            }
            ConnectionType::Shell { auth_ref, .. } => auth_ref.as_deref(),
        }
    }

    pub fn auth_ref_mut(&mut self) -> Option<&mut String> {
        match &mut self.kind {
            ConnectionType::Ssh { auth_ref, .. } => {
                if auth_ref.is_empty() {
                    None
                } else {
                    Some(auth_ref)
                }
            }
            ConnectionType::Shell { auth_ref, .. } => auth_ref.as_mut(),
        }
    }
}

pub fn resolve_secret(secret: &str) -> String {
    let path = crate::config::expand_user_path(secret);
    if !looks_like_private_key(secret)
        && path.is_file()
        && let Ok(content) = fs::read_to_string(&path)
    {
        return content;
    }
    secret.to_string()
}

pub fn non_empty(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

pub fn shell_name_for_command(command: &str) -> String {
    let command = non_empty(command, "bash");
    std::path::Path::new(&command)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(command.as_str())
        .to_string()
}

pub fn split_args(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            ch if ch.is_whitespace() => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if escaped {
        current.push('\\');
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

pub fn looks_like_private_key(value: &str) -> bool {
    value.contains("BEGIN ") && value.contains("PRIVATE KEY")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_args_empty() {
        assert_eq!(split_args(""), Vec::<String>::new());
    }

    #[test]
    fn split_args_simple() {
        assert_eq!(split_args("hello world foo"), vec!["hello", "world", "foo"]);
    }

    #[test]
    fn split_args_single_quotes() {
        assert_eq!(
            split_args("hello 'world baz' foo"),
            vec!["hello", "world baz", "foo"]
        );
    }

    #[test]
    fn split_args_double_quotes() {
        assert_eq!(
            split_args("hello \"world baz\" foo"),
            vec!["hello", "world baz", "foo"]
        );
    }

    #[test]
    fn split_args_escaped() {
        assert_eq!(split_args(r"hello\ world foo"), vec!["hello world", "foo"]);
    }

    #[test]
    fn split_args_trailing_backslash() {
        assert_eq!(split_args(r"hello\"), vec!["hello\\"]);
    }

    #[test]
    fn looks_like_private_key_positive() {
        assert!(looks_like_private_key(
            "-----BEGIN OPENSSH PRIVATE KEY-----"
        ));
        assert!(looks_like_private_key(
            "-----BEGIN RSA PRIVATE KEY-----\nsome data"
        ));
    }

    #[test]
    fn looks_like_private_key_negative() {
        assert!(!looks_like_private_key("just a password"));
        assert!(!looks_like_private_key("BEGIN something"));
    }

    #[test]
    fn smart_score_basic() {
        let make_profile = |usage: u64, order: u64| ConnectionProfile {
            tags: vec![],
            local_tags: vec![],
            source: crate::config::ConnectionSource::Manual,
            added_order: order,
            usage_count: usage,
            kind: crate::config::ConnectionType::Ssh {
                host: "h".into(),
                port: 22,
                user: "u".into(),
                auth_ref: "a".into(),
                sync: true,
            },
        };
        assert_eq!(smart_score(&make_profile(0, 1)), 1);
        assert_eq!(smart_score(&make_profile(3, 5)), 30_005);
        assert!(smart_score(&make_profile(5, 1)) > smart_score(&make_profile(1, 5)));
    }

    #[test]
    fn matches_search_by_name() {
        let profile = ConnectionProfile {
            tags: vec![],
            local_tags: vec![],
            source: crate::config::ConnectionSource::Manual,
            added_order: 1,
            usage_count: 0,
            kind: ConnectionType::Ssh {
                host: "example.com".into(),
                port: 22,
                user: "admin".into(),
                auth_ref: "a".into(),
                sync: true,
            },
        };
        assert!(matches_search("myserver", &profile, "myserver"));
        assert!(matches_search("myserver", &profile, "My"));
        assert!(!matches_search("myserver", &profile, "other"));
    }

    #[test]
    fn matches_search_by_host() {
        let profile = ConnectionProfile {
            tags: vec![],
            local_tags: vec![],
            source: crate::config::ConnectionSource::Manual,
            added_order: 1,
            usage_count: 0,
            kind: ConnectionType::Ssh {
                host: "example.com".into(),
                port: 22,
                user: "admin".into(),
                auth_ref: "a".into(),
                sync: true,
            },
        };
        assert!(matches_search("x", &profile, "example"));
        assert!(matches_search("x", &profile, "admin@example"));
    }

    #[test]
    fn matches_search_empty_query() {
        let profile = ConnectionProfile {
            tags: vec![],
            local_tags: vec![],
            source: crate::config::ConnectionSource::Manual,
            added_order: 1,
            usage_count: 0,
            kind: ConnectionType::Ssh {
                host: "h".into(),
                port: 22,
                user: "u".into(),
                auth_ref: "a".into(),
                sync: true,
            },
        };
        assert!(matches_search("anything", &profile, ""));
    }

    #[test]
    fn shell_name_for_command_path() {
        assert_eq!(shell_name_for_command("/bin/bash"), "bash");
        assert_eq!(shell_name_for_command("/usr/bin/zsh"), "zsh");
    }

    #[test]
    fn shell_name_for_command_empty() {
        assert_eq!(shell_name_for_command(""), "bash");
    }

    #[test]
    fn shell_name_for_command_bare() {
        assert_eq!(shell_name_for_command("bash"), "bash");
    }

    #[test]
    fn shell_name_for_command_windows_path() {
        // Use forward slashes so Path::file_name() works on all platforms
        assert_eq!(
            shell_name_for_command("C:/Windows/System32/cmd.exe"),
            "cmd.exe"
        );
        assert_eq!(
            shell_name_for_command("C:/Program Files/Git/bin/bash.exe"),
            "bash.exe"
        );
    }
}
