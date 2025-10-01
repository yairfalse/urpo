//! Settings panel for viewing and displaying configuration

use crate::core::Config;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Draw the settings panel
pub fn draw_settings(frame: &mut Frame, area: Rect, config: &Config) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Help
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Settings")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Settings content
    let items: Vec<ListItem> = vec![
        create_section("Server Configuration"),
        create_item("GRPC Port", &config.server.grpc_port.to_string()),
        create_item("HTTP Port", &config.server.http_port.to_string()),
        create_item("Bind Address", &config.server.bind_address.to_string()),
        create_item("Max Connections", &config.server.max_connections.to_string()),
        create_empty(),
        create_section("Storage Configuration"),
        create_item("Max Spans", &config.storage.max_spans.to_string()),
        create_item("Max Memory", &format!("{}MB", config.storage.max_memory_mb)),
        create_item(
            "Retention",
            &format!("{}s", config.storage.retention_duration.as_secs()),
        ),
        create_item("Compression", if config.storage.compression_enabled { "Enabled" } else { "Disabled" }),
        create_item("Persistent", if config.storage.persistent { "Enabled" } else { "Disabled" }),
        create_empty(),
        create_section("UI Configuration"),
        create_item(
            "Refresh Rate",
            &format!("{}ms", config.ui.refresh_rate.as_millis()),
        ),
        create_item("Theme", &format!("{:?}", config.ui.theme)),
        create_item("Vim Mode", if config.ui.vim_mode { "Enabled" } else { "Disabled" }),
        create_empty(),
        create_section("Sampling Configuration"),
        create_item("Default Rate", &format!("{:.1}%", config.sampling.default_rate * 100.0)),
        create_item("Adaptive", if config.sampling.adaptive { "Enabled" } else { "Disabled" }),
        create_empty(),
        create_section("Monitoring Configuration"),
        create_item("Metrics", if config.monitoring.metrics_enabled { "Enabled" } else { "Disabled" }),
        create_item("Error Threshold", &format!("{:.1}%", config.monitoring.alerts.error_rate_threshold)),
        create_empty(),
        create_info("💡 To modify settings, edit ~/.config/urpo/config.yaml"),
        create_info("   or use --grpc-port / --http-port CLI flags"),
    ];

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Configuration"))
        .style(Style::default().fg(Color::White));

    frame.render_widget(list, chunks[1]);

    // Help footer
    let help = Paragraph::new(" [Esc]back [q]quit ")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(help, chunks[2]);
}

/// Create a section header
fn create_section(title: &str) -> ListItem<'static> {
    ListItem::new(Line::from(vec![Span::styled(
        format!("━━ {} ", title),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
}

/// Create a setting item
fn create_item(key: &str, value: &str) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(
            format!("  {:.<25} ", key),
            Style::default().fg(Color::Gray),
        ),
        Span::styled(
            value.to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
}

/// Create an info message
fn create_info(msg: &str) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        msg.to_string(),
        Style::default().fg(Color::Blue),
    )))
}

/// Create an empty line
fn create_empty() -> ListItem<'static> {
    ListItem::new(Line::from(""))
}
