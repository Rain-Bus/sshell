use super::{TextEditing, char_len};
use crate::app::AuthKind;
use crate::config::{ConnectionProfile, ConnectionType, CredentialEntry, SshellConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Name,
    Type,
    Host,
    Port,
    User,
    Command,
    SyncArgs,
    LocalArgs,
    Auth,
    CredId,
    Secret,
    Tags,
    Sync,
}

impl FormField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Type => "Type",
            Self::Host => "Host",
            Self::Port => "Port",
            Self::User => "User",
            Self::Command => "Command",
            Self::SyncArgs => "Sync Args",
            Self::LocalArgs => "Local Args",
            Self::Auth => "Auth",
            Self::CredId => "Cred ID",
            Self::Secret => "Secret",
            Self::Tags => "Tags",
            Self::Sync => "Sync",
        }
    }

    pub fn is_toggle(self) -> bool {
        matches!(self, Self::Type | Self::Auth | Self::Sync)
    }

    pub fn is_text(self) -> bool {
        matches!(
            self,
            Self::Name
                | Self::Host
                | Self::Port
                | Self::User
                | Self::Command
                | Self::SyncArgs
                | Self::LocalArgs
                | Self::CredId
                | Self::Secret
                | Self::Tags
        )
    }
}

#[derive(Debug, Clone)]
pub struct FormState {
    pub edit_name: Option<String>,
    pub active: FormField,
    pub cursor: usize,
    pub name: String,
    pub is_shell: bool,
    pub host: String,
    pub port: String,
    pub user: String,
    pub auth_kind: AuthKind,
    pub auth_ref: String,
    pub secret: String,
    pub sync: bool,
    pub command: String,
    pub sync_args: String,
    pub local_args: String,
    pub tags: String,
}

impl FormState {
    pub fn blank() -> Self {
        Self {
            edit_name: None,
            active: FormField::Name,
            cursor: 0,
            name: String::new(),
            is_shell: false,
            host: String::new(),
            port: "22".to_string(),
            user: whoami::username(),
            auth_kind: AuthKind::Password,
            auth_ref: String::new(),
            secret: String::new(),
            sync: true,
            command: "bash".to_string(),
            sync_args: String::new(),
            local_args: String::new(),
            tags: String::new(),
        }
    }

    pub fn from_profile(name: &str, profile: &ConnectionProfile, cfg: &SshellConfig) -> Self {
        let mut form = Self::blank();
        form.edit_name = Some(name.to_string());
        form.name = name.strip_prefix('$').unwrap_or(name).to_string();
        form.tags = profile.tags.join(", ");
        match &profile.kind {
            ConnectionType::Ssh {
                host,
                port,
                user,
                auth_ref,
                sync,
            } => {
                form.host = host.clone();
                form.port = port.to_string();
                form.user = user.clone();
                form.auth_ref = auth_ref.clone();
                form.sync = *sync;
            }
            ConnectionType::Shell {
                shell_name: _,
                auth_ref,
                command,
                sync_args,
                local_args,
                sync,
            } => {
                form.is_shell = true;
                form.auth_ref = auth_ref.clone().unwrap_or_default();
                form.command = command.clone();
                form.sync_args = sync_args.join(" ");
                form.local_args = local_args.join(" ");
                form.sync = *sync;
            }
        }
        if let Some(auth_ref) = profile.auth_ref()
            && let Some(credential) = cfg.credential(auth_ref)
        {
            form.auth_kind = match credential {
                CredentialEntry::Password { .. } => AuthKind::Password,
                CredentialEntry::PrivateKey { .. } => AuthKind::PrivateKey,
            };
        }
        form.cursor = char_len(form.active_text());
        form
    }

    pub fn visible_fields(&self) -> Vec<FormField> {
        let mut fields = vec![FormField::Name, FormField::Type];
        if self.is_shell {
            fields.extend_from_slice(&[
                FormField::Command,
                FormField::SyncArgs,
                FormField::LocalArgs,
            ]);
            fields.extend_from_slice(&[FormField::Tags, FormField::Sync]);
        } else {
            fields.extend_from_slice(&[FormField::Host, FormField::Port, FormField::User]);
            fields.extend_from_slice(&[
                FormField::Auth,
                FormField::CredId,
                FormField::Secret,
                FormField::Tags,
                FormField::Sync,
            ]);
        }
        fields
    }

    pub fn next_field(&mut self) {
        let fields = self.visible_fields();
        if let Some(idx) = fields.iter().position(|&f| f == self.active) {
            self.active = fields[(idx + 1) % fields.len()];
        }
        self.cursor = char_len(self.active_text());
    }

    pub fn prev_field(&mut self) {
        let fields = self.visible_fields();
        if let Some(idx) = fields.iter().position(|&f| f == self.active) {
            self.active = fields[(idx + fields.len() - 1) % fields.len()];
        }
        self.cursor = char_len(self.active_text());
    }

    pub fn field_value(&self, field: FormField) -> &str {
        match field {
            FormField::Name => &self.name,
            FormField::Host => &self.host,
            FormField::Port => &self.port,
            FormField::User => &self.user,
            FormField::CredId => &self.auth_ref,
            FormField::Secret => &self.secret,
            FormField::Command => &self.command,
            FormField::SyncArgs => &self.sync_args,
            FormField::LocalArgs => &self.local_args,
            FormField::Tags => &self.tags,
            _ => "",
        }
    }

    pub fn ensure_active_visible(&mut self) {
        let fields = self.visible_fields();
        if !fields.contains(&self.active) {
            self.active = fields[0];
            self.cursor = char_len(self.active_text());
        }
    }
}

impl TextEditing for FormState {
    fn active_text(&self) -> &str {
        match self.active {
            FormField::Name => &self.name,
            FormField::Host => &self.host,
            FormField::Port => &self.port,
            FormField::User => &self.user,
            FormField::CredId => &self.auth_ref,
            FormField::Secret => &self.secret,
            FormField::Command => &self.command,
            FormField::SyncArgs => &self.sync_args,
            FormField::LocalArgs => &self.local_args,
            FormField::Tags => &self.tags,
            _ => "",
        }
    }

    fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.active {
            FormField::Name => Some(&mut self.name),
            FormField::Host => Some(&mut self.host),
            FormField::Port => Some(&mut self.port),
            FormField::User => Some(&mut self.user),
            FormField::CredId => Some(&mut self.auth_ref),
            FormField::Secret => Some(&mut self.secret),
            FormField::Command => Some(&mut self.command),
            FormField::SyncArgs => Some(&mut self.sync_args),
            FormField::LocalArgs => Some(&mut self.local_args),
            FormField::Tags => Some(&mut self.tags),
            _ => None,
        }
    }

    fn cursor(&self) -> usize {
        self.cursor
    }

    fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos;
    }
}

// ── Operations ──────────────────────────────────────────────────

use crate::config::ConnectionSource;
use anyhow::{Result, bail};

use super::profile_ext::{non_empty, resolve_secret, shell_name_for_command, split_args};
use super::{App, Mode};

impl App {
    pub fn new_form(&mut self) {
        self.session.form = FormState::blank();
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
        self.session.form = FormState::from_profile(&name, profile, &self.config);
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
