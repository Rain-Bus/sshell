use crate::config::{ConnectionProfile, ConnectionSource, ConnectionType, CredentialEntry};
use anyhow::{Result, bail};

use super::form::FormState;
use super::profile_ext::{non_empty, resolve_secret, shell_name_for_command, split_args};
use super::{App, AuthKind, Mode};

impl App {
    pub fn new_form(&mut self) {
        self.session.form = super::form::FormState::blank();
        self.session.mode = Mode::Form;
    }

    pub fn edit_form(&mut self) {
        let Some(name) = self.selected_name() else {
            self.toast("no connection selected", false);
            return;
        };
        let Some(profile) = self.config.connections.get(&name) else {
            return;
        };
        self.session.form = super::form::FormState::from_profile(&name, profile, &self.config);
        self.session.mode = Mode::Form;
    }

    pub fn delete_selected(&mut self) -> Result<()> {
        let Some(name) = self.selected_name() else {
            bail!("no connection selected");
        };
        let removed = self.config.connections.shift_remove(&name);
        if let Some(profile) = removed {
            if let Some(auth_ref) = profile.auth_ref() {
                let still_used = self
                    .config
                    .connections
                    .values()
                    .any(|p| p.auth_ref() == Some(auth_ref));
                if !still_used {
                    self.config.credentials.entries.shift_remove(auth_ref);
                }
            }
            self.config.save()?;
            self.session.home.selected = self
                .session
                .home
                .selected
                .min(self.entries().len().saturating_sub(1));
        }
        Ok(())
    }

    pub fn save_form(&mut self) -> Result<()> {
        let name = self.validate_form_name()?;
        let auth_ref = auth_ref_for_form(&self.session.form, &name);
        let old_auth_ref = self.old_form_auth_ref();
        let profile = self.build_form_profile(&auth_ref)?;

        self.remove_renamed_connection(&name);
        self.save_form_credential(&name, &auth_ref, old_auth_ref);

        self.config.connections.insert(name, profile);
        self.config.save()?;
        self.session.mode = Mode::Home;
        Ok(())
    }

    fn validate_form_name(&self) -> Result<String> {
        let name = self.session.form.name.trim().to_string();
        if name.is_empty() {
            bail!("name is required");
        }
        let name = if self.session.form.is_shell && !name.starts_with('$') {
            format!("${name}")
        } else {
            name
        };
        if self.session.form.edit_name.as_deref() != Some(&name)
            && self.config.connections.contains_key(&name)
        {
            bail!("connection name already exists");
        }
        Ok(name)
    }

    fn old_form_auth_ref(&self) -> Option<String> {
        self.session
            .form
            .edit_name
            .as_ref()
            .and_then(|old| self.config.connections.get(old))
            .and_then(|profile| profile.auth_ref())
            .map(ToString::to_string)
    }

    fn build_form_profile(&self, auth_ref: &str) -> Result<ConnectionProfile> {
        let tags = parse_tags(&self.session.form.tags);
        let local_tags = self.form_local_tags();
        let (source, added_order, usage_count) = self.form_existing_metadata();

        if self.session.form.is_shell {
            Ok(ConnectionProfile {
                tags,
                local_tags,
                source,
                added_order,
                usage_count,
                kind: ConnectionType::Shell {
                    shell_name: shell_name_for_command(&self.session.form.command),
                    auth_ref: None,
                    command: non_empty(&self.session.form.command, "bash"),
                    sync_args: split_args(&self.session.form.sync_args),
                    local_args: split_args(&self.session.form.local_args),
                    sync: self.session.form.sync,
                },
            })
        } else {
            let host = self.session.form.host.trim().to_string();
            let user = self.session.form.user.trim().to_string();
            if host.is_empty() || user.is_empty() {
                bail!("ssh host and user are required");
            }
            let port = self.session.form.port.trim().parse::<u16>().unwrap_or(22);
            Ok(ConnectionProfile {
                tags,
                local_tags,
                source,
                added_order,
                usage_count,
                kind: ConnectionType::Ssh {
                    host,
                    port,
                    user,
                    auth_ref: auth_ref.to_string(),
                    sync: self.session.form.sync,
                },
            })
        }
    }

    fn form_existing_metadata(&self) -> (ConnectionSource, u64, u64) {
        self.session
            .form
            .edit_name
            .as_ref()
            .and_then(|old| self.config.connections.get(old))
            .map(|profile| (profile.source, profile.added_order, profile.usage_count))
            .unwrap_or((ConnectionSource::Manual, self.config.next_added_order(), 0))
    }

    fn form_local_tags(&self) -> Vec<String> {
        self.session
            .form
            .edit_name
            .as_ref()
            .and_then(|old| self.config.connections.get(old))
            .map(|profile| profile.local_tags.clone())
            .unwrap_or_default()
    }

    fn remove_renamed_connection(&mut self, name: &str) {
        if let Some(old) = self.session.form.edit_name.take()
            && old != name
        {
            self.config.connections.shift_remove(&old);
        }
    }

    fn save_form_credential(&mut self, name: &str, auth_ref: &str, old_auth_ref: Option<String>) {
        if self.session.form.is_shell {
            self.remove_unused_old_credential(name, old_auth_ref);
        } else if !self.session.form.secret.is_empty() {
            let secret = resolve_secret(&self.session.form.secret);
            let entry = match self.session.form.auth_kind {
                AuthKind::Password => CredentialEntry::password(secret),
                AuthKind::PrivateKey => CredentialEntry::private_key(secret),
            };
            self.config
                .credentials
                .entries
                .insert(auth_ref.to_string(), entry);
        }
    }

    fn remove_unused_old_credential(&mut self, editing_name: &str, old_auth_ref: Option<String>) {
        let Some(old_auth_ref) = old_auth_ref else {
            return;
        };
        let still_used = self
            .config
            .connections
            .iter()
            .filter(|(conn_name, _)| conn_name.as_str() != editing_name)
            .any(|(_, profile)| profile.auth_ref() == Some(old_auth_ref.as_str()));
        if !still_used {
            self.config.credentials.entries.shift_remove(&old_auth_ref);
        }
    }
}

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn auth_ref_for_form(form: &FormState, name: &str) -> String {
    if !form.auth_ref.trim().is_empty() {
        return form.auth_ref.trim().to_string();
    }

    match form.auth_kind {
        AuthKind::Password => format!("{name}-password"),
        AuthKind::PrivateKey => format!("{name}-key"),
    }
}
