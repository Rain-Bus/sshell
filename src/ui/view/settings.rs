use crate::app::{App, Mode, SettingsField, SettingsState, TextEditing, char_len};
use crate::config::SyncBackend;
use crate::ui::component::{FormRow, badge_span};
use crate::ui::{ACCENT, GREEN, ORANGE, PURPLE, RED};

use super::View;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, layout::Rect};

pub struct SettingsView;

impl View for SettingsView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        let settings = &app.session.settings;
        let fields = settings.visible_fields();
        let mut rows: Vec<FormRow> = Vec::new();

        for (i, &field) in fields.iter().enumerate() {
            let active = settings.active == field;
            let label = field.label().to_string();

            if i == 2 || matches!(field, SettingsField::SyncUsage) {
                rows.push(FormRow::Separator);
            }

            if field.is_toggle() {
                let badge = match field {
                    SettingsField::Backend => match settings.backend {
                        SyncBackend::Gist => badge_span("Gist", ACCENT),
                        SyncBackend::Webdav => badge_span("WebDAV", ORANGE),
                        SyncBackend::S3 => badge_span("S3", PURPLE),
                    },
                    SettingsField::SyncUsage => {
                        if settings.sync_usage {
                            badge_span("Yes", GREEN)
                        } else {
                            badge_span("No", RED)
                        }
                    }
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
                        | SettingsField::S3SecretKey
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

        let subtitle = "Tab ▽  ↑ △  Enter save/toggle  Esc cancel  Ctrl+U clear";
        crate::ui::component::draw_form_list(frame, area, "Settings", subtitle, rows);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_settings(app, key)
    }
}

// ── Key handling ───────────────────────────────────────────────

fn handle_settings(app: &mut App, key: KeyEvent) -> Result<()> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let settings = &mut app.session.settings;

    match key.code {
        KeyCode::Esc => {
            app.session.mode = Mode::Home;
        }

        KeyCode::Tab | KeyCode::Down => {
            settings.next_field();
        }
        KeyCode::BackTab | KeyCode::Up => {
            settings.prev_field();
        }

        KeyCode::Enter => {
            if settings.active.is_toggle() {
                settings_toggle(settings);
            } else {
                match app.save_settings() {
                    Ok(()) => app.toast("settings saved", true),
                    Err(err) => app.toast(err.to_string(), false),
                }
            }
        }

        KeyCode::Backspace if settings.active.is_text() => {
            settings.delete_char();
        }
        KeyCode::Delete if settings.active.is_text() => {
            settings.delete_next_char();
        }
        KeyCode::Left if settings.active.is_text() => {
            settings.move_cursor_left();
        }
        KeyCode::Right if settings.active.is_text() => {
            settings.move_cursor_right();
        }
        KeyCode::Home if settings.active.is_text() => {
            settings.cursor_home();
        }
        KeyCode::End if settings.active.is_text() => {
            settings.cursor_end();
        }
        KeyCode::Char('a') if ctrl && settings.active.is_text() => {
            settings.cursor_home();
        }
        KeyCode::Char('e') if ctrl && settings.active.is_text() => {
            settings.cursor_end();
        }
        KeyCode::Char('u') if ctrl && settings.active.is_text() => {
            settings.clear_field();
        }

        KeyCode::Char(' ') => {
            if settings.active.is_toggle() {
                settings_toggle(settings);
            } else if settings.active.is_text() {
                settings.insert_char(' ');
            }
        }

        KeyCode::Char(c) if !ctrl && settings.active.is_text() => {
            settings.insert_char(c);
        }

        _ => {}
    }
    Ok(())
}

fn settings_toggle(settings: &mut SettingsState) {
    match settings.active {
        SettingsField::Backend => {
            settings.backend = match settings.backend {
                SyncBackend::Gist => SyncBackend::Webdav,
                SyncBackend::Webdav => SyncBackend::S3,
                SyncBackend::S3 => SyncBackend::Gist,
            };
            settings.ensure_active_visible();
        }
        SettingsField::SyncUsage => {
            settings.sync_usage = !settings.sync_usage;
        }
        _ => {}
    }
}
