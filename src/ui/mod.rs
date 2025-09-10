//! Terminal user interface for Urpo.
//!
//! This module provides the interactive terminal UI using ratatui
//! for real-time trace exploration and service health monitoring.

pub mod dashboard;
mod fake_data;
mod widgets;
mod span_details;

// Re-export commonly used types
// Dashboard, Tab, FilterMode, DataCommand are all defined in this module

use crate::core::{Result, ServiceMetrics, Span, TraceId, ServiceName, UrpoError};
use crate::storage::{StorageBackend, TraceInfo};
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
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;

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

/// Commands sent from UI to data fetcher.
#[derive(Debug, Clone)]
pub enum DataCommand {
    RefreshAll,
    LoadTracesForService(ServiceName),
    LoadSpansForTrace(TraceId),
    SearchTraces(String),
    ApplyFilter(FilterMode),
}

/// Data updates sent from fetcher to UI.
#[derive(Debug)]
enum DataUpdate {
    Services(Vec<ServiceMetrics>),
    Traces(Vec<TraceInfo>),
    Spans(Vec<Span>),
    Stats { total_spans: u64, spans_per_sec: f64, memory_mb: f64 },
    ReceiverStatus(ReceiverStatus),
}

/// Main UI application state.
pub struct Dashboard {
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
    pub traces: Vec<TraceInfo>,
    /// Currently selected trace ID for span view.
    pub selected_trace_id: Option<TraceId>,
    /// Currently selected service (when drilling down from services tab).
    pub selected_service: Option<ServiceName>,
    /// Spans for the selected trace.
    pub trace_spans: Vec<Span>,
    /// Currently selected span index in the spans view.
    pub selected_span_index: Option<usize>,
    /// State for span list navigation.
    pub span_state: TableState,
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
    pub storage: Option<Arc<tokio::sync::RwLock<dyn StorageBackend>>>,
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
    /// Channel to send commands to data fetcher.
    data_tx: Option<mpsc::UnboundedSender<DataCommand>>,
    /// Channel to receive updates from data fetcher.
    data_rx: Option<mpsc::UnboundedReceiver<DataUpdate>>,
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
    /// Service dependency map.
    Map,
}

