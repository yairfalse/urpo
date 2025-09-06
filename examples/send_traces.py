#!/usr/bin/env python3
"""
Example: Send live OTEL traces to Urpo
Requires: pip install opentelemetry-api opentelemetry-sdk opentelemetry-exporter-otlp
"""

from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.resources import Resource
import time
import random

# Configure OTEL to send to Urpo (GRPC on port 4317)
resource = Resource.create({
    "service.name": "demo-service",
    "service.version": "1.0.0",
})

# Setup tracer provider
provider = TracerProvider(resource=resource)

# Configure OTLP exporter to send to Urpo
otlp_exporter = OTLPSpanExporter(
    endpoint="localhost:4317",
    insecure=True,  # Urpo doesn't use TLS
)

# Add batch processor
provider.add_span_processor(BatchSpanProcessor(otlp_exporter))

# Set as global provider
trace.set_tracer_provider(provider)

# Get a tracer
tracer = trace.get_tracer("demo-app", "1.0.0")

def simulate_api_call():
    """Simulate an API call with nested spans"""
    with tracer.start_as_current_span("api-request") as api_span:
        api_span.set_attribute("http.method", "GET")
        api_span.set_attribute("http.url", "/api/users")
        api_span.set_attribute("http.status_code", 200)
        
        # Simulate database query
        with tracer.start_as_current_span("db-query") as db_span:
            db_span.set_attribute("db.system", "postgresql")
            db_span.set_attribute("db.statement", "SELECT * FROM users")
            time.sleep(random.uniform(0.01, 0.1))  # Simulate query time
            
        # Simulate cache lookup
        with tracer.start_as_current_span("cache-lookup") as cache_span:
            cache_span.set_attribute("cache.hit", random.choice([True, False]))
            time.sleep(random.uniform(0.001, 0.01))  # Simulate cache time
            
        # Sometimes simulate an error
        if random.random() < 0.2:  # 20% error rate
            api_span.set_status(trace.Status(trace.StatusCode.ERROR, "Random error"))
            api_span.record_exception(Exception("Something went wrong"))

def simulate_background_job():
    """Simulate a background job with multiple steps"""
    with tracer.start_as_current_span("background-job") as job_span:
        job_span.set_attribute("job.name", "data-processing")
        
        # Step 1: Fetch data
        with tracer.start_as_current_span("fetch-data"):
            time.sleep(random.uniform(0.1, 0.3))
            
        # Step 2: Process data
        with tracer.start_as_current_span("process-data"):
            time.sleep(random.uniform(0.2, 0.5))
            
        # Step 3: Store results  
        with tracer.start_as_current_span("store-results"):
            time.sleep(random.uniform(0.05, 0.15))

if __name__ == "__main__":
    print("🚀 Sending live OTEL traces to Urpo on localhost:4317")
    print("Make sure Urpo is running: cargo run")
    print("Press Ctrl+C to stop\n")
    
    try:
        while True:
            # Mix different types of operations
            if random.random() < 0.7:
                simulate_api_call()
                print(".", end="", flush=True)
            else:
                simulate_background_job()
                print("J", end="", flush=True)
                
            time.sleep(random.uniform(0.5, 2))  # Wait between operations
            
    except KeyboardInterrupt:
        print("\n\nShutting down...")
        # Ensure all spans are exported
        provider.shutdown()
        print("✅ Done!")