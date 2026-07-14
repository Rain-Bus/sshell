use crate::app::{App, Mode};
use crate::ui::component::{ListAction, handle_list_nav};
use crate::ui::{ACCENT, MUTED, PANEL, SELECTED_BG, TEXT};

use super::View;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, ListState},
};

const ACTIONS: &[(&str, &str)] = &[
    ("Import", "scan shells & import SSH config"),
    ("Sync", "bidirectional cloud sync"),
    ("Credentials", "manage passwords & keys"),
    ("Settings", "preferences & sync config"),
];

pub struct ActionMenuView;

impl View for ActionMenuView {
    fn title(&self) -> &'static str {
        "Actions"
    }
    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        vec![("j/k", "move"), ("Enter", "select"), ("Esc", "cancel")]
    }

    fn draw(&self, frame: &mut Frame<'_>, _app: &App, area: Rect) {
        let list_width = 52u16.min(area.width.saturating_sub(4));
        let list_height = (ACTIONS.len() as u16 + 4).min(area.height);
        let x = area.x + (area.width.saturating_sub(list_width)) / 2;
        let y = area.y + (area.height.saturating_sub(list_height)) / 2;
        let list_area = Rect {
            x,
            y,
            width: list_width,
            height: list_height,
        };

        frame.render_widget(Clear, list_area);

        let items: Vec<Line<'_>> = ACTIONS
            .iter()
            .map(|(label, desc)| {
                Line::from(vec![
                    Span::styled(format!("  {label:<14}"), Style::default().fg(TEXT)),
                    Span::styled(desc.to_string(), Style::default().fg(MUTED)),
                ])
            })
            .collect();

        let list = ratatui::widgets::List::new(items)
            .block(
                Block::default()
                    .title(" Actions ")
                    .title_style(Style::default().fg(ACCENT).bold())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(ACCENT))
                    .bg(PANEL),
            )
            .highlight_style(
                Style::default()
                    .bg(SELECTED_BG)
                    .fg(TEXT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">");

        let mut state = ListState::default();
        state.select(Some(_app.session.action_menu.cursor));

        frame.render_stateful_widget(list, list_area, &mut state);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Char(':') {
            app.session.mode = Mode::Home;
            return Ok(());
        }
        match handle_list_nav(&mut app.session.action_menu.cursor, ACTIONS.len(), key) {
            ListAction::Cancel => {
                app.session.mode = Mode::Home;
            }
            ListAction::Select => {
                app.session.mode = Mode::Home;
                match app.session.action_menu.cursor {
                    0 => app.enter_import_selector()?,
                    1 => app.sync_with_toast(),
                    2 => app.enter_credentials(),
                    3 => app.enter_settings(),
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }
}