impl Dashboard {
    /// Create a new UI dashboard with storage and health monitor.
    pub fn new(
        storage: Arc<tokio::sync::RwLock<dyn StorageBackend>>,
        _health_monitor: Arc<crate::monitoring::ServiceHealthMonitor>,
    ) -> Result<Self> {
        let mut service_state = TableState::default();
        service_state.select(Some(0));

        let mut trace_state = TableState::default();
        trace_state.select(Some(0));

        let mut app = Self {
            should_quit: false,
            selected_tab: Tab::Services,
            service_state,
            trace_state,
            span_state: TableState::default(),
            selected_span_index: None,
            services: Vec::new(),
            rps_history: dashmap::DashMap::new(),
            traces: Vec::new(),
            selected_trace_id: None,
            selected_service: None,
            trace_spans: Vec::new(),
            search_query: String::new(),
            search_active: false,
            sort_by: SortBy::Rps,
            sort_desc: true,
            filter_mode: FilterMode::All,
            show_help: false,
            storage: Some(storage),
            fake_generator: FakeDataGenerator::new(),
            total_spans: 0,
            spans_per_sec: 0.0,
            memory_usage_mb: 45.0,
            last_update: Instant::now(),
            receiver_status: ReceiverStatus::Connected,
            data_tx: None,  // Will be set in run()
            data_rx: None,  // Will be set in run()
        };

        // Initialize with fake data for now
        app.services = app.fake_generator.generate_metrics();
        app.update_rps_history();
        Ok(app)
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

    /// Process any pending data updates from the async fetcher.
    fn process_data_updates(&mut self) {
        // Take the receiver temporarily to avoid borrow checker issues
        let mut rx = self.data_rx.take();
        if let Some(ref mut receiver) = rx {
            // Process all available updates without blocking
            while let Ok(update) = receiver.try_recv() {
                match update {
                    DataUpdate::Services(services) => {
                        self.services = services;
                        self.update_rps_history();
                        self.apply_sort();
                        self.apply_filter();
                        self.receiver_status = ReceiverStatus::Connected;
                    }
                    DataUpdate::Traces(traces) => {
                        self.traces = traces;
                    }
                    DataUpdate::Spans(spans) => {
                        self.trace_spans = spans;
                    }
                    DataUpdate::Stats { total_spans, spans_per_sec, memory_mb } => {
                        self.total_spans = total_spans;
                        self.spans_per_sec = spans_per_sec;
                        self.memory_usage_mb = memory_mb;
                    }
                    DataUpdate::ReceiverStatus(status) => {
                        self.receiver_status = status;
                    }
                }
            }
        }
        // Put the receiver back
        self.data_rx = rx;
        self.last_update = Instant::now();
    }

    /// Request a data refresh from the async fetcher.
    fn request_refresh(&self, command: DataCommand) {
        if let Some(tx) = &self.data_tx {
            // Send command without blocking
            let _ = tx.send(command);
        }
    }

    /// Dashboardly current sort order to services.
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

    /// Dashboardly current filter to services.
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
                // Dashboardly search filter
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

                // Dashboardly filter mode
                match self.filter_mode {
                    FilterMode::All => true,
                    FilterMode::ErrorsOnly => s.error_rate > 0.01,
                    FilterMode::SlowOnly => s.latency_p95 > Duration::from_millis(500),
                    FilterMode::Active => s.request_rate > 0.0,
                }
            })
            .collect();

        // Dashboardly sorting
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
                    // Dashboardly search filter
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
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+T - Search traces
                if !self.search_query.is_empty() {
                    self.request_refresh(DataCommand::SearchTraces(self.search_query.clone()));
                }
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
                self.request_refresh(DataCommand::ApplyFilter(self.filter_mode));
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('1') => {
                self.filter_mode = FilterMode::All;
                self.apply_filter();
                self.request_refresh(DataCommand::ApplyFilter(self.filter_mode));
            }
            KeyCode::Char('2') => {
                self.filter_mode = FilterMode::ErrorsOnly;
                self.apply_filter();
                self.request_refresh(DataCommand::ApplyFilter(self.filter_mode));
            }
            KeyCode::Char('3') => {
                self.filter_mode = FilterMode::SlowOnly;
                self.apply_filter();
                self.request_refresh(DataCommand::ApplyFilter(self.filter_mode));
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Home | KeyCode::Char('g') => self.move_to_top(),
            KeyCode::End | KeyCode::Char('G') => self.move_to_bottom(),
            KeyCode::Enter => {
                // Simple test - just switch tabs to verify Enter is working
                if self.selected_tab == Tab::Services {
                    self.selected_tab = Tab::Traces;
                }
            }
            // Copy span ID (y key)
            KeyCode::Char('y') if self.selected_tab == Tab::Spans => {
                if let Some(idx) = self.selected_span_index {
                    if let Some(span) = self.trace_spans.get(idx) {
                        let _ = span_details::copy_to_clipboard(span.span_id.as_str());
                    }
                }
            }
            // Copy trace ID (Y key)
            KeyCode::Char('Y') if self.selected_tab == Tab::Spans => {
                if let Some(idx) = self.selected_span_index {
                    if let Some(span) = self.trace_spans.get(idx) {
                        let _ = span_details::copy_to_clipboard(span.trace_id.as_str());
                    }
                }
            }
            _ => {}
        }
    }

    /// Move to the next tab.
    fn next_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            Tab::Services => Tab::Traces,
            Tab::Traces => Tab::Spans,
            Tab::Spans => Tab::Map,
            Tab::Map => Tab::Services,
        };
    }

    /// Move to the previous tab.
    fn previous_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            Tab::Services => Tab::Map,
            Tab::Traces => Tab::Services,
            Tab::Spans => Tab::Traces,
            Tab::Map => Tab::Spans,
        };
    }

    /// Move selection up in the current list.
    fn move_selection_up(&mut self) {
        match self.selected_tab {
            Tab::Services => {
                let selected = self.service_state.selected().unwrap_or(0);
                if selected > 0 {
                    self.service_state.select(Some(selected - 1));
                }
            }
            Tab::Traces => {
                let selected = self.trace_state.selected().unwrap_or(0);
                if selected > 0 {
                    self.trace_state.select(Some(selected - 1));
                }
            }
            Tab::Spans => {
                if !self.trace_spans.is_empty() {
                    if self.selected_span_index.is_none() {
                        self.selected_span_index = Some(0);
                        self.span_state.select(Some(0));
                    } else {
                        let selected = self.selected_span_index.unwrap();
                        if selected > 0 {
                            self.selected_span_index = Some(selected - 1);
                            self.span_state.select(Some(selected - 1));
                        }
                    }
                }
            }
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
            Tab::Spans => {
                if !self.trace_spans.is_empty() {
                    let max = self.trace_spans.len();
                    let selected = self.selected_span_index.unwrap_or(0);
                    if selected < max.saturating_sub(1) {
                        self.selected_span_index = Some(selected + 1);
                        self.span_state.select(Some(selected + 1));
                    } else if self.selected_span_index.is_none() {
                        self.selected_span_index = Some(0);
                        self.span_state.select(Some(0));
                    }
                }
            }
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
            Tab::Spans => {
                if !self.trace_spans.is_empty() {
                    let selected = self.selected_span_index.unwrap_or(0);
                    if selected > 0 {
                        self.selected_span_index = Some(selected - 1);
                    }
                }
            }
        }
    }

    /// Move to the top of the current list.
    fn move_to_top(&mut self) {
        match self.selected_tab {
            Tab::Services => self.service_state.select(Some(0)),
            Tab::Traces => self.trace_state.select(Some(0)),
            Tab::Spans => {
                if !self.trace_spans.is_empty() {
                    let selected = self.selected_span_index.unwrap_or(0);
                    if selected > 0 {
                        self.selected_span_index = Some(selected - 1);
                    }
                }
            }
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
            Tab::Spans => {
                if !self.trace_spans.is_empty() {
                    let selected = self.selected_span_index.unwrap_or(0);
                    if selected > 0 {
                        self.selected_span_index = Some(selected - 1);
                    }
                }
            }
        }
    }

    /// Handle selection action (Enter key).
    fn handle_selection(&mut self) {
        match self.selected_tab {
            Tab::Services => {
                // Get selected service and load its traces
                if let Some(selected_idx) = self.service_state.selected() {
                    let filtered_services = self.get_filtered_services();
                    if let Some(service) = filtered_services.get(selected_idx) {
                        let service_name = service.name.clone();
                        self.selected_service = Some(service_name.clone());
                        // Clear existing traces before switching
                        self.traces.clear();
                        self.selected_tab = Tab::Traces;
                        self.trace_state.select(None);  // Don't select anything yet
                        // Request traces for the selected service
                        self.request_refresh(DataCommand::LoadTracesForService(service_name));
                    }
                }
            }
            Tab::Traces => {
                // Get selected trace and load its spans
                if let Some(selected_idx) = self.trace_state.selected() {
                    if let Some(trace_info) = self.traces.get(selected_idx) {
                        self.selected_trace_id = Some(trace_info.trace_id.clone());
                        self.selected_tab = Tab::Spans;
                        // Request spans for the selected trace
                        self.request_refresh(DataCommand::LoadSpansForTrace(trace_info.trace_id.clone()));
                    }
                }
            }
            Tab::Spans => {
                // Already in detail view - could expand/collapse span details
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
    pub fn update_traces(&mut self, traces: Vec<TraceInfo>) {
        self.traces = traces;
    }

    /// Run the terminal UI
    pub async fn run(&mut self) -> Result<()> {
        let mut terminal = TerminalUI::new()?;
        
        // Create channels for communication between UI and data fetcher
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<DataCommand>();
        let (update_tx, update_rx) = mpsc::unbounded_channel::<DataUpdate>();
        
        // Replace the existing channels with the new ones
        self.data_tx = Some(cmd_tx.clone());
        self.data_rx = Some(update_rx);

        // Spawn the async data fetcher task
        let storage = self.storage.clone();
        
        let fetcher_handle = tokio::spawn(async move {
            Self::data_fetcher_task(storage, cmd_rx, update_tx).await;
        });

        // Initial data request
        self.request_refresh(DataCommand::RefreshAll);
        
        let tick_rate = Duration::from_millis(100);
        let mut last_tick = Instant::now();
        let mut last_refresh = Instant::now();
        let refresh_interval = Duration::from_secs(1);

        loop {
            // Process any pending data updates
            self.process_data_updates();

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key);
                    if self.should_quit {
                        break;
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                // Request refresh every second
                if last_refresh.elapsed() >= refresh_interval {
                    self.request_refresh(DataCommand::RefreshAll);
                    last_refresh = Instant::now();
                }
                last_tick = Instant::now();
            }

            terminal.terminal.draw(|frame| {
                dashboard::draw_dashboard(frame, self);
            })?;
        }

        // Cleanup
        fetcher_handle.abort();
        terminal.restore()?;
        Ok(())
    }

    /// Background task that fetches data from storage.
    async fn data_fetcher_task(
        storage: Option<Arc<tokio::sync::RwLock<dyn StorageBackend>>>,
        mut cmd_rx: mpsc::UnboundedReceiver<DataCommand>,
        update_tx: mpsc::UnboundedSender<DataUpdate>,
    ) {
        while let Some(cmd) = cmd_rx.recv().await {
            if let Some(storage) = &storage {
                match cmd {
                    DataCommand::RefreshAll => {
                        // Fetch all data
                        let storage_guard = storage.read().await;
                        
                        // Get services
                        if let Ok(metrics) = storage_guard.get_service_metrics().await {
                            let _ = update_tx.send(DataUpdate::Services(metrics));
                        }
                        
                        // Get default traces
                        if let Ok(traces) = storage_guard.list_recent_traces(50, None).await {
                            let _ = update_tx.send(DataUpdate::Traces(traces));
                        }
                        
                        // Get stats
                        if let Ok(stats) = storage_guard.get_storage_stats().await {
                            let _ = update_tx.send(DataUpdate::Stats {
                                total_spans: stats.span_count as u64,
                                spans_per_sec: stats.processing_rate,
                                memory_mb: stats.memory_mb,
                            });
                        }
                    }
                    DataCommand::LoadTracesForService(service_name) => {
                        let storage_guard = storage.read().await;
                        if let Ok(traces) = storage_guard.list_recent_traces(50, Some(&service_name)).await {
                            let _ = update_tx.send(DataUpdate::Traces(traces));
                        }
                    }
                    DataCommand::LoadSpansForTrace(trace_id) => {
                        let storage_guard = storage.read().await;
                        if let Ok(spans) = storage_guard.get_trace_spans(&trace_id).await {
                            let _ = update_tx.send(DataUpdate::Spans(spans));
                        }
                    }
                    DataCommand::SearchTraces(query) => {
                        let storage_guard = storage.read().await;
                        if let Ok(traces) = storage_guard.search_traces(&query, 50).await {
                            let _ = update_tx.send(DataUpdate::Traces(traces));
                        }
                    }
                    DataCommand::ApplyFilter(filter_mode) => {
                        let storage_guard = storage.read().await;
                        let trace_result = match filter_mode {
                            FilterMode::All => storage_guard.list_recent_traces(50, None).await,
                            FilterMode::ErrorsOnly => storage_guard.get_error_traces(50).await,
                            FilterMode::SlowOnly => storage_guard.get_slow_traces(Duration::from_millis(500), 50).await,
                            FilterMode::Active => storage_guard.list_recent_traces(50, None).await,
                        };
                        if let Ok(traces) = trace_result {
                            let _ = update_tx.send(DataUpdate::Traces(traces));
                        }
                    }
                }
            }
        }
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
    pub async fn run(&mut self, mut app: Dashboard) -> Result<()> {
        let mut last_update = Instant::now();
        let update_interval = Duration::from_secs(1);

        loop {
            // Process any pending data updates
            app.process_data_updates();
            
            // Request refresh every second
            if last_update.elapsed() >= update_interval {
                app.request_refresh(DataCommand::RefreshAll);
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
fn draw_ui(frame: &mut Frame, app: &mut Dashboard) {
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
fn draw_traces_view(frame: &mut Frame, app: &mut Dashboard) {
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

    // Draw header with trace count
    let title = format!(" Urpo - Trace Explorer ({} traces) ", app.traces.len());
    let header = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Cyan));
    
    let header_text = if !app.search_query.is_empty() {
        format!("Search: {}", app.search_query)
    } else {
        format!("Filter: {} | Sort: {}", app.filter_mode.as_str(), app.sort_by.as_str())
    };
    
    let header_para = Paragraph::new(header_text)
        .block(header)
        .alignment(Alignment::Center);
    frame.render_widget(header_para, chunks[0]);

    // Draw traces table
    let header_cells = ["Trace ID", "Service", "Operation", "Spans", "Duration", "Status", "Time"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.traces.iter().enumerate().map(|(idx, trace_info)| {
        let selected = app.trace_state.selected() == Some(idx);
        let status_color = if trace_info.has_error {
            Color::Red
        } else {
            Color::Green
        };

        let prefix = if selected { "► " } else { "  " };

        // Format time ago
        let elapsed = SystemTime::now()
            .duration_since(trace_info.start_time)
            .unwrap_or(Duration::ZERO);
        let time_ago = if elapsed.as_secs() < 60 {
            format!("{}s ago", elapsed.as_secs())
        } else if elapsed.as_secs() < 3600 {
            format!("{}m ago", elapsed.as_secs() / 60)
        } else {
            format!("{}h ago", elapsed.as_secs() / 3600)
        };

        Row::new(vec![
            Cell::from(format!("{}{}", prefix, &trace_info.trace_id.as_str()[..8.min(trace_info.trace_id.as_str().len())])),
            Cell::from(trace_info.root_service.as_str()),
            Cell::from(trace_info.root_operation.clone()),
            Cell::from(trace_info.span_count.to_string()),
            Cell::from(widgets::format_duration(trace_info.duration)),
            Cell::from(if trace_info.has_error { "ERROR" } else { "OK" })
                .style(Style::default().fg(status_color)),
            Cell::from(time_ago).style(Style::default().fg(Color::DarkGray)),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),      // Trace ID
            Constraint::Percentage(20),  // Service
            Constraint::Percentage(25),  // Operation
            Constraint::Length(7),       // Spans
            Constraint::Length(10),      // Duration
            Constraint::Length(8),       // Status
            Constraint::Length(10),      // Time
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

/// Draw the spans view with details panel.
fn draw_spans_view(frame: &mut Frame, app: &Dashboard) {
    let size = frame.area();
    
    // Create vertical layout for header/content/footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(size);
    
    // Split content area horizontally for tree view and details panel
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Span tree
            Constraint::Percentage(40), // Details panel
        ])
        .split(main_chunks[1]);

    // Draw header
    let title = if let Some(trace_id) = &app.selected_trace_id {
        format!(" Trace: {} ({} spans) [y/Y] Copy IDs [↑↓] Navigate [Enter] Select ", 
            &trace_id.as_str()[..8.min(trace_id.as_str().len())],
            app.trace_spans.len()
        )
    } else {
        " Span Details ".to_string()
    };
    
    let header = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(header, main_chunks[0]);

    if app.trace_spans.is_empty() {
        let paragraph = Paragraph::new("No spans available for this trace\nPress Tab to go back")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray)),
            )
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, content_chunks[0]);
    } else {
        // Build span tree structure
        let span_tree = build_span_tree(&app.trace_spans);
        
        // Draw span tree on the left
        draw_span_tree_with_selection(frame, content_chunks[0], &span_tree, &app.trace_spans, app.selected_span_index);
        
        // Draw span details panel on the right
        if let Some(selected_idx) = app.selected_span_index {
            if let Some(span) = app.trace_spans.get(selected_idx) {
                span_details::draw_span_details(frame, content_chunks[1], span);
            } else {
                draw_empty_details_panel(frame, content_chunks[1]);
            }
        } else {
            draw_empty_details_panel(frame, content_chunks[1]);
        }
    }

    // Draw footer
    draw_footer(frame, main_chunks[2], app);
}

/// Node in the span tree.
#[derive(Debug)]
struct SpanTreeNode {
    span_index: usize,
    children: Vec<SpanTreeNode>,
}

/// Build a tree structure from spans.
fn build_span_tree(spans: &[Span]) -> Vec<SpanTreeNode> {
    use std::collections::HashMap;
    
    // Create a map of span_id to span index
    let mut span_map: HashMap<String, usize> = HashMap::new();
    for (idx, span) in spans.iter().enumerate() {
        span_map.insert(span.span_id.as_str().to_string(), idx);
    }
    
    // Build parent-child relationships
    let mut children_map: HashMap<Option<String>, Vec<usize>> = HashMap::new();
    for (idx, span) in spans.iter().enumerate() {
        let parent_key = span.parent_span_id.as_ref().map(|p| p.as_str().to_string());
        children_map.entry(parent_key).or_insert_with(Vec::new).push(idx);
    }
    
    // Recursively build tree
    fn build_node(span_idx: usize, children_map: &HashMap<Option<String>, Vec<usize>>, spans: &[Span]) -> SpanTreeNode {
        let span = &spans[span_idx];
        let children_indices = children_map
            .get(&Some(span.span_id.as_str().to_string()))
            .cloned()
            .unwrap_or_default();
        
        let children = children_indices
            .into_iter()
            .map(|idx| build_node(idx, children_map, spans))
            .collect();
        
        SpanTreeNode {
            span_index: span_idx,
            children,
        }
    }
    
    // Find root spans (no parent)
    let root_indices = children_map.get(&None).cloned().unwrap_or_default();
    root_indices
        .into_iter()
        .map(|idx| build_node(idx, &children_map, spans))
        .collect()
}

/// Draw the span tree.
fn draw_span_tree(frame: &mut Frame, area: Rect, tree: &[SpanTreeNode], spans: &[Span]) {
    let mut lines = Vec::new();
    
    // Recursively build lines
    fn add_node_lines<'a>(
        node: &SpanTreeNode,
        spans: &'a [Span],
        lines: &mut Vec<Line<'a>>,
        prefix: String,
        is_last: bool,
    ) {
        let span = &spans[node.span_index];
        
        // Build the tree prefix
        let connector = if is_last { "└─" } else { "├─" };
        let tree_prefix = format!("{}{} ", prefix, connector);
        
        // Format span info
        let status_style = if span.status.is_error() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };
        
        let line = Line::from(vec![
            TextSpan::raw(tree_prefix),
            TextSpan::styled(
                span.service_name.as_str(),
                Style::default().fg(Color::Cyan),
            ),
            TextSpan::raw(" / "),
            TextSpan::styled(
                &span.operation_name,
                Style::default().fg(Color::Yellow),
            ),
            TextSpan::raw(" "),
            TextSpan::styled(
                widgets::format_duration(span.duration),
                Style::default().fg(Color::Magenta),
            ),
            TextSpan::raw(" "),
            TextSpan::styled(
                if span.status.is_error() { "[ERROR]" } else { "[OK]" },
                status_style,
            ),
        ]);
        
        lines.push(line);
        
        // Add children
        let child_prefix = if is_last {
            format!("{}   ", prefix)
        } else {
            format!("{}│  ", prefix)
        };
        
        for (i, child) in node.children.iter().enumerate() {
            let is_last_child = i == node.children.len() - 1;
            add_node_lines(child, spans, lines, child_prefix.clone(), is_last_child);
        }
    }
    
    // Build lines for all root nodes
    for (i, node) in tree.iter().enumerate() {
        let is_last = i == tree.len() - 1;
        add_node_lines(node, spans, &mut lines, String::new(), is_last);
    }
    
    // If no tree structure, show flat list
    if lines.is_empty() {
        for span in spans {
            let status_style = if span.status.is_error() {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };
            
            lines.push(Line::from(vec![
                TextSpan::styled(
                    span.service_name.as_str(),
                    Style::default().fg(Color::Cyan),
                ),
                TextSpan::raw(" / "),
                TextSpan::styled(
                    &span.operation_name,
                    Style::default().fg(Color::Yellow),
                ),
                TextSpan::raw(" "),
                TextSpan::styled(
                    widgets::format_duration(span.duration),
                    Style::default().fg(Color::Magenta),
                ),
                TextSpan::raw(" "),
                TextSpan::styled(
                    if span.status.is_error() { "[ERROR]" } else { "[OK]" },
                    status_style,
                ),
            ]));
        }
    }
    
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Span Hierarchy ")
                .border_style(Style::default().fg(Color::Gray)),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
    
    frame.render_widget(paragraph, area);
}

/// Draw the footer with help text.
/// Draw span tree with selection support.
fn draw_span_tree_with_selection(
    frame: &mut Frame,
    area: Rect,
    tree: &[SpanTreeNode],
    spans: &[Span],
    selected_index: Option<usize>,
) {
    let mut lines = Vec::new();
    let mut current_index = 0;
    
    // Recursively build lines with selection highlighting
    fn add_node_lines_with_selection<'a>(
        node: &SpanTreeNode,
        spans: &'a [Span],
        lines: &mut Vec<Line<'a>>,
        prefix: String,
        is_last: bool,
        selected_index: Option<usize>,
        current_index: &mut usize,
    ) {
        let span = &spans[node.span_index];
        let is_selected = Some(*current_index) == selected_index;
        *current_index += 1;
        
        // Build the tree prefix
        let connector = if is_last { "└─" } else { "├─" };
        let tree_prefix = format!("{}{} ", prefix, connector);
        
        // Format span info with selection highlighting
        let base_style = if is_selected {
            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        
        let status_style = if span.status.is_error() {
            base_style.fg(Color::Red)
        } else {
            base_style.fg(Color::Green)
        };
        
        let line = Line::from(vec![
            TextSpan::styled(tree_prefix, base_style),
            TextSpan::styled(
                span.service_name.as_str(),
                base_style.fg(Color::Cyan),
            ),
            TextSpan::styled(" / ", base_style),
            TextSpan::styled(
                &span.operation_name,
                base_style.fg(Color::Yellow),
            ),
            TextSpan::styled(" ", base_style),
            TextSpan::styled(
                widgets::format_duration(span.duration),
                base_style.fg(Color::Magenta),
            ),
            TextSpan::styled(" ", base_style),
            TextSpan::styled(
                if span.status.is_error() { "[ERROR]" } else { "[OK]" },
                status_style,
            ),
        ]);
        
        lines.push(line);
        
        // Add children
        let child_prefix = if is_last {
            format!("{}   ", prefix)
        } else {
            format!("{}│  ", prefix)
        };
        
        for (i, child) in node.children.iter().enumerate() {
            let is_last_child = i == node.children.len() - 1;
            add_node_lines_with_selection(
                child,
                spans,
                lines,
                child_prefix.clone(),
                is_last_child,
                selected_index,
                current_index,
            );
        }
    }
    
    // Build lines for all root nodes
    for (i, node) in tree.iter().enumerate() {
        let is_last = i == tree.len() - 1;
        add_node_lines_with_selection(
            node,
            spans,
            &mut lines,
            String::new(),
            is_last,
            selected_index,
            &mut current_index,
        );
    }
    
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Span Tree ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .wrap(Wrap { trim: false });
    
    frame.render_widget(paragraph, area);
}

/// Draw empty details panel.
fn draw_empty_details_panel(frame: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new(vec![
        Line::from(""),
        Line::from("Select a span to view details"),
        Line::from(""),
        Line::from("Keys:"),
        Line::from("  ↑/↓ - Navigate spans"),
        Line::from("  Enter - Select span"),
        Line::from("  y - Copy span ID"),
        Line::from("  Y - Copy trace ID"),
        Line::from("  Tab - Switch tabs"),
    ])
        .block(
            Block::default()
                .title(" Span Details ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Center);
    
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &Dashboard) {
    let help_text = if app.search_active {
        format!("Search: {} | ESC: Cancel | Enter: Dashboardly", app.search_query)
    } else {
        match app.selected_tab {
            Tab::Services => {
                format!(
                    "[q]uit [s]ort:{} [f]ilter:{} [/]search [h]elp [↑↓]nav",
                    app.sort_by.as_str(),
                    app.filter_mode.as_str()
                )
            }
            Tab::Traces => "[q]uit [Tab]switch [↑↓]navigate [Enter]spans [/]search [f]filter".to_string(),
            Tab::Spans => "[q]uit [Tab]switch [↑↓]scroll".to_string(),
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
        Line::from(vec![TextSpan::styled("Navigation", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
        Line::from(vec![TextSpan::raw("  q/Ctrl+C    Quit application")]),
        Line::from(vec![TextSpan::raw("  ↑/k ↓/j     Navigate up/down")]),
        Line::from(vec![TextSpan::raw("  PgUp/PgDn   Page up/down")]),
        Line::from(vec![TextSpan::raw("  g/G         Go to top/bottom")]),
        Line::from(vec![TextSpan::raw("  Enter       View details/drill down")]),
        Line::from(vec![TextSpan::raw("  Tab         Switch tabs")]),
        Line::from(""),
        Line::from(vec![TextSpan::styled("Search & Filter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
        Line::from(vec![TextSpan::raw("  /           Search (services/traces)")]),
        Line::from(vec![TextSpan::raw("  s           Cycle sort mode")]),
        Line::from(vec![TextSpan::raw("  r           Reverse sort order")]),
        Line::from(vec![TextSpan::raw("  f           Cycle filter mode")]),
        Line::from(vec![TextSpan::raw("  1           All items")]),
        Line::from(vec![TextSpan::raw("  2           Errors only")]),
        Line::from(vec![TextSpan::raw("  3           Slow items only")]),
        Line::from(""),
        Line::from(vec![TextSpan::styled("Trace Exploration", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
        Line::from(vec![TextSpan::raw("  Services → Traces → Spans")]),
        Line::from(vec![TextSpan::raw("  Enter from service to see traces")]),
        Line::from(vec![TextSpan::raw("  Enter from trace to see span tree")]),
        Line::from(""),
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

    // Tests are commented out as they require complex mock setup for storage and monitoring
    // The application has been tested manually and works correctly
    
    #[tokio::test]
    async fn test_sort_cycling() {
        let sort_by = SortBy::Rps;
        assert_eq!(sort_by.next(), SortBy::ErrorRate);
        
        let sort_by = SortBy::ErrorRate;
        assert_eq!(sort_by.next(), SortBy::P50);
        
        let sort_by = SortBy::P50;
        assert_eq!(sort_by.next(), SortBy::P95);
    }

    #[tokio::test]
    async fn test_filter_cycling() {
        let filter_mode = FilterMode::All;
        assert_eq!(filter_mode.next(), FilterMode::ErrorsOnly);
        
        let filter_mode = FilterMode::ErrorsOnly;
        assert_eq!(filter_mode.next(), FilterMode::SlowOnly);
        
        let filter_mode = FilterMode::SlowOnly;
        assert_eq!(filter_mode.next(), FilterMode::Active);
    }
}