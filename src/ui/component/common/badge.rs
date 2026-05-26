use crate::ui::{BG, ORANGE};
use ratatui::{
    style::{Color, Style},
    text::Span,
};

pub fn badge_span(text: &str, bg: Color) -> Span<'static> {
    Span::styled(format!(" {text} "), Style::default().fg(BG).bg(bg).bold())
}

pub fn tag_badge(tag: &str) -> Span<'static> {
    Span::styled(
        format!(" #{tag} "),
        Style::default().fg(Color::Rgb(18, 20, 24)).bg(ORANGE),
    )
}
