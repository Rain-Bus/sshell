use crate::app::{App, Mode};
use crate::ui::{ACCENT, DIM_BORDER, MUTED, PANEL};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

struct Hint {
    key: &'static str,
    desc: &'static str,
}

fn home_hints() -> Vec<Hint> {
    vec![
        Hint { key: "j/k", desc: "move" },
        Hint { key: "Tab", desc: "group" },
        Hint { key: "Enter", desc: "connect" },
        Hint { key: "Ctrl+Q", desc: "quick" },
        Hint { key: "/", desc: "search" },
        Hint { key: "a", desc: "add" },
        Hint { key: "e", desc: "edit" },
        Hint { key: "d", desc: "delete" },
        Hint { key: ":", desc: "actions" },
        Hint { key: "q", desc: "quit" },
    ]
}

fn search_hints() -> Vec<Hint> {
    vec![
        Hint { key: "type", desc: "filter" },
        Hint { key: "j/k", desc: "move" },
        Hint { key: "Esc", desc: "close" },
    ]
}

fn quick_select_hints() -> Vec<Hint> {
    vec![
        Hint { key: "1-9", desc: "connect" },
        Hint { key: "Tab", desc: "sort" },
        Hint { key: "Esc", desc: "cancel" },
    ]
}

fn form_hints() -> Vec<Hint> {
    vec![
        Hint { key: "Tab", desc: "next" },
        Hint { key: "Enter", desc: "save" },
        Hint { key: "Esc", desc: "cancel" },
        Hint { key: "Ctrl+U", desc: "clear" },
    ]
}

fn delete_hints() -> Vec<Hint> {
    vec![
        Hint { key: "Enter", desc: "confirm" },
        Hint { key: "Esc", desc: "cancel" },
    ]
}

fn credentials_hints() -> Vec<Hint> {
    vec![
        Hint { key: "j/k", desc: "move" },
        Hint { key: "Enter", desc: "edit" },
        Hint { key: "a", desc: "add" },
        Hint { key: "d", desc: "delete" },
        Hint { key: "Esc", desc: "back" },
    ]
}

fn cred_form_hints() -> Vec<Hint> {
    vec![
        Hint { key: "Tab", desc: "next" },
        Hint { key: "Enter", desc: "save" },
        Hint { key: "Esc", desc: "back" },
        Hint { key: "Ctrl+U", desc: "clear" },
    ]
}

fn import_hints() -> Vec<Hint> {
    vec![
        Hint { key: "j/k", desc: "move" },
        Hint { key: "Space", desc: "toggle" },
        Hint { key: "a/A", desc: "all/none" },
        Hint { key: "Enter", desc: "import" },
        Hint { key: "Esc", desc: "cancel" },
    ]
}

fn settings_hints() -> Vec<Hint> {
    vec![
        Hint { key: "type", desc: "edit" },
        Hint { key: "Enter", desc: "save" },
        Hint { key: "Esc", desc: "cancel" },
    ]
}

fn action_menu_hints() -> Vec<Hint> {
    vec![
        Hint { key: "j/k", desc: "move" },
        Hint { key: "Enter", desc: "select" },
        Hint { key: "Esc", desc: "cancel" },
    ]
}

pub fn draw_help(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let hints = match app.session.mode {
        Mode::Home => home_hints(),
        Mode::ActionMenu => action_menu_hints(),
        Mode::Search => search_hints(),
        Mode::QuickSelect => quick_select_hints(),
        Mode::Form => form_hints(),
        Mode::DeleteConfirm => delete_hints(),
        Mode::Credentials => credentials_hints(),
        Mode::CredForm => cred_form_hints(),
        Mode::ImportSelector => import_hints(),
        Mode::Settings => settings_hints(),
    };

    let sep_width: usize = 5; // "  |  "
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut used: usize = 0;
    let max_w = area.width as usize;

    for (i, hint) in hints.iter().enumerate() {
        let hint_span = key_hint(hint.key, hint.desc);
        let needed = hint_span.width() + if i > 0 { sep_width } else { 0 };
        if used + needed > max_w {
            break;
        }
        if i > 0 {
            spans.push(sep());
        }
        spans.push(hint_span);
        used += needed;
    }

    Paragraph::new(Line::from(spans))
        .fg(MUTED)
        .bg(PANEL)
        .alignment(Alignment::Center)
        .render(area, frame.buffer_mut());
}

fn key_hint(key: &str, desc: &str) -> Span<'static> {
    Span::styled(
        format!(" {key} {desc} "),
        Style::default()
            .fg(ACCENT)
            .bg(PANEL)
            .add_modifier(Modifier::BOLD),
    )
}

fn sep() -> Span<'static> {
    Span::styled("  |  ", Style::default().fg(DIM_BORDER))
}
