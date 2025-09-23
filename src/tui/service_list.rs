//! Minimal service list view for TUI - htop-like simplicity

use crate::core::ServiceMetrics;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use std::time::Duration;

/// Draw service list table
#[inline]
pub fn draw_service_table(
    frame: &mut Frame,
    area: Rect,
    services: &[ServiceMetrics],
    selected: Option<usize>,
) {
    // Header
    let header = Row::new(vec!["Service", "RPS", "Error%", "P50", "P95", "P99"])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    // Rows
    let rows = services.iter().enumerate().map(|(idx, service)| {
        let is_selected = selected == Some(idx);
        let style = if is_selected {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        Row::new(vec![
            Cell::from(service.name.as_str()),
            Cell::from(format!("{:.1}", service.request_rate)),
            Cell::from(format!("{:.2}%", service.error_rate * 100.0))
                .style(error_color(service.error_rate)),
            Cell::from(format_duration(service.latency_p50)),
            Cell::from(format_duration(service.latency_p95)),
            Cell::from(format_duration(service.latency_p99)),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Percentage(30),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Services ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(table, area);
}

/// Get color for error rate
#[inline(always)]
fn error_color(error_rate: f64) -> Style {
    if error_rate > 0.05 {
        Style::default().fg(Color::Red)
    } else if error_rate > 0.01 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    }
}

/// Format duration for display
#[inline(always)]
fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}