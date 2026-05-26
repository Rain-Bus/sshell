use anyhow::Result;

use crate::config::SyncBackend;
use super::{App, Mode};

impl App {
    pub fn request_quit(&mut self) {
        self.session.should_quit = true;
    }

    pub fn enter_action_menu(&mut self) {
        self.session.action_menu.cursor = 0;
        self.session.mode = Mode::ActionMenu;
    }

    pub fn enter_combined_import(&mut self) -> Result<()> {
        self.session.import.candidates = crate::import::load_candidates(&self.config)?;
        self.session.import.selected = vec![false; self.session.import.candidates.len()];
        self.session.import.shell_candidates = self.config.local_shell_candidates();
        self.session.import.shell_selected = vec![false; self.session.import.shell_candidates.len()];
        self.session.import.cursor = 0;
        self.session.mode = Mode::ImportSelector;
        Ok(())
    }

    pub fn enter_quick_select(&mut self) {
        if self.entries().is_empty() {
            self.toast("no connections available", false);
        } else {
            self.session.mode = Mode::QuickSelect;
            self.session.home.selected = 0;
            self.toast("press 1-9 to connect, Tab to sort, Esc to cancel", true);
        }
    }

    pub fn connect_selected(&mut self) -> Result<()> {
        if let Some(name) = self.selected_name() {
            self.record_use(&name)?;
            crate::connection::connect(&name, &self.config)?;
        }
        Ok(())
    }

    pub fn enter_search(&mut self) {
        self.session.mode = Mode::Search;
        self.session.home.search.clear();
        self.session.home.selected = 0;
    }

    pub fn enter_delete_confirm_for_selected(&mut self) {
        if self.selected_name().is_some() {
            self.session.mode = Mode::DeleteConfirm;
        }
    }

    pub fn enter_import_selector(&mut self) -> Result<()> {
        self.session.import.candidates = crate::import::load_candidates(&self.config)?;
        self.session.import.selected = vec![false; self.session.import.candidates.len()];
        self.session.import.shell_candidates = self.config.local_shell_candidates();
        self.session.import.shell_selected = vec![false; self.session.import.shell_candidates.len()];
        self.session.import.cursor = 0;
        self.session.mode = Mode::ImportSelector;
        Ok(())
    }

    pub fn push_sync_with_toast(&mut self) {
        let result = match self.config.settings.backend {
            SyncBackend::Gist => crate::sync::gist::push(&mut self.config).map(|id| format!("pushed ({id})")),
            SyncBackend::Webdav => crate::sync::webdav::push(&mut self.config).map(|_| "pushed".to_string()),
            SyncBackend::S3 => crate::sync::s3::push(&mut self.config).map(|_| "pushed".to_string()),
        };
        match result {
            Ok(msg) => self.toast(msg, true),
            Err(err) => self.toast(err.to_string(), false),
        }
    }

    pub fn pull_sync_with_toast(&mut self) {
        let result = match self.config.settings.backend {
            SyncBackend::Gist => crate::sync::gist::pull_with_strategy(&mut self.config, crate::sync::PullStrategy::Merge),
            SyncBackend::Webdav => crate::sync::webdav::pull_with_strategy(&mut self.config, crate::sync::PullStrategy::Merge),
            SyncBackend::S3 => crate::sync::s3::pull_with_strategy(&mut self.config, crate::sync::PullStrategy::Merge),
        };
        match result {
            Ok(count) => self.toast(format!("pulled {count} items"), true),
            Err(err) => self.toast(err.to_string(), false),
        }
    }
}
