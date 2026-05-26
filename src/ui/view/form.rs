use crate::app::{App, AuthKind, FormField, FormState, Mode, TextEditing, char_len};
use crate::ui::component::{FormRow, badge_span};
use crate::ui::{ACCENT, GREEN, ORANGE, PURPLE, RED};

use super::View;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, layout::Rect};

pub struct FormView;

impl View for FormView {
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

        let subtitle = "Tab ▽  ↑ △  Enter save/toggle  Esc cancel  Ctrl+U clear";
        crate::ui::component::draw_form_list(frame, area, title, subtitle, rows);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_form(app, key)
    }
}

// ── Key handling ───────────────────────────────────────────────

fn handle_form(app: &mut App, key: KeyEvent) -> Result<()> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Esc => app.session.mode = Mode::Home,

        KeyCode::Tab | KeyCode::Down => {
            app.session.form.next_field();
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.session.form.prev_field();
        }

        KeyCode::Enter => {
            if app.session.form.active.is_toggle() {
                toggle_field(&mut app.session.form);
            } else {
                match app.save_form() {
                    Ok(()) => app.toast("saved", true),
                    Err(err) => app.toast(err.to_string(), false),
                }
            }
        }

        KeyCode::Backspace => {
            app.session.form.delete_char();
        }
        KeyCode::Delete => {
            app.session.form.delete_next_char();
        }
        KeyCode::Left => {
            app.session.form.move_cursor_left();
        }
        KeyCode::Right => {
            app.session.form.move_cursor_right();
        }
        KeyCode::Home => app.session.form.cursor_home(),
        KeyCode::End => app.session.form.cursor_end(),

        KeyCode::Char('a') if ctrl => app.session.form.cursor_home(),
        KeyCode::Char('e') if ctrl => app.session.form.cursor_end(),
        KeyCode::Char('u') if ctrl => app.session.form.clear_field(),

        KeyCode::Char(' ') => {
            if app.session.form.active.is_toggle() {
                toggle_field(&mut app.session.form);
            } else {
                app.session.form.insert_char(' ');
            }
        }

        KeyCode::Char(c) if !ctrl && !app.session.form.active.is_toggle() => {
            app.session.form.insert_char(c);
        }

        _ => {}
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
