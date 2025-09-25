//! Common test utilities and fixtures.

use std::time::{Duration, SystemTime};
use urpo_lib::core::{ServiceName, Span, SpanId, SpanStatus, TraceId};
use urpo_lib::storage::{InMemoryStorage, StorageBackend};

/// Test fixture builder for creating spans with sensible defaults.
pub struct TestSpanBuilder {
    trace_num: u32,
    span_num: u32,
    service: String,
    has_error: bool,
    is_root: bool,
    duration_ms: u64,
}

impl TestSpanBuilder {
    pub fn new(trace_num: u32, span_num: u32) -> Self {
        Self {
            trace_num,
            span_num,
            service: "test-service".to_string(),
            has_error: false,
            is_root: false,
            duration_ms: 100,
        }
    }

    pub fn service(mut self, service: &str) -> Self {
        self.service = service.to_string();
        self
    }

    pub fn with_error(mut self) -> Self {
        self.has_error = true;
        self
    }

    pub fn as_root(mut self) -> Self {
        self.is_root = true;
        self
    }

    pub fn duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    pub fn build(self) -> Span {
        let trace_id = TraceId::new(format!("trace_{:04}", self.trace_num)).unwrap();
        let span_id = if self.is_root {
            SpanId::new(format!("span_root_{:04}", self.trace_num)).unwrap()
        } else {
            SpanId::new(format!("span_{:04}_{:02}", self.trace_num, self.span_num)).unwrap()
        };

        let mut builder = Span::builder()
            .trace_id(trace_id)
            .span_id(span_id)
            .service_name(ServiceName::new(self.service).unwrap())
            .operation_name(format!("operation-{}", self.span_num))
            .start_time(SystemTime::now() - Duration::from_secs(self.trace_num as u64 * 10))
            .duration(Duration::from_millis(self.duration_ms))
            .status(if self.has_error {
                SpanStatus::Error("Test error".to_string())
            } else {
                SpanStatus::Ok
            });

        if !self.is_root {
            builder = builder
                .parent_span_id(SpanId::new(format!("span_root_{:04}", self.trace_num)).unwrap());
        }

        builder.build().unwrap()
    }
}

/// Create a test trace with root and child spans.
pub async fn create_test_trace(
    storage: &InMemoryStorage,
    trace_num: u32,
    num_children: u32,
    has_error: bool,
) {
    // Root span
    let root = TestSpanBuilder::new(trace_num, 0)
        .as_root()
        .service("api-gateway")
        .build();
    storage.store_span(root).await.unwrap();

    // Child spans
    for i in 0..num_children {
        let mut builder = TestSpanBuilder::new(trace_num, i + 1).service(&format!("service-{}", i));

        if has_error && i == 1 {
            builder = builder.with_error();
        }

        storage.store_span(builder.build()).await.unwrap();
    }
}

/// Create multiple test traces.
pub async fn create_test_traces(storage: &InMemoryStorage, num_traces: u32, spans_per_trace: u32) {
    for i in 0..num_traces {
        let has_error = i % 2 == 0;
        create_test_trace(storage, i, spans_per_trace, has_error).await;
    }
}

/// Macro for creating a test span quickly.
#[macro_export]
macro_rules! test_span {
    ($trace_num:expr, $span_num:expr) => {
        TestSpanBuilder::new($trace_num, $span_num).build()
    };
    ($trace_num:expr, $span_num:expr, error) => {
        TestSpanBuilder::new($trace_num, $span_num)
            .with_error()
            .build()
    };
    ($trace_num:expr, $span_num:expr, root) => {
        TestSpanBuilder::new($trace_num, $span_num)
            .as_root()
            .build()
    };
    ($trace_num:expr, $span_num:expr, $service:expr) => {
        TestSpanBuilder::new($trace_num, $span_num)
            .service($service)
            .build()
    };
}

/// Macro for common test assertions.
#[macro_export]
macro_rules! assert_trace_properties {
    ($trace:expr, spans: $span_count:expr) => {
        assert_eq!($trace.span_count, $span_count, "Incorrect span count");
    };
    ($trace:expr, spans: $span_count:expr, service: $service:expr) => {
        assert_eq!($trace.span_count, $span_count, "Incorrect span count");
        assert_eq!($trace.root_service.as_str(), $service, "Incorrect root service");
    };
    ($trace:expr, spans: $span_count:expr, service: $service:expr, has_error: $has_error:expr) => {
        assert_eq!($trace.span_count, $span_count, "Incorrect span count");
        assert_eq!($trace.root_service.as_str(), $service, "Incorrect root service");
        assert_eq!($trace.has_error, $has_error, "Incorrect error status");
    };
}

/// Macro for testing storage queries.
#[macro_export]
macro_rules! test_storage_query {
    ($storage:expr, recent_traces($limit:expr)) => {
        $storage.list_recent_traces($limit, None).await.unwrap()
    };
    ($storage:expr, recent_traces($limit:expr, $service:expr)) => {
        $storage
            .list_recent_traces($limit, Some($service))
            .await
            .unwrap()
    };
    ($storage:expr, error_traces($limit:expr)) => {
        $storage.get_error_traces($limit).await.unwrap()
    };
    ($storage:expr, slow_traces($threshold:expr, $limit:expr)) => {
        $storage.get_slow_traces($threshold, $limit).await.unwrap()
    };
    ($storage:expr, search($query:expr, $limit:expr)) => {
        $storage.search_traces($query, $limit).await.unwrap()
    };
}

/// Verify traces are sorted by time (newest first).
pub fn assert_traces_sorted_by_time(traces: &[urpo_lib::storage::TraceInfo]) {
    for i in 1..traces.len() {
        assert!(
            traces[i - 1].start_time >= traces[i].start_time,
            "Traces not sorted by start time"
        );
    }
}

/// Verify traces are sorted by duration (slowest first).
pub fn assert_traces_sorted_by_duration(traces: &[urpo_lib::storage::TraceInfo]) {
    for i in 1..traces.len() {
        assert!(traces[i - 1].duration >= traces[i].duration, "Traces not sorted by duration");
    }
}
