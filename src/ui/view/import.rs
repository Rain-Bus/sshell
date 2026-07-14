use crate::app::App;
use crate::ui::component::{ListAction, handle_list_nav, panel, panel_with_subtitle};
use crate::ui::{BLUE, GREEN, MUTED, SELECTED_BG, TEXT};

use super::View;
use super::{scroll_indexed_rows, section_row};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Cell, Paragraph, Row, Table, Widget},
};

pub struct ImportView;

impl View for ImportView {
    fn title(&self) -> &'static str {
        "Import"
    }
    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("j/k", "move"),
            ("Space", "toggle"),
            ("a/A", "all/none"),
            ("Enter", "import"),
            ("Esc", "cancel"),
        ]
    }

    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        draw_import(frame, app, area);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_import(app, key)
    }
}

// ── Rendering ──────────────────────────────────────────────────

fn draw_import(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let shell_len = app.session.import.shell_candidates.len();
    let ssh_len = app.session.import.candidates.len();
    let total = shell_len + ssh_len;

    if total == 0 {
        Paragraph::new("\n  No importable hosts or shells found")
            .fg(MUTED)
            .alignment(Alignment::Center)
            .block(panel("Import"))
            .render(area, frame.buffer_mut());
        return;
    }

    let mut rows: Vec<Row<'_>> = Vec::new();
    let mut entry_row: Vec<usize> = Vec::new(); // item_idx -> visual row

    if shell_len > 0 {
        rows.push(section_row("Shell", shell_len));
    }
    for (idx, item) in app.session.import.shell_candidates.iter().enumerate() {
        entry_row.push(rows.len());
        let selected_row = idx == app.session.import.cursor;
        let checked = app
            .session
            .import
            .shell_selected
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
            .map(|c| format!("conflict: {}", c.name))
            .unwrap_or_else(|| "-".to_string());
        let check_cell_style = if selected_row {
            Style::default().bg(SELECTED_BG)
        } else {
            Style::default()
        };
        rows.push(Row::new([
            Cell::from(if checked { "  [x]" } else { "  [ ]" })
                .style(check_style.patch(check_cell_style)),
            Cell::from(item.name.clone()).style(style),
            Cell::from(item.path.display().to_string()).style(style),
            Cell::from(status).style(style),
        ]));
    }

    if shell_len > 0 && ssh_len > 0 {
        rows.push(Row::new(["", "", "", ""]).height(1));
    }

    if ssh_len > 0 {
        rows.push(section_row("SSH", ssh_len));
    }
    for (idx, item) in app.session.import.candidates.iter().enumerate() {
        entry_row.push(rows.len());
        let selected_row = (shell_len + idx) == app.session.import.cursor;
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
        let check_cell_style = if selected_row {
            Style::default().bg(SELECTED_BG)
        } else {
            Style::default()
        };
        rows.push(Row::new([
            Cell::from(if checked { "  [x]" } else { "  [ ]" })
                .style(check_style.patch(check_cell_style)),
            Cell::from(item.name.clone()).style(style),
            Cell::from(format!("{}@{}:{}", item.user, item.host, item.port)).style(style),
            Cell::from(
                item.identity_file
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "-".to_string()),
            )
            .style(style),
        ]));
    }

    // Scroll to keep selected entry visible
    rows = scroll_indexed_rows(rows, &entry_row, app.session.import.cursor, area.height);

    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(22),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ],
    )
    .header(
        Row::new(["  Use", "Name", "Target", "Identity / Status"])
            .style(Style::default().fg(BLUE).add_modifier(Modifier::BOLD)),
    )
    .block(panel_with_subtitle(
        "Import",
        "Space toggle  a all  A none  Enter import  Esc cancel",
    ))
    .column_spacing(0)
    .row_highlight_style(Style::default().bg(SELECTED_BG));
    frame.render_widget(table, area);
}

// ── Key handling ───────────────────────────────────────────────

fn handle_import(app: &mut App, key: KeyEvent) -> Result<()> {
    let shell_len = app.session.import.shell_candidates.len();
    let total = shell_len + app.session.import.candidates.len();

    match handle_list_nav(&mut app.session.import.cursor, total, key) {
        ListAction::Cancel => app.session.mode = crate::app::Mode::Home,
        ListAction::Select => {
            let mut imported = 0;
            let picked_shells: Vec<_> = app
                .session
                .import
                .shell_candidates
                .iter()
                .zip(&app.session.import.shell_selected)
                .filter_map(|(item, &selected)| selected.then_some(item.clone()))
                .collect();
            for candidate in &picked_shells {
                if candidate.conflict.is_none() {
                    app.config.add_local_shell(candidate)?;
                    imported += 1;
                }
            }
            let picked_ssh: Vec<_> = app
                .session
                .import
                .candidates
                .iter()
                .zip(&app.session.import.selected)
                .filter_map(|(item, &selected)| selected.then_some(item.clone()))
                .collect();
            if !picked_ssh.is_empty() {
                match crate::import::import_candidates(&mut app.config, &picked_ssh) {
                    Ok(count) => imported += count,
                    Err(err) => {
                        app.toast(err.to_string(), false);
                        app.session.mode = crate::app::Mode::Home;
                        return Ok(());
                    }
                }
            }
            if imported > 0 {
                app.config.save()?;
                app.toast(format!("imported {imported} items"), true);
            } else {
                app.toast("nothing selected", false);
            }
            app.session.mode = crate::app::Mode::Home;
        }
        ListAction::None => match key.code {
            KeyCode::Char(' ') => {
                if app.session.import.cursor < shell_len {
                    let can_toggle = app
                        .session
                        .import
                        .shell_candidates
                        .get(app.session.import.cursor)
                        .is_some_and(|c| c.conflict.is_none());
                    if can_toggle
                        && let Some(v) = app
                            .session
                            .import
                            .shell_selected
                            .get_mut(app.session.import.cursor)
                    {
                        *v = !*v;
                    }
                } else {
                    let ssh_idx = app.session.import.cursor - shell_len;
                    if let Some(v) = app.session.import.selected.get_mut(ssh_idx) {
                        *v = !*v;
                    }
                }
            }
            KeyCode::Char('a') => {
                app.session
                    .import
                    .shell_selected
                    .iter_mut()
                    .zip(&app.session.import.shell_candidates)
                    .for_each(|(sel, c)| *sel = c.conflict.is_none());
                app.session.import.selected.fill(true);
            }
            KeyCode::Char('A') => {
                app.session.import.shell_selected.fill(false);
                app.session.import.selected.fill(false);
            }
            _ => {}
        },
    }
    Ok(())
}
