use super::{FormNav, TextEditing, char_len};
use crate::app::AuthKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredFormField {
    Name,
    Kind,
    Value,
}

impl CredFormField {
    pub const ALL: [CredFormField; 3] = [
        CredFormField::Name,
        CredFormField::Kind,
        CredFormField::Value,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Kind => "Kind",
            Self::Value => "Value",
        }
    }

    pub fn is_toggle(self) -> bool {
        matches!(self, Self::Kind)
    }

    pub fn is_text(self) -> bool {
        matches!(self, Self::Name | Self::Value)
    }
}

#[derive(Debug, Clone)]
pub struct CredFormState {
    pub edit_name: Option<String>,
    pub active: CredFormField,
    pub cursor: usize,
    pub name: String,
    pub kind: AuthKind,
    pub value: String,
}

impl CredFormState {
    pub fn blank() -> Self {
        Self {
            edit_name: None,
            active: CredFormField::Name,
            cursor: 0,
            name: String::new(),
            kind: AuthKind::Password,
            value: String::new(),
        }
    }

    pub fn next_field(&mut self) {
        let idx = CredFormField::ALL
            .iter()
            .position(|&f| f == self.active)
            .unwrap_or(0);
        self.active = CredFormField::ALL[(idx + 1) % CredFormField::ALL.len()];
        self.cursor = char_len(self.active_text());
    }

    pub fn prev_field(&mut self) {
        let idx = CredFormField::ALL
            .iter()
            .position(|&f| f == self.active)
            .unwrap_or(0);
        self.active =
            CredFormField::ALL[(idx + CredFormField::ALL.len() - 1) % CredFormField::ALL.len()];
        self.cursor = char_len(self.active_text());
    }

    pub fn field_value(&self, field: CredFormField) -> &str {
        match field {
            CredFormField::Name => &self.name,
            CredFormField::Value => &self.value,
            _ => "",
        }
    }
}

impl FormNav for CredFormState {
    fn nav_next(&mut self) {
        self.next_field();
    }
    fn nav_prev(&mut self) {
        self.prev_field();
    }
    fn active_is_toggle(&self) -> bool {
        self.active.is_toggle()
    }
    fn active_is_text(&self) -> bool {
        self.active.is_text()
    }
}

impl TextEditing for CredFormState {
    fn active_text(&self) -> &str {
        self.field_value(self.active)
    }

    fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.active {
            CredFormField::Name => Some(&mut self.name),
            CredFormField::Value => Some(&mut self.value),
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

use crate::config::CredentialEntry;
use anyhow::{Result, bail};

use super::{App, Mode};

impl App {
    pub fn cred_entries(&self) -> Vec<(&String, &CredentialEntry)> {
        self.config.credentials.entries.iter().collect()
    }

    pub fn selected_cred_name(&self) -> Option<String> {
        self.cred_entries()
            .get(self.session.credentials.selected)
            .map(|(name, _)| (*name).clone())
    }

    pub fn cred_referenced_by(&self, cred_name: &str) -> Vec<&String> {
        self.config
            .connections
            .iter()
            .filter(|(_, profile)| profile.auth_ref() == Some(cred_name))
            .map(|(name, _)| name)
            .collect()
    }

    pub fn enter_credentials(&mut self) {
        self.session.credentials.selected = 0;
        self.session.mode = Mode::Credentials;
    }

    pub fn new_cred_form(&mut self) {
        self.session.credentials.form = CredFormState::blank();
        self.session.mode = Mode::CredForm;
    }

    pub fn edit_cred_form(&mut self) {
        let Some(name) = self.selected_cred_name() else {
            self.toast("no credential selected", false);
            return;
        };
        let Some(entry) = self.config.credentials.entries.get(&name) else {
            return;
        };
        let mut form = CredFormState::blank();
        form.edit_name = Some(name.clone());
        form.name = name;
        form.kind = match entry {
            CredentialEntry::Password { .. } => AuthKind::Password,
            CredentialEntry::PrivateKey { .. } => AuthKind::PrivateKey,
        };
        form.value = entry.value().to_string();
        form.cursor = char_len(form.active_text());
        self.session.credentials.form = form;
        self.session.mode = Mode::CredForm;
    }

    pub fn save_cred_form(&mut self) -> Result<()> {
        let name = self.session.credentials.form.name.trim().to_string();
        if name.is_empty() {
            bail!("name is required");
        }
        if self.session.credentials.form.edit_name.as_deref() != Some(&name)
            && self.config.credentials.entries.contains_key(&name)
        {
            bail!("credential name already exists");
        }

        let value = self.session.credentials.form.value.clone();
        let entry = match self.session.credentials.form.kind {
            AuthKind::Password => CredentialEntry::password(value),
            AuthKind::PrivateKey => CredentialEntry::private_key(value),
        };

        if let Some(old) = self.session.credentials.form.edit_name.take()
            && old != name
        {
            for profile in self.config.connections.values_mut() {
                if let Some(auth_ref) = profile.auth_ref_mut()
                    && *auth_ref == old
                {
                    *auth_ref = name.clone();
                }
            }
            self.config.credentials.entries.shift_remove(&old);
        }

        self.config.credentials.entries.insert(name, entry);
        self.config.save()?;
        self.session.mode = Mode::Credentials;
        Ok(())
    }

    pub fn delete_cred(&mut self) -> Result<()> {
        let Some(name) = self.selected_cred_name() else {
            bail!("no credential selected");
        };
        let refs = self.cred_referenced_by(&name);
        if !refs.is_empty() {
            let list = refs
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            bail!("still referenced by: {list}");
        }
        self.config.credentials.entries.shift_remove(&name);
        self.config.save()?;
        self.session.credentials.selected = self
            .session
            .credentials
            .selected
            .min(self.config.credentials.entries.len().saturating_sub(1));
        Ok(())
    }
}
