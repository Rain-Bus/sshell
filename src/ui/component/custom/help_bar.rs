use crate::ui::{ACCENT, DIM_BORDER, MUTED, PANEL};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub fn draw_help(frame: &mut ratatui::Frame<'_>, hints: &[(&str, &str)], area: Rect) {
    let sep_width: usize = 5; // "  |  "
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut used: usize = 0;
    let max_w = area.width as usize;

    for (i, hint) in hints.iter().enumerate() {
        let hint_span = key_hint(hint.0, hint.1);
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
