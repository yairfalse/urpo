//! Logs viewer component for TUI
//!
//! Interactive logs viewer with search, filtering, and trace correlation.

use crate::logs::{storage::LogStorage, types::{LogRecord, LogSeverity}};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Logs viewer component
pub struct LogsView {
    /// Logs storage backend
    storage: Arc<Mutex<LogStorage>>,
    /// Cached logs data
    logs: Vec<LogRecord>,
    /// Current search query
    search_query: String,
    /// Currently selected log index
    selected_index: usize,
    /// Severity filter
    severity_filter: Option<LogSeverity>,
    /// Search mode active
    search_active: bool,
    /// Last update timestamp
    last_update: std::time::Instant,
}

impl LogsView {
    /// Create new logs viewer
    pub fn new(storage: Arc<Mutex<LogStorage>>) -> Self {
        Self {
            storage,
            logs: Vec::new(),
            search_query: String::new(),
            selected_index: 0,
            severity_filter: None,
            search_active: false,
            last_update: std::time::Instant::now(),
        }
    }

    /// Update logs from storage
    pub async fn update_logs(&mut self) -> Result<(), String> {
        // Only update if 500ms has passed for real-time feel
        if self.last_update.elapsed().as_millis() < 500 {
            return Ok(());
        }

        let storage = self.storage.lock().await;

        self.logs = if !self.search_query.is_empty() {
            // Search mode
            storage.search_logs(&self.search_query, 1000)
        } else if let Some(severity) = self.severity_filter {
            // Filter by severity
            storage.filter_by_severity(severity, 1000)
        } else {
            // Show recent logs
            storage.get_recent_logs(1000)
        };

        self.last_update = std::time::Instant::now();
        Ok(())
    }

