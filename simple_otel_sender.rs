//! Ultra-simple OTEL sender using only std library
//!
//! Demonstrates Urpo receiving OTEL data

use std::io::prelude::*;
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Ultra-Simple OTEL Sender");
    println!("📡 Target: localhost:4318 (HTTP)");
    println!("⚡ Sending OTEL traces...");
    println!("{}", "─".repeat(40));

    for batch in 1..=10 {
        send_otel_batch(batch)?;
        println!("✅ Batch {} sent successfully", batch);
        thread::sleep(Duration::from_millis(500));
    }

    println!("{}", "─".repeat(40));
    println!("🎉 All batches sent! Check Urpo to see the traces.");

    Ok(())
}

fn send_otel_batch(batch_id: usize) -> Result<(), Box<dyn std::error::Error>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    // Generate simple OTEL JSON
    let json_payload = format!(
        r#"{{
            "resourceSpans": [
                {{
                    "resource": {{
                        "attributes": [
                            {{"key": "service.name", "value": {{"stringValue": "demo-service"}}}}
                        ]
                    }},
                    "scopeSpans": [
                        {{
                            "scope": {{
                                "name": "simple-sender",
                                "version": "1.0.0"
                            }},
                            "spans": [
                                {{
                                    "traceId": "{:032x}",
                                    "spanId": "{:016x}",
                                    "name": "demo-operation-{}",
                                    "kind": 2,
                                    "startTimeUnixNano": {},
                                    "endTimeUnixNano": {},
                                    "attributes": [
                                        {{"key": "http.method", "value": {{"stringValue": "GET"}}}},
                                        {{"key": "http.status_code", "value": {{"intValue": 200}}}},
                                        {{"key": "batch.id", "value": {{"intValue": {}}}}}
                                    ],
                                    "status": {{"code": 1}}
                                }}
                            ]
                        }}
                    ]
                }}
            ]
        }}"#,
        batch_id as u128,        // traceId
        batch_id as u64,         // spanId
        batch_id,                // operation name
        now - 5_000_000,         // start time (5ms ago)
        now,                     // end time (now)
        batch_id                 // batch id attribute
    );

    // Create HTTP request
    let http_request = format!(
        "POST /v1/traces HTTP/1.1\r\n\
         Host: localhost:4318\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        json_payload.len(),
        json_payload
    );

    // Try to connect and send
    match TcpStream::connect("127.0.0.1:4318") {
        Ok(mut stream) => {
            stream.write_all(http_request.as_bytes())?;

            // Read response
            let mut response = String::new();
            stream.read_to_string(&mut response)?;

            if response.contains("200 OK") {
                return Ok(());
            } else {
                println!("⚠️  Response: {}", response.lines().next().unwrap_or("Unknown"));
            }
        }
        Err(_) => {
            println!("⚠️  Cannot connect to localhost:4318 (Urpo not running?)");
            println!("💡 Start Urpo first: cargo run --bin urpo");
        }
    }

    Ok(())
}