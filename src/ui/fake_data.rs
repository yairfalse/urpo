//! Fake data generator for UI development and testing.

use crate::core::{ServiceMetrics, ServiceName, Span, SpanId, SpanKind, SpanStatus, TraceId};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::collections::HashMap;
use std::time::Duration;

/// Service definitions for fake data
#[derive(Debug, Clone)]
pub struct FakeService {
    pub name: String,
    pub base_rps: f64,
    pub base_error_rate: f64,
    pub base_p50: u64,
    pub base_p95: u64,
    pub base_p99: u64,
    pub health_status: HealthStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Generates fake service metrics with realistic variations
pub struct FakeDataGenerator {
    services: Vec<FakeService>,
    iteration: u64,
}

impl FakeDataGenerator {
    /// Create a new fake data generator with predefined services
    pub fn new() -> Self {
        let services = vec![
            FakeService {
                name: "api-gateway".to_string(),
                base_rps: 245.0,
                base_error_rate: 0.1,
                base_p50: 12,
                base_p95: 45,
                base_p99: 89,
                health_status: HealthStatus::Healthy,
            },
            FakeService {
                name: "user-service".to_string(),
                base_rps: 156.0,
                base_error_rate: 0.8,
                base_p50: 23,
                base_p95: 78,
                base_p99: 156,
                health_status: HealthStatus::Healthy,
            },
            FakeService {
                name: "payment-service".to_string(),
                base_rps: 89.0,
                base_error_rate: 12.1,
                base_p50: 234,
                base_p95: 567,
                base_p99: 1200,
                health_status: HealthStatus::Unhealthy,
            },
            FakeService {
                name: "inventory-svc".to_string(),
                base_rps: 234.0,
                base_error_rate: 2.3,
                base_p50: 34,
                base_p95: 89,
                base_p99: 234,
                health_status: HealthStatus::Degraded,
            },
            FakeService {
                name: "notification-api".to_string(),
                base_rps: 67.0,
                base_error_rate: 0.2,
                base_p50: 15,
                base_p95: 38,
                base_p99: 72,
                health_status: HealthStatus::Healthy,
            },
        ];

        Self {
            services,
            iteration: 0,
        }
    }

    /// Generate the next set of service metrics with slight variations
    pub fn generate_metrics(&mut self) -> Vec<ServiceMetrics> {
        self.iteration += 1;
        let mut rng = thread_rng();

        self.services
            .iter()
            .map(|service| {
                // Add some realistic variations based on iteration
                let variation = (self.iteration as f64 * 0.1).sin() * 0.1;
                let jitter = rng.gen_range(-0.05..0.05);

                // Calculate varied metrics
                let rps = service.base_rps * (1.0 + variation + jitter);
                let rps = (rps * 10.0).round() / 10.0; // Round to 1 decimal

                // Error rate with some random spikes
                let error_spike = if rng.gen_bool(0.05) {
                    rng.gen_range(0.0..5.0)
                } else {
                    0.0
                };
                let error_rate = (service.base_error_rate + error_spike).min(100.0);

                // Latencies with realistic variations
                let latency_factor = 1.0 + variation * 0.5 + jitter * 0.3;
                let p50 = (service.base_p50 as f64 * latency_factor) as u64;
                let p95 = (service.base_p95 as f64 * latency_factor) as u64;
                let p99 = (service.base_p99 as f64 * latency_factor) as u64;

                // Calculate span counts based on RPS
                let span_count = (rps * 60.0) as u64; // Approximation for last minute
                let error_count = (span_count as f64 * (error_rate / 100.0)) as u64;

                ServiceMetrics {
                    service_name: ServiceName::new(service.name.clone()).unwrap(),
                    span_count,
                    error_count,
                    avg_duration_ms: p50,
                    p50_latency_ms: p50,
                    p95_latency_ms: p95,
                    p99_latency_ms: p99,
                    rps,
                    last_updated: Utc::now(),
                }
            })
            .collect()
    }

    /// Generate fake trace spans
    pub fn generate_traces(&mut self, count: usize) -> Vec<Span> {
        let mut rng = thread_rng();
        let mut spans = Vec::new();

        for _ in 0..count {
            let service = &self.services[rng.gen_range(0..self.services.len())];
            
            // Generate random trace ID (32 chars)
            let trace_id_str: String = (0..32)
                .map(|_| rng.sample(Alphanumeric) as char)
                .collect();
            let trace_id = TraceId::new(trace_id_str).unwrap();

            // Generate random span ID (16 chars)
            let span_id_str: String = (0..16)
                .map(|_| rng.sample(Alphanumeric) as char)
                .collect();
            let span_id = SpanId::new(span_id_str).unwrap();

            // Random operation names
            let operations = vec![
                "GET /api/users",
                "POST /api/orders",
                "GET /api/products",
                "PUT /api/inventory",
                "DELETE /api/sessions",
                "GET /health",
                "POST /api/payments",
            ];
            let operation = operations[rng.gen_range(0..operations.len())];

            // Generate span status based on service health
            let status = if rng.gen_range(0.0..100.0) < service.base_error_rate {
                SpanStatus::Error("Internal Server Error".to_string())
            } else {
                SpanStatus::Ok
            };

            // Random duration based on service latencies
            let duration_ms = match rng.gen_range(0..100) {
                0..=50 => service.base_p50,
                51..=95 => service.base_p95,
                _ => service.base_p99,
            };

            let now = Utc::now();
            let start_time = now - chrono::Duration::milliseconds(duration_ms as i64);

            let span = Span {
                span_id,
                trace_id,
                parent_span_id: None,
                service_name: ServiceName::new(service.name.clone()).unwrap(),
                operation_name: operation.to_string(),
                kind: SpanKind::Server,
                start_time,
                end_time: now,
                status,
                attributes: HashMap::new(),
                events: Vec::new(),
            };

            spans.push(span);
        }

        spans
    }

    /// Get health status color for a service
    pub fn health_color(metrics: &ServiceMetrics) -> HealthStatus {
        if metrics.error_rate() > 10.0 {
            HealthStatus::Unhealthy
        } else if metrics.error_rate() > 2.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
}

impl Default for FakeDataGenerator {
    fn default() -> Self {
        Self::new()
    }
}