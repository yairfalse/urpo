//! Minimal terminal UI for Urpo - htop-like simplicity
//!
//! Focus: Service health monitoring and recent traces
//! No bloat, just essential information

mod keybindings;
mod service_list;
mod settings;
mod trace_list;

use crate::core::{Config, Result, ServiceMetrics, UrpoError};
use crate::storage::{StorageBackend, TraceInfo};
use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// View mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum View {
    Services,
    Traces,
    Metrics,
    Settings,
}

/// TUI application state
pub struct App {
    view: View,
    should_quit: bool,
    services: Vec<ServiceMetrics>,
    traces: Vec<TraceInfo>,
    service_health: Vec<(String, crate::metrics::ServiceHealth)>,
    selected_service: Option<usize>,
    selected_trace: Option<usize>,
    storage: Arc<RwLock<dyn StorageBackend>>,
    metrics_storage: Option<Arc<tokio::sync::Mutex<crate::metrics::MetricStorage>>>,
    config: Config,
    last_refresh: Instant,
}

impl App {
    /// Create new TUI app
    pub fn new(storage: Arc<RwLock<dyn StorageBackend>>, config: Config) -> Self {
        Self {
            view: View::Services,
            should_quit: false,
            services: Vec::new(),
            traces: Vec::new(),
            service_health: Vec::new(),
            selected_service: Some(0),
            selected_trace: Some(0),
            storage,
            metrics_storage: None,
            config,
            last_refresh: Instant::now(),
        }
    }

    /// Set metrics storage for real-time metrics display
    pub fn with_metrics(mut self, metrics_storage: Arc<tokio::sync::Mutex<crate::metrics::MetricStorage>>) -> Self {
        self.metrics_storage = Some(metrics_storage);
        self
    }

    /// Refresh data from storage
    async fn refresh(&mut self) -> Result<()> {
        let storage = self.storage.read().await;

        // Get service metrics
        if let Ok(metrics) = storage.get_service_metrics().await {
            self.services = metrics;
        }

        // Get recent traces
        if let Ok(traces) = storage.list_recent_traces(50, None).await {
            self.traces = traces;
        }

        // Get real-time metrics if available
        if let Some(ref metrics_storage) = self.metrics_storage {
            let storage = metrics_storage.lock().await;
            let service_ids = storage.list_services();

            self.service_health.clear();
            for service_id in service_ids {
                if let Some(health) = storage.get_service_health(service_id) {
                    // TODO: Get actual service name from string pool
                    let service_name = format!("service-{}", service_id);
                    self.service_health.push((service_name, health));
                }
            }
        }

        self.last_refresh = Instant::now();
        Ok(())
    }

    /// Handle keyboard input
    fn handle_input(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        use keybindings::{handle_key, Action};

        // Handle Escape key to exit settings
        if matches!(key.code, KeyCode::Esc) && self.view == View::Settings {
            self.view = View::Services;
            return;
        }

        match handle_key(key) {
            Action::Quit => self.should_quit = true,
            Action::OpenSettings => {
                self.view = View::Settings;
            },
            Action::ToggleView => {
                self.view = match self.view {
                    View::Services => View::Traces,
                    View::Traces => if self.metrics_storage.is_some() { View::Metrics } else { View::Services },
                    View::Metrics => View::Services,
                    View::Settings => View::Services,
                };
            },
            Action::MoveUp => {
                // Skip navigation in settings/metrics view
                if self.view == View::Settings || self.view == View::Metrics {
                    return;
                }
                let selected = match self.view {
                    View::Services => &mut self.selected_service,
                    View::Traces => &mut self.selected_trace,
                    _ => return,
                };
                if let Some(idx) = selected {
                    if *idx > 0 {
                        *idx -= 1;
                    }
                }
            },
            Action::MoveDown => {
                // Skip navigation in settings/metrics view
                if self.view == View::Settings || self.view == View::Metrics {
                    return;
                }
                let (selected, max) = match self.view {
                    View::Services => (&mut self.selected_service, self.services.len()),
                    View::Traces => (&mut self.selected_trace, self.traces.len()),
                    _ => return,
                };
                if let Some(idx) = selected {
                    if *idx < max.saturating_sub(1) {
                        *idx += 1;
                    }
                }
            },
            Action::PageUp => {
                // Skip navigation in settings/metrics view
                if self.view == View::Settings || self.view == View::Metrics {
                    return;
                }
                let selected = match self.view {
                    View::Services => &mut self.selected_service,
                    View::Traces => &mut self.selected_trace,
                    _ => return,
                };
                if let Some(idx) = selected {
                    *idx = idx.saturating_sub(10);
                }
            },
            Action::PageDown => {
                // Skip navigation in settings/metrics view
                if self.view == View::Settings || self.view == View::Metrics {
                    return;
                }
                let (selected, max) = match self.view {
                    View::Services => (&mut self.selected_service, self.services.len()),
                    View::Traces => (&mut self.selected_trace, self.traces.len()),
                    _ => return,
                };
                if let Some(idx) = selected {
                    *idx = (*idx + 10).min(max.saturating_sub(1));
                }
            },
            _ => {},
        }
    }

