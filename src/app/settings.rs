use super::{FormNav, TextEditing};
use crate::config::SyncBackend;

/// Trim whitespace and return None if the result is empty.
fn trimmed_opt(s: &str) -> Option<String> {
    let s = s.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    SyncPassword,
    Backend,
    SyncOnStart,
    GistId,
    WebdavUrl,
    WebdavUser,
    WebdavPassword,
}

impl SettingsField {
    pub fn visible_fields(backend: SyncBackend) -> Vec<SettingsField> {
        let mut fields = vec![SettingsField::SyncPassword, SettingsField::Backend, SettingsField::SyncOnStart];
        match backend {
            SyncBackend::Gist => fields.push(SettingsField::GistId),
            SyncBackend::Webdav => {
                fields.extend_from_slice(&[
                    SettingsField::WebdavUrl,
                    SettingsField::WebdavUser,
                    SettingsField::WebdavPassword,
                ]);
            }
        }
        fields
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SyncPassword => "Encrypt Pwd",
            Self::Backend => "Backend",
            Self::SyncOnStart => "Auto Sync",
            Self::GistId => "Gist ID",
            Self::WebdavUrl => "URL",
            Self::WebdavUser => "Username",
            Self::WebdavPassword => "Password",
            }
    }

    pub fn is_toggle(self) -> bool {
        matches!(self, Self::Backend | Self::SyncOnStart)
    }

    pub fn is_text(self) -> bool {
        !self.is_toggle()
    }
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    pub password: String,
    pub backend: SyncBackend,
    pub sync_on_start: bool,
    pub gist_id: String,
    pub webdav_url: String,
    pub webdav_user: String,
    pub webdav_password: String,
    pub active: SettingsField,
    pub cursor: usize,
}

impl SettingsState {
    pub fn field_text(&self, field: SettingsField) -> &str {
        match field {
            SettingsField::SyncPassword => &self.password,
            SettingsField::GistId => &self.gist_id,
            SettingsField::WebdavUrl => &self.webdav_url,
            SettingsField::WebdavUser => &self.webdav_user,
            SettingsField::WebdavPassword => &self.webdav_password,
            _ => "",
        }
    }

    pub fn visible_fields(&self) -> Vec<SettingsField> {
        SettingsField::visible_fields(self.backend)
    }

    pub fn next_field(&mut self) {
        let fields = self.visible_fields();
        if let Some(idx) = fields.iter().position(|&f| f == self.active) {
            self.active = fields[(idx + 1) % fields.len()];
        }
        self.cursor = self.active_text().chars().count();
    }

    pub fn prev_field(&mut self) {
        let fields = self.visible_fields();
        if let Some(idx) = fields.iter().position(|&f| f == self.active) {
            self.active = fields[(idx + fields.len() - 1) % fields.len()];
        }
        self.cursor = self.active_text().chars().count();
    }

    pub fn ensure_active_visible(&mut self) {
        let fields = self.visible_fields();
        if !fields.contains(&self.active) {
            self.active = fields[0];
            self.cursor = self.active_text().chars().count();
        }
    }
}

impl FormNav for SettingsState {
    fn nav_next(&mut self) { self.next_field(); }
    fn nav_prev(&mut self) { self.prev_field(); }
    fn active_is_toggle(&self) -> bool { self.active.is_toggle() }
    fn active_is_text(&self) -> bool { self.active.is_text() }
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            password: String::new(),
            backend: SyncBackend::Gist,
            sync_on_start: false,
            gist_id: String::new(),
            webdav_url: String::new(),
            webdav_user: String::new(),
            webdav_password: String::new(),
            active: SettingsField::SyncPassword,
            cursor: 0,
        }
    }
}

impl TextEditing for SettingsState {
    fn active_text(&self) -> &str {
        self.field_text(self.active)
    }

    fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.active {
            SettingsField::SyncPassword => Some(&mut self.password),
            SettingsField::GistId => Some(&mut self.gist_id),
            SettingsField::WebdavUrl => Some(&mut self.webdav_url),
            SettingsField::WebdavUser => Some(&mut self.webdav_user),
            SettingsField::WebdavPassword => Some(&mut self.webdav_password),
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

use anyhow::Result;

use super::{App, Mode, char_len};

impl App {
    pub fn enter_settings(&mut self) {
        self.session.settings.password = self
            .config
            .settings
            .sync_password
            .clone()
            .unwrap_or_default();
        self.session.settings.backend = self.config.settings.backend;
        self.session.settings.sync_on_start = self.config.settings.sync_on_start;
        self.session.settings.gist_id = self.config.settings.gist_id.clone().unwrap_or_default();
        self.session.settings.webdav_url =
            self.config.settings.webdav_url.clone().unwrap_or_default();
        self.session.settings.webdav_user =
            self.config.settings.webdav_user.clone().unwrap_or_default();
        self.session.settings.webdav_password =
            self.config.settings.webdav_password.clone().unwrap_or_default();
        self.session.settings.active = SettingsField::SyncPassword;
        self.session.settings.cursor = char_len(self.session.settings.active_text());
        self.session.mode = Mode::Settings;
    }

    pub fn save_settings(&mut self) -> Result<()> {
        self.config.settings.sync_password = trimmed_opt(&self.session.settings.password);
        self.config.settings.backend = self.session.settings.backend;
        self.config.settings.sync_on_start = self.session.settings.sync_on_start;
        self.config.settings.gist_id = trimmed_opt(&self.session.settings.gist_id);
        self.config.settings.webdav_url = trimmed_opt(&self.session.settings.webdav_url);
        self.config.settings.webdav_user = trimmed_opt(&self.session.settings.webdav_user);
        self.config.settings.webdav_password = trimmed_opt(&self.session.settings.webdav_password);
        self.config.save()?;
        self.session.mode = Mode::Home;
        Ok(())
    }
}
