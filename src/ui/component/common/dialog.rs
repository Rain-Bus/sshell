use crate::ui::PANEL;
use ratatui::{
    layout::Alignment,
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap},
};

use super::layout::centered_rect;

pub fn draw_dialog(
    frame: &mut ratatui::Frame<'_>,
    width: u16,
    height: u16,
    border_color: Color,
    title: &str,
    content: &str,
) {
    let area = centered_rect(width, height, frame.area());
    frame.render_widget(Clear, area);
    Paragraph::new(content.to_string())
        .fg(crate::ui::TEXT)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(title.to_string())
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .bg(PANEL),
        )
        .render(area, frame.buffer_mut());
}
