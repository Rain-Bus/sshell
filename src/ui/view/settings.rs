use crate::app::{App, FormAction, Mode, SettingsField, SettingsState, char_len};
use crate::config::SyncBackend;
use crate::ui::component::{FormRow, badge_span};
use crate::ui::{ACCENT, ORANGE};

use super::{View, handle_form_nav};

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

pub struct SettingsView;

impl View for SettingsView {
    fn title(&self) -> &'static str { "Settings" }
    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("↑/↓", "move"),
            ("Tab", "toggle"),
            ("Enter", "save"),
            ("Esc", "cancel"),
        ]
    }

    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        let settings = &app.session.settings;
        let fields = settings.visible_fields();
        let mut rows: Vec<FormRow> = Vec::new();

        for (i, &field) in fields.iter().enumerate() {
            let active = settings.active == field;
            let label = field.label().to_string();

            if i == 2 {
                rows.push(FormRow::Separator);
            }

            if field.is_toggle() {
                let badge = match field {
                    SettingsField::Backend => match settings.backend {
                        SyncBackend::Gist => badge_span("Gist", ACCENT),
                        SyncBackend::Webdav => badge_span("WebDAV", ORANGE),
                    },
                    _ => unreachable!(),
                };
                rows.push(FormRow::Toggle {
                    label,
                    active,
                    badge,
                });
            } else {
                let raw = settings.field_text(field).to_string();
                let is_secret = matches!(
                    field,
                    SettingsField::SyncPassword
                        | SettingsField::WebdavPassword
                );
                let (display, secret_cursor) = if is_secret {
                    if raw.is_empty() {
                        (String::new(), settings.cursor)
                    } else if active {
                        let d: String = "*".repeat(raw.chars().count());
                        (d, settings.cursor)
                    } else {
                        ("<set>".into(), 0)
                    }
                } else {
                    (raw, settings.cursor)
                };
                let cursor = if active {
                    secret_cursor.min(char_len(&display))
                } else {
                    char_len(&display)
                };
                let placeholder = if is_secret || matches!(field, SettingsField::GistId) {
                    Some("<not set>".to_string())
                } else {
                    None
                };
                rows.push(FormRow::Text {
                    label,
                    active,
                    display,
                    cursor,
                    placeholder,
                });
            }
        }

        let subtitle = "↓ ▽  ↑ △  Tab toggle  Enter save  Esc cancel  Ctrl+U clear";
        crate::ui::component::draw_form_list(frame, area, "Settings", subtitle, rows);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_settings(app, key)
    }
}

// ── Key handling ───────────────────────────────────────────────

fn handle_settings(app: &mut App, key: KeyEvent) -> Result<()> {
    if let Some(action) = handle_form_nav(&mut app.session.settings, key) {
        match action {
            FormAction::Toggle => settings_toggle(&mut app.session.settings),
            FormAction::Save => match app.save_settings() {
                Ok(()) => app.toast("settings saved", true),
                Err(err) => app.toast(err.to_string(), false),
            },
            FormAction::Cancel => app.session.mode = Mode::Home,
        }
    }
    Ok(())
}

fn settings_toggle(settings: &mut SettingsState) {
    match settings.active {
        SettingsField::Backend => {
            settings.backend = match settings.backend {
                SyncBackend::Gist => SyncBackend::Webdav,
                SyncBackend::Webdav => SyncBackend::Gist,
            };
            settings.ensure_active_visible();
        }
        _ => {}
    }
}
