use super::{ConnectionType, CredentialEntry};
use std::fs;

impl super::SshellConfig {
    pub(super) fn migrate_path_to_embedded(&mut self) {
        for entry in self.credentials.entries.values_mut() {
            let CredentialEntry::PrivateKey { value, path, .. } = entry else {
                continue;
            };
            if (value.is_none() || value.as_deref().is_some_and(|v| v.is_empty()))
                && let Some(p) = path.take()
            {
                let expanded = super::expand_user_path(&p);
                if let Ok(content) = fs::read_to_string(&expanded) {
                    *value = Some(content);
                }
            }
            *path = None;
        }
    }

    pub(super) fn migrate_shell_prefix(&mut self) {
        let keys: Vec<String> = self
            .connections
            .iter()
            .filter(|(key, profile)| {
                matches!(&profile.kind, ConnectionType::Shell { .. }) && !key.starts_with('$')
            })
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys {
            self.connections.shift_remove(&key);
        }
    }
}
