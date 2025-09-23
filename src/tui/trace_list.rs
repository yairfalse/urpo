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
    let header = Row::new(vec!["Trace", "Service", "Duration", "Spans", "Status", "Age"])
        .style(Style::default().fg(Color::Yellow));

    let rows = traces.iter().enumerate().map(|(idx, trace)| {
        let is_selected = selected == Some(idx);
        let style = if is_selected {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        let status_style = if trace.has_error {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };

        Row::new(vec![
            Cell::from(&trace.trace_id.as_str()[..8]),
            Cell::from(trace.root_service.as_str()),
            Cell::from(format_duration(trace.duration)),
            Cell::from(trace.span_count.to_string()),
            Cell::from(if trace.has_error { "ERROR" } else { "OK" }).style(status_style),
            Cell::from(format_age(trace.start_time)),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Length(10),
            ratatui::layout::Constraint::Percentage(25),
            ratatui::layout::Constraint::Length(10),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Recent Traces ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

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