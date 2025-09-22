//! Integration test for metrics storage

#[cfg(test)]
mod integration_tests {
    use crate::metrics::{storage::MetricStorage, types::MetricPoint};

    #[test]
    fn test_full_metrics_integration() {
        let mut storage = MetricStorage::new(1024, 10);

        // Test complete flow: metrics -> aggregation -> health calculation
        let metrics = vec![
            MetricPoint::new(1234567890, 1, 1, 1500.0), // Service 1, latency metric
            MetricPoint::new(1234567891, 1, 2, 0.8),    // Service 1, error metric
            MetricPoint::new(1234567892, 1, 1, 1200.0), // Service 1, latency metric
            MetricPoint::new(1234567893, 2, 1, 800.0),  // Service 2, latency metric
        ];

        // Process metrics
        let processed = storage.process_metrics(&metrics).unwrap();
        assert_eq!(processed, 4);

        // Verify services are tracked
        let services = storage.list_services();
        assert_eq!(services.len(), 2);
        assert!(services.contains(&1));
        assert!(services.contains(&2));

        // Get service health for service 1
        let health1 = storage.get_service_health(1).unwrap();
        assert_eq!(health1.service_id, 1);
        assert!(health1.request_rate > 0.0);
        assert_eq!(health1.error_rate, 33.33333333333333); // 1 error out of 3 metrics for service 1
        assert!((health1.avg_latency_ms - 1350.0).abs() < 50.0); // Average of 1500, 1200

        // Get service health for service 2
        let health2 = storage.get_service_health(2).unwrap();
        assert_eq!(health2.service_id, 2);
        assert!(health2.request_rate > 0.0);
        assert_eq!(health2.error_rate, 0.0); // No errors
        assert!((health2.avg_latency_ms - 800.0).abs() < 10.0);

        // Verify memory usage is reasonable
        let memory_usage = storage.get_memory_usage();
        assert!(memory_usage > 0);
        println!("Memory usage for 4 metrics across 2 services: {} bytes", memory_usage);
    }

    #[test]
    fn test_metrics_performance_target() {
        let mut storage = MetricStorage::new(8192, 1000);

        // Generate 1000 metrics to test performance
        let metrics: Vec<MetricPoint> = (0..1000)
            .map(|i| {
                MetricPoint::new(
                    1234567890 + i,
                    (i % 10) as u16, // 10 different services
                    1,
                    1000.0 + (i % 500) as f64, // Varying latencies
                )
            })
            .collect();

        let start = std::time::Instant::now();
        let processed = storage.process_metrics(&metrics).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(processed, 1000);

        // Target: <5μs per metric = 5ms for 1000 metrics
        println!(
            "Processed {} metrics in {:?} ({:.2}μs per metric)",
            processed,
            elapsed,
            elapsed.as_micros() as f64 / processed as f64
        );

        // Verify all services are tracked
        let services = storage.list_services();
        assert_eq!(services.len(), 10);

        // Verify memory usage is reasonable (<30MB for 500K points, so much less for 1K)
        let memory_usage = storage.get_memory_usage();
        println!(
            "Memory usage for {} metrics: {} bytes ({:.2} KB)",
            processed,
            memory_usage,
            memory_usage as f64 / 1024.0
        );
        assert!(memory_usage < 100_000); // Should be well under 100KB for 1K metrics
    }
}
