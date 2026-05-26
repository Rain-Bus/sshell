use crate::app::App;
use crate::ui::component::{panel, panel_with_subtitle};
use crate::ui::{BLUE, GREEN, MUTED, SELECTED_BG, TEXT};

use super::{View, scroll_rows};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Cell, Paragraph, Row, Table, Widget},
};

pub struct ImportView;
pub struct ShellImportView;

impl View for ImportView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        draw_import(frame, app, area);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_import(app, key)
    }
}

impl View for ShellImportView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        draw_shell_import(frame, app, area);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_shell_import(app, key)
    }
}

// ── Rendering ──────────────────────────────────────────────────

fn draw_import(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if app.session.import.candidates.is_empty() {
        Paragraph::new("\n  No importable hosts found in ~/.ssh/config")
            .fg(MUTED)
            .alignment(Alignment::Center)
            .block(panel("SSH Config Import"))
            .render(area, frame.buffer_mut());
        return;
    }
    let rows: Vec<Row<'_>> = app
        .session
        .import
        .candidates
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let selected_row = idx == app.session.import.cursor;
            let checked = app
                .session
                .import
                .selected
                .get(idx)
                .copied()
                .unwrap_or(false);
            let style = if selected_row {
                Style::default()
                    .bg(SELECTED_BG)
                    .fg(TEXT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };
            let check_style = if checked {
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MUTED)
            };
            Row::new([
                Cell::from(if checked { "  [x]" } else { "  [ ]" }).style(check_style),
                Cell::from(item.name.clone()).style(style),
                Cell::from(format!("{}@{}:{}", item.user, item.host, item.port)).style(style),
                Cell::from(
                    item.identity_file
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "-".to_string()),
                )
                .style(style),
            ])
        })
        .collect();

    let rows = scroll_rows(rows, app.session.import.cursor, area.height);

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(24),
            Constraint::Percentage(35),
            Constraint::Percentage(45),
        ],
    )
    .header(
        Row::new(["  Use", "Name", "Target", "Identity File"])
            .style(Style::default().fg(BLUE).bold()),
    )
    .block(panel_with_subtitle(
        "SSH Config Import",
        "Space toggle  a all  A none  Enter import  Esc cancel",
    ))
    .column_spacing(2);
    frame.render_widget(table, area);
}

fn draw_shell_import(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if app.session.shell_import.candidates.is_empty() {
        Paragraph::new("\n  No new local shells found\n\n  Press Esc to return home")
            .fg(MUTED)
            .alignment(Alignment::Center)
            .block(panel("Detected Shells"))
            .render(area, frame.buffer_mut());
        return;
    }

    let rows: Vec<Row<'_>> = app
        .session
        .shell_import
        .candidates
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let selected_row = idx == app.session.shell_import.cursor;
            let checked = app
                .session
                .shell_import
                .selected
                .get(idx)
                .copied()
                .unwrap_or(false);
            let has_conflict = item.conflict.is_some();
            let style = if selected_row {
                Style::default()
                    .bg(SELECTED_BG)
                    .fg(TEXT)
                    .add_modifier(Modifier::BOLD)
            } else if has_conflict {
                Style::default().fg(MUTED)
            } else {
                Style::default().fg(TEXT)
            };
            let check_style = if checked {
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MUTED)
            };
            let status = item
                .conflict
                .as_ref()
                .map(|conflict| format!("conflict: {}", conflict.name))
                .unwrap_or_else(|| "ready".to_string());
            Row::new([
                Cell::from(if checked { "  [x]" } else { "  [ ]" }).style(check_style),
                Cell::from(item.name.clone()).style(style),
                Cell::from(item.path.display().to_string()).style(style),
                Cell::from(status).style(style),
            ])
        })
        .collect();

    let rows = scroll_rows(rows, app.session.shell_import.cursor, area.height);

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(20),
            Constraint::Percentage(50),
            Constraint::Percentage(30),
        ],
    )
    .header(Row::new(["  Use", "Name", "Path", "Status"]).style(Style::default().fg(BLUE).bold()))
    .block(panel_with_subtitle(
        "Detected Shells",
        "Space toggle  a all  A none  Enter enable  Esc skip",
    ))
    .column_spacing(2);
    frame.render_widget(table, area);
}

// ── Key handling ───────────────────────────────────────────────

fn handle_import(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => app.session.mode = crate::app::Mode::Home,
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Char(' ') => {
            if let Some(v) = app
                .session
                .import
                .selected
                .get_mut(app.session.import.cursor)
            {
                *v = !*v;
            }
        }
        KeyCode::Char('a') => app.session.import.selected.fill(true),
        KeyCode::Char('A') => app.session.import.selected.fill(false),
        KeyCode::Enter => {
            let picked: Vec<_> = app
                .session
                .import
                .candidates
                .iter()
                .zip(&app.session.import.selected)
                .filter_map(|(item, selected)| selected.then_some(item.clone()))
                .collect();
            match crate::import::import_candidates(&mut app.config, &picked) {
                Ok(count) => app.toast(format!("imported {count} connections"), true),
                Err(err) => app.toast(err.to_string(), false),
            }
            app.session.mode = crate::app::Mode::Home;
        }
        _ => {}
    }
    Ok(())
}

fn handle_shell_import(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => app.session.mode = crate::app::Mode::Home,
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Char(' ') => {
            let can_select = app
                .session
                .shell_import
                .candidates
                .get(app.session.shell_import.cursor)
                .is_some_and(|candidate| candidate.conflict.is_none());
            if can_select
                && let Some(v) = app
                    .session
                    .shell_import
                    .selected
                    .get_mut(app.session.shell_import.cursor)
            {
                *v = !*v;
            }
        }
        KeyCode::Char('a') => {
            for (selected, candidate) in app
                .session
                .shell_import
                .selected
                .iter_mut()
                .zip(&app.session.shell_import.candidates)
            {
                *selected = candidate.conflict.is_none();
            }
        }
        KeyCode::Char('A') => app.session.shell_import.selected.fill(false),
        KeyCode::Enter => {
            match app.import_selected_shells() {
                Ok(count) => app.toast(format!("enabled {count} shells"), true),
                Err(err) => app.toast(err.to_string(), false),
            }
            app.session.mode = crate::app::Mode::Home;
        }
        _ => {}
    }
    Ok(())
}
