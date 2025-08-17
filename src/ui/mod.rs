//! Terminal user interface for Urpo.
//!
//! This module provides the interactive terminal UI using ratatui
//! for real-time trace exploration and service health monitoring.

mod dashboard;
mod fake_data;
mod widgets;

use crate::core::{Result, ServiceMetrics, Span, UrpoError};
use crate::storage::StorageBackend;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fake_data::FakeDataGenerator;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span as TextSpan},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use dashboard::draw_dashboard;
pub use widgets::{health_symbol, sparkline_trend};

/// Sorting options for the service table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Rps,
    ErrorRate,
    P50,
    P95,
    P99,
}

impl SortBy {
    fn next(self) -> Self {
        match self {
            Self::Name => Self::Rps,
            Self::Rps => Self::ErrorRate,
            Self::ErrorRate => Self::P50,
            Self::P50 => Self::P95,
            Self::P95 => Self::P99,
            Self::P99 => Self::Name,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Rps => "RPS",
            Self::ErrorRate => "Error%",
            Self::P50 => "P50",
            Self::P95 => "P95",
            Self::P99 => "P99",
        }
    }
}

/// Filter mode for services.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    ErrorsOnly,
    SlowOnly,
    Active,
}

impl FilterMode {
    fn next(self) -> Self {
        match self {
            Self::All => Self::ErrorsOnly,
            Self::ErrorsOnly => Self::SlowOnly,
            Self::SlowOnly => Self::Active,
            Self::Active => Self::All,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::ErrorsOnly => "Errors",
            Self::SlowOnly => "Slow",
            Self::Active => "Active",
        }
    }
}

/// Main UI application state.
pub struct App {
    /// Whether the application should quit.
    pub should_quit: bool,
    /// Currently selected tab.
    pub selected_tab: Tab,
    /// Service list state.
    pub service_state: TableState,
    /// Trace list state.
    pub trace_state: TableState,
    /// Current services data.
    pub services: Vec<ServiceMetrics>,
    /// Historical RPS data for sparklines (service_name -> last 60 values).
    pub rps_history: dashmap::DashMap<String, VecDeque<f64>>,
    /// Current traces data.
    pub traces: Vec<Span>,
    /// Search query.
    pub search_query: String,
    /// Whether search mode is active.
    pub search_active: bool,
    /// Current sort mode.
    pub sort_by: SortBy,
    /// Sort descending.
    pub sort_desc: bool,
    /// Filter mode.
    pub filter_mode: FilterMode,
    /// Show help panel.
    pub show_help: bool,
    /// Storage backend for real data.
    pub storage: Option<Arc<dyn StorageBackend>>,
    /// Fake data generator for demo mode.
    pub fake_generator: FakeDataGenerator,
    /// Total spans processed.
    pub total_spans: u64,
    /// Spans processing rate.
    pub spans_per_sec: f64,
    /// Memory usage in MB.
    pub memory_usage_mb: f64,
    /// Time of last data update.
    pub last_update: Instant,
    /// GRPC receiver status.
    pub receiver_status: ReceiverStatus,
}

/// Status of the GRPC receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverStatus {
    Connected,
    Listening,
    Disconnected,
}

/// Available UI tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Service health overview.
    Services,
    /// Trace list view.
    Traces,
    /// Detailed span view.
    Spans,
}

impl App {
    /// Create a new UI application.
    pub fn new() -> Self {
        let mut service_state = TableState::default();
        service_state.select(Some(0));

        let mut trace_state = TableState::default();
        trace_state.select(Some(0));

        let mut app = Self {
            should_quit: false,
            selected_tab: Tab::Services,
            service_state,
            trace_state,
            services: Vec::new(),
            rps_history: dashmap::DashMap::new(),
            traces: Vec::new(),
            search_query: String::new(),
            search_active: false,
            sort_by: SortBy::Rps,
            sort_desc: true,
            filter_mode: FilterMode::All,
            show_help: false,
            storage: None,
            fake_generator: FakeDataGenerator::new(),
            total_spans: 0,
            spans_per_sec: 0.0,
            memory_usage_mb: 45.0, // Mock value
            last_update: Instant::now(),
            receiver_status: ReceiverStatus::Listening,
        };

        // Initialize with fake data
        app.services = app.fake_generator.generate_metrics();
        app.traces = app.fake_generator.generate_traces(20);
        app.update_rps_history();
        app
    }

    /// Create a new UI application with storage backend.
    pub fn with_storage(storage: Arc<dyn StorageBackend>) -> Self {
        let mut app = Self::new();
        app.storage = Some(storage);
        app.receiver_status = ReceiverStatus::Connected;
        app
    }

