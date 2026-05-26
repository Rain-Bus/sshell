use crate::app::{App, Mode};
use crate::app::display_name;
use crate::config::{ConnectionSource, ConnectionType, CredentialEntry};
use crate::ui::component::{badge_span, draw_input, panel, tag_badge};
use crate::ui::{ACCENT, BLUE, GREEN, MUTED, PANEL_ALT, PURPLE, RED, SELECTED_BG, TEXT};

use super::View;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, Widget},
};

pub struct HomeListView;

impl View for HomeListView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        let has_search = app.session.mode == Mode::Search;
        let top_height = if has_search { 3 } else { 0 };

        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(top_height), Constraint::Min(4)])
            .split(area);

        if has_search {
            draw_search_box(frame, app, outer[0]);
        }

        if outer[1].width < 96 {
            let panels = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(58),
                    Constraint::Length(1),
                    Constraint::Min(8),
                ])
                .split(outer[1]);
            draw_connection_list(frame, app, panels[0]);
            draw_detail_panel(frame, app, panels[2]);
        } else {
            let panels = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(60),
                    Constraint::Length(1),
                    Constraint::Percentage(40),
                ])
                .split(outer[1]);

            draw_connection_list(frame, app, panels[0]);
            draw_detail_panel(frame, app, panels[2]);
        }
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        handle_home(app, key)
    }
}

// ── Rendering ──────────────────────────────────────────────────

pub fn draw_search_box(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let is_empty = app.session.home.search.is_empty();
    let value = if is_empty {
        ""
    } else {
        &app.session.home.search
    };
    let placeholder = if is_empty { "type to filter..." } else { "" };
    draw_input(frame, area, " Search ", value, placeholder, true);
}

pub fn draw_connection_list(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let entries = if app.session.mode == Mode::QuickSelect {
        app.quick_entries()
    } else {
        app.entries()
    };
    if entries.is_empty() {
        let message = if app.session.home.search.is_empty() {
            "\n  No connections yet\n\n  Press a to add or : for actions"
        } else {
            "\n  No matching connections\n\n  Press Esc to clear search"
        };
        Paragraph::new(message)
            .fg(MUTED)
            .alignment(Alignment::Center)
            .block(panel("Connections"))
            .render(area, frame.buffer_mut());
        return;
    }

    let mut rows = Vec::new();
    let mut entry_row = Vec::new(); // entry_idx -> visual row index
    if app.session.mode == Mode::QuickSelect {
        rows.push(section_row(
            &format!("Quick {}", app.session.home.quick_sort.label()),
            entries.len().min(9),
        ));
        for (idx, (name, profile)) in entries.iter().take(9).enumerate() {
            entry_row.push(rows.len());
            rows.push(connection_row(app, idx, name, profile));
        }
    } else {
        let mut entry_idx = 0;
        let ssh_count = entries
            .iter()
            .filter(|(_, profile)| matches!(profile.kind, ConnectionType::Ssh { .. }))
            .count();
        let shell_count = entries.len().saturating_sub(ssh_count);

        if ssh_count > 0 {
            rows.push(section_row("SSH", ssh_count));
        }
        for (name, profile) in entries
            .iter()
            .filter(|(_, profile)| matches!(profile.kind, ConnectionType::Ssh { .. }))
        {
            entry_row.push(rows.len());
            rows.push(connection_row(app, entry_idx, name, profile));
            entry_idx += 1;
        }

        if ssh_count > 0 && shell_count > 0 {
            rows.push(Row::new(["", "", "", ""]).height(1));
        }

        if shell_count > 0 {
            rows.push(section_row("Shell", shell_count));
        }
        for (name, profile) in entries
            .iter()
            .filter(|(_, profile)| matches!(profile.kind, ConnectionType::Shell { .. }))
        {
            entry_row.push(rows.len());
            rows.push(connection_row(app, entry_idx, name, profile));
            entry_idx += 1;
        }
    }

    // Scroll to keep selected entry visible
    let visible = area.height.saturating_sub(3) as usize; // 2 borders + 1 header
    if visible > 0 && !entry_row.is_empty() {
        let sel_row = entry_row[app.session.home.selected.min(entry_row.len() - 1)];
        let total = rows.len();
        if total > visible {
            let scroll = if sel_row < visible / 2 {
                0
            } else if sel_row + visible / 2 >= total {
                total.saturating_sub(visible)
            } else {
                sel_row - visible / 2
            };
            rows = rows.into_iter().skip(scroll).take(visible).collect();
        }
    }

    let title = if app.session.mode == Mode::QuickSelect {
        "Connections - Quick Select"
    } else {
        "Connections"
    };

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(28),
            Constraint::Length(6),
            Constraint::Percentage(50),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new(["  Name", "Type", "Target", "Auth"])
            .style(Style::default().fg(BLUE).add_modifier(Modifier::BOLD)),
    )
    .block(panel(title))
    .column_spacing(0)
    .row_highlight_style(Style::default().bg(SELECTED_BG));
    frame.render_widget(table, area);
}

