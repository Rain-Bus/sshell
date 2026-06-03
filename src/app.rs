mod home_ops;
pub mod latency;
mod profile_ext;
mod types;

pub mod cred;
pub mod form;
pub mod settings;

pub use cred::{CredFormField, CredFormState};
pub use form::{FormField, FormState};
pub use profile_ext::{display_name, split_args};
pub use settings::{SettingsField, SettingsState};
pub use types::*;

use crate::config::{ConnectionProfile, ConnectionType, SshellConfig};
use anyhow::Result;
use std::cmp::Reverse;

pub struct App {
    pub config: SshellConfig,
    pub session: Session,
}

impl App {
    pub fn load() -> Result<Self> {
        let config = SshellConfig::load()?;
        let app = Self {
            config,
            session: Session::new(),
        };
        Ok(app)
    }

    pub fn entries(&self) -> Vec<(&String, &ConnectionProfile)> {
        let mut ssh_entries: Vec<_> = self
            .config
            .connections
            .iter()
            .filter(|(name, profile)| {
                profile_ext::matches_search(name, profile, &self.session.home.search)
            })
            .filter(|(_, profile)| matches!(profile.kind, ConnectionType::Ssh { .. }))
            .collect();
        let shell_entries: Vec<_> = self
            .config
            .connections
            .iter()
            .filter(|(name, profile)| {
                profile_ext::matches_search(name, profile, &self.session.home.search)
            })
            .filter(|(_, profile)| matches!(profile.kind, ConnectionType::Shell { .. }))
            .collect();
        ssh_entries.extend(shell_entries);
        ssh_entries
    }

    pub fn quick_entries(&self) -> Vec<(&String, &ConnectionProfile)> {
        let mut entries = self.entries();
        match self.session.home.quick_sort {
            QuickSortMode::Usage => entries.sort_by(|a, b| {
                b.1.usage_count
                    .cmp(&a.1.usage_count)
                    .then_with(|| a.1.added_order.cmp(&b.1.added_order))
            }),
            QuickSortMode::Added => entries.sort_by_key(|entry| Reverse(entry.1.added_order)),
            QuickSortMode::Name => entries.sort_by(|a, b| {
                a.0.to_lowercase()
                    .cmp(&b.0.to_lowercase())
                    .then_with(|| a.1.added_order.cmp(&b.1.added_order))
            }),
            QuickSortMode::Smart => entries.sort_by(|a, b| {
                profile_ext::smart_score(b.1)
                    .cmp(&profile_ext::smart_score(a.1))
                    .then_with(|| b.1.usage_count.cmp(&a.1.usage_count))
                    .then_with(|| a.1.added_order.cmp(&b.1.added_order))
            }),
        }
        entries
    }

    pub fn selected_name(&self) -> Option<String> {
        self.entries()
            .get(self.session.home.selected)
            .map(|(name, _)| (*name).clone())
    }

    pub fn move_selection(&mut self, delta: isize) {
        let len = match self.session.mode {
            Mode::ImportSelector => {
                self.session.import.shell_candidates.len() + self.session.import.candidates.len()
            }
            Mode::Credentials => self.config.credentials.entries.len(),
            _ => self.entries().len(),
        };
        let selected = match self.session.mode {
            Mode::ImportSelector => &mut self.session.import.cursor,
            Mode::Credentials => &mut self.session.credentials.selected,
            _ => &mut self.session.home.selected,
        };
        if len == 0 {
            *selected = 0;
            return;
        }
        *selected = (*selected as isize + delta).rem_euclid(len as isize) as usize;
    }

    pub fn jump_group(&mut self) {
        let entries = self.entries();
        let Some((_, current)) = entries.get(self.session.home.selected) else {
            return;
        };
        let target_is_shell = matches!(current.kind, ConnectionType::Ssh { .. });
        if let Some(idx) = entries.iter().position(|(_, profile)| {
            matches!(
                (&profile.kind, target_is_shell),
                (ConnectionType::Shell { .. }, true) | (ConnectionType::Ssh { .. }, false)
            )
        }) {
            self.session.home.selected = idx;
        }
    }

    pub fn record_use(&mut self, name: &str) -> Result<()> {
        if let Some(profile) = self.config.connections.get_mut(name) {
            profile.usage_count = profile.usage_count.saturating_add(1);
            self.config.save()?;
        }
        Ok(())
    }

    pub fn toast(&mut self, message: impl Into<String>, success: bool) {
        self.session.toast = Some(Toast {
            message: message.into(),
            success,
            born: std::time::Instant::now(),
        });
    }
}
