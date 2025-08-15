//! Terminal user interface for Urpo.
//!
//! This module provides the interactive terminal UI using ratatui
//! for real-time trace exploration and service health monitoring.

mod fake_data;

use crate::core::{Result, ServiceMetrics, Span, UrpoError};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fake_data::{FakeDataGenerator, HealthStatus};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span as TextSpan},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use std::time::Duration;

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
    /// Current traces data.
    pub traces: Vec<Span>,
    /// Search query.
    pub search_query: String,
    /// Whether search mode is active.
    pub search_active: bool,
    /// Fake data generator for demo mode.
    pub fake_generator: FakeDataGenerator,
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
            traces: Vec::new(),
            search_query: String::new(),
            search_active: false,
            fake_generator: FakeDataGenerator::new(),
        };
        
        // Initialize with fake data
        app.services = app.fake_generator.generate_metrics();
        app.traces = app.fake_generator.generate_traces(20);
        app
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
                    self.apply_search();
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
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Refresh data
                self.services = self.fake_generator.generate_metrics();
                self.traces = self.fake_generator.generate_traces(20);
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection_down(),
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
        let (state, max) = match self.selected_tab {
            Tab::Services => (&mut self.service_state, self.services.len()),
            Tab::Traces => (&mut self.trace_state, self.traces.len()),
            Tab::Spans => return,
        };

        let selected = state.selected().unwrap_or(0);
        if selected < max.saturating_sub(1) {
            state.select(Some(selected + 1));
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
                if !self.services.is_empty() {
                    self.service_state.select(Some(self.services.len() - 1));
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

    /// Apply search filter to current view.
    fn apply_search(&mut self) {
        // This would filter the displayed data based on search query
        if !self.search_query.is_empty() {
            tracing::debug!("Applying search filter: {}", self.search_query);
        }
    }

    /// Update services data.
    pub fn update_services(&mut self, services: Vec<ServiceMetrics>) {
        self.services = services;
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
        enable_raw_mode().map_err(|e| UrpoError::render(format!("Failed to enable raw mode: {}", e)))?;
        
        let mut stdout = io::stdout();
        stdout
            .execute(EnterAlternateScreen)
            .map_err(|e| UrpoError::render(format!("Failed to enter alternate screen: {}", e)))?;
        
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)
            .map_err(|e| UrpoError::render(format!("Failed to create terminal: {}", e)))?;
        
        Ok(Self { terminal })
    }

    /// Run the UI event loop.
    pub async fn run(&mut self, mut app: App) -> Result<()> {
        let mut last_update = std::time::Instant::now();
        let update_interval = Duration::from_secs(1);
        
        loop {
            // Update fake data every second
            if last_update.elapsed() >= update_interval {
                app.services = app.fake_generator.generate_metrics();
                app.traces = app.fake_generator.generate_traces(20);
                last_update = std::time::Instant::now();
            }
            
            self.terminal
                .draw(|f| draw_ui(f, &mut app))
                .map_err(|e| UrpoError::render(format!("Failed to draw UI: {}", e)))?;

            if event::poll(Duration::from_millis(100))
                .map_err(|e| UrpoError::render(format!("Failed to poll events: {}", e)))?
            {
                if let Event::Key(key) = event::read()
                    .map_err(|e| UrpoError::render(format!("Failed to read event: {}", e)))?
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
        disable_raw_mode()
            .map_err(|e| UrpoError::render(format!("Failed to disable raw mode: {}", e)))?;
        
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
    let size = frame.area();

    // For services view, we don't need the tab header
    if app.selected_tab == Tab::Services {
        // Create layout without header tabs
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(size);

        draw_services_tab(frame, chunks[0], app);
        draw_footer(frame, chunks[1], app);
    } else {
        // Create main layout with header for other tabs
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(size);

        // Draw header
        draw_header(frame, chunks[0], app);

        // Draw content based on selected tab
        match app.selected_tab {
            Tab::Traces => draw_traces_tab(frame, chunks[1], app),
            Tab::Spans => draw_spans_tab(frame, chunks[1], app),
            _ => {}
        }

        // Draw footer
        draw_footer(frame, chunks[2], app);
    }
}

/// Draw the header with tab navigation.
fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles = ["Services", "Traces", "Spans"];
    let selected_index = match app.selected_tab {
        Tab::Services => 0,
        Tab::Traces => 1,
        Tab::Spans => 2,
    };

    let header_text: Vec<TextSpan> = titles
        .iter()
        .enumerate()
        .map(|(i, title)| {
            let style = if i == selected_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            TextSpan::styled(format!(" {} ", title), style)
        })
        .collect();

    let header = Paragraph::new(Line::from(header_text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Urpo - OTEL Trace Explorer ")
                .title_alignment(Alignment::Center),
        )
        .alignment(Alignment::Center);

    frame.render_widget(header, area);
}

/// Draw the services tab.
fn draw_services_tab(frame: &mut Frame, area: Rect, app: &mut App) {
    // Split area for title and table
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(0),     // Table
        ])
        .split(area);

    // Draw title with service count
    let title = format!(" Urpo: Service Health ({} services) ", app.services.len());
    let title_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(title_block, chunks[0]);

    // Create table header
    let header_cells = ["Service", "RPS", "Error%", "P50", "P95", "P99", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    // Create table rows with formatted data
    let rows = app.services.iter().enumerate().map(|(idx, service)| {
        let selected = app.service_state.selected() == Some(idx);
        
        // Determine health status and colors
        let (status_text, status_color, status_symbol) = if service.error_rate() > 10.0 {
            ("Unhealthy", Color::Red, "●")
        } else if service.error_rate() > 2.0 {
            ("Degraded", Color::Yellow, "⚠")
        } else {
            ("Healthy", Color::Green, "●")
        };

        // Format latencies with appropriate units
        let format_latency = |ms: u64| {
            if ms >= 1000 {
                format!("{:.1}s", ms as f64 / 1000.0)
            } else {
                format!("{}ms", ms)
            }
        };

        // Add selection indicator
        let service_name = if selected {
            format!("→ {}", service.service_name.as_str())
        } else {
            format!("  {}", service.service_name.as_str())
        };

        Row::new(vec![
            Cell::from(service_name),
            Cell::from(format!("{:.1}", service.rps)),
            Cell::from(format!("{:.1}%", service.error_rate()))
                .style(Style::default().fg(status_color)),
            Cell::from(format_latency(service.p50_latency_ms)),
            Cell::from(format_latency(service.p95_latency_ms)),
            Cell::from(format_latency(service.p99_latency_ms)),
            Cell::from(format!("{} {}", status_symbol, status_text))
                .style(Style::default().fg(status_color)),
        ])
    });

    let table = Table::new(rows, [
        Constraint::Percentage(25),  // Service
        Constraint::Percentage(10),  // RPS
        Constraint::Percentage(10),  // Error%
        Constraint::Percentage(12),  // P50
        Constraint::Percentage(12),  // P95
        Constraint::Percentage(12),  // P99
        Constraint::Percentage(19),  // Status
    ])
    .header(header)
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray)))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(table, chunks[1], &mut app.service_state);
}