fn section_row(label: &str, count: usize) -> Row<'static> {
    Row::new([
        Cell::from(format!("  {label} ({count})"))
            .style(Style::default().fg(BLUE).add_modifier(Modifier::BOLD)),
        Cell::from(""),
        Cell::from(""),
        Cell::from(""),
    ])
    .height(1)
}

fn connection_row(
    app: &App,
    idx: usize,
    name: &str,
    profile: &crate::config::ConnectionProfile,
) -> Row<'static> {
    let selected = idx == app.session.home.selected;
    let marker = if app.session.mode == Mode::QuickSelect {
        quick_key(idx).unwrap_or(' ')
    } else if selected {
        '>'
    } else {
        ' '
    };
    let row_style = if selected {
        Style::default()
            .bg(SELECTED_BG)
            .fg(TEXT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT)
    };
    let is_shell = matches!(profile.kind, ConnectionType::Shell { .. });
    let type_badge = if is_shell { "SHL" } else { "SSH" };
    let badge_color = if is_shell { PURPLE } else { ACCENT };

    let target = match &profile.kind {
        ConnectionType::Ssh {
            host, port, user, ..
        } => {
            if *port == 22 {
                format!("{user}@{host}")
            } else {
                format!("{user}@{host}:{port}")
            }
        }
        ConnectionType::Shell {
            command,
            sync_args,
            local_args,
            ..
        } => {
            let merged_args = shell_args(sync_args, local_args);
            if merged_args.is_empty() {
                command.clone()
            } else {
                format!("{command} {}", merged_args.join(" "))
            }
        }
    };

    let auth_state = profile
        .auth_ref()
        .and_then(|auth| app.config.credential(auth))
        .map(|cred| if cred.has_value() { "ready" } else { "empty" })
        .unwrap_or("none");
    let auth_color = match auth_state {
        "ready" => GREEN,
        "empty" => RED,
        _ => MUTED,
    };

    let badge_style = if selected {
        Style::default().fg(badge_color).bg(SELECTED_BG).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(crate::ui::BG).bg(badge_color).add_modifier(Modifier::BOLD)
    };

    Row::new([
        Cell::from(format!("{marker} {}", display_name(name))).style(row_style),
        Cell::from(Line::from(vec![
            Span::styled(format!(" {} ", type_badge), badge_style),
        ])).style(row_style),
        Cell::from(target).style(row_style),
        Cell::from(auth_state).style(if selected {
            Style::default().fg(auth_color).bg(SELECTED_BG)
        } else {
            Style::default().fg(auth_color)
        }),
    ])
    .height(1)
}

pub fn quick_key(idx: usize) -> Option<char> {
    (idx < 9).then(|| (b'1' + idx as u8) as char)
}

pub fn draw_detail_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let entries = if app.session.mode == Mode::QuickSelect {
        app.quick_entries()
    } else {
        app.entries()
    };
    let Some((name, profile)) = entries.get(app.session.home.selected) else {
        Paragraph::new("\n  Select a connection")
            .fg(MUTED)
            .alignment(Alignment::Center)
            .block(panel("Detail"))
            .render(area, frame.buffer_mut());
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    let is_shell = matches!(profile.kind, ConnectionType::Shell { .. });
    let badge = if is_shell { "SHL" } else { "SSH" };
    let badge_color = if is_shell { PURPLE } else { ACCENT };
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        badge_span(badge, badge_color),
        Span::raw("  "),
        Span::styled(display_name(name).to_string(), Style::default().fg(TEXT).bold()),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            if is_shell {
                "local command profile"
            } else {
                "remote ssh profile"
            },
            Style::default().fg(MUTED),
        ),
    ]));
    lines.push(Line::raw(""));

    match &profile.kind {
        ConnectionType::Ssh {
            host,
            port,
            user,
            auth_ref,
            sync,
        } => {
            lines.push(detail_text("Target", &format!("{user}@{host}")));
            lines.push(detail_text("Port", &port.to_string()));
            lines.push(detail_text("User", user));
            lines.push(detail_text("Protocol", "ssh"));
            lines.push(detail_text(
                "Sync",
                if *sync { "enabled" } else { "disabled" },
            ));
            lines.push(detail_text("Source", source_label(profile.source)));
            lines.push(detail_text("Uses", &profile.usage_count.to_string()));
            lines.push(Line::raw(""));
            lines.push(detail_credential_line(app, auth_ref));
        }
        ConnectionType::Shell {
            shell_name,
            auth_ref,
            command,
            sync_args,
            local_args,
            sync,
            ..
        } => {
            lines.push(detail_text("Shell", shell_name));
            lines.push(detail_text("Command", command));
            let merged_args = shell_args(sync_args, local_args);
            if !merged_args.is_empty() {
                lines.push(detail_text("Args", &merged_args.join(" ")));
            }
            lines.push(detail_text(
                "Sync",
                if *sync { "enabled" } else { "disabled" },
            ));
            lines.push(detail_text("Source", source_label(profile.source)));
            lines.push(detail_text("Uses", &profile.usage_count.to_string()));
            lines.push(Line::raw(""));
            if let Some(ref_val) = auth_ref {
                lines.push(detail_credential_line(app, ref_val));
            } else {
                lines.push(detail_text("Auth", "not required"));
            }
        }
    }

    if !profile.tags.is_empty() {
        lines.push(Line::raw(""));
        let mut tag_spans: Vec<Span> = Vec::new();
        for tag in &profile.tags {
            if !tag_spans.is_empty() {
                tag_spans.push(Span::raw(" "));
            }
            tag_spans.push(tag_badge(tag));
        }
        lines.push(detail_line("Tags", tag_spans));
    }

    Paragraph::new(lines)
        .style(Style::default().bg(PANEL_ALT))
        .block(crate::ui::component::panel_accent("Detail"))
        .render(area, frame.buffer_mut());
}

