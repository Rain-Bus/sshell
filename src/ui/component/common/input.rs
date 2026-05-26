use crate::ui::{ACCENT, DIM_BORDER, MUTED, PANEL_ALT, SELECTED_BG, TEXT};
use ratatui::{
    style::{Style, Stylize},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub fn draw_input(
    frame: &mut ratatui::Frame<'_>,
    area: ratatui::layout::Rect,
    title: &str,
    value: &str,
    placeholder: &str,
    focused: bool,
) {
    let display = if value.is_empty() { placeholder } else { value };
    let fg = if value.is_empty() { MUTED } else { TEXT };
    Paragraph::new(format!("  {display}"))
        .fg(fg)
        .bg(if focused { SELECTED_BG } else { PANEL_ALT })
        .block(
            Block::default()
                .title(title.to_string())
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(if focused {
                    Style::default().fg(ACCENT)
                } else {
                    Style::default().fg(DIM_BORDER)
                })
                .bg(PANEL_ALT),
        )
        .render(area, frame.buffer_mut());
}
