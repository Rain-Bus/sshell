use crate::app::{App, Mode};

use super::View;
use super::home_list::HomeListView;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, layout::Rect};

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