/// Draw the traces tab.
fn draw_traces_tab(frame: &mut Frame, area: Rect, app: &mut App) {
    let header_cells = ["Trace ID", "Service", "Operation", "Duration", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.traces.iter().map(|span| {
        let status_color = if span.is_error() {
            Color::Red
        } else {
            Color::Green
        };

        Row::new(vec![
            Cell::from(&span.trace_id.as_str()[..8]),
            Cell::from(span.service_name.as_str()),
            Cell::from(span.operation_name.as_str()),
            Cell::from(format!("{:.2}ms", span.duration().as_secs_f64() * 1000.0)),
            Cell::from(match &span.status {
                crate::core::SpanStatus::Ok => "OK",
                crate::core::SpanStatus::Error(_) => "ERROR",
                crate::core::SpanStatus::Unset => "-",
            })
            .style(Style::default().fg(status_color)),
        ])
    });

    let table = Table::new(rows, [
        Constraint::Percentage(15),
        Constraint::Percentage(25),
        Constraint::Percentage(30),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Traces "))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(table, area, &mut app.trace_state);
}

/// Draw the spans tab.
fn draw_spans_tab(frame: &mut Frame, area: Rect, _app: &mut App) {
    let paragraph = Paragraph::new("Span details will be shown here")
        .block(Block::default().borders(Borders::ALL).title(" Span Details "))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Draw the footer with help text.
fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let help_text = if app.search_active {
        format!("Search: {} | ESC: Cancel | Enter: Apply", app.search_query)
    } else {
        "[q] Quit  [j/k] Navigate  [Enter] Details  [r] Refresh".to_string()
    };

    let footer = Paragraph::new(help_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    frame.render_widget(footer, area);
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