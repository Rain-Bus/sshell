use crate::app::App;
use crate::config::SyncBackend;
use crate::ui::{ACCENT, BG, BLUE, DIM_BORDER, GREEN, MUTED, ORANGE, PANEL, PURPLE, TEXT};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

pub fn draw_header(frame: &mut ratatui::Frame<'_>, app: &App, title: &str, area: Rect) {
    let synced = app.config.connections.values().filter(|p| p.sync()).count();
    let (sync_text, sync_color) = if synced > 0 {
        (format!("synced {synced}"), GREEN)
    } else {
        ("no sync".to_string(), MUTED)
    };
    let creds = app.config.credentials.entries.len();
    let backend = match app.config.settings.backend {
        SyncBackend::Gist => {
            if app.config.settings.gist_id.is_some() {
                ("gist ready", BLUE)
            } else {
                ("gist not set", MUTED)
            }
        }
        SyncBackend::Webdav => {
            if app.config.settings.webdav_url.is_some() {
                ("webdav ready", ORANGE)
            } else {
                ("webdav not set", MUTED)
            }
        }
        SyncBackend::S3 => {
            if app.config.settings.s3_endpoint.is_some() {
                ("s3 ready", PURPLE)
            } else {
                ("s3 not set", MUTED)
            }
        }
    };

    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(
            " SSHELL ",
            Style::default().fg(Color::Black).bg(ACCENT).bold(),
        ),
        Span::raw("  "),
        Span::styled(title.to_string(), Style::default().fg(TEXT).bold()),
    ];

    let conn_pill = pill(
        "connections",
        &app.config.connections.len().to_string(),
        ACCENT,
    );
    let cred_pill = pill("credentials", &creds.to_string(), BLUE);
    let sync_span = Span::styled(sync_text, Style::default().fg(sync_color));
    let backend_span = Span::styled(backend.0.to_string(), Style::default().fg(backend.1));

    let base_width: u16 = spans.iter().map(|s| s.width() as u16).sum();
    let w = area.width;

    if w >= base_width + 1 + conn_pill.width() as u16 {
        spans.push(Span::raw("  "));
        spans.push(conn_pill);
    }
    if w >= spans.iter().map(|s| s.width() as u16).sum::<u16>() + 1 + cred_pill.width() as u16 {
        spans.push(Span::raw(" "));
        spans.push(cred_pill);
    }
    if w >= spans.iter().map(|s| s.width() as u16).sum::<u16>() + 2 + sync_span.width() as u16 {
        spans.push(Span::raw(" "));
        spans.push(sync_span);
    }
    if w >= spans.iter().map(|s| s.width() as u16).sum::<u16>() + 2 + backend_span.width() as u16 {
        spans.push(Span::raw("  "));
        spans.push(backend_span);
    }

    let mut lines = vec![Line::from(spans)];
    if w >= 35 {
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled("fast ssh and shell launcher", Style::default().fg(MUTED)),
        ]));
    }

    Paragraph::new(lines)
        .block(
            Block::default()
                .bg(PANEL)
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(DIM_BORDER)),
        )
        .alignment(Alignment::Left)
        .style(Style::default().bg(BG))
        .render(area, frame.buffer_mut());
}

fn pill(label: &str, value: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {label} {value} "),
        Style::default().fg(color).bg(BG).bold(),
    )
}
