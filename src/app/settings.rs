use super::TextEditing;
use crate::config::SyncBackend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    SyncPassword,
    Backend,
    GistId,
    WebdavUrl,
    WebdavUser,
    WebdavPassword,
    S3Endpoint,
    S3Bucket,
    S3AccessKey,
    S3SecretKey,
    SyncUsage,
}

impl SettingsField {
    pub fn visible_fields(backend: SyncBackend) -> Vec<SettingsField> {
        let mut fields = vec![SettingsField::SyncPassword, SettingsField::Backend];
        match backend {
            SyncBackend::Gist => fields.push(SettingsField::GistId),
            SyncBackend::Webdav => {
                fields.extend_from_slice(&[
                    SettingsField::WebdavUrl,
                    SettingsField::WebdavUser,
                    SettingsField::WebdavPassword,
                ]);
            }
            SyncBackend::S3 => {
                fields.extend_from_slice(&[
                    SettingsField::S3Endpoint,
                    SettingsField::S3Bucket,
                    SettingsField::S3AccessKey,
                    SettingsField::S3SecretKey,
                ]);
            }
        }
        fields.push(SettingsField::SyncUsage);
        fields
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SyncPassword => "Encrypt Pwd",
            Self::Backend => "Backend",
            Self::GistId => "Gist ID",
            Self::WebdavUrl => "URL",
            Self::WebdavUser => "Username",
            Self::WebdavPassword => "Password",
            Self::S3Endpoint => "Endpoint",
            Self::S3Bucket => "Bucket",
            Self::S3AccessKey => "Access Key",
            Self::S3SecretKey => "Secret Key",
            Self::SyncUsage => "Sync Usage",
        }
    }

    pub fn is_toggle(self) -> bool {
        matches!(self, Self::Backend | Self::SyncUsage)
    }

    pub fn is_text(self) -> bool {
        !self.is_toggle()
    }
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    pub password: String,
    pub backend: SyncBackend,
    pub gist_id: String,
    pub webdav_url: String,
    pub webdav_user: String,
    pub webdav_password: String,
    pub s3_endpoint: String,
    pub s3_bucket: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub sync_usage: bool,
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
            SettingsField::S3Endpoint => &self.s3_endpoint,
            SettingsField::S3Bucket => &self.s3_bucket,
            SettingsField::S3AccessKey => &self.s3_access_key,
            SettingsField::S3SecretKey => &self.s3_secret_key,
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

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            password: String::new(),
            backend: SyncBackend::Gist,
            gist_id: String::new(),
            webdav_url: String::new(),
            webdav_user: String::new(),
            webdav_password: String::new(),
            s3_endpoint: String::new(),
            s3_bucket: String::new(),
            s3_access_key: String::new(),
            s3_secret_key: String::new(),
            sync_usage: false,
            active: SettingsField::SyncPassword,
            cursor: 0,
        }
    }
}

impl TextEditing for SettingsState {
    fn active_text(&self) -> &str {
        match self.active {
            SettingsField::SyncPassword => &self.password,
            SettingsField::GistId => &self.gist_id,
            SettingsField::WebdavUrl => &self.webdav_url,
            SettingsField::WebdavUser => &self.webdav_user,
            SettingsField::WebdavPassword => &self.webdav_password,
            SettingsField::S3Endpoint => &self.s3_endpoint,
            SettingsField::S3Bucket => &self.s3_bucket,
            SettingsField::S3AccessKey => &self.s3_access_key,
            SettingsField::S3SecretKey => &self.s3_secret_key,
            _ => "",
        }
    }

    fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.active {
            SettingsField::SyncPassword => Some(&mut self.password),
            SettingsField::GistId => Some(&mut self.gist_id),
            SettingsField::WebdavUrl => Some(&mut self.webdav_url),
            SettingsField::WebdavUser => Some(&mut self.webdav_user),
            SettingsField::WebdavPassword => Some(&mut self.webdav_password),
            SettingsField::S3Endpoint => Some(&mut self.s3_endpoint),
            SettingsField::S3Bucket => Some(&mut self.s3_bucket),
            SettingsField::S3AccessKey => Some(&mut self.s3_access_key),
            SettingsField::S3SecretKey => Some(&mut self.s3_secret_key),
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
