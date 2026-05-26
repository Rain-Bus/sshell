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
