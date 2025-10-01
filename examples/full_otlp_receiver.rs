//! Example showing full OTLP receiver with traces, metrics, and logs support.

use std::sync::Arc;
use urpo::monitoring::Monitor;
use urpo::receiver::OtelReceiver;
use urpo::storage::memory::InMemoryStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();

    // Create storage backend
    let storage = Arc::new(tokio::sync::RwLock::new(InMemoryStorage::new(10_000)));

    // Create health monitor
    let monitor = Arc::new(Monitor::new());

    // Create receiver with full OTLP support
    let receiver = OtelReceiver::with_storage(
        4317, // GRPC port (standard OTLP)
        4318, // HTTP port (standard OTLP)
        &urpo::storage::UnifiedStorage::new(storage),
        monitor,
    )
    .with_batch_processing(1024)           // Batch up to 1024 spans
    .with_smart_sampling(100)              // Smart sampling with 100GB budget
    .with_metrics(100_000, 1000)           // Support 100K metrics, 1K services
    .with_logs(50_000, 1000); // Support 50K logs, 1K services

    println!("🚀 Starting full OTLP receiver...");
    println!("   - Traces:  GRPC :4317, HTTP :4318");
    println!("   - Metrics: GRPC :4317");
    println!("   - Logs:    GRPC :4317");
    println!("   - Features: Batch processing, Smart sampling, Full OTEL compliance");

    // Run the receiver
    Arc::new(receiver).run().await?;

    Ok(())
}
