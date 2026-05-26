use crate::app::{App, AuthKind, FormAction, FormField, FormState, Mode, char_len};
use crate::ui::component::{FormRow, badge_span};
use crate::ui::{ACCENT, GREEN, ORANGE, PURPLE, RED};

use super::{View, handle_form_nav};

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

pub struct FormView;

impl View for FormView {
    fn title(&self) -> &'static str { "Editor" }
    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("↑/↓", "move"),
            ("Tab", "toggle"),
            ("Enter", "save"),
            ("Esc", "cancel"),
            ("Ctrl+U", "clear"),
        ]
    }

    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        let form = &app.session.form;
        let fields = form.visible_fields();
        let is_new = form.edit_name.is_none();
        let title = if is_new {
            "New Connection"
        } else {
            "Edit Connection"
        };

        let mut rows: Vec<FormRow> = Vec::new();

        for (i, &field) in fields.iter().enumerate() {
            let active = form.active == field;

            if i == 1 || matches!(field, FormField::Auth) || matches!(field, FormField::Tags) {
                rows.push(FormRow::Separator);
            }

            let label = field.label().to_string();

            if field.is_toggle() {
                let badge = match field {
                    FormField::Type => {
                        if form.is_shell {
                            badge_span("Shell", PURPLE)
                        } else {
                            badge_span("SSH", ACCENT)
                        }
                    }
                    FormField::Auth => match form.auth_kind {
                        AuthKind::Password => badge_span("Password", GREEN),
                        AuthKind::PrivateKey => badge_span("Private Key", ORANGE),
                    },
                    FormField::Sync => {
                        if form.sync {
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
                let raw = form.field_value(field).to_string();
                let (display, secret_cursor) = if matches!(field, FormField::Secret) {
                    if raw.is_empty() {
                        ("<unchanged>".into(), 0)
                    } else if active && matches!(form.auth_kind, AuthKind::Password) {
                        let d: String = "*".repeat(raw.chars().count());
                        (d, form.cursor)
                    } else {
                        ("<set>".into(), 0)
                    }
                } else {
                    (raw, form.cursor)
                };
                let cursor = if active && field.is_text() {
                    secret_cursor.min(char_len(&display))
                } else {
                    char_len(&display)
                };
                rows.push(FormRow::Text {
                    label,
                    active,
                    display,
                    cursor,
                    placeholder: None,
                });
            }
        }

        let subtitle = "↓ ▽  ↑ △  Tab toggle  Enter save  Esc cancel  Ctrl+U clear";
        crate::ui::component::draw_form_list(frame, area, title, subtitle, rows);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_form(app, key)
    }
}

// ── Key handling ───────────────────────────────────────────────

fn handle_form(app: &mut App, key: KeyEvent) -> Result<()> {
    if let Some(action) = handle_form_nav(&mut app.session.form, key) {
        match action {
            FormAction::Toggle => toggle_field(&mut app.session.form),
            FormAction::Save => match app.save_form() {
                Ok(()) => app.toast("saved", true),
                Err(err) => app.toast(err.to_string(), false),
            },
            FormAction::Cancel => app.session.mode = Mode::Home,
        }
    }
    Ok(())
}

fn toggle_field(form: &mut FormState) {
    match form.active {
        FormField::Type => {
            form.is_shell = !form.is_shell;
            if form.is_shell {
                form.auth_ref.clear();
                form.secret.clear();
                form.sync = false;
            } else {
                form.sync = true;
            }
            form.ensure_active_visible();
        }
        FormField::Auth => {
            form.auth_kind = match form.auth_kind {
                AuthKind::Password => AuthKind::PrivateKey,
                AuthKind::PrivateKey => AuthKind::Password,
            };
        }
        FormField::Sync => {
            form.sync = !form.sync;
        }
        _ => {}
    }
}
