//! Example of sending OpenTelemetry traces to Urpo.
//! 
//! This demonstrates how to configure an OTEL exporter to send traces
//! to Urpo's GRPC receiver on port 4317.

use opentelemetry::{global, trace::{Tracer, TracerProvider as _}};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    runtime,
    trace::{self, RandomIdGenerator, Sampler},
    Resource,
};
use opentelemetry::KeyValue;
use std::time::Duration;
use opentelemetry::trace::TraceError;

fn init_tracer() -> Result<opentelemetry_sdk::trace::Tracer, TraceError> {
    // Set up the OTLP exporter to send to Urpo
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://localhost:4317");

    let trace_config = trace::Config::default()
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_max_events_per_span(64)
        .with_max_attributes_per_span(32)
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", "example-service"),
            KeyValue::new("service.version", "1.0.0"),
        ]));

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .install_batch(runtime::Tokio)?;
    
    global::set_tracer_provider(provider.clone());
    global::set_text_map_propagator(TraceContextPropagator::new());
    
    Ok(provider.tracer("example-tracer"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Initialize the OTEL tracer
    let tracer = init_tracer()?;
    
    tracing::info!("Starting to send traces to Urpo on localhost:4317");

    // Create some example traces
    for i in 0..10 {
        // Create a root span
        let mut root_span = tracer
            .span_builder(format!("request_{}", i))
            .with_kind(opentelemetry::trace::SpanKind::Server)
            .with_attributes(vec![
                KeyValue::new("http.method", "GET"),
                KeyValue::new("http.url", format!("/api/users/{}", i)),
                KeyValue::new("http.status_code", 200i64),
            ])
            .start(&tracer);

        // Simulate some work
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Create a child span
        tracer.in_span("database_query", |cx| {
            cx.span().set_attributes(vec![
                KeyValue::new("db.system", "postgresql"),
                KeyValue::new("db.statement", "SELECT * FROM users WHERE id = ?"),
            ]);
            
            // Simulate database query
            std::thread::sleep(Duration::from_millis(5));
        });

        // Create another child span with an error
        if i % 3 == 0 {
            tracer.in_span("cache_lookup", |cx| {
                cx.span().set_attributes(vec![
                    KeyValue::new("cache.type", "redis"),
                    KeyValue::new("cache.hit", false),
                ]);
                cx.span().record_error(&"Cache miss");
                cx.span().set_status(opentelemetry::trace::Status::error("Cache miss"));
            });
        }

        root_span.end();
        
        tracing::info!("Sent trace {} to Urpo", i);
        
        // Small delay between traces
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Give time for the last traces to be exported
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Shutdown the tracer provider to flush any remaining spans
    global::shutdown_tracer_provider();
    
    tracing::info!("Finished sending traces to Urpo");
    
    Ok(())
}