    /// Update RPS history for sparklines.
    fn update_rps_history(&mut self) {
        for service in &self.services {
            let mut entry = self
                .rps_history
                .entry(service.name.as_str().to_string())
                .or_insert_with(|| VecDeque::with_capacity(60));

            entry.push_back(service.request_rate);
            if entry.len() > 60 {
                entry.pop_front();
            }
        }
    }

    /// Refresh data from storage or fake generator.
    pub async fn refresh_data(&mut self) {
        if let Some(storage) = &self.storage {
            // Get real metrics from storage
            match storage.get_service_metrics().await {
                Ok(metrics) => {
                    self.services = metrics;
                    self.receiver_status = ReceiverStatus::Connected;
                }
                Err(e) => {
                    tracing::warn!("Failed to get metrics from storage: {}", e);
                    // Fall back to fake data
                    self.services = self.fake_generator.generate_metrics();
                    self.receiver_status = ReceiverStatus::Disconnected;
                }
            }
            // For now, still use fake traces (we'll implement trace listing later)
            self.traces = self.fake_generator.generate_traces(20);
        } else {
            // No storage, use fake data
            self.services = self.fake_generator.generate_metrics();
            self.traces = self.fake_generator.generate_traces(20);
        }

        // Update history and stats
        self.update_rps_history();
        self.total_spans += rand::random::<u64>() % 1000 + 500;
        self.spans_per_sec = (rand::random::<f64>() * 500.0) + 800.0;
        self.last_update = Instant::now();

        // Apply sorting and filtering
        self.apply_sort();
        self.apply_filter();
    }