    /// Render the logs viewer
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar
                Constraint::Min(0),    // Logs table
                Constraint::Length(2), // Status bar
            ])
            .split(area);

        // Render search bar
        self.render_search_bar(f, chunks[0]);

        // Render logs table
        self.render_logs_table(f, chunks[1]);

        // Render status bar
        self.render_status_bar(f, chunks[2]);
    }

    /// Render search bar
    fn render_search_bar(&self, f: &mut Frame, area: Rect) {
        let search_text = if self.search_active {
            format!("Search: {}█", self.search_query)
        } else if !self.search_query.is_empty() {
            format!("Search: {} (Press / to edit)", self.search_query)
        } else {
            "Press / to search logs".to_string()
        };

        let search_style = if self.search_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let search_bar = Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Search ")
                    .border_style(if self.search_active {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Gray)
                    }),
            )
            .style(search_style);

        f.render_widget(search_bar, area);
    }

    /// Render logs table
    fn render_logs_table(&self, f: &mut Frame, area: Rect) {
        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);

        let selected_style = Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD);

        // Create header
        let header = Row::new(vec![
            Cell::from("Time"),
            Cell::from("Level"),
            Cell::from("Service"),
            Cell::from("Message"),
            Cell::from("Trace"),
        ])
        .style(header_style)
        .height(1);

        // Create rows
        let rows: Vec<Row> = self.logs
            .iter()
            .enumerate()
            .map(|(i, log)| {
                let style = if i == self.selected_index {
                    selected_style
                } else {
                    Style::default()
                };

                let severity_color = match log.severity {
                    LogSeverity::Fatal | LogSeverity::Error => Color::Red,
                    LogSeverity::Warn => Color::Yellow,
                    LogSeverity::Info => Color::Green,
                    LogSeverity::Debug => Color::Blue,
                    LogSeverity::Trace => Color::Magenta,
                };

                let time_str = format_timestamp(log.timestamp);
                let service_str = format!("service-{}", log.service_id);
                let trace_str = log.trace_id
                    .as_ref()
                    .map(|t| t.as_str()[..8.min(t.as_str().len())].to_string())
                    .unwrap_or_else(|| "-".to_string());

                // Truncate message if too long
                let message = if log.body.len() > 80 {
                    format!("{}...", &log.body[..77])
                } else {
                    log.body.clone()
                };

                Row::new(vec![
                    Cell::from(time_str),
                    Cell::from(Span::styled(
                        log.severity.as_str(),
                        Style::default().fg(severity_color).add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(service_str),
                    Cell::from(message),
                    Cell::from(trace_str),
                ])
                .style(style)
            })
            .collect();

        // Create table
        let table = Table::new(rows)
            .header(header)
            .block(
                Block::default()
                    .title(format!(" Logs ({}) ", self.logs.len()))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White)),
            )
            .widths(&[
                Constraint::Length(12), // Time
                Constraint::Length(8),  // Level
                Constraint::Length(15), // Service
                Constraint::Min(30),    // Message
                Constraint::Length(10), // Trace
            ]);

        f.render_widget(table, area);

        // Show empty state if no logs
        if self.logs.is_empty() {
            let empty_msg = if !self.search_query.is_empty() {
                vec![
                    Line::from(""),
                    Line::from(format!("No logs found for: \"{}\"", self.search_query)),
                    Line::from(""),
                    Line::from("Try a different search term"),
                ]
            } else {
                vec![
                    Line::from(""),
                    Line::from("No logs available"),
                    Line::from(""),
                    Line::from("Waiting for log data..."),
                ]
            };

            let paragraph = Paragraph::new(empty_msg)
                .block(
                    Block::default()
                        .title(" Logs ")
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::Gray))
                .alignment(ratatui::layout::Alignment::Center);

            f.render_widget(paragraph, area);
        }
    }

    /// Render status bar
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_text = if let Some(severity) = self.severity_filter {
            format!("Filter: {} | Total: {} logs", severity.as_str(), self.logs.len())
        } else {
            format!("Total: {} logs | Press [1-6] to filter by severity", self.logs.len())
        };

        let status_bar = Paragraph::new(status_text)
            .style(Style::default().fg(Color::White))
            .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(status_bar, area);
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: char) {
        if self.search_active {
            match key {
                '\x1b' => { // Escape
                    self.search_active = false;
                },
                '\n' => { // Enter
                    self.search_active = false;
                },
                '\x08' => { // Backspace
                    self.search_query.pop();
                },
                c if c.is_ascii() && !c.is_control() => {
                    self.search_query.push(c);
                },
                _ => {},
            }
        } else {
            match key {
                '/' => {
                    self.search_active = true;
                },
                'c' => {
                    self.search_query.clear();
                    self.severity_filter = None;
                },
                '1' => self.severity_filter = Some(LogSeverity::Fatal),
                '2' => self.severity_filter = Some(LogSeverity::Error),
                '3' => self.severity_filter = Some(LogSeverity::Warn),
                '4' => self.severity_filter = Some(LogSeverity::Info),
                '5' => self.severity_filter = Some(LogSeverity::Debug),
                '6' => self.severity_filter = Some(LogSeverity::Trace),
                '0' => self.severity_filter = None,
                'j' | 'J' => self.move_down(),
                'k' | 'K' => self.move_up(),
                'g' => self.move_to_top(),
                'G' => self.move_to_bottom(),
                _ => {},
            }
        }
    }

    fn move_down(&mut self) {
        if !self.logs.is_empty() && self.selected_index < self.logs.len() - 1 {
            self.selected_index += 1;
        }
    }

    fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn move_to_top(&mut self) {
        self.selected_index = 0;
    }

    fn move_to_bottom(&mut self) {
        if !self.logs.is_empty() {
            self.selected_index = self.logs.len() - 1;
        }
    }

    /// Get currently selected log
    pub fn selected_log(&self) -> Option<&LogRecord> {
        self.logs.get(self.selected_index)
    }

    /// Get search query
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Get severity filter
    pub fn severity_filter(&self) -> Option<LogSeverity> {
        self.severity_filter
    }
}

