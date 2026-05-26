pub mod app;
pub mod component;
pub mod view;

use crate::app::Mode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Stylize},
    widgets::{Block, Paragraph, Widget},
};

pub use app::restore_terminal;

use component::{draw_delete_confirm, draw_header, draw_help, draw_toast};

// ── Theme ────────────────────────────────────────────────────

pub const BG: Color = Color::Rgb(12, 14, 17);
pub const PANEL: Color = Color::Rgb(20, 24, 29);
pub const PANEL_ALT: Color = Color::Rgb(25, 30, 36);
pub const ACCENT: Color = Color::Rgb(79, 209, 197);
pub const BLUE: Color = Color::Rgb(96, 165, 250);
pub const GREEN: Color = Color::Rgb(74, 222, 128);
pub const RED: Color = Color::Rgb(248, 113, 113);
pub const ORANGE: Color = Color::Rgb(251, 146, 60);
pub const YELLOW: Color = Color::Rgb(250, 204, 21);
pub const PURPLE: Color = Color::Rgb(167, 139, 250);
pub const TEXT: Color = Color::Rgb(232, 238, 247);
pub const MUTED: Color = Color::Rgb(139, 150, 166);
pub const DIM_BORDER: Color = Color::Rgb(45, 52, 62);
pub const SELECTED_BG: Color = Color::Rgb(34, 48, 58);

// ── Draw dispatcher ──────────────────────────────────────────

const MIN_WIDTH: u16 = 50;
const MIN_HEIGHT: u16 = 25;

pub fn draw(frame: &mut Frame<'_>, app: &mut crate::app::App) {
    frame.render_widget(Block::new().bg(BG), frame.area());
    if let Some(toast) = &app.session.toast
        && toast.born.elapsed().as_secs() > 3
    {
        app.session.toast = None;
    }

    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = format!(
            "Terminal too small: {}x{}  (min {}x{})",
            area.width, area.height, MIN_WIDTH, MIN_HEIGHT,
        );
        Paragraph::new(msg)
            .fg(ACCENT)
            .alignment(Alignment::Center)
            .render(
                Rect { x: 0, y: area.height / 2, width: area.width, height: 1 },
                frame.buffer_mut(),
            );
        return;
    }

    let shell = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(area);

    let content = shell[1].inner(Margin {
        horizontal: 1,
        vertical: 1,
    });

    use view::View;
    let (title, hints): (&str, Vec<_>) = match app.session.mode {
        Mode::Home => (view::HomeListView.title(), view::HomeListView.hints()),
        Mode::ActionMenu => (view::ActionMenuView.title(), view::ActionMenuView.hints()),
        Mode::Search => (view::SearchView.title(), view::SearchView.hints()),
        Mode::QuickSelect => (view::QuickSelectView.title(), view::QuickSelectView.hints()),
        Mode::DeleteConfirm => (view::DeleteConfirmView.title(), view::DeleteConfirmView.hints()),
        Mode::Form => (view::FormView.title(), view::FormView.hints()),
        Mode::ImportSelector => (view::ImportView.title(), view::ImportView.hints()),
        Mode::Credentials => (view::CredListView.title(), view::CredListView.hints()),
        Mode::CredForm => (view::CredFormView.title(), view::CredFormView.hints()),
        Mode::Settings => (view::SettingsView.title(), view::SettingsView.hints()),
    };

    draw_header(frame, app, title, shell[0]);

    match app.session.mode {
        Mode::Home => view::HomeListView.draw(frame, app, content),
        Mode::ActionMenu => view::HomeListView.draw(frame, app, content),
        Mode::Search => view::SearchView.draw(frame, app, content),
        Mode::QuickSelect => view::QuickSelectView.draw(frame, app, content),
        Mode::DeleteConfirm => view::DeleteConfirmView.draw(frame, app, content),
        Mode::Form => view::FormView.draw(frame, app, content),
        Mode::ImportSelector => view::ImportView.draw(frame, app, content),
        Mode::Credentials => view::CredListView.draw(frame, app, content),
        Mode::CredForm => view::CredFormView.draw(frame, app, content),
        Mode::Settings => view::SettingsView.draw(frame, app, content),
    }

    draw_help(frame, &hints, shell[2]);
    if app.session.mode == Mode::DeleteConfirm {
        draw_delete_confirm(frame, app);
    }
    if app.session.mode == Mode::ActionMenu {
        view::ActionMenuView.draw(frame, app, content);
    }
    if let Some(toast) = &app.session.toast {
        draw_toast(frame, toast.message.as_str(), toast.success);
    }
}
