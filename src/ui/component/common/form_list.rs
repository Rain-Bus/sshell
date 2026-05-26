use crate::ui::{ACCENT, BLUE, DIM_BORDER, MUTED, PANEL_ALT, SELECTED_BG, TEXT};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Widget},
};

use super::panel::panel_with_subtitle;

// ── Row descriptor ──────────────────────────────────────────

pub enum FormRow {
    Separator,
    Toggle {
        label: String,
        active: bool,
        badge: Span<'static>,
    },
    Text {
        label: String,
        active: bool,
        display: String,
        cursor: usize,
        placeholder: Option<String>,
    },
}

// ── Rendering ───────────────────────────────────────────────

const ACTIVE_BG: Color = SELECTED_BG;

pub fn draw_form_list(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    subtitle: &str,
    rows: Vec<FormRow>,
) {
    let mut items: Vec<ListItem> = Vec::new();

    for row in rows {
        match row {
            FormRow::Separator => {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  ─────────────────────────────────────────────",
                    Style::default().fg(DIM_BORDER),
                ))));
            }
            FormRow::Toggle {
                label,
                active,
                badge,
            } => {
                let marker = if active { ">" } else { " " };
                let label_span = Span::styled(
                    format!("{marker} {label:<13} "),
                    Style::default().fg(if active { ACCENT } else { MUTED }),
                );
                let hint = Span::styled("  Tab toggles", Style::default().fg(BLUE));
                let line = if active {
                    Line::from(vec![label_span, badge, hint]).style(Style::default().bg(ACTIVE_BG))
                } else {
                    Line::from(vec![label_span, badge, hint])
                };
                items.push(ListItem::new(line));
            }
            FormRow::Text {
                label,
                active,
                display,
                cursor,
                placeholder,
            } => {
                let marker = if active { ">" } else { " " };
                let label_span = Span::styled(
                    format!("{marker} {label:<13} "),
                    Style::default().fg(if active { ACCENT } else { MUTED }),
                );

                let (shown, is_placeholder) = if display.is_empty() {
                    match &placeholder {
                        Some(ph) => (ph.clone(), true),
                        None => (String::new(), false),
                    }
                } else {
                    (display, false)
                };

                let val_color = if is_placeholder || shown.is_empty() {
                    MUTED
                } else {
                    TEXT
                };

                let (before, after) = if active {
                    let pos = cursor.min(shown.len());
                    let b: String = shown.chars().take(pos).collect();
                    let a: String = shown.chars().skip(pos).collect();
                    (b, a)
                } else {
                    (shown, String::new())
                };

                let val_span = Span::styled(before, Style::default().fg(val_color));
                let cursor_span = if active {
                    Span::styled("▌", Style::default().fg(ACCENT))
                } else {
                    Span::raw("")
                };
                let after_span = Span::styled(after, Style::default().fg(val_color));

                let row_bg = if active { ACTIVE_BG } else { PANEL_ALT };
                let line = Line::from(vec![label_span, val_span, cursor_span, after_span])
                    .style(Style::default().bg(row_bg));
                items.push(ListItem::new(line));
            }
        }
    }

    List::new(items)
        .block(panel_with_subtitle(title, subtitle))
        .render(area, frame.buffer_mut());
}
