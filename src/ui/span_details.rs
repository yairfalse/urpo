//! Span details panel implementation.
//!
//! This module provides a detailed view of individual spans,
//! showing all attributes, events, and metadata.

use crate::core::Span;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span as TextSpan},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::collections::BTreeMap;

/// Draw the span details panel.
pub fn draw_span_details(frame: &mut Frame, area: Rect, span: &Span) {
    // Create layout for the details panel
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),  // Basic info
            Constraint::Min(5),     // Attributes
            Constraint::Length(6),  // Events/Logs
        ])
        .split(area);

    // Draw basic info section
    draw_basic_info(frame, chunks[0], span);
    
    // Draw attributes section
    draw_attributes(frame, chunks[1], span);
    
    // Draw events section
    draw_events(frame, chunks[2], span);
}

/// Draw basic span information.
fn draw_basic_info(frame: &mut Frame, area: Rect, span: &Span) {
    let mut lines = vec![];
    
    // Span ID (copyable with 'y')
    lines.push(Line::from(vec![
        TextSpan::styled("Span ID: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            &span.span_id.as_str()[..16.min(span.span_id.as_str().len())],
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        TextSpan::styled(" [y]", Style::default().fg(Color::DarkGray)),
    ]));
    
    // Trace ID (copyable with 'Y')
    lines.push(Line::from(vec![
        TextSpan::styled("Trace ID: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            &span.trace_id.as_str()[..16.min(span.trace_id.as_str().len())],
            Style::default().fg(Color::Yellow),
        ),
        TextSpan::styled(" [Y]", Style::default().fg(Color::DarkGray)),
    ]));
    
    // Service
    lines.push(Line::from(vec![
        TextSpan::styled("Service: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            span.service_name.as_str(),
            Style::default().fg(Color::Green),
        ),
    ]));
    
    // Operation
    lines.push(Line::from(vec![
        TextSpan::styled("Operation: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            &span.operation_name,
            Style::default().fg(Color::Magenta),
        ),
    ]));
    
    // Duration
    lines.push(Line::from(vec![
        TextSpan::styled("Duration: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            &format!("{}μs", span.duration),
            Style::default().fg(Color::Blue),
        ),
    ]));
    
    // Status
    let status_style = if span.status.is_error() {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    
    lines.push(Line::from(vec![
        TextSpan::styled("Status: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            if span.status.is_error() { "ERROR" } else { "OK" },
            status_style,
        ),
    ]));
    
    // Timestamps
    lines.push(Line::from(vec![
        TextSpan::styled("Start: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            &format_timestamp(span.start_time),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" SPAN DETAILS ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: true });
    
    frame.render_widget(paragraph, area);
}

/// Draw span attributes.
fn draw_attributes(frame: &mut Frame, area: Rect, span: &Span) {
    // Convert attributes to sorted BTreeMap for consistent ordering
    let attributes: BTreeMap<String, String> = span.attributes
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    
    let items: Vec<ListItem> = attributes
        .iter()
        .map(|(key, value)| {
            let content = Line::from(vec![
                TextSpan::styled(
                    &format!("{}: ", key),
                    Style::default().fg(Color::Yellow),
                ),
                TextSpan::styled(
                    value,
                    Style::default().fg(Color::White),
                ),
            ]);
            ListItem::new(content)
        })
        .collect();
    
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" ATTRIBUTES ({}) ", attributes.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("→ ");
    
    frame.render_widget(list, area);
}

/// Draw span events/logs.
fn draw_events(frame: &mut Frame, area: Rect, span: &Span) {
    let mut lines = vec![];
    
    // For now, show some placeholder events
    // In a real implementation, these would come from span.events
    lines.push(Line::from(vec![
        TextSpan::styled("10ms: ", Style::default().fg(Color::DarkGray)),
        TextSpan::styled("Request started", Style::default().fg(Color::White)),
    ]));
    
    lines.push(Line::from(vec![
        TextSpan::styled("15ms: ", Style::default().fg(Color::DarkGray)),
        TextSpan::styled("Connected to database", Style::default().fg(Color::Green)),
    ]));
    
    if span.status.is_error() {
        lines.push(Line::from(vec![
            TextSpan::styled("20ms: ", Style::default().fg(Color::DarkGray)),
            TextSpan::styled("Error occurred", Style::default().fg(Color::Red)),
        ]));
    }
    
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" EVENTS ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );
    
    frame.render_widget(paragraph, area);
}

/// Format a timestamp for display.
fn format_timestamp(timestamp: u64) -> String {
    use chrono::{DateTime, Local, TimeZone};
    
    // Convert nanoseconds to DateTime
    let secs = (timestamp / 1_000_000_000) as i64;
    let nanos = (timestamp % 1_000_000_000) as u32;
    
    if let Some(dt) = Local.timestamp_opt(secs, nanos).single() {
        dt.format("%H:%M:%S.%3f").to_string()
    } else {
        format!("{}ns", timestamp)
    }
}

/// Check if clipboard support is available and copy text.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(feature = "clipboard")]
    {
        use clipboard::{ClipboardContext, ClipboardProvider};
        
        let mut ctx: ClipboardContext = match ClipboardProvider::new() {
            Ok(ctx) => ctx,
            Err(e) => return Err(format!("Failed to access clipboard: {}", e)),
        };
        
        match ctx.set_contents(text.to_string()) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to copy: {}", e)),
        }
    }
    
    #[cfg(not(feature = "clipboard"))]
    {
        // Fallback: print to stdout for piping
        println!("{}", text);
        Ok(())
    }
}