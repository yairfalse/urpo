//! Service health dashboard component for TUI
//!
//! Displays real-time service health metrics including request rates,
//! error percentages, and latency percentiles.

use crate::metrics::storage::{MetricStorage, ServiceHealth};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Service health dashboard view
pub struct ServiceHealthView {
    /// Metrics storage backend
    storage: Arc<Mutex<MetricStorage>>,
    /// Currently selected service index
    selected_index: usize,
    /// Cached service health data
    services: Vec<ServiceHealth>,
    /// Last update timestamp
    last_update: std::time::Instant,
}

impl ServiceHealthView {
    /// Create new service health dashboard
    pub fn new(storage: Arc<Mutex<MetricStorage>>) -> Self {
        Self {
            storage,
            selected_index: 0,
            services: Vec::new(),
            last_update: std::time::Instant::now(),
        }
    }

    /// Update metrics from storage
    pub async fn update_metrics(&mut self) -> Result<(), String> {
        // Only update if 1 second has passed
        if self.last_update.elapsed().as_secs() < 1 {
            return Ok(());
        }

        let storage = self.storage.lock().await;
        let service_ids = storage.list_services();

        let mut services = Vec::new();
        for service_id in service_ids {
            if let Some(health) = storage.get_service_health(service_id) {
                services.push(health);
            }
        }

        // Sort by request rate (highest first)
        services.sort_by(|a, b| {
            b.request_rate
                .partial_cmp(&a.request_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.services = services;
        self.last_update = std::time::Instant::now();
        Ok(())
    }

    /// Render the service health dashboard
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);

        let selected_style = Style::default()
            .bg(Color::Gray)
            .add_modifier(Modifier::BOLD);

        // Create header
        let header = Row::new(vec![
            Cell::from("Service"),
            Cell::from("Req/s"),
            Cell::from("Error %"),
            Cell::from("Avg Latency"),
            Cell::from("P95 Latency"),
        ])
        .style(header_style)
        .height(1);

        // Create rows
        let rows: Vec<Row> = self
            .services
            .iter()
            .enumerate()
            .map(|(i, health)| {
                let style = if i == self.selected_index {
                    selected_style
                } else {
                    Style::default()
                };

                let error_color = if health.error_rate > 10.0 {
                    Color::Red
                } else if health.error_rate > 5.0 {
                    Color::Yellow
                } else {
                    Color::Green
                };

                Row::new(vec![
                    Cell::from(format!("service-{}", health.service_id)),
                    Cell::from(format!("{:.1}", health.request_rate)),
                    Cell::from(Span::styled(
                        format!("{:.1}%", health.error_rate),
                        Style::default().fg(error_color),
                    )),
                    Cell::from(format!("{:.0}ms", health.avg_latency_ms)),
                    Cell::from(format!("{:.0}ms", health.p95_latency_ms)),
                ])
                .style(style)
            })
            .collect();

        // Create table
        let table = Table::new(
            rows,
            &[
                Constraint::Percentage(30),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(" Service Health Dashboard ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        );

        f.render_widget(table, area);

        // Show empty state if no services
        if self.services.is_empty() {
            let empty_msg = vec![
                Line::from(""),
                Line::from("No services detected"),
                Line::from(""),
                Line::from("Waiting for metrics..."),
            ];

            let paragraph = ratatui::widgets::Paragraph::new(empty_msg)
                .block(
                    Block::default()
                        .title(" Service Health Dashboard ")
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::Gray))
                .alignment(ratatui::layout::Alignment::Center);

            f.render_widget(paragraph, area);
        }
    }

    /// Handle keyboard navigation
    pub fn handle_navigation(&mut self, key: char) {
        match key {
            'j' | 'J' => self.move_down(),
            'k' | 'K' => self.move_up(),
            'g' => self.move_to_top(),
            'G' => self.move_to_bottom(),
            _ => {},
        }
    }

    fn move_down(&mut self) {
        if !self.services.is_empty() && self.selected_index < self.services.len() - 1 {
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
        if !self.services.is_empty() {
            self.selected_index = self.services.len() - 1;
        }
    }

    /// Get currently selected service
    pub fn selected_service(&self) -> Option<&ServiceHealth> {
        self.services.get(self.selected_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::types::MetricPoint;

    async fn create_test_view() -> ServiceHealthView {
        let storage = Arc::new(Mutex::new(MetricStorage::new(1024, 10)));
        ServiceHealthView::new(storage)
    }

    #[tokio::test]
    async fn test_service_health_view_creation() {
        let view = create_test_view().await;
        assert_eq!(view.selected_index, 0);
        assert!(view.services.is_empty());
    }

    #[tokio::test]
    async fn test_update_metrics_empty() {
        let mut view = create_test_view().await;
        let result = view.update_metrics().await;
        assert!(result.is_ok());
        assert!(view.services.is_empty());
    }

    #[tokio::test]
    async fn test_update_metrics_with_data() {
        let storage = Arc::new(Mutex::new(MetricStorage::new(1024, 10)));

        // Add some test metrics
        {
            let mut storage_guard = storage.lock().await;
            let metrics = vec![
                MetricPoint::new(1234567890, 1, 1, 1500.0),
                MetricPoint::new(1234567891, 1, 1, 1200.0),
                MetricPoint::new(1234567892, 2, 1, 800.0),
            ];
            storage_guard.process_metrics(&metrics).unwrap();
        }

        let mut view = ServiceHealthView::new(storage);
        view.update_metrics().await.unwrap();

        assert_eq!(view.services.len(), 2);
    }

    #[test]
    fn test_navigation_down() {
        let storage = Arc::new(Mutex::new(MetricStorage::new(1024, 10)));
        let mut view = ServiceHealthView {
            storage,
            selected_index: 0,
            services: vec![
                ServiceHealth {
                    service_id: 1,
                    request_rate: 100.0,
                    error_rate: 0.0,
                    avg_latency_ms: 50.0,
                    p95_latency_ms: 100.0,
                    last_updated: std::time::SystemTime::now(),
                },
                ServiceHealth {
                    service_id: 2,
                    request_rate: 200.0,
                    error_rate: 5.0,
                    avg_latency_ms: 75.0,
                    p95_latency_ms: 150.0,
                    last_updated: std::time::SystemTime::now(),
                },
            ],
            last_update: std::time::Instant::now(),
        };

        view.handle_navigation('j');
        assert_eq!(view.selected_index, 1);

        // Should not go beyond last item
        view.handle_navigation('j');
        assert_eq!(view.selected_index, 1);
    }

    #[test]
    fn test_navigation_up() {
        let storage = Arc::new(Mutex::new(MetricStorage::new(1024, 10)));
        let mut view = ServiceHealthView {
            storage,
            selected_index: 1,
            services: vec![
                ServiceHealth {
                    service_id: 1,
                    request_rate: 100.0,
                    error_rate: 0.0,
                    avg_latency_ms: 50.0,
                    p95_latency_ms: 100.0,
                    last_updated: std::time::SystemTime::now(),
                },
                ServiceHealth {
                    service_id: 2,
                    request_rate: 200.0,
                    error_rate: 5.0,
                    avg_latency_ms: 75.0,
                    p95_latency_ms: 150.0,
                    last_updated: std::time::SystemTime::now(),
                },
            ],
            last_update: std::time::Instant::now(),
        };

        view.handle_navigation('k');
        assert_eq!(view.selected_index, 0);

        // Should not go below 0
        view.handle_navigation('k');
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn test_navigation_top_bottom() {
        let storage = Arc::new(Mutex::new(MetricStorage::new(1024, 10)));
        let mut view = ServiceHealthView {
            storage,
            selected_index: 1,
            services: vec![
                ServiceHealth {
                    service_id: 1,
                    request_rate: 100.0,
                    error_rate: 0.0,
                    avg_latency_ms: 50.0,
                    p95_latency_ms: 100.0,
                    last_updated: std::time::SystemTime::now(),
                },
                ServiceHealth {
                    service_id: 2,
                    request_rate: 200.0,
                    error_rate: 5.0,
                    avg_latency_ms: 75.0,
                    p95_latency_ms: 150.0,
                    last_updated: std::time::SystemTime::now(),
                },
                ServiceHealth {
                    service_id: 3,
                    request_rate: 150.0,
                    error_rate: 10.0,
                    avg_latency_ms: 60.0,
                    p95_latency_ms: 120.0,
                    last_updated: std::time::SystemTime::now(),
                },
            ],
            last_update: std::time::Instant::now(),
        };

        view.handle_navigation('G');
        assert_eq!(view.selected_index, 2);

        view.handle_navigation('g');
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn test_selected_service() {
        let storage = Arc::new(Mutex::new(MetricStorage::new(1024, 10)));
        let view = ServiceHealthView {
            storage,
            selected_index: 1,
            services: vec![
                ServiceHealth {
                    service_id: 1,
                    request_rate: 100.0,
                    error_rate: 0.0,
                    avg_latency_ms: 50.0,
                    p95_latency_ms: 100.0,
                    last_updated: std::time::SystemTime::now(),
                },
                ServiceHealth {
                    service_id: 2,
                    request_rate: 200.0,
                    error_rate: 5.0,
                    avg_latency_ms: 75.0,
                    p95_latency_ms: 150.0,
                    last_updated: std::time::SystemTime::now(),
                },
            ],
            last_update: std::time::Instant::now(),
        };

        let selected = view.selected_service().unwrap();
        assert_eq!(selected.service_id, 2);
    }

    #[test]
    fn test_selected_service_empty() {
        let storage = Arc::new(Mutex::new(MetricStorage::new(1024, 10)));
        let view = ServiceHealthView {
            storage,
            selected_index: 0,
            services: vec![],
            last_update: std::time::Instant::now(),
        };

        assert!(view.selected_service().is_none());
    }
}
