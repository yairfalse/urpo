//! Minimal trace list view for TUI

use crate::storage::TraceInfo;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use std::time::{Duration, SystemTime};

/// Draw trace list table
#[inline]
pub fn draw_trace_table(
    frame: &mut Frame,
    area: Rect,
    traces: &[TraceInfo],
    selected: Option<usize>,
) {
    // If no traces, show empty state
    if traces.is_empty() {
        let empty = ratatui::widgets::Paragraph::new("No traces captured yet...")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Recent Traces ")
                    .border_style(Style::default().fg(Color::Cyan)),
            );
        frame.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec!["Trace ID", "Service", "Duration", "Spans", "Status", "Age"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .height(1);

    let rows: Vec<Row> = traces
        .iter()
        .enumerate()
        .map(|(idx, trace)| {
            let is_selected = selected == Some(idx);
            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default()
            };

            let status_style = if trace.has_error {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };

            // Safely get first 8 chars of trace ID
            let trace_id_short = if trace.trace_id.as_str().len() >= 8 {
                &trace.trace_id.as_str()[..8]
            } else {
                trace.trace_id.as_str()
            };

            Row::new(vec![
                Cell::from(trace_id_short),
                Cell::from(trace.root_service.as_str()),
                Cell::from(format_duration(trace.duration)),
                Cell::from(trace.span_count.to_string()),
                Cell::from(if trace.has_error { "ERROR" } else { "OK" }).style(status_style),
                Cell::from(format_age(trace.start_time)),
            ])
            .style(style)
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Length(12), // Trace ID
            ratatui::layout::Constraint::Min(20),    // Service
            ratatui::layout::Constraint::Length(12), // Duration
            ratatui::layout::Constraint::Length(8),  // Spans
            ratatui::layout::Constraint::Length(8),  // Status
            ratatui::layout::Constraint::Length(10), // Age
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Recent Traces ({}) ", traces.len()))
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .highlight_style(Style::default().bg(Color::DarkGray));

    frame.render_widget(table, area);
}

#[inline(always)]
fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

#[inline(always)]
fn format_age(time: SystemTime) -> String {
    let elapsed = SystemTime::now()
        .duration_since(time)
        .unwrap_or(Duration::ZERO);

    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}
