//! Test program to verify OTLP receiver endpoints are properly working
//!
//! This program starts an OTLP receiver and verifies it's listening on the correct ports.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use urpo_lib::{
    monitoring::Monitor,
    receiver::OtelReceiver,
    storage::{InMemoryStorage, StorageBackend},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("urpo=info,test_receiver=info")
        .init();

    tracing::info!("🚀 Starting OTLP receiver test");

    // Create storage and monitor
    let storage: Arc<RwLock<dyn StorageBackend>> = Arc::new(RwLock::new(InMemoryStorage::new(1000)));
    let monitor = Arc::new(Monitor::new());

    // Create receiver with 100% sampling
    let receiver = OtelReceiver::new(4317, 4318, storage, monitor);
    let receiver_arc = Arc::new(receiver);

    tracing::info!("🔧 Starting OTLP receiver on ports 4317 (gRPC) and 4318 (HTTP)");

    // Start receiver
    let receiver_clone = Arc::clone(&receiver_arc);
    let receiver_handle = tokio::spawn(async move {
        if let Err(e) = receiver_clone.run().await {
            tracing::error!("Receiver error: {}", e);
        }
    });

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(1000)).await;

    tracing::info!("✅ OTLP receiver should now be running on:");
    tracing::info!("   - gRPC: localhost:4317");
    tracing::info!("   - HTTP: localhost:4318");
    tracing::info!("");
    tracing::info!("🔍 To test with otelgen, run:");
    tracing::info!("   otelgen --protocol grpc --endpoint localhost:4317 --insecure traces --count 10");
    tracing::info!("");
    tracing::info!("Press Ctrl+C to stop...");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    tracing::info!("🛑 Shutting down receiver...");
    receiver_handle.abort();

    Ok(())
}