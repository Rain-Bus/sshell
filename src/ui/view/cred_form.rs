use crate::app::{App, AuthKind, CredFormField, CredFormState, Mode, TextEditing, char_len};
use crate::ui::component::{FormRow, badge_span};
use crate::ui::{GREEN, ORANGE};

use super::View;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, layout::Rect};

pub struct CredFormView;

impl View for CredFormView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        let form = &app.session.credentials.form;
        let is_new = form.edit_name.is_none();
        let title = if is_new {
            "New Credential"
        } else {
            "Edit Credential"
        };

        let mut rows: Vec<FormRow> = Vec::new();

        for (i, &field) in CredFormField::ALL.iter().enumerate() {
            let active = form.active == field;

            if i == 1 {
                rows.push(FormRow::Separator);
            }

            let label = field.label().to_string();

            if field.is_toggle() {
                let badge = match field {
                    CredFormField::Kind => match form.kind {
                        AuthKind::Password => badge_span("Password", GREEN),
                        AuthKind::PrivateKey => badge_span("Private Key", ORANGE),
                    },
                    _ => unreachable!(),
                };
                rows.push(FormRow::Toggle {
                    label,
                    active,
                    badge,
                });
            } else {
                let raw = form.field_value(field).to_string();
                let (display, secret_cursor) = if matches!(field, CredFormField::Value) && !raw.is_empty() {
                    if active && matches!(form.kind, AuthKind::Password) {
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

        let subtitle = "Tab ▽  ↑ △  Enter save/toggle  Esc back  Ctrl+U clear";
        crate::ui::component::draw_form_list(frame, area, title, subtitle, rows);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_cred_form(app, key)
    }
}

// ── Key handling ───────────────────────────────────────────────

fn handle_cred_form(app: &mut App, key: KeyEvent) -> Result<()> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Esc => {
            app.session.mode = Mode::Credentials;
        }

        KeyCode::Tab | KeyCode::Down => {
            app.session.credentials.form.next_field();
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.session.credentials.form.prev_field();
        }

        KeyCode::Enter => {
            if app.session.credentials.form.active.is_toggle() {
                toggle_cred_field(&mut app.session.credentials.form);
            } else {
                match app.save_cred_form() {
                    Ok(()) => app.toast("saved", true),
                    Err(err) => app.toast(err.to_string(), false),
                }
            }
        }

        KeyCode::Backspace => {
            app.session.credentials.form.delete_char();
        }
        KeyCode::Left => app.session.credentials.form.move_cursor_left(),
        KeyCode::Right => app.session.credentials.form.move_cursor_right(),
        KeyCode::Home => app.session.credentials.form.cursor_home(),
        KeyCode::End => app.session.credentials.form.cursor_end(),
        KeyCode::Char('a') if ctrl => app.session.credentials.form.cursor_home(),
        KeyCode::Char('e') if ctrl => app.session.credentials.form.cursor_end(),
        KeyCode::Char('u') if ctrl => app.session.credentials.form.clear_field(),

        KeyCode::Char(' ') => {
            if app.session.credentials.form.active.is_toggle() {
                toggle_cred_field(&mut app.session.credentials.form);
            } else {
                app.session.credentials.form.insert_char(' ');
            }
        }

        KeyCode::Char(c) if !ctrl && !app.session.credentials.form.active.is_toggle() => {
            app.session.credentials.form.insert_char(c);
        }

        _ => {}
    }
    Ok(())
}

fn toggle_cred_field(form: &mut CredFormState) {
    if form.active == CredFormField::Kind {
        form.kind = match form.kind {
            AuthKind::Password => AuthKind::PrivateKey,
            AuthKind::PrivateKey => AuthKind::Password,
        };
    }
}
