use anyhow::Result;

use super::settings::SettingsField;
use super::{App, Mode, TextEditing, char_len};

impl App {
    pub fn enter_settings(&mut self) {
        self.session.settings.password = self
            .config
            .settings
            .sync_password
            .clone()
            .unwrap_or_default();
        self.session.settings.backend = self.config.settings.backend;
        self.session.settings.gist_id = self.config.settings.gist_id.clone().unwrap_or_default();
        self.session.settings.webdav_url =
            self.config.settings.webdav_url.clone().unwrap_or_default();
        self.session.settings.webdav_user =
            self.config.settings.webdav_user.clone().unwrap_or_default();
        self.session.settings.webdav_password =
            self.config.settings.webdav_password.clone().unwrap_or_default();
        self.session.settings.s3_endpoint =
            self.config.settings.s3_endpoint.clone().unwrap_or_default();
        self.session.settings.s3_bucket =
            self.config.settings.s3_bucket.clone().unwrap_or_default();
        self.session.settings.s3_access_key =
            self.config.settings.s3_access_key.clone().unwrap_or_default();
        self.session.settings.s3_secret_key =
            self.config.settings.s3_secret_key.clone().unwrap_or_default();
        self.session.settings.sync_usage = self.config.settings.sync_usage_count;
        self.session.settings.active = SettingsField::SyncPassword;
        self.session.settings.cursor = char_len(self.session.settings.active_text());
        self.session.mode = Mode::Settings;
    }

    pub fn save_settings(&mut self) -> Result<()> {
        let pw = self.session.settings.password.trim().to_string();
        self.config.settings.sync_password = if pw.is_empty() { None } else { Some(pw) };
        self.config.settings.backend = self.session.settings.backend;
        let gist = self.session.settings.gist_id.trim().to_string();
        self.config.settings.gist_id = if gist.is_empty() { None } else { Some(gist) };
        let url = self.session.settings.webdav_url.trim().to_string();
        self.config.settings.webdav_url = if url.is_empty() { None } else { Some(url) };
        let user = self.session.settings.webdav_user.trim().to_string();
        self.config.settings.webdav_user = if user.is_empty() { None } else { Some(user) };
        let wd_pw = self.session.settings.webdav_password.trim().to_string();
        self.config.settings.webdav_password = if wd_pw.is_empty() {
            None
        } else {
            Some(wd_pw)
        };
        let s3_ep = self.session.settings.s3_endpoint.trim().to_string();
        self.config.settings.s3_endpoint = if s3_ep.is_empty() { None } else { Some(s3_ep) };
        let s3_bk = self.session.settings.s3_bucket.trim().to_string();
        self.config.settings.s3_bucket = if s3_bk.is_empty() { None } else { Some(s3_bk) };
        let s3_ak = self.session.settings.s3_access_key.trim().to_string();
        self.config.settings.s3_access_key = if s3_ak.is_empty() { None } else { Some(s3_ak) };
        let s3_sk = self.session.settings.s3_secret_key.trim().to_string();
        self.config.settings.s3_secret_key = if s3_sk.is_empty() {
            None
        } else {
            Some(s3_sk)
        };
        self.config.settings.sync_usage_count = self.session.settings.sync_usage;
        self.config.save()?;
        self.session.mode = Mode::Home;
        Ok(())
    }
}
