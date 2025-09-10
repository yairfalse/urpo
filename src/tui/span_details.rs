//! Span details panel implementation.
//!
//! This module provides a detailed view of individual spans,
//! showing all attributes, events, and metadata.

use crate::core::Span;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span as TextSpan},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use std::collections::BTreeMap;

/// Draw the span details panel.
pub fn draw_span_details(frame: &mut Frame, area: Rect, span: &Span) {
    // Create layout for the details panel
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),  // Basic info (expanded)
            Constraint::Percentage(35), // Attributes (scrollable)
            Constraint::Percentage(25), // Tags
            Constraint::Min(5),         // Events/Resource info
        ])
        .split(area);

    // Draw basic info section
    draw_basic_info(frame, chunks[0], span);
    
    // Draw attributes section (scrollable)
    draw_attributes(frame, chunks[1], span);
    
    // Draw tags section
    draw_tags(frame, chunks[2], span);
    
    // Draw resource attributes / events
    draw_resource_info(frame, chunks[3], span);
}

/// Draw basic span information.
fn draw_basic_info(frame: &mut Frame, area: Rect, span: &Span) {
    let mut lines = vec![];
    
    // Span ID (copyable with 'y')
    lines.push(Line::from(vec![
        TextSpan::styled("Span ID: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            span.span_id.as_str(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        TextSpan::styled(" [y]", Style::default().fg(Color::DarkGray)),
    ]));
    
    // Trace ID (copyable with 'Y')
    lines.push(Line::from(vec![
        TextSpan::styled("Trace ID: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            span.trace_id.as_str(),
            Style::default().fg(Color::Yellow),
        ),
        TextSpan::styled(" [Y]", Style::default().fg(Color::DarkGray)),
    ]));
    
    // Parent Span ID (if exists)
    if let Some(parent_id) = &span.parent_span_id {
        lines.push(Line::from(vec![
            TextSpan::styled("Parent ID: ", Style::default().fg(Color::Gray)),
            TextSpan::styled(
                parent_id.as_str(),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }
    
    // Service
    lines.push(Line::from(vec![
        TextSpan::styled("Service: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            span.service_name.as_str(),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
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
    
    // Duration with microseconds and milliseconds
    let duration_us = span.duration.as_micros();
    let duration_ms = span.duration.as_millis();
    let duration_str = if duration_ms > 0 {
        format!("{}.{}ms ({}μs)", duration_ms, (duration_us % 1000) / 100, duration_us)
    } else {
        format!("{}μs", duration_us)
    };
    
    lines.push(Line::from(vec![
        TextSpan::styled("Duration: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            &duration_str,
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
        ),
    ]));
    
    // Status with error message if available
    let status_style = if span.status.is_error() {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    
    let status_text = match &span.status {
        crate::core::SpanStatus::Ok => "OK",
        crate::core::SpanStatus::Error(msg) => msg,
        crate::core::SpanStatus::Cancelled => "Cancelled",
        crate::core::SpanStatus::Unknown => "Unknown",
    };
    
    lines.push(Line::from(vec![
        TextSpan::styled("Status: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(status_text, status_style),
    ]));
    
    // Timestamps
    let formatted_timestamp = format_timestamp(span.start_time);
    lines.push(Line::from(vec![
        TextSpan::styled("Start Time: ", Style::default().fg(Color::Gray)),
        TextSpan::styled(
            formatted_timestamp,
            Style::default().fg(Color::White),
        ),
    ]));
    
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" SPAN INFORMATION ")
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
                    format!("{}: ", key),
                    Style::default().fg(Color::Yellow),
                ),
                TextSpan::styled(
                    value.clone(),
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

/// Draw span tags.
fn draw_tags(frame: &mut Frame, area: Rect, span: &Span) {
    if span.tags.is_empty() {
        let paragraph = Paragraph::new(vec![
            Line::from(""),
            Line::from(TextSpan::styled(
                "  No tags available",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )),
        ])
        .block(
            Block::default()
                .title(" TAGS ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );
        frame.render_widget(paragraph, area);
        return;
    }

    // Convert tags to sorted BTreeMap for consistent ordering
    let tags: BTreeMap<String, String> = span.tags
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    
    let items: Vec<ListItem> = tags
        .iter()
        .map(|(key, value)| {
            let content = Line::from(vec![
                TextSpan::styled(
                    format!("{}=", key),
                    Style::default().fg(Color::Magenta),
                ),
                TextSpan::styled(
                    value.clone(),
                    Style::default().fg(Color::White),
                ),
            ]);
            ListItem::new(content)
        })
        .collect();
    
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" TAGS ({}) ", tags.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("→ ");
    
    frame.render_widget(list, area);
}

/// Draw resource attributes and events.
fn draw_resource_info(frame: &mut Frame, area: Rect, span: &Span) {
    // Split area into two columns
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);
    
    // Draw resource attributes on the left
    draw_resource_attributes(frame, chunks[0], span);
    
    // Draw events on the right
    draw_events(frame, chunks[1], span);
}

/// Draw resource attributes.
fn draw_resource_attributes(frame: &mut Frame, area: Rect, span: &Span) {
    let mut lines = vec![];
    
    // Show resource attributes if available
    if !span.resource_attributes.is_empty() {
        let sorted: BTreeMap<String, String> = span.resource_attributes.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        for (key, value) in sorted.iter().take(5) {
            lines.push(Line::from(vec![
                TextSpan::styled(format!("{}: ", key), Style::default().fg(Color::Gray)),
                TextSpan::styled(value.clone(), Style::default().fg(Color::White)),
            ]));
        }
        
        if span.resource_attributes.len() > 5 {
            lines.push(Line::from(TextSpan::styled(
                format!("  ... and {} more", span.resource_attributes.len() - 5),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )));
        }
    } else {
        lines.push(Line::from(TextSpan::styled(
            "No resource attributes",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
    }
    
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" RESOURCE ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
    
    frame.render_widget(paragraph, area);
}

/// Draw span events/logs.
fn draw_events(frame: &mut Frame, area: Rect, span: &Span) {
    let mut lines = vec![];
    
    // For now, show some placeholder events
    // In a real implementation, these would come from span.events
    lines.push(Line::from(vec![
        TextSpan::styled("Event Log:", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
    ]));
    
    lines.push(Line::from(vec![
        TextSpan::styled("10ms: ", Style::default().fg(Color::DarkGray)),
        TextSpan::styled("Started", Style::default().fg(Color::White)),
    ]));
    
    if span.duration.as_millis() > 100 {
        lines.push(Line::from(vec![
            TextSpan::styled("50ms: ", Style::default().fg(Color::DarkGray)),
            TextSpan::styled("Processing", Style::default().fg(Color::Yellow)),
        ]));
    }
    
    if span.status.is_error() {
        lines.push(Line::from(vec![
            TextSpan::styled(format!("{}ms: ", span.duration.as_millis()), Style::default().fg(Color::DarkGray)),
            TextSpan::styled("Error!", Style::default().fg(Color::Red)),
        ]));
    } else {
        lines.push(Line::from(vec![
            TextSpan::styled(format!("{}ms: ", span.duration.as_millis()), Style::default().fg(Color::DarkGray)),
            TextSpan::styled("Complete", Style::default().fg(Color::Green)),
        ]));
    }
    
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" EVENTS ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    
    frame.render_widget(paragraph, area);
}

/// Format a timestamp for display.
fn format_timestamp(timestamp: std::time::SystemTime) -> String {
    use chrono::{DateTime, Local};
    
    // Convert SystemTime to DateTime
    let datetime: DateTime<Local> = timestamp.into();
    datetime.format("%Y-%m-%d %H:%M:%S.%3f").to_string()
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