    /// Apply current sort order to services.
    fn apply_sort(&mut self) {
        self.services.sort_by(|a, b| {
            let ordering = match self.sort_by {
                SortBy::Name => a.name.as_str().cmp(b.name.as_str()),
                SortBy::Rps => a.request_rate.partial_cmp(&b.request_rate).unwrap(),
                SortBy::ErrorRate => a.error_rate.partial_cmp(&b.error_rate).unwrap(),
                SortBy::P50 => a.latency_p50.cmp(&b.latency_p50),
                SortBy::P95 => a.latency_p95.cmp(&b.latency_p95),
                SortBy::P99 => a.latency_p99.cmp(&b.latency_p99),
            };

            if self.sort_desc {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }

    /// Apply current filter to services.
    fn apply_filter(&mut self) {
        // This would filter services based on filter_mode
        // For now, we'll keep all services visible
    }

    /// Get filtered services based on search query and filter mode.
    pub fn get_filtered_services(&self) -> Vec<&ServiceMetrics> {
        let mut filtered: Vec<&ServiceMetrics> = self
            .services
            .iter()
            .filter(|s| {
                // Apply search filter
                if !self.search_query.is_empty() {
                    if !s
                        .name
                        .as_str()
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                    {
                        return false;
                    }
                }

                // Apply filter mode
                match self.filter_mode {
                    FilterMode::All => true,
                    FilterMode::ErrorsOnly => s.error_rate > 0.01,
                    FilterMode::SlowOnly => s.latency_p95 > Duration::from_millis(500),
                    FilterMode::Active => s.request_rate > 0.0,
                }
            })
            .collect();

        // Apply sorting
        filtered.sort_by(|a, b| {
            let ordering = match self.sort_by {
                SortBy::Name => a.name.as_str().cmp(b.name.as_str()),
                SortBy::Rps => a.request_rate.partial_cmp(&b.request_rate).unwrap(),
                SortBy::ErrorRate => a.error_rate.partial_cmp(&b.error_rate).unwrap(),
                SortBy::P50 => a.latency_p50.cmp(&b.latency_p50),
                SortBy::P95 => a.latency_p95.cmp(&b.latency_p95),
                SortBy::P99 => a.latency_p99.cmp(&b.latency_p99),
            };

            if self.sort_desc {
                ordering.reverse()
            } else {
                ordering
            }
        });

        filtered
    }

    /// Get total RPS across all services.
    pub fn get_total_rps(&self) -> f64 {
        self.services.iter().map(|s| s.request_rate).sum()
    }

    /// Get overall error rate.
    pub fn get_overall_error_rate(&self) -> f64 {
        if self.services.is_empty() {
            return 0.0;
        }

        let total_requests: f64 = self.services.iter().map(|s| s.request_rate).sum();
        if total_requests == 0.0 {
            return 0.0;
        }

        let total_errors: f64 = self
            .services
            .iter()
            .map(|s| s.request_rate * s.error_rate)
            .sum();

        total_errors / total_requests
    }

    /// Handle keyboard input.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.search_active {
            match key.code {
                KeyCode::Esc => {
                    self.search_active = false;
                    self.search_query.clear();
                }
                KeyCode::Enter => {
                    self.search_active = false;
                    // Apply search filter
                    self.apply_filter();
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.previous_tab(),
            KeyCode::Char('/') => {
                self.search_active = true;
                self.search_query.clear();
            }
            KeyCode::Char('s') => {
                self.sort_by = self.sort_by.next();
                self.apply_sort();
            }
            KeyCode::Char('r') => {
                self.sort_desc = !self.sort_desc;
                self.apply_sort();
            }
            KeyCode::Char('f') => {
                self.filter_mode = self.filter_mode.next();
                self.apply_filter();
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('1') => {
                self.filter_mode = FilterMode::All;
                self.apply_filter();
            }
            KeyCode::Char('2') => {
                self.filter_mode = FilterMode::ErrorsOnly;
                self.apply_filter();
            }
            KeyCode::Char('3') => {
                self.filter_mode = FilterMode::SlowOnly;
                self.apply_filter();
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Home | KeyCode::Char('g') => self.move_to_top(),
            KeyCode::End | KeyCode::Char('G') => self.move_to_bottom(),
            KeyCode::Enter => self.handle_selection(),
            _ => {}
        }
    }

    /// Move to the next tab.
    fn next_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            Tab::Services => Tab::Traces,
            Tab::Traces => Tab::Spans,
            Tab::Spans => Tab::Services,
        };
    }

    /// Move to the previous tab.
    fn previous_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            Tab::Services => Tab::Spans,
            Tab::Traces => Tab::Services,
            Tab::Spans => Tab::Traces,
        };
    }

    /// Move selection up in the current list.
    fn move_selection_up(&mut self) {
        let state = match self.selected_tab {
            Tab::Services => &mut self.service_state,
            Tab::Traces => &mut self.trace_state,
            Tab::Spans => return,
        };

        let selected = state.selected().unwrap_or(0);
        if selected > 0 {
            state.select(Some(selected - 1));
        }
    }

    /// Move selection down in the current list.
    fn move_selection_down(&mut self) {
        match self.selected_tab {
            Tab::Services => {
                let max = self.get_filtered_services().len();
                let selected = self.service_state.selected().unwrap_or(0);
                if selected < max.saturating_sub(1) {
                    self.service_state.select(Some(selected + 1));
                }
            }
            Tab::Traces => {
                let max = self.traces.len();
                let selected = self.trace_state.selected().unwrap_or(0);
                if selected < max.saturating_sub(1) {
                    self.trace_state.select(Some(selected + 1));
                }
            }
            Tab::Spans => {}
        }
    }

    /// Move up by a page.
    fn page_up(&mut self) {
        let state = match self.selected_tab {
            Tab::Services => &mut self.service_state,
            Tab::Traces => &mut self.trace_state,
            Tab::Spans => return,
        };

        let selected = state.selected().unwrap_or(0);
        let new_selected = selected.saturating_sub(10);
        state.select(Some(new_selected));
    }

    /// Move down by a page.
    fn page_down(&mut self) {
        match self.selected_tab {
            Tab::Services => {
                let max = self.get_filtered_services().len();
                let selected = self.service_state.selected().unwrap_or(0);
                let new_selected = (selected + 10).min(max.saturating_sub(1));
                self.service_state.select(Some(new_selected));
            }
            Tab::Traces => {
                let max = self.traces.len();
                let selected = self.trace_state.selected().unwrap_or(0);
                let new_selected = (selected + 10).min(max.saturating_sub(1));
                self.trace_state.select(Some(new_selected));
            }
            Tab::Spans => {}
        }
    }

    /// Move to the top of the current list.
    fn move_to_top(&mut self) {
        match self.selected_tab {
            Tab::Services => self.service_state.select(Some(0)),
            Tab::Traces => self.trace_state.select(Some(0)),
            Tab::Spans => {}
        }
    }

    /// Move to the bottom of the current list.
    fn move_to_bottom(&mut self) {
        match self.selected_tab {
            Tab::Services => {
                let len = self.get_filtered_services().len();
                if len > 0 {
                    self.service_state.select(Some(len - 1));
                }
            }
            Tab::Traces => {
                if !self.traces.is_empty() {
                    self.trace_state.select(Some(self.traces.len() - 1));
                }
            }
            Tab::Spans => {}
        }
    }

    /// Handle selection action (Enter key).
    fn handle_selection(&mut self) {
        // This would typically navigate to a detail view
        match self.selected_tab {
            Tab::Services => {
                // Navigate to traces for selected service
                self.selected_tab = Tab::Traces;
            }
            Tab::Traces => {
                // Navigate to spans for selected trace
                self.selected_tab = Tab::Spans;
            }
            Tab::Spans => {
                // Already in detail view
            }
        }
    }

    /// Update services data.
    pub fn update_services(&mut self, services: Vec<ServiceMetrics>) {
        self.services = services;
        self.update_rps_history();
        self.apply_sort();
        self.apply_filter();
    }

    /// Update traces data.
    pub fn update_traces(&mut self, traces: Vec<Span>) {
        self.traces = traces;
    }
}

/// Terminal UI manager.
pub struct TerminalUI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalUI {
    /// Create a new terminal UI.
    pub fn new() -> Result<Self> {
        enable_raw_mode()
            .map_err(|e| UrpoError::render(format!("Failed to enable raw mode: {}", e)))?;

        let mut stdout = io::stdout();
        stdout
            .execute(EnterAlternateScreen)
            .map_err(|e| UrpoError::render(format!("Failed to enter alternate screen: {}", e)))?;

        let backend = CrosstermBackend::new(stdout);
        let terminal =
            Terminal::new(backend).map_err(|e| UrpoError::render(format!("Failed to create terminal: {}", e)))?;

        Ok(Self { terminal })
    }