    /// Draw the UI
    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(2), // Footer
            ])
            .split(frame.area());

        // Header
        self.draw_header(frame, chunks[0]);

        // Content
        match self.view {
            View::Services => {
                service_list::draw_service_table(
                    frame,
                    chunks[1],
                    &self.services,
                    self.selected_service,
                );
            },
            View::Traces => {
                trace_list::draw_trace_table(frame, chunks[1], &self.traces, self.selected_trace);
            },
            View::Metrics => {
                self.draw_metrics_dashboard(frame, chunks[1]);
            },
            View::Settings => {
                settings::draw_settings(frame, chunks[1], &self.config);
            },
        }

        // Footer
        self.draw_footer(frame, chunks[2]);
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let title = format!(
            " Urpo - {} | {} services | {} traces ",
            match self.view {
                View::Services => "Services",
                View::Traces => "Traces",
                View::Metrics => "Metrics",
                View::Settings => "Settings",
            },
            self.services.len(),
            self.traces.len()
        );

        let header = Paragraph::new(title)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );

        frame.render_widget(header, area);
    }

    fn draw_footer(&self, frame: &mut Frame, area: Rect) {
        let help = if self.view == View::Settings {
            " [Esc]back [q]uit "
        } else {
            " [q]uit [s]ettings [Tab]switch [↑↓]navigate [r]efresh "
        };
        let footer = Paragraph::new(help)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::TOP));

        frame.render_widget(footer, area);
    }

    /// Draw metrics dashboard with service health
    fn draw_metrics_dashboard(&self, frame: &mut Frame, area: Rect) {
        use ratatui::widgets::{Cell, Row, Table};
        use ratatui::layout::Constraint as C;

        if self.service_health.is_empty() {
            let no_data = Paragraph::new("No metrics data available")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).title(" Metrics "));
            frame.render_widget(no_data, area);
            return;
        }

        // Build table rows
        let rows: Vec<Row> = self.service_health
            .iter()
            .map(|(service_name, health)| {
                let error_color = if health.error_rate > 5.0 {
                    Color::Red
                } else if health.error_rate > 1.0 {
                    Color::Yellow
                } else {
                    Color::Green
                };

                Row::new(vec![
                    Cell::from(service_name.clone()),
                    Cell::from(format!("{:.1}/s", health.request_rate)),
                    Cell::from(format!("{:.1}%", health.error_rate)).style(Style::default().fg(error_color)),
                    Cell::from(format!("{:.1}ms", health.avg_latency_ms)),
                    Cell::from(format!("{:.1}ms", health.p95_latency_ms)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [C::Percentage(30), C::Percentage(15), C::Percentage(15), C::Percentage(20), C::Percentage(20)]
        )
        .header(
            Row::new(vec!["Service", "Req/s", "Errors", "Avg Latency", "P95 Latency"])
                .style(Style::default().fg(Color::Yellow))
        )
        .block(Block::default().borders(Borders::ALL).title(" Real-Time Service Metrics "))
        .style(Style::default().fg(Color::White));

        frame.render_widget(table, area);
    }
}

/// Run the TUI
pub async fn run_tui(
    storage: Arc<RwLock<dyn StorageBackend>>,
    _monitor: Arc<crate::monitoring::Monitor>,
    config: Config,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()
        .map_err(|e| UrpoError::render(format!("Failed to enable raw mode: {}", e)))?;
    let mut stdout = io::stdout();
    stdout
        .execute(EnterAlternateScreen)
        .map_err(|e| UrpoError::render(format!("Failed to enter alternate screen: {}", e)))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .map_err(|e| UrpoError::render(format!("Failed to create terminal: {}", e)))?;

    // Create app
    let mut app = App::new(storage, config);

    // Initial data load
    app.refresh().await?;

    // Main loop
    let tick_rate = Duration::from_millis(100);
    let refresh_rate = Duration::from_secs(2);
    let mut last_tick = Instant::now();

    loop {
        // Draw UI
        terminal
            .draw(|f| app.draw(f))
            .map_err(|e| UrpoError::render(format!("Failed to draw: {}", e)))?;

        // Handle input
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)
            .map_err(|e| UrpoError::render(format!("Failed to poll events: {}", e)))?
        {
            if let Event::Key(key) = event::read()
                .map_err(|e| UrpoError::render(format!("Failed to read event: {}", e)))?
            {
                app.handle_input(key);
                if app.should_quit {
                    break;
                }
            }
        }

        // Refresh data periodically
        if app.last_refresh.elapsed() >= refresh_rate {
            app.refresh().await?;
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    // Cleanup
    disable_raw_mode()
        .map_err(|e| UrpoError::render(format!("Failed to disable raw mode: {}", e)))?;
    terminal
        .backend_mut()
        .execute(LeaveAlternateScreen)
        .map_err(|e| UrpoError::render(format!("Failed to leave alternate screen: {}", e)))?;
    terminal
        .show_cursor()
        .map_err(|e| UrpoError::render(format!("Failed to show cursor: {}", e)))?;

    Ok(())
}
