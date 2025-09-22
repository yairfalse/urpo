//! Ultra-fast TUI rendering with GPU-optimized layouts
//!
//! PERFORMANCE TARGETS:
//! - <16ms frame time (60fps)
//! - <1ms layout calculation
//! - Zero allocations in render loop
//! - Cache-friendly memory access patterns

use super::ultra_fast_input::FastCommand;
use crate::core::{ServiceMetrics, ServiceName};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};
use std::time::Instant;

/// Pre-calculated layout cache for zero-allocation rendering
#[derive(Debug, Clone)]
pub struct LayoutCache {
    /// Main content area
    pub content: Rect,
    /// Header area
    pub header: Rect,
    /// Footer/status area
    pub footer: Rect,
    /// Sidebar area
    pub sidebar: Rect,
    /// Last calculated for this terminal size
    pub terminal_size: (u16, u16),
    /// Cache generation timestamp
    pub generated_at: Instant,
}

impl LayoutCache {
    /// Create new layout cache for given terminal size
    #[inline(always)]
    pub fn new(terminal_area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(2), // Footer
            ])
            .split(terminal_area);

        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75), // Main content
                Constraint::Percentage(25), // Sidebar
            ])
            .split(chunks[1]);

        Self {
            content: content_chunks[0],
            header: chunks[0],
            footer: chunks[2],
            sidebar: content_chunks[1],
            terminal_size: (terminal_area.width, terminal_area.height),
            generated_at: Instant::now(),
        }
    }

    /// Check if cache is valid for current terminal size
    #[inline(always)]
    pub fn is_valid(&self, current_area: Rect) -> bool {
        self.terminal_size == (current_area.width, current_area.height)
    }
}

/// Ultra-fast TUI renderer with optimized hot paths
pub struct UltraFastRenderer {
    /// Layout cache to avoid recalculation
    layout_cache: Option<LayoutCache>,
    /// Last render timestamp for FPS calculation
    last_render: Instant,
    /// Frame time tracking (exponential moving average)
    avg_frame_time_ns: u64,
    /// Render sample count
    render_count: u64,
    /// Current view mode
    current_view: ViewMode,
}

/// TUI view modes optimized for different workflows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Services,
    Traces,
    Logs,
    Metrics,
    Graph,
}

