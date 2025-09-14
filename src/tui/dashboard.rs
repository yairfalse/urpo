//! Service health dashboard implementation.
//!
//! This module provides the beautiful, professional service health dashboard
//! that shows real-time metrics in an htop-like interface.

use super::{Dashboard, ReceiverStatus, SortBy};
use crate::core::ServiceMetrics;
use chrono::Local;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use std::time::Duration;

/// Draw the main service dashboard.
pub fn draw_dashboard(frame: &mut Frame, app: &mut Dashboard) {
    let size = frame.area();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header
            Constraint::Length(3), // Stats bar
            Constraint::Min(10),   // Service table
            Constraint::Length(3), // Footer
        ])
        .split(size);

    // Draw components
    draw_header(frame, chunks[0], app);
    draw_stats_bar(frame, chunks[1], app);
    draw_service_table(frame, chunks[2], app);
    draw_footer_bar(frame, chunks[3], app);

    // Draw search overlay if active
    if app.search_active {
        draw_search_overlay(frame, &app.search_query);
    }
}

/// Draw the dashboard header.
fn draw_header(frame: &mut Frame, area: Rect, app: &Dashboard) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // Urpo title
            Constraint::Min(20),    // Center stats
            Constraint::Length(25), // Time
        ])
        .split(area);

    // Left: Urpo branding
    let title = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  URPO ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("v0.1.0", Style::default().fg(Color::DarkGray)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(title, chunks[0]);

    // Center: Service stats
    let total_rps = app.get_total_rps();
    let error_rate = app.get_overall_error_rate() * 100.0;
    let receiver_status = match app.receiver_status {
        ReceiverStatus::Connected => ("●", Color::Green, "Connected"),
        ReceiverStatus::Listening => ("●", Color::Yellow, "Listening"),
        ReceiverStatus::Disconnected => ("○", Color::Red, "Disconnected"),
    };

    let stats = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Services: "),
            Span::styled(
                format!("{}", app.get_filtered_services().len()),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | RPS: "),
            Span::styled(
                format!("{:.0}", total_rps),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | Errors: "),
            Span::styled(
                format!("{:.1}%", error_rate),
                Style::default()
                    .fg(if error_rate > 5.0 {
                        Color::Red
                    } else if error_rate > 1.0 {
                        Color::Yellow
                    } else {
                        Color::Green
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | GRPC: "),
            Span::styled(receiver_status.0, Style::default().fg(receiver_status.1)),
            Span::raw(" "),
            Span::styled(receiver_status.2, Style::default().fg(Color::DarkGray)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(stats, chunks[1]);

    // Right: Current time
    let now = Local::now();
    let time_display = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(now.format("%Y-%m-%d").to_string(), Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(
                now.format("%H:%M:%S").to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(time_display, chunks[2]);
}

/// Draw the statistics bar with system health.
fn draw_stats_bar(frame: &mut Frame, area: Rect, app: &Dashboard) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Find top services by different metrics
    let mut services_by_error: Vec<&ServiceMetrics> = app.services.iter().collect();
    services_by_error.sort_by(|a, b| {
        b.error_rate.partial_cmp(&a.error_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut services_by_latency: Vec<&ServiceMetrics> = app.services.iter().collect();
    services_by_latency.sort_by(|a, b| b.latency_p99.cmp(&a.latency_p99));

    // Top error service
    let error_info = if let Some(worst) = services_by_error.first() {
        if worst.error_rate > 0.0 {
            format!("{}: {:.1}%", worst.name.as_str(), worst.error_rate * 100.0)
        } else {
            "All healthy".to_string()
        }
    } else {
        "No services".to_string()
    };

    let error_widget = Paragraph::new(vec![Line::from(vec![
        Span::raw(" ⚠ Highest Error: "),
        Span::styled(error_info, Style::default().fg(Color::Yellow)),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(error_widget, chunks[0]);

    // Slowest service
    let latency_info = if let Some(slowest) = services_by_latency.first() {
        format!("{}: {}ms", slowest.name.as_str(), slowest.latency_p99.as_millis())
    } else {
        "No services".to_string()
    };

    let latency_widget = Paragraph::new(vec![Line::from(vec![
        Span::raw(" ⏱ Slowest P99: "),
        Span::styled(latency_info, Style::default().fg(Color::Magenta)),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(latency_widget, chunks[1]);

    // Current sort and filter
    let sort_filter = Paragraph::new(vec![Line::from(vec![
        Span::raw(" Sort: "),
        Span::styled(
            format!("{} {}", app.sort_by.as_str(), if app.sort_desc { "↓" } else { "↑" }),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(" | Filter: "),
        Span::styled(app.filter_mode.as_str(), Style::default().fg(Color::Cyan)),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(sort_filter, chunks[2]);

    // System health status (production monitoring)
    let (health_icon, health_color, health_text) = get_system_health_info();
    let health_info = Paragraph::new(vec![Line::from(vec![
        Span::raw(" "),
        Span::styled(health_icon, Style::default().fg(health_color)),
        Span::raw(" System: "),
        Span::styled(health_text, Style::default().fg(health_color)),
        Span::raw(" | "),
        Span::styled(
            format!("{}MB", app.memory_usage_mb as u32),
            Style::default().fg(if app.memory_usage_mb > 256.0 {
                Color::Red
            } else {
                Color::Green
            }),
        ),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(health_info, chunks[3]);
}

/// Get system health information for display.
fn get_system_health_info() -> (&'static str, Color, &'static str) {
    // In a real implementation, this would check actual system health
    // For now, return placeholder values based on simple heuristics

    // Simulate health check (this would use the monitoring module)
    let health_status = "Healthy"; // SystemHealth::Healthy, Degraded, Unhealthy, Critical

    match health_status {
        "Healthy" => ("●", Color::Green, "Healthy"),
        "Degraded" => ("●", Color::Yellow, "Degraded"),
        "Unhealthy" => ("●", Color::Red, "Unhealthy"),
        "Critical" => ("●", Color::Magenta, "Critical"),
        _ => ("○", Color::Gray, "Unknown"),
    }
}

/// Draw the main service table.
fn draw_service_table(frame: &mut Frame, area: Rect, app: &mut Dashboard) {
    let services = app.get_filtered_services();

    // Create header row
    let header_items = [
        ("Service", false),
        ("RPS", app.sort_by == SortBy::Rps),
        ("Trend", false),
        ("Error%", app.sort_by == SortBy::ErrorRate),
        ("P50", app.sort_by == SortBy::P50),
        ("P95", app.sort_by == SortBy::P95),
        ("P99", app.sort_by == SortBy::P99),
        ("Health", false),
    ];

    let header_cells: Vec<Cell> = header_items
        .into_iter()
        .map(|(title, is_sorted)| {
            let mut cell_text = title.to_string();
            if is_sorted {
                cell_text.push_str(if app.sort_desc { " ↓" } else { " ↑" });
            }
            Cell::from(cell_text).style(
                Style::default()
                    .fg(if is_sorted {
                        Color::Cyan
                    } else {
                        Color::Yellow
                    })
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    // Create data rows
    let rows = services.iter().enumerate().map(|(idx, service)| {
        let selected = app.service_state.selected() == Some(idx);

        // Determine health status
        let error_rate_pct = service.error_rate * 100.0;
        let (health_symbol, health_color) = if error_rate_pct > 5.0 {
            ("✖", Color::Red)
        } else if error_rate_pct > 1.0 {
            ("⚠", Color::Yellow)
        } else {
            ("●", Color::Green)
        };

        // Format latencies
        let format_latency = |duration: Duration| {
            let ms = duration.as_millis();
            if ms >= 1000 {
                format!("{:.1}s", ms as f64 / 1000.0)
            } else {
                format!("{}ms", ms)
            }
        };

        // Get sparkline data for this service
        let sparkline_data = app
            .rps_history
            .get(service.name.as_str())
            .map(|entry| {
                let data: Vec<u64> = entry.iter().map(|v| *v as u64).collect();
                create_mini_sparkline(&data)
            })
            .unwrap_or_else(|| "     ".to_string());

        // Service name with selection indicator
        let service_name = if selected {
            format!("► {}", service.name.as_str())
        } else {
            format!("  {}", service.name.as_str())
        };

        // Determine if service is inactive (no recent requests)
        let is_inactive = service.request_rate < 0.1;
        let base_style = if is_inactive {
            Style::default().fg(Color::DarkGray)
        } else if selected {
            Style::default().fg(Color::White)
        } else {
            Style::default()
        };

        // Color code error rate
        let error_style = Style::default().fg(if error_rate_pct > 5.0 {
            Color::Red
        } else if error_rate_pct > 1.0 {
            Color::Yellow
        } else {
            Color::Green
        });

        // Color code latencies
        let latency_color = |ms: u128| {
            if ms > 1000 {
                Color::Red
            } else if ms > 500 {
                Color::Yellow
            } else {
                Color::Green
            }
        };

        Row::new(vec![
            Cell::from(service_name).style(base_style.add_modifier(Modifier::BOLD)),
            Cell::from(format!("{:.1}", service.request_rate)).style(base_style),
            Cell::from(sparkline_data).style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{:.2}%", error_rate_pct)).style(error_style),
            Cell::from(format_latency(service.latency_p50))
                .style(Style::default().fg(latency_color(service.latency_p50.as_millis()))),
            Cell::from(format_latency(service.latency_p95))
                .style(Style::default().fg(latency_color(service.latency_p95.as_millis()))),
            Cell::from(format_latency(service.latency_p99))
                .style(Style::default().fg(latency_color(service.latency_p99.as_millis()))),
            Cell::from(health_symbol).style(Style::default().fg(health_color)),
        ])
        .style(if selected {
            Style::default().bg(Color::DarkGray)
        } else if idx % 2 == 0 {
            Style::default().bg(Color::Black)
        } else {
            Style::default()
        })
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25), // Service
            Constraint::Length(8),      // RPS
            Constraint::Length(7),      // Trend
            Constraint::Length(9),      // Error%
            Constraint::Length(8),      // P50
            Constraint::Length(8),      // P95
            Constraint::Length(8),      // P99
            Constraint::Length(6),      // Health
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(format!(" Services ({}/{}) ", services.len(), app.services.len()))
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_stateful_widget(table, area, &mut app.service_state);
}

/// Draw the footer bar.
fn draw_footer_bar(frame: &mut Frame, area: Rect, app: &Dashboard) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(50), Constraint::Length(40)])
        .split(area);

    // Keyboard shortcuts
    let shortcuts = if app.search_active {
        vec![Line::from(vec![
            Span::raw(" Search: "),
            Span::styled(&app.search_query, Style::default().fg(Color::Cyan)),
            Span::raw(" | "),
            Span::styled("ESC", Style::default().fg(Color::Yellow)),
            Span::raw(" Cancel | "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" Dashboardly"),
        ])]
    } else {
        vec![Line::from(vec![
            Span::raw(" "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit | "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate | "),
            Span::styled("s", Style::default().fg(Color::Yellow)),
            Span::raw(" Sort | "),
            Span::styled("f", Style::default().fg(Color::Yellow)),
            Span::raw(" Filter | "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" Search | "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" Details | "),
            Span::styled("h", Style::default().fg(Color::Yellow)),
            Span::raw(" Help"),
        ])]
    };

    let shortcuts_widget = Paragraph::new(shortcuts)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Left);
    frame.render_widget(shortcuts_widget, chunks[0]);

    // System stats
    let memory_pct = (app.memory_usage_mb / 1000.0 * 100.0).min(100.0) as u16;
    let memory_color = if memory_pct > 80 {
        Color::Red
    } else if memory_pct > 60 {
        Color::Yellow
    } else {
        Color::Green
    };

    let system_stats = Paragraph::new(vec![Line::from(vec![
        Span::raw(" Memory: "),
        Span::styled(format!("{:.0}MB", app.memory_usage_mb), Style::default().fg(memory_color)),
        Span::raw(" | Processing: "),
        Span::styled(format!("{:.0}/s", app.spans_per_sec), Style::default().fg(Color::Green)),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .alignment(Alignment::Right);
    frame.render_widget(system_stats, chunks[1]);
}

/// Draw search overlay.
fn draw_search_overlay(frame: &mut Frame, query: &str) {
    let size = frame.area();

    // Create centered search box
    let search_width = 60;
    let search_height = 3;
    let x = (size.width.saturating_sub(search_width)) / 2;
    let y = size.height / 3;

    let search_area = Rect::new(x, y, search_width, search_height);

    let search_text = vec![Line::from(vec![
        Span::raw(" Search: "),
        Span::styled(query, Style::default().fg(Color::Cyan)),
        Span::styled(
            "_",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::SLOW_BLINK),
        ),
    ])];

    let search_box = Paragraph::new(search_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" Filter Services ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Black));

    frame.render_widget(search_box, search_area);
}

/// Create a mini sparkline string from data.
fn create_mini_sparkline(data: &[u64]) -> String {
    if data.is_empty() {
        return "     ".to_string();
    }

    let spark_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = *data.iter().max().unwrap_or(&1).max(&1) as f64;

    let last_5: Vec<char> = data
        .iter()
        .rev()
        .take(5)
        .rev()
        .map(|&v| {
            let normalized = (v as f64 / max * 7.0) as usize;
            spark_chars[normalized.min(7)]
        })
        .collect();

    last_5.iter().collect()
}