/// Format timestamp for display
fn format_timestamp(timestamp_nanos: u64) -> String {
    let timestamp_secs = timestamp_nanos / 1_000_000_000;
    let dt = chrono::DateTime::from_timestamp(timestamp_secs as i64, 0)
        .unwrap_or_else(|| chrono::Utc::now());
    dt.format("%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::storage::LogStorageConfig;

    async fn create_test_view() -> LogsView {
        let storage = Arc::new(Mutex::new(LogStorage::new(LogStorageConfig::default())));
        LogsView::new(storage)
    }

    #[tokio::test]
    async fn test_logs_view_creation() {
        let view = create_test_view().await;
        assert_eq!(view.selected_index, 0);
        assert!(view.logs.is_empty());
        assert_eq!(view.search_query, "");
        assert!(view.severity_filter.is_none());
    }

    #[tokio::test]
    async fn test_update_logs_empty() {
        let mut view = create_test_view().await;
        let result = view.update_logs().await;
        assert!(result.is_ok());
        assert!(view.logs.is_empty());
    }

    #[tokio::test]
    async fn test_search_functionality() {
        let storage = Arc::new(Mutex::new(LogStorage::new(LogStorageConfig::default())));

        // Add test logs
        {
            let mut storage_guard = storage.lock().await;
            let log1 = LogRecord::new(1234567890, 1, LogSeverity::Info, "User login successful".to_string());
            let log2 = LogRecord::new(1234567891, 1, LogSeverity::Error, "Database connection failed".to_string());
            storage_guard.store_log(log1).unwrap();
            storage_guard.store_log(log2).unwrap();
        }

        let mut view = LogsView::new(storage);
        view.search_query = "database".to_string();
        view.update_logs().await.unwrap();

        assert_eq!(view.logs.len(), 1);
        assert!(view.logs[0].body.contains("Database"));
    }

    #[test]
    fn test_keyboard_navigation() {
        let storage = Arc::new(Mutex::new(LogStorage::new(LogStorageConfig::default())));
        let mut view = LogsView {
            storage,
            logs: vec![
                LogRecord::new(1, 1, LogSeverity::Info, "Log 1".to_string()),
                LogRecord::new(2, 1, LogSeverity::Info, "Log 2".to_string()),
                LogRecord::new(3, 1, LogSeverity::Info, "Log 3".to_string()),
            ],
            selected_index: 0,
            search_query: String::new(),
            severity_filter: None,
            search_active: false,
            last_update: std::time::Instant::now(),
        };

        view.handle_key('j');
        assert_eq!(view.selected_index, 1);

        view.handle_key('j');
        assert_eq!(view.selected_index, 2);

        // Should not go beyond last item
        view.handle_key('j');
        assert_eq!(view.selected_index, 2);

        view.handle_key('k');
        assert_eq!(view.selected_index, 1);

        view.handle_key('g');
        assert_eq!(view.selected_index, 0);

        view.handle_key('G');
        assert_eq!(view.selected_index, 2);
    }

    #[test]
    fn test_search_mode() {
        let storage = Arc::new(Mutex::new(LogStorage::new(LogStorageConfig::default())));
        let mut view = LogsView::new(storage);

        assert!(!view.search_active);

        view.handle_key('/');
        assert!(view.search_active);

        view.handle_key('t');
        view.handle_key('e');
        view.handle_key('s');
        view.handle_key('t');
        assert_eq!(view.search_query, "test");

        view.handle_key('\x08'); // Backspace
        assert_eq!(view.search_query, "tes");

        view.handle_key('\x1b'); // Escape
        assert!(!view.search_active);
    }

    #[test]
    fn test_severity_filtering() {
        let storage = Arc::new(Mutex::new(LogStorage::new(LogStorageConfig::default())));
        let mut view = LogsView::new(storage);

        view.handle_key('2');
        assert_eq!(view.severity_filter, Some(LogSeverity::Error));

        view.handle_key('3');
        assert_eq!(view.severity_filter, Some(LogSeverity::Warn));

        view.handle_key('0');
        assert!(view.severity_filter.is_none());
    }

    #[test]
    fn test_format_timestamp() {
        let timestamp = 1234567890_000_000_000; // 2009-02-13 23:31:30 UTC
        let formatted = format_timestamp(timestamp);
        assert_eq!(formatted.len(), 8); // HH:MM:SS format
        assert!(formatted.contains(':'));
    }

    #[test]
    fn test_selected_log() {
        let storage = Arc::new(Mutex::new(LogStorage::new(LogStorageConfig::default())));
        let view = LogsView {
            storage,
            logs: vec![
                LogRecord::new(1, 1, LogSeverity::Info, "Log 1".to_string()),
                LogRecord::new(2, 1, LogSeverity::Info, "Log 2".to_string()),
            ],
            selected_index: 1,
            search_query: String::new(),
            severity_filter: None,
            search_active: false,
            last_update: std::time::Instant::now(),
        };

        let selected = view.selected_log().unwrap();
        assert_eq!(selected.body, "Log 2");
    }
}