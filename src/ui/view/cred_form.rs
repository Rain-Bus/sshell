use crate::app::{App, AuthKind, CredFormField, CredFormState, FormAction, Mode, char_len};
use crate::ui::component::{FormRow, badge_span};
use crate::ui::{GREEN, ORANGE};

use super::{View, handle_form_nav};

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

pub struct CredFormView;

impl View for CredFormView {
    fn title(&self) -> &'static str {
        "Cred Editor"
    }
    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("↑/↓", "move"),
            ("Tab", "toggle"),
            ("Enter", "save"),
            ("Esc", "back"),
            ("Ctrl+U", "clear"),
        ]
    }

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
                let (display, secret_cursor) =
                    if matches!(field, CredFormField::Value) && !raw.is_empty() {
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

        let subtitle = "↓ ▽  ↑ △  Tab toggle  Enter save  Esc back  Ctrl+U clear";
        crate::ui::component::draw_form_list(frame, area, title, subtitle, rows);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_cred_form(app, key)
    }
}

// ── Key handling ───────────────────────────────────────────────

fn handle_cred_form(app: &mut App, key: KeyEvent) -> Result<()> {
    if let Some(action) = handle_form_nav(&mut app.session.credentials.form, key) {
        match action {
            FormAction::Toggle => toggle_cred_field(&mut app.session.credentials.form),
            FormAction::Save => match app.save_cred_form() {
                Ok(()) => app.toast("saved", true),
                Err(err) => app.toast(err.to_string(), false),
            },
            FormAction::Cancel => app.session.mode = Mode::Credentials,
        }
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
