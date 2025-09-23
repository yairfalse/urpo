//! Minimal terminal UI for Urpo - htop-like simplicity
//!
//! Focus: Service health monitoring and recent traces
//! No bloat, just essential information

mod keybindings;
mod service_list;
mod trace_list;

use crate::core::{Result, ServiceMetrics, UrpoError};
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
}

/// TUI application state
pub struct App {
    view: View,
    should_quit: bool,
    services: Vec<ServiceMetrics>,
    traces: Vec<TraceInfo>,
    selected_service: Option<usize>,
    selected_trace: Option<usize>,
    storage: Arc<RwLock<dyn StorageBackend>>,
    last_refresh: Instant,
}

impl App {
    /// Create new TUI app
    pub fn new(storage: Arc<RwLock<dyn StorageBackend>>) -> Self {
        Self {
            view: View::Services,
            should_quit: false,
            services: Vec::new(),
            traces: Vec::new(),
            selected_service: Some(0),
            selected_trace: Some(0),
            storage,
            last_refresh: Instant::now(),
        }
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

        self.last_refresh = Instant::now();
        Ok(())
    }

    /// Handle keyboard input
    fn handle_input(&mut self, key: crossterm::event::KeyEvent) {
        use keybindings::{handle_key, Action};

        match handle_key(key) {
            Action::Quit => self.should_quit = true,
            Action::ToggleView => {
                self.view = match self.view {
                    View::Services => View::Traces,
                    View::Traces => View::Services,
                };
            }
            Action::MoveUp => {
                let selected = match self.view {
                    View::Services => &mut self.selected_service,
                    View::Traces => &mut self.selected_trace,
                };
                if let Some(idx) = selected {
                    if *idx > 0 {
                        *idx -= 1;
                    }
                }
            }
            Action::MoveDown => {
                let (selected, max) = match self.view {
                    View::Services => (&mut self.selected_service, self.services.len()),
                    View::Traces => (&mut self.selected_trace, self.traces.len()),
                };
                if let Some(idx) = selected {
                    if *idx < max.saturating_sub(1) {
                        *idx += 1;
                    }
                }
            }
            Action::PageUp => {
                let selected = match self.view {
                    View::Services => &mut self.selected_service,
                    View::Traces => &mut self.selected_trace,
                };
                if let Some(idx) = selected {
                    *idx = idx.saturating_sub(10);
                }
            }
            Action::PageDown => {
                let (selected, max) = match self.view {
                    View::Services => (&mut self.selected_service, self.services.len()),
                    View::Traces => (&mut self.selected_trace, self.traces.len()),
                };
                if let Some(idx) = selected {
                    *idx = (*idx + 10).min(max.saturating_sub(1));
                }
            }
            _ => {}
        }
    }

    /// Draw the UI
    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Content
                Constraint::Length(2),  // Footer
            ])
            .split(frame.area());

        // Header
        self.draw_header(frame, chunks[0]);

        // Content
        match self.view {
            View::Services => {
                service_list::draw_service_table(frame, chunks[1], &self.services, self.selected_service);
            }
            View::Traces => {
                trace_list::draw_trace_table(frame, chunks[1], &self.traces, self.selected_trace);
            }
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
        let help = " [q]uit [Tab]switch [↑↓]navigate [r]efresh ";
        let footer = Paragraph::new(help)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::TOP));

        frame.render_widget(footer, area);
    }
}

/// Run the TUI
pub async fn run_tui(storage: Arc<RwLock<dyn StorageBackend>>) -> Result<()> {
    // Setup terminal
    enable_raw_mode().map_err(|e| UrpoError::render(format!("Failed to enable raw mode: {}", e)))?;
    let mut stdout = io::stdout();
    stdout
        .execute(EnterAlternateScreen)
        .map_err(|e| UrpoError::render(format!("Failed to enter alternate screen: {}", e)))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .map_err(|e| UrpoError::render(format!("Failed to create terminal: {}", e)))?;

    // Create app
    let mut app = App::new(storage);

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
    disable_raw_mode().map_err(|e| UrpoError::render(format!("Failed to disable raw mode: {}", e)))?;
    terminal
        .backend_mut()
        .execute(LeaveAlternateScreen)
        .map_err(|e| UrpoError::render(format!("Failed to leave alternate screen: {}", e)))?;
    terminal
        .show_cursor()
        .map_err(|e| UrpoError::render(format!("Failed to show cursor: {}", e)))?;

    Ok(())
}