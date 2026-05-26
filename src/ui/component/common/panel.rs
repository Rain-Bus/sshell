use crate::ui::{ACCENT, DIM_BORDER, MUTED, PANEL, TEXT};
use ratatui::{
    style::Style,
    style::Stylize,
    text::Line,
    widgets::{Block, BorderType, Borders},
};

pub fn panel(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .title(Line::from(format!(" {} ", title.into())).fg(TEXT).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DIM_BORDER))
        .bg(PANEL)
}

pub fn panel_accent(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .title(Line::from(format!(" {} ", title.into())).fg(ACCENT).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .bg(PANEL)
}

pub fn panel_with_subtitle(title: &str, subtitle: &str) -> Block<'static> {
    panel(title).title_bottom(Line::from(format!(" {subtitle} ")).fg(MUTED))
}
