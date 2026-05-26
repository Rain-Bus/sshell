use crate::ui::{GREEN, PANEL_ALT, RED};
use ratatui::{
    style::{Style, Stylize},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

pub fn draw_toast(frame: &mut ratatui::Frame<'_>, message: &str, success: bool) {
    let width = (message.len() as u16 + 6).clamp(28, 70);
    let area = ratatui::layout::Rect {
        x: frame.area().right().saturating_sub(width + 2),
        y: frame.area().bottom().saturating_sub(4),
        width,
        height: 3,
    };
    frame.render_widget(Clear, area);
    let border_color = if success { GREEN } else { RED };
    Paragraph::new(format!(" {}", message))
        .fg(border_color)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .bg(PANEL_ALT),
        )
        .render(area, frame.buffer_mut());
}
