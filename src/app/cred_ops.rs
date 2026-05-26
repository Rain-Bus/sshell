use crate::config::CredentialEntry;
use anyhow::{Result, bail};

use super::{App, AuthKind, Mode, TextEditing, char_len};

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
        self.session.credentials.form = super::cred::CredFormState::blank();
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
        let mut form = super::cred::CredFormState::blank();
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