    /// Run the UI event loop.
    pub async fn run(&mut self, mut app: App) -> Result<()> {
        let mut last_update = Instant::now();
        let update_interval = Duration::from_secs(1);

        loop {
            // Update data every second
            if last_update.elapsed() >= update_interval {
                app.refresh_data().await;
                last_update = Instant::now();
            }

            self.terminal
                .draw(|f| draw_ui(f, &mut app))
                .map_err(|e| UrpoError::render(format!("Failed to draw UI: {}", e)))?;

            if event::poll(Duration::from_millis(100))
                .map_err(|e| UrpoError::render(format!("Failed to poll events: {}", e)))?
            {
                if let Event::Key(key) =
                    event::read().map_err(|e| UrpoError::render(format!("Failed to read event: {}", e)))?
                {
                    app.handle_key(key);
                }
            }

            if app.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Restore terminal to original state.
    pub fn restore(&mut self) -> Result<()> {
        disable_raw_mode().map_err(|e| UrpoError::render(format!("Failed to disable raw mode: {}", e)))?;

        self.terminal
            .backend_mut()
            .execute(LeaveAlternateScreen)
            .map_err(|e| UrpoError::render(format!("Failed to leave alternate screen: {}", e)))?;

        self.terminal
            .show_cursor()
            .map_err(|e| UrpoError::render(format!("Failed to show cursor: {}", e)))?;

        Ok(())
    }
}

impl Drop for TerminalUI {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// Draw the main UI.
fn draw_ui(frame: &mut Frame, app: &mut App) {
    match app.selected_tab {
        Tab::Services => dashboard::draw_dashboard(frame, app),
        Tab::Traces => draw_traces_view(frame, app),
        Tab::Spans => draw_spans_view(frame, app),
    }

    // Draw help overlay if active
    if app.show_help {
        draw_help_overlay(frame);
    }
}

/// Draw the traces view.
fn draw_traces_view(frame: &mut Frame, app: &mut App) {
    let size = frame.area();
    
    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(size);

    // Draw header
    let header = Block::default()
        .borders(Borders::ALL)
        .title(" Urpo - Trace Explorer ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(header, chunks[0]);

    // Draw traces table
    let header_cells = ["Trace ID", "Service", "Operation", "Duration", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.traces.iter().enumerate().map(|(idx, span)| {
        let selected = app.trace_state.selected() == Some(idx);
        let status_color = if span.status.is_error() {
            Color::Red
        } else {
            Color::Green
        };

        let prefix = if selected { "► " } else { "  " };

        Row::new(vec![
            Cell::from(format!("{}{}", prefix, &span.trace_id.as_str()[..8.min(span.trace_id.as_str().len())])),
            Cell::from(span.service_name.as_str()),
            Cell::from(span.operation_name.as_str()),
            Cell::from(format!("{:.2}ms", span.duration.as_secs_f64() * 1000.0)),
            Cell::from(match &span.status {
                crate::core::SpanStatus::Ok => "OK",
                crate::core::SpanStatus::Error(_) => "ERROR",
                crate::core::SpanStatus::Cancelled => "CANCELLED",
                crate::core::SpanStatus::Unknown => "-",
            })
            .style(Style::default().fg(status_color)),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    frame.render_stateful_widget(table, chunks[1], &mut app.trace_state);

    // Draw footer
    draw_footer(frame, chunks[2], app);
}

/// Draw the spans view.
fn draw_spans_view(frame: &mut Frame, _app: &App) {
    let size = frame.area();
    
    let paragraph = Paragraph::new("Span details will be shown here\nPress Tab to go back to Services view")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Span Details ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, size);
}

/// Draw the footer with help text.
fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let help_text = if app.search_active {
        format!("Search: {} | ESC: Cancel | Enter: Apply", app.search_query)
    } else {
        match app.selected_tab {
            Tab::Services => {
                format!(
                    "[q]uit [s]ort:{} [f]ilter:{} [/]search [h]elp [↑↓]nav",
                    app.sort_by.as_str(),
                    app.filter_mode.as_str()
                )
            }
            Tab::Traces | Tab::Spans => "[q]uit [Tab]switch [↑↓]navigate [Enter]details".to_string(),
        }
    };

    let footer = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    frame.render_widget(footer, area);
}

/// Draw help overlay.
fn draw_help_overlay(frame: &mut Frame) {
    let size = frame.area();
    
    // Create centered help window
    let help_width = 60;
    let help_height = 20;
    let x = (size.width.saturating_sub(help_width)) / 2;
    let y = (size.height.saturating_sub(help_height)) / 2;
    
    let help_area = Rect::new(x, y, help_width, help_height);

    let help_text = vec![
        Line::from(""),
        Line::from(vec![TextSpan::styled("Keyboard Shortcuts", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![TextSpan::raw("  q/Ctrl+C    Quit application")]),
        Line::from(vec![TextSpan::raw("  ↑/k ↓/j     Navigate up/down")]),
        Line::from(vec![TextSpan::raw("  PgUp/PgDn   Page up/down")]),
        Line::from(vec![TextSpan::raw("  g/G         Go to top/bottom")]),
        Line::from(vec![TextSpan::raw("  Enter       View details")]),
        Line::from(vec![TextSpan::raw("  Tab         Switch tabs")]),
        Line::from(vec![TextSpan::raw("  /           Search services")]),
        Line::from(vec![TextSpan::raw("  s           Cycle sort mode")]),
        Line::from(vec![TextSpan::raw("  r           Reverse sort order")]),
        Line::from(vec![TextSpan::raw("  f           Cycle filter mode")]),
        Line::from(vec![TextSpan::raw("  1-3         Quick filters")]),
        Line::from(vec![TextSpan::raw("  h/?         Toggle this help")]),
        Line::from(""),
        Line::from(vec![TextSpan::styled("Press any key to close", Style::default().fg(Color::DarkGray))]),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left)
        .style(Style::default().bg(Color::Black));

    frame.render_widget(help, help_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert!(!app.should_quit);
        assert_eq!(app.selected_tab, Tab::Services);
        assert!(!app.services.is_empty());
        assert!(!app.traces.is_empty());
    }

    #[test]
    fn test_sort_cycling() {
        let mut app = App::new();
        
        assert_eq!(app.sort_by, SortBy::Rps);
        app.sort_by = app.sort_by.next();
        assert_eq!(app.sort_by, SortBy::ErrorRate);
        app.sort_by = app.sort_by.next();
        assert_eq!(app.sort_by, SortBy::P50);
    }

    #[test]
    fn test_filter_cycling() {
        let mut app = App::new();
        
        assert_eq!(app.filter_mode, FilterMode::All);
        app.filter_mode = app.filter_mode.next();
        assert_eq!(app.filter_mode, FilterMode::ErrorsOnly);
        app.filter_mode = app.filter_mode.next();
        assert_eq!(app.filter_mode, FilterMode::SlowOnly);
    }

    #[test]
    fn test_tab_navigation() {
        let mut app = App::new();

        app.next_tab();
        assert_eq!(app.selected_tab, Tab::Traces);

        app.next_tab();
        assert_eq!(app.selected_tab, Tab::Spans);

        app.next_tab();
        assert_eq!(app.selected_tab, Tab::Services);

        app.previous_tab();
        assert_eq!(app.selected_tab, Tab::Spans);
    }

    #[test]
    fn test_quit_handling() {
        let mut app = App::new();

        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));
        assert!(app.should_quit);

        let mut app = App::new();
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn test_search_mode() {
        let mut app = App::new();

        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        assert!(app.search_active);

        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        assert_eq!(app.search_query, "test");

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(!app.search_active);
        assert!(app.search_query.is_empty());
    }
}