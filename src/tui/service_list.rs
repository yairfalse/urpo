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
    // If no services, show empty state
    if services.is_empty() {
        let empty = ratatui::widgets::Paragraph::new("No services detected yet...")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Services ")
                    .border_style(Style::default().fg(Color::Cyan)),
            );
        frame.render_widget(empty, area);
        return;
    }

    // Header
    let header = Row::new(vec!["Service", "RPS", "Error%", "P50", "P95", "P99"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .height(1);

    // Rows
    let rows: Vec<Row> = services
        .iter()
        .enumerate()
        .map(|(idx, service)| {
            let is_selected = selected == Some(idx);
            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
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
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Min(20),    // Service name
            ratatui::layout::Constraint::Length(10), // RPS
            ratatui::layout::Constraint::Length(10), // Error%
            ratatui::layout::Constraint::Length(10), // P50
            ratatui::layout::Constraint::Length(10), // P95
            ratatui::layout::Constraint::Length(10), // P99
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Services ({}) ", services.len()))
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray));

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