fn source_label(source: ConnectionSource) -> &'static str {
    match source {
        ConnectionSource::Manual => "manual",
        ConnectionSource::Imported => "imported",
        ConnectionSource::Scanned => "scanned",
    }
}

fn detail_text(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<11}", label), Style::default().fg(MUTED)),
        Span::styled(value.to_string(), Style::default().fg(TEXT)),
    ])
}

fn shell_args(sync_args: &[String], local_args: &[String]) -> Vec<String> {
    let mut out = sync_args.to_vec();
    out.extend(local_args.iter().cloned());
    out
}

fn detail_line(label: &str, spans: Vec<Span<'static>>) -> Line<'static> {
    let mut out = vec![Span::styled(
        format!("  {:<11}", label),
        Style::default().fg(MUTED),
    )];
    out.extend(spans);
    Line::from(out)
}

fn detail_credential_line(app: &App, auth_ref: &str) -> Line<'static> {
    let (dot_color, status_text) = match app.config.credential(auth_ref) {
        Some(cred) if cred.has_value() => (GREEN, "set"),
        Some(_) => (RED, "empty"),
        None => (RED, "missing"),
    };
    let auth_type = match app.config.credential(auth_ref) {
        Some(CredentialEntry::Password { .. }) => "password",
        Some(CredentialEntry::PrivateKey { .. }) => "private key",
        None => "?",
    };
    Line::from(vec![
        Span::styled("  Auth       ", Style::default().fg(MUTED)),
        Span::styled("● ", Style::default().fg(dot_color)),
        Span::styled(format!("{auth_type} "), Style::default().fg(TEXT)),
        Span::styled(format!("({status_text})"), Style::default().fg(dot_color)),
    ])
}

// ── Key handling ───────────────────────────────────────────────

fn handle_home(app: &mut App, key: KeyEvent) -> Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
        app.enter_quick_select();
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.request_quit(),
        KeyCode::Tab => app.jump_group(),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Enter => app.connect_selected()?,
        KeyCode::Char('/') => app.enter_search(),
        KeyCode::Char('a') => app.new_form(),
        KeyCode::Char('e') => app.edit_form(),
        KeyCode::Char('d') => app.enter_delete_confirm_for_selected(),
        KeyCode::Char(':') => app.enter_action_menu(),
        _ => {}
    }
    Ok(())
}

// ── Search View ──────────────────────────────────────────────────

pub struct SearchView;

impl View for SearchView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        HomeListView.draw(frame, app, area);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.session.mode = Mode::Home;
            }
            KeyCode::Char('j') => app.move_selection(1),
            KeyCode::Char('k') => app.move_selection(-1),
            KeyCode::Down => app.move_selection(1),
            KeyCode::Up => app.move_selection(-1),
            KeyCode::Backspace => {
                app.session.home.search.pop();
                app.session.home.selected = 0;
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                app.session.home.search.push(c);
                app.session.home.selected = 0;
            }
            _ => {}
        }
        Ok(())
    }
}

// ── Quick Select View ────────────────────────────────────────────

pub struct QuickSelectView;

impl View for QuickSelectView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        HomeListView.draw(frame, app, area);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => app.session.mode = Mode::Home,
            KeyCode::Tab => {
                app.session.home.quick_sort = app.session.home.quick_sort.next();
                app.toast(
                    format!(
                        "quick select sorted by {}",
                        app.session.home.quick_sort.label()
                    ),
                    true,
                );
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                if !('1'..='9').contains(&c) {
                    return Ok(());
                }
                let idx = (c as u8 - b'1') as usize;
                let entries = app.quick_entries();
                if let Some((name, _)) = entries.get(idx) {
                    let name = (*name).clone();
                    if let Some(home_idx) = app
                        .entries()
                        .iter()
                        .position(|(entry_name, _)| entry_name.as_str() == name)
                    {
                        app.session.home.selected = home_idx;
                    }
                    app.record_use(&name)?;
                    crate::connection::connect(&name, &app.config)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

// ── Delete Confirm View ──────────────────────────────────────────

pub struct DeleteConfirmView;

impl View for DeleteConfirmView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        HomeListView.draw(frame, app, area);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                match app.delete_selected() {
                    Ok(()) => app.toast("deleted", true),
                    Err(err) => app.toast(err.to_string(), false),
                }
                app.session.mode = Mode::Home;
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                app.session.mode = Mode::Home;
            }
            _ => {}
        }
        Ok(())
    }
}
