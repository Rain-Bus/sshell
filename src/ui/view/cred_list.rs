use crate::app::{App, Mode};
use crate::config::CredentialEntry;
use crate::ui::component::panel;
use crate::ui::{BLUE, GREEN, MUTED, RED, SELECTED_BG, TEXT};

use super::{View, scroll_rows};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Cell, Paragraph, Row, Table, Widget},
};

pub struct CredListView;

impl View for CredListView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        draw_credentials(frame, app, area);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_credentials(app, key)
    }
}

// ── Rendering ──────────────────────────────────────────────────

fn draw_credentials(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let entries: Vec<_> = app.config.credentials.entries.iter().collect();
    if entries.is_empty() {
        Paragraph::new("\n  No credentials configured\n\n  Press a to add one")
            .fg(MUTED)
            .alignment(Alignment::Center)
            .block(panel("Credentials"))
            .render(area, frame.buffer_mut());
        return;
    }

    let rows: Vec<Row<'_>> = entries
        .iter()
        .enumerate()
        .map(|(idx, (name, entry))| {
            let selected = idx == app.session.credentials.selected;
            let style = if selected {
                Style::default()
                    .bg(SELECTED_BG)
                    .fg(TEXT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };

            let has_value = entry.has_value();
            let dot_color = if has_value { GREEN } else { RED };

            let kind_text = match entry {
                CredentialEntry::Password { .. } => "password",
                CredentialEntry::PrivateKey { .. } => "private key",
            };

            let refs = app.cred_referenced_by(name);
            let ref_text = if refs.is_empty() {
                "(unused)".to_string()
            } else {
                refs.iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            Row::new([
                Cell::from(format!("{} {}", if selected { ">" } else { " " }, (*name)))
                    .style(style),
                Cell::from(format!("● {kind_text}")).style(if selected {
                    Style::default().fg(dot_color).bg(SELECTED_BG)
                } else {
                    Style::default().fg(dot_color)
                }),
                Cell::from(ref_text).style(style),
            ])
        })
        .collect();

    let rows = scroll_rows(rows, app.session.credentials.selected, area.height);

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(50),
        ],
    )
    .header(
        Row::new(["  Name", "Type", "Used by"])
            .style(Style::default().fg(BLUE).add_modifier(Modifier::BOLD)),
    )
    .block(panel("Credentials"))
    .column_spacing(0)
    .row_highlight_style(Style::default().bg(SELECTED_BG));
    frame.render_widget(table, area);
}

// ── Key handling ───────────────────────────────────────────────

fn handle_credentials(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => app.session.mode = Mode::Home,
        KeyCode::Down | KeyCode::Char('j') => {
            let len = app.config.credentials.entries.len();
            if len > 0 {
                app.session.credentials.selected = (app.session.credentials.selected as isize + 1)
                    .rem_euclid(len as isize)
                    as usize;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let len = app.config.credentials.entries.len();
            if len > 0 {
                app.session.credentials.selected = (app.session.credentials.selected as isize - 1)
                    .rem_euclid(len as isize)
                    as usize;
            }
        }
        KeyCode::Enter => {
            app.edit_cred_form();
        }
        KeyCode::Char('a') => {
            app.new_cred_form();
        }
        KeyCode::Char('d') => match app.delete_cred() {
            Ok(()) => app.toast("deleted", true),
            Err(err) => app.toast(err.to_string(), false),
        },
        _ => {}
    }
    Ok(())
}
