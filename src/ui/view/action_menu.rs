use crate::app::{App, Mode};
use crate::ui::component::common::layout::centered_rect;
use crate::ui::{ACCENT, PANEL, SELECTED_BG, TEXT};

use super::View;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, ListItem, ListState},
};

const ACTIONS: &[(&str, &str)] = &[
    ("Import", "scan shells & import SSH config"),
    ("Push Sync", "upload to cloud"),
    ("Pull Sync", "download from cloud"),
    ("Credentials", "manage passwords & keys"),
    ("Settings", "preferences & sync config"),
];

pub struct ActionMenuView;

impl View for ActionMenuView {
    fn draw(&self, frame: &mut Frame<'_>, _app: &App, _area: Rect) {
        let width = 44u16;
        let height = (ACTIONS.len() as u16) + 4;
        let area = centered_rect(width, height, frame.area());

        frame.render_widget(Clear, area);

        let items: Vec<ListItem<'_>> = ACTIONS
            .iter()
            .map(|(label, desc)| {
                ListItem::new(format!("  {label:<14}{desc}"))
            })
            .collect();

        let list = ratatui::widgets::List::new(items)
            .block(
                Block::default()
                    .title(" Actions ")
                    .title_style(Style::default().fg(ACCENT).bold())
                    .borders(Borders::ALL)
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

        frame.render_stateful_widget(list, area, &mut state);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        let len = ACTIONS.len();
        match key.code {
            KeyCode::Char(':') | KeyCode::Esc => {
                app.session.mode = Mode::Home;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.session.action_menu.cursor =
                    (app.session.action_menu.cursor + 1) % len;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.session.action_menu.cursor =
                    (app.session.action_menu.cursor + len - 1) % len;
            }
            KeyCode::Enter => {
                app.session.mode = Mode::Home;
                match app.session.action_menu.cursor {
                    0 => app.enter_combined_import()?,
                    1 => app.push_sync_with_toast(),
                    2 => app.pull_sync_with_toast(),
                    3 => app.enter_credentials(),
                    4 => app.enter_settings(),
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }
}
