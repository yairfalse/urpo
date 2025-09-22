//! Simple OTEL load generator - sends real spans to Urpo
//!
//! Usage: cargo run --example otel_load -- --rate 10000

use clap::Parser;
use opentelemetry_proto::tonic::collector::trace::v1::{
    trace_service_client::TraceServiceClient, ExportTraceServiceRequest,
};
use opentelemetry_proto::tonic::common::v1::{AnyValue, InstrumentationScope, KeyValue};
use opentelemetry_proto::tonic::resource::v1::Resource;
use opentelemetry_proto::tonic::trace::v1::{ResourceSpans, ScopeSpans, Span, Status};
use rand::Rng;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tonic::Request;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Target URL (default: localhost:4317)
    #[arg(short, long, default_value = "http://localhost:4317")]
    target: String,

    /// Spans per second to send
    #[arg(short, long, default_value = "1000")]
    rate: u32,

    /// Total spans to send (0 = infinite)
    #[arg(short = 'n', long, default_value = "0")]
    count: u64,

    /// Number of services to simulate
    #[arg(short, long, default_value = "10")]
    services: u32,

    /// Batch size for sending
    #[arg(short, long, default_value = "100")]
    batch: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("🚀 OTEL Load Generator");
    println!("📡 Target: {}", args.target);
    println!("⚡ Rate: {} spans/sec", args.rate);
    println!("📦 Batch size: {}", args.batch);
    println!("🔧 Services: {}", args.services);
    println!("{}", "─".repeat(50));

    let mut client = TraceServiceClient::connect(args.target.clone()).await?;

    let mut total_sent = 0u64;
    let mut total_errors = 0u64;
    let start_time = Instant::now();
    let mut last_report = Instant::now();
    let mut rng = rand::thread_rng();

    // Calculate delay between batches
    let batch_delay = Duration::from_millis(1000 * args.batch as u64 / args.rate as u64);

    loop {
        let batch_start = Instant::now();

        // Generate batch of spans
        let mut spans = Vec::with_capacity(args.batch);
        for _ in 0..args.batch {
            spans.push(generate_span(&mut rng, args.services));
        }

        // Group spans by service
        let resource_spans = group_spans_by_service(spans);

        // Send to OTLP endpoint
        let request = Request::new(ExportTraceServiceRequest { resource_spans });

        match client.export(request).await {
            Ok(_) => {
                total_sent += args.batch as u64;
            },
            Err(e) => {
                total_errors += 1;
                eprintln!("❌ Error sending: {}", e);
            },
        }

        // Report progress every second
        if last_report.elapsed() >= Duration::from_secs(1) {
            let elapsed = start_time.elapsed().as_secs_f64();
            let actual_rate = total_sent as f64 / elapsed;
            println!(
                "📊 Sent: {} | Rate: {:.0}/s | Errors: {} | Time: {:.1}s",
                total_sent, actual_rate, total_errors, elapsed
            );
            last_report = Instant::now();
        }

        // Check if we've sent enough
        if args.count > 0 && total_sent >= args.count {
            break;
        }

        // Rate limiting
        let batch_duration = batch_start.elapsed();
        if batch_duration < batch_delay {
            tokio::time::sleep(batch_delay - batch_duration).await;
        }
    }

    // Final report
    let elapsed = start_time.elapsed().as_secs_f64();
    let actual_rate = total_sent as f64 / elapsed;

    println!("{}", "─".repeat(50));
    println!("✅ Complete!");
    println!("📈 Total sent: {} spans", total_sent);
    println!("⚡ Average rate: {:.0} spans/sec", actual_rate);
    println!("❌ Errors: {}", total_errors);
    println!("⏱️  Duration: {:.1}s", elapsed);

    Ok(())
}

fn generate_span(rng: &mut impl Rng, num_services: u32) -> (String, Span) {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let trace_id = rng.gen::<[u8; 16]>();
    let span_id = rng.gen::<[u8; 8]>();
    let service_id = rng.gen_range(0..num_services);
    let service_name = format!("service-{}", service_id);

    // Random operation names
    let operations = [
        "GET /api/users",
        "POST /api/orders",
        "SELECT * FROM users",
        "cache.get",
        "queue.publish",
        "http.request",
    ];
    let operation = operations[rng.gen_range(0..operations.len())];

    // Random duration between 1ms and 500ms
    let duration_ns = rng.gen_range(1_000_000..500_000_000);

    // 5% error rate
    let status = if rng.gen_bool(0.05) {
        Some(Status {
            code: 2, // ERROR
            message: "Internal server error".to_string(),
        })
    } else {
        Some(Status {
            code: 1, // OK
            message: String::new(),
        })
    };

    let span = Span {
        trace_id: trace_id.to_vec(),
        span_id: span_id.to_vec(),
        trace_state: String::new(),
        parent_span_id: vec![],
        name: operation.to_string(),
        kind: 2,  // SERVER
        flags: 0, // Not sampled
        start_time_unix_nano: now.as_nanos() as u64 - duration_ns,
        end_time_unix_nano: now.as_nanos() as u64,
        attributes: vec![
            KeyValue {
                key: "http.method".to_string(),
                value: Some(AnyValue {
                    value: Some(
                        opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(
                            "GET".to_string(),
                        ),
                    ),
                }),
            },
            KeyValue {
                key: "http.status_code".to_string(),
                value: Some(AnyValue {
                    value: Some(
                        opentelemetry_proto::tonic::common::v1::any_value::Value::IntValue(200),
                    ),
                }),
            },
        ],
        dropped_attributes_count: 0,
        events: vec![],
        dropped_events_count: 0,
        links: vec![],
        dropped_links_count: 0,
        status,
    };

    (service_name, span)
}

fn group_spans_by_service(spans: Vec<(String, Span)>) -> Vec<ResourceSpans> {
    use std::collections::HashMap;

    let mut services: HashMap<String, Vec<Span>> = HashMap::new();

    for (service, span) in spans {
        services.entry(service).or_default().push(span);
    }

    services
        .into_iter()
        .map(|(service_name, spans)| ResourceSpans {
            resource: Some(Resource {
                attributes: vec![KeyValue {
                    key: "service.name".to_string(),
                    value: Some(AnyValue {
                        value: Some(
                            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(
                                service_name,
                            ),
                        ),
                    }),
                }],
                dropped_attributes_count: 0,
            }),
            scope_spans: vec![ScopeSpans {
                scope: Some(InstrumentationScope {
                    name: "otel-load-generator".to_string(),
                    version: "1.0.0".to_string(),
                    attributes: vec![],
                    dropped_attributes_count: 0,
                }),
                spans,
                schema_url: String::new(),
            }],
            schema_url: String::new(),
        })
        .collect()
}