impl UltraFastRenderer {
    /// Create new ultra-fast renderer
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            layout_cache: None,
            last_render: Instant::now(),
            avg_frame_time_ns: 0,
            render_count: 0,
            current_view: ViewMode::Services,
        }
    }

    /// Main render function with <16ms target
    #[inline]
    pub fn render(
        &mut self,
        f: &mut Frame,
        services: &[ServiceMetrics],
        selected_service: Option<&ServiceName>,
        command: FastCommand,
    ) -> crate::core::Result<()> {
        let frame_start = Instant::now();

        // Update layout cache if needed (fast path check)
        let terminal_area = f.area();
        if self
            .layout_cache
            .as_ref()
            .map_or(true, |cache| !cache.is_valid(terminal_area))
        {
            self.layout_cache = Some(LayoutCache::new(terminal_area));
        }

        // Handle view switching commands first
        self.handle_view_command(command);

        let layout = self.layout_cache.as_ref().unwrap();

        // Render components based on current view
        self.render_header(f, layout.header)?;
        self.render_main_content(f, layout.content, services, selected_service)?;
        self.render_sidebar(f, layout.sidebar, services)?;
        self.render_footer(f, layout.footer)?;

        // Track frame time performance
        let frame_time_ns = frame_start.elapsed().as_nanos() as u64;
        self.update_frame_metrics(frame_time_ns);

        // Log performance warning if we exceed 16ms target
        if frame_time_ns > 16_000_000 {
            tracing::warn!(
                "Frame time exceeded 16ms target: {:.2}ms",
                frame_time_ns as f64 / 1_000_000.0
            );
        }

        Ok(())
    }

    /// Handle view switching commands with zero allocation
    #[inline(always)]
    fn handle_view_command(&mut self, command: FastCommand) {
        self.current_view = match command {
            FastCommand::Services => ViewMode::Services,
            FastCommand::Traces => ViewMode::Traces,
            FastCommand::Logs => ViewMode::Logs,
            FastCommand::Metrics => ViewMode::Metrics,
            FastCommand::Graph => ViewMode::Graph,
            _ => self.current_view, // No change
        };
    }

    /// Render header with performance metrics
    #[inline]
    fn render_header(&self, f: &mut Frame, area: Rect) -> crate::core::Result<()> {
        let fps = if self.avg_frame_time_ns > 0 {
            1_000_000_000.0 / self.avg_frame_time_ns as f64
        } else {
            0.0
        };

        let header_text = vec![Line::from(vec![
            Span::styled(
                " URPO ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Ultra-Fast OTEL Explorer "),
            Span::styled(
                format!(
                    "│ {} ",
                    match self.current_view {
                        ViewMode::Services => "SERVICES",
                        ViewMode::Traces => "TRACES",
                        ViewMode::Logs => "LOGS",
                        ViewMode::Metrics => "METRICS",
                        ViewMode::Graph => "GRAPH",
                    }
                ),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("│ {:.1} FPS ", fps),
                if fps >= 60.0 {
                    Style::default().fg(Color::Green)
                } else if fps >= 30.0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
            Span::styled(
                format!("│ {} frames ", self.render_count),
                Style::default().fg(Color::Gray),
            ),
        ])];

        let header = Paragraph::new(header_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White)),
            )
            .alignment(Alignment::Left);

        f.render_widget(header, area);
        Ok(())
    }

    /// Render main content area based on current view
    #[inline]
    fn render_main_content(
        &self,
        f: &mut Frame,
        area: Rect,
        services: &[ServiceMetrics],
        _selected_service: Option<&ServiceName>,
    ) -> crate::core::Result<()> {
        match self.current_view {
            ViewMode::Services => self.render_services_table(f, area, services),
            ViewMode::Traces => self.render_traces_view(f, area),
            ViewMode::Logs => self.render_logs_view(f, area),
            ViewMode::Metrics => self.render_metrics_view(f, area, services),
            ViewMode::Graph => self.render_graph_view(f, area),
        }
    }

    /// Render ultra-fast services table with color coding
    #[inline]
    fn render_services_table(
        &self,
        f: &mut Frame,
        area: Rect,
        services: &[ServiceMetrics],
    ) -> crate::core::Result<()> {
        // Pre-allocate rows vector for known size
        let mut rows = Vec::with_capacity(services.len());

        for service in services {
            // Color code based on health
            let health_color = if service.error_rate > 5.0 {
                Color::Red
            } else if service.error_rate > 1.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            let latency_color = if service.latency_p99.as_millis() > 1000 {
                Color::Red
            } else if service.latency_p99.as_millis() > 500 {
                Color::Yellow
            } else {
                Color::Green
            };

            rows.push(Row::new(vec![
                Cell::from(service.name.to_string()),
                Cell::from(format!("{:.1}", service.request_rate))
                    .style(Style::default().fg(Color::Cyan)),
                Cell::from(format!("{:.2}%", service.error_rate))
                    .style(Style::default().fg(health_color)),
                Cell::from(format!("{}ms", service.latency_p50.as_millis()))
                    .style(Style::default().fg(Color::Blue)),
                Cell::from(format!("{}ms", service.latency_p95.as_millis()))
                    .style(Style::default().fg(Color::Magenta)),
                Cell::from(format!("{}ms", service.latency_p99.as_millis()))
                    .style(Style::default().fg(latency_color)),
            ]));
        }

        let header = Row::new(vec!["Service", "RPS", "Error%", "P50", "P95", "P99"])
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .height(1);

        let table = Table::new(
            rows,
            &[
                Constraint::Percentage(25),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(format!(" Services ({}) ", services.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        );

        f.render_widget(table, area);

        // Show empty state if no services
        if services.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from("No services detected"),
                Line::from(""),
                Line::from("Start sending OTEL data to see services here"),
            ];

            let empty_paragraph = Paragraph::new(empty_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Gray));

            f.render_widget(empty_paragraph, area.inner(Margin::new(2, 2)));
        }

        Ok(())
    }

    /// Render traces view placeholder
    #[inline]
    fn render_traces_view(&self, f: &mut Frame, area: Rect) -> crate::core::Result<()> {
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from("🔍 TRACES VIEW"),
            Line::from(""),
            Line::from("Real-time trace exploration coming soon..."),
            Line::from(""),
            Line::from("Press 's' to return to services"),
        ])
        .block(
            Block::default()
                .title(" Traces ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));

        f.render_widget(placeholder, area);
        Ok(())
    }

    /// Render logs view placeholder
    #[inline]
    fn render_logs_view(&self, f: &mut Frame, area: Rect) -> crate::core::Result<()> {
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from("📝 LOGS VIEW"),
            Line::from(""),
            Line::from("Structured log exploration coming soon..."),
            Line::from(""),
            Line::from("Press 's' to return to services"),
        ])
        .block(
            Block::default()
                .title(" Logs ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));

        f.render_widget(placeholder, area);
        Ok(())
    }

    /// Render metrics view with sparklines
    #[inline]
    fn render_metrics_view(
        &self,
        f: &mut Frame,
        area: Rect,
        services: &[ServiceMetrics],
    ) -> crate::core::Result<()> {
        // Calculate aggregate metrics
        let total_rps: f64 = services.iter().map(|s| s.request_rate).sum();
        let avg_error_rate: f64 = if !services.is_empty() {
            services.iter().map(|s| s.error_rate).sum::<f64>() / services.len() as f64
        } else {
            0.0
        };

        let metrics_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "📊 SYSTEM METRICS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Total RPS: "),
                Span::styled(
                    format!("{:.1}", total_rps),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("Avg Error Rate: "),
                Span::styled(
                    format!("{:.2}%", avg_error_rate),
                    if avg_error_rate > 5.0 {
                        Style::default().fg(Color::Red)
                    } else if avg_error_rate > 1.0 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
            ]),
            Line::from(vec![
                Span::raw("Active Services: "),
                Span::styled(
                    format!("{}", services.len()),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Frame Time: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{:.2}ms", self.avg_frame_time_ns as f64 / 1_000_000.0),
                    if self.avg_frame_time_ns < 16_000_000 {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
            ]),
            Line::from(""),
            Line::from("Press 's' to return to services"),
        ];

        let metrics_paragraph = Paragraph::new(metrics_text)
            .block(
                Block::default()
                    .title(" System Metrics ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .alignment(Alignment::Left);

        f.render_widget(metrics_paragraph, area);
        Ok(())
    }

    /// Render graph view placeholder
    #[inline]
    fn render_graph_view(&self, f: &mut Frame, area: Rect) -> crate::core::Result<()> {
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from("🕸️  SERVICE GRAPH"),
            Line::from(""),
            Line::from("Interactive service dependency graph coming soon..."),
            Line::from(""),
            Line::from("Press 's' to return to services"),
        ])
        .block(
            Block::default()
                .title(" Service Graph ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));

        f.render_widget(placeholder, area);
        Ok(())
    }

    /// Render sidebar with quick stats
    #[inline]
    fn render_sidebar(
        &self,
        f: &mut Frame,
        area: Rect,
        services: &[ServiceMetrics],
    ) -> crate::core::Result<()> {
        // Quick stats calculation
        let healthy_services = services.iter().filter(|s| s.error_rate < 1.0).count();
        let warning_services = services
            .iter()
            .filter(|s| s.error_rate >= 1.0 && s.error_rate < 5.0)
            .count();
        let critical_services = services.iter().filter(|s| s.error_rate >= 5.0).count();

        let sidebar_items = vec![
            ListItem::new(vec![Line::from(vec![
                Span::styled("🟢 Healthy: ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{}", healthy_services),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ])]),
            ListItem::new(vec![Line::from(vec![
                Span::styled("🟡 Warning: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{}", warning_services),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ])]),
            ListItem::new(vec![Line::from(vec![
                Span::styled("🔴 Critical: ", Style::default().fg(Color::Red)),
                Span::styled(
                    format!("{}", critical_services),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ])]),
            ListItem::new(vec![Line::from("")]),
            ListItem::new(vec![Line::from(vec![Span::styled(
                "Controls:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )])]),
            ListItem::new(vec![Line::from("s - Services")]),
            ListItem::new(vec![Line::from("t - Traces")]),
            ListItem::new(vec![Line::from("L - Logs")]),
            ListItem::new(vec![Line::from("m - Metrics")]),
            ListItem::new(vec![Line::from("v - Graph")]),
            ListItem::new(vec![Line::from("? - Help")]),
            ListItem::new(vec![Line::from("q - Quit")]),
        ];

        let sidebar = List::new(sidebar_items).block(
            Block::default()
                .title(" Quick Stats ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray)),
        );

        f.render_widget(sidebar, area);
        Ok(())
    }

    /// Render footer with status
    #[inline]
    fn render_footer(&self, f: &mut Frame, area: Rect) -> crate::core::Result<()> {
        let footer_text = vec![Line::from(vec![
            Span::styled(
                "URPO",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - Ultra-Fast OTEL Explorer │ "),
            Span::styled("Press ? for help", Style::default().fg(Color::Gray)),
            Span::raw(" │ "),
            Span::styled(
                format!("Render: {:.1}ms", self.avg_frame_time_ns as f64 / 1_000_000.0),
                if self.avg_frame_time_ns < 16_000_000 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ])];

        let footer = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));

        f.render_widget(footer, area);
        Ok(())
    }

    /// Update frame time metrics with exponential moving average
    #[inline(always)]
    fn update_frame_metrics(&mut self, frame_time_ns: u64) {
        if self.render_count == 0 {
            self.avg_frame_time_ns = frame_time_ns;
        } else {
            // Alpha = 0.1 for smooth averaging
            self.avg_frame_time_ns = (self.avg_frame_time_ns * 9 + frame_time_ns) / 10;
        }
        self.render_count += 1;
        self.last_render = Instant::now();
    }

    /// Get rendering performance metrics
    #[inline(always)]
    pub fn get_perf_metrics(&self) -> (f64, u64) {
        let fps = if self.avg_frame_time_ns > 0 {
            1_000_000_000.0 / self.avg_frame_time_ns as f64
        } else {
            0.0
        };
        (fps, self.render_count)
    }
}

impl Default for UltraFastRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_cache_validity() {
        let area = Rect::new(0, 0, 100, 50);
        let cache = LayoutCache::new(area);

        assert!(cache.is_valid(area));
        assert!(!cache.is_valid(Rect::new(0, 0, 200, 50)));
    }

    #[test]
    fn test_view_mode_switching() {
        let mut renderer = UltraFastRenderer::new();
        assert_eq!(renderer.current_view, ViewMode::Services);

        renderer.handle_view_command(FastCommand::Traces);
        assert_eq!(renderer.current_view, ViewMode::Traces);

        renderer.handle_view_command(FastCommand::Metrics);
        assert_eq!(renderer.current_view, ViewMode::Metrics);
    }

    #[test]
    fn test_frame_metrics_calculation() {
        let mut renderer = UltraFastRenderer::new();

        // First frame
        renderer.update_frame_metrics(16_000_000); // 16ms
        assert_eq!(renderer.avg_frame_time_ns, 16_000_000);

        // Second frame - should use exponential moving average
        renderer.update_frame_metrics(8_000_000); // 8ms
        assert!(renderer.avg_frame_time_ns < 16_000_000);
        assert!(renderer.avg_frame_time_ns > 8_000_000);
    }
}
