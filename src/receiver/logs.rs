//! OTLP Logs receiver implementation.
//!
//! This module implements gRPC receiver for OpenTelemetry logs
//! following the OTLP specification.

use crate::core::{otel_compliance, Result, SpanId, TraceId};
use crate::logs::{
    storage::LogStorage,
    types::{LogRecord, LogSeverity},
};
use crate::metrics::string_pool::StringPool;
use opentelemetry_proto::tonic::collector::logs::v1::{
    logs_service_server::{LogsService, LogsServiceServer},
    ExportLogsServiceRequest, ExportLogsServiceResponse,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

/// OTLP Logs receiver service
pub struct OtelLogsReceiver {
    /// Logs storage engine
    log_storage: Arc<Mutex<LogStorage>>,
    /// String interning pool for service names
    string_pool: Arc<StringPool>,
}

impl OtelLogsReceiver {
    /// Create new logs receiver
    pub fn new(log_storage: Arc<Mutex<LogStorage>>) -> Self {
        Self {
            log_storage,
            string_pool: Arc::new(StringPool::new()),
        }
    }

    /// Convert OTLP log record to internal format
    fn convert_otlp_log(
        &self,
        log: &opentelemetry_proto::tonic::logs::v1::LogRecord,
        service_id: u16,
    ) -> Result<LogRecord> {
        // Convert timestamp
        let timestamp = if log.time_unix_nano > 0 {
            log.time_unix_nano
        } else {
            otel_compliance::system_time_to_nanos(std::time::SystemTime::now())
        };

        // Convert severity
        let severity = LogSeverity::from_otlp(log.severity_number);

        // Extract body
        let body = if let Some(body_value) = &log.body {
            extract_string_value(body_value)
        } else {
            String::new()
        };

        // Create log record
        let mut log_record = LogRecord::new(timestamp, service_id, severity, body);

        // Add trace context if present
        if !log.trace_id.is_empty() {
            let trace_id_hex = hex::encode(&log.trace_id);
            if let Ok(trace_id) = TraceId::new(trace_id_hex) {
                log_record = log_record.with_trace_id(trace_id);
            }
        }

        if !log.span_id.is_empty() {
            let span_id_hex = hex::encode(&log.span_id);
            if let Ok(span_id) = SpanId::new(span_id_hex) {
                log_record = log_record.with_span_id(span_id);
            }
        }

        // Add attributes
        for attribute in &log.attributes {
            if let Some(value) = &attribute.value {
                log_record =
                    log_record.with_attribute(attribute.key.clone(), extract_string_value(value));
            }
        }

        Ok(log_record)
    }

    /// Extract service ID from resource attributes
    fn extract_service_id(
        &self,
        resource: &opentelemetry_proto::tonic::resource::v1::Resource,
    ) -> u16 {
        // Look for service.name attribute
        for attribute in &resource.attributes {
            if attribute.key == otel_compliance::attributes::SERVICE_NAME {
                if let Some(value) = &attribute.value {
                    if let Some(string_val) = &value.value {
                        if let opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(name) = string_val {
                            return self.string_pool.intern(name).0;
                        }
                    }
                }
            }
        }

        // Default service ID if not found
        self.string_pool.intern("unknown_service").0
    }
}

/// Extract string value from AnyValue
fn extract_string_value(value: &opentelemetry_proto::tonic::common::v1::AnyValue) -> String {
    use opentelemetry_proto::tonic::common::v1::any_value::Value;

    match &value.value {
        Some(Value::StringValue(s)) => s.clone(),
        Some(Value::BoolValue(b)) => b.to_string(),
        Some(Value::IntValue(i)) => i.to_string(),
        Some(Value::DoubleValue(d)) => d.to_string(),
        Some(Value::BytesValue(bytes)) => format!("bytes({})", bytes.len()),
        Some(Value::ArrayValue(arr)) => {
            let values: Vec<String> = arr.values.iter().map(extract_string_value).collect();
            format!("[{}]", values.join(", "))
        },
        Some(Value::KvlistValue(kv)) => {
            let pairs: Vec<String> = kv
                .values
                .iter()
                .map(|kv| {
                    let val = kv
                        .value
                        .as_ref()
                        .map(extract_string_value)
                        .unwrap_or_default();
                    format!("{}={}", kv.key, val)
                })
                .collect();
            format!("{{{}}}", pairs.join(", "))
        },
        None => String::new(),
    }
}

#[tonic::async_trait]
impl LogsService for OtelLogsReceiver {
    async fn export(
        &self,
        request: Request<ExportLogsServiceRequest>,
    ) -> std::result::Result<Response<ExportLogsServiceResponse>, Status> {
        let request = request.into_inner();
        let mut total_logs = 0;
        let mut processed_logs = 0;

        for resource_logs in request.resource_logs {
            let service_id = if let Some(resource) = &resource_logs.resource {
                self.extract_service_id(resource)
            } else {
                self.string_pool.intern("unknown_service").0
            };

            for scope_logs in resource_logs.scope_logs {
                for log_record in scope_logs.log_records {
                    total_logs += 1;

                    match self.convert_otlp_log(&log_record, service_id) {
                        Ok(converted_log) => {
                            let storage = self.log_storage.lock().await;
                            if storage.store_log(converted_log).is_ok() {
                                processed_logs += 1;
                            }
                        },
                        Err(e) => {
                            tracing::warn!("Failed to convert log record: {}", e);
                        },
                    }
                }
            }
        }

        tracing::debug!("Processed {} out of {} log records", processed_logs, total_logs);

        Ok(Response::new(ExportLogsServiceResponse {
            partial_success: None,
        }))
    }
}

/// Create LogsServiceServer for gRPC
pub fn create_logs_service_server(
    log_storage: Arc<Mutex<LogStorage>>,
) -> LogsServiceServer<OtelLogsReceiver> {
    let receiver = OtelLogsReceiver::new(log_storage);
    LogsServiceServer::new(receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::storage::LogStorageConfig;
    use opentelemetry_proto::tonic::{
        common::v1::{any_value::Value, AnyValue, KeyValue},
        logs::v1::{LogRecord as OtelLogRecord, ResourceLogs, ScopeLogs, SeverityNumber},
        resource::v1::Resource,
    };

    fn create_test_log_storage() -> Arc<Mutex<LogStorage>> {
        Arc::new(Mutex::new(LogStorage::new(LogStorageConfig::default())))
    }

    #[test]
    fn test_logs_receiver_creation() {
        let storage = create_test_log_storage();
        let receiver = OtelLogsReceiver::new(storage);

        assert_eq!(receiver.string_pool.len(), 0);
    }

    #[test]
    fn test_extract_string_value() {
        let value = AnyValue {
            value: Some(Value::StringValue("test".to_string())),
        };
        assert_eq!(extract_string_value(&value), "test");

        let value = AnyValue {
            value: Some(Value::IntValue(42)),
        };
        assert_eq!(extract_string_value(&value), "42");

        let value = AnyValue {
            value: Some(Value::BoolValue(true)),
        };
        assert_eq!(extract_string_value(&value), "true");

        let value = AnyValue { value: None };
        assert_eq!(extract_string_value(&value), "");
    }

    #[test]
    fn test_extract_service_id() {
        let storage = create_test_log_storage();
        let receiver = OtelLogsReceiver::new(storage);

        let resource = Resource {
            attributes: vec![KeyValue {
                key: "service.name".to_string(),
                value: Some(AnyValue {
                    value: Some(Value::StringValue("test-service".to_string())),
                }),
            }],
            dropped_attributes_count: 0,
        };

        let service_id = receiver.extract_service_id(&resource);
        assert_eq!(service_id, receiver.string_pool.intern("test-service").0);
    }

    #[test]
    fn test_extract_service_id_unknown() {
        let storage = create_test_log_storage();
        let receiver = OtelLogsReceiver::new(storage);

        let resource = Resource {
            attributes: vec![],
            dropped_attributes_count: 0,
        };

        let service_id = receiver.extract_service_id(&resource);
        assert_eq!(service_id, receiver.string_pool.intern("unknown_service").0);
    }

    #[test]
    fn test_convert_otlp_log() {
        let storage = create_test_log_storage();
        let receiver = OtelLogsReceiver::new(storage);

        let otlp_log = OtelLogRecord {
            time_unix_nano: 1234567890000000000,
            severity_number: SeverityNumber::Error as i32,
            severity_text: "ERROR".to_string(),
            body: Some(AnyValue {
                value: Some(Value::StringValue("Error occurred".to_string())),
            }),
            attributes: vec![KeyValue {
                key: "http.method".to_string(),
                value: Some(AnyValue {
                    value: Some(Value::StringValue("GET".to_string())),
                }),
            }],
            trace_id: hex::decode("4bf92f3577b34da6a3ce929d0e0e4736").unwrap(),
            span_id: hex::decode("00f067aa0ba902b7").unwrap(),
            ..Default::default()
        };

        let log_record = receiver.convert_otlp_log(&otlp_log, 1).unwrap();

        assert_eq!(log_record.timestamp, 1234567890000000000);
        assert_eq!(log_record.severity, LogSeverity::Error);
        assert_eq!(log_record.body, "Error occurred");
        assert!(log_record.trace_id.is_some());
        assert!(log_record.span_id.is_some());
        assert_eq!(log_record.attributes.get("http.method"), Some(&"GET".to_string()));
    }

    #[test]
    fn test_convert_otlp_log_minimal() {
        let storage = create_test_log_storage();
        let receiver = OtelLogsReceiver::new(storage);

        let otlp_log = OtelLogRecord {
            time_unix_nano: 0,  // Will use current time
            severity_number: 0, // Will default to INFO
            body: Some(AnyValue {
                value: Some(Value::StringValue("Simple log".to_string())),
            }),
            ..Default::default()
        };

        let log_record = receiver.convert_otlp_log(&otlp_log, 1).unwrap();

        assert!(log_record.timestamp > 0);
        assert_eq!(log_record.severity, LogSeverity::Info);
        assert_eq!(log_record.body, "Simple log");
        assert!(log_record.trace_id.is_none());
        assert!(log_record.span_id.is_none());
    }

    #[tokio::test]
    async fn test_export_request_processing() {
        let storage = create_test_log_storage();
        let receiver = OtelLogsReceiver::new(storage.clone());

        let request = ExportLogsServiceRequest {
            resource_logs: vec![ResourceLogs {
                resource: Some(Resource {
                    attributes: vec![KeyValue {
                        key: "service.name".to_string(),
                        value: Some(AnyValue {
                            value: Some(Value::StringValue("test-service".to_string())),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_logs: vec![ScopeLogs {
                    scope: None,
                    log_records: vec![
                        OtelLogRecord {
                            time_unix_nano: 1234567890000000000,
                            severity_number: SeverityNumber::Info as i32,
                            body: Some(AnyValue {
                                value: Some(Value::StringValue("Test log 1".to_string())),
                            }),
                            ..Default::default()
                        },
                        OtelLogRecord {
                            time_unix_nano: 1234567891000000000,
                            severity_number: SeverityNumber::Warn as i32,
                            body: Some(AnyValue {
                                value: Some(Value::StringValue("Test log 2".to_string())),
                            }),
                            ..Default::default()
                        },
                    ],
                    schema_url: "".to_string(),
                }],
                schema_url: "".to_string(),
            }],
        };

        let result = receiver.export(Request::new(request)).await;
        assert!(result.is_ok());

        // Verify logs were stored
        let storage_guard = storage.lock().await;
        let recent_logs = storage_guard.get_recent_logs(10);
        assert_eq!(recent_logs.len(), 2);
        assert_eq!(recent_logs[0].body, "Test log 2"); // Most recent first
        assert_eq!(recent_logs[1].body, "Test log 1");
    }

    #[test]
    fn test_create_logs_service_server() {
        let storage = create_test_log_storage();
        let _server = create_logs_service_server(storage);

        // Just verify it creates without panic
        assert!(true);
    }
}
