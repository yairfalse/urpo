//! Export functionality for traces.
//!
//! Supports multiple export formats including JSON, CSV, and compatibility
//! formats for other tracing systems.

use crate::core::{Result, Span, TraceId, UrpoError};
use crate::storage::{StorageBackend, TraceInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Export format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Native Urpo JSON format
    Json,
    /// Jaeger-compatible JSON format
    Jaeger,
    /// OpenTelemetry JSON format
    OpenTelemetry,
    /// CSV format for spreadsheet analysis
    Csv,
}

impl std::str::FromStr for ExportFormat {
    type Err = String;
    
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "jaeger" => Ok(ExportFormat::Jaeger),
            "otel" | "opentelemetry" => Ok(ExportFormat::OpenTelemetry),
            "csv" => Ok(ExportFormat::Csv),
            _ => Err(format!("Unknown export format: {}", s)),
        }
    }
}

/// Export options for trace export.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Export format
    pub format: ExportFormat,
    /// Output file path (None for stdout)
    pub output: Option<PathBuf>,
    /// Service filter
    pub service: Option<String>,
    /// Time range start (unix timestamp)
    pub start_time: Option<u64>,
    /// Time range end (unix timestamp)
    pub end_time: Option<u64>,
    /// Maximum number of traces to export
    pub limit: Option<usize>,
    /// Only export traces with errors
    pub errors_only: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Json,
            output: None,
            service: None,
            start_time: None,
            end_time: None,
            limit: None,
            errors_only: false,
        }
    }
}

/// Trace exporter.
pub struct TraceExporter<'a> {
    storage: &'a dyn StorageBackend,
}

impl<'a> TraceExporter<'a> {
    /// Create a new trace exporter.
    pub fn new(storage: &'a dyn StorageBackend) -> Self {
        Self { storage }
    }
    
    /// Export a single trace by ID.
    pub async fn export_trace(
        &self,
        trace_id: &TraceId,
        format: ExportFormat,
    ) -> Result<String> {
        // Get spans for the trace
        let spans = self.storage.get_trace_spans(trace_id.clone()).await?;
        
        if spans.is_empty() {
            return Err(UrpoError::NotFound(format!("Trace {} not found", trace_id.as_str())));
        }
        
        match format {
            ExportFormat::Json => self.export_json(&spans),
            ExportFormat::Jaeger => self.export_jaeger(&spans),
            ExportFormat::OpenTelemetry => self.export_otel(&spans),
            ExportFormat::Csv => self.export_csv(&spans),
        }
    }
    
    /// Export a single trace with provided spans.
    pub async fn export_single_trace(
        &self,
        trace_id: &TraceId,
        spans: &[Span],
        options: &ExportOptions,
    ) -> Result<String> {
        if spans.is_empty() {
            return Err(UrpoError::NotFound(format!("Trace {} not found", trace_id.as_str())));
        }
        
        match options.format {
            ExportFormat::Json => self.export_json(spans),
            ExportFormat::Jaeger => self.export_jaeger(spans),
            ExportFormat::OpenTelemetry => self.export_otel(spans),
            ExportFormat::Csv => self.export_csv(spans),
        }
    }
    
    /// Export multiple traces based on options.
    pub async fn export_traces(&self, options: &ExportOptions) -> Result<String> {
        // Query traces based on filters
        let traces = self.storage.list_traces(
            options.service.as_deref(),
            options.start_time,
            options.end_time,
            options.limit.unwrap_or(1000),
        ).await?;
        
        // Filter by error status if requested
        let filtered_traces: Vec<TraceInfo> = if options.errors_only {
            traces.into_iter().filter(|t| t.has_error).collect()
        } else {
            traces
        };
        
        if filtered_traces.is_empty() {
            return Ok("[]".to_string());
        }
        
        // Export based on format
        match options.format {
            ExportFormat::Json => self.export_traces_json(&filtered_traces).await,
            ExportFormat::Jaeger => self.export_traces_jaeger(&filtered_traces).await,
            ExportFormat::OpenTelemetry => self.export_traces_otel(&filtered_traces).await,
            ExportFormat::Csv => self.export_traces_csv(&filtered_traces).await,
        }
    }
    
    /// Export spans as native JSON.
    fn export_json(&self, spans: &[Span]) -> Result<String> {
        serde_json::to_string_pretty(spans)
            .map_err(|e| UrpoError::Serialization(e.to_string()))
    }
    
    /// Export spans as Jaeger-compatible JSON.
    fn export_jaeger(&self, spans: &[Span]) -> Result<String> {
        let jaeger_trace = convert_to_jaeger_format(spans);
        serde_json::to_string_pretty(&jaeger_trace)
            .map_err(|e| UrpoError::Serialization(e.to_string()))
    }
    
    /// Export spans as OpenTelemetry JSON.
    fn export_otel(&self, spans: &[Span]) -> Result<String> {
        let otel_trace = convert_to_otel_format(spans);
        serde_json::to_string_pretty(&otel_trace)
            .map_err(|e| UrpoError::Serialization(e.to_string()))
    }
    
    /// Export spans as CSV.
    fn export_csv(&self, spans: &[Span]) -> Result<String> {
        let mut csv_output = String::new();
        
        // Header
        csv_output.push_str("trace_id,span_id,parent_span_id,service,operation,start_time,duration_us,status,attributes\n");
        
        // Data rows
        for span in spans {
            csv_output.push_str(&format!(
                "{},{},{},{},{},{},{},{},\"{}\"\n",
                span.trace_id.as_str(),
                span.span_id.as_str(),
                span.parent_span_id.as_ref().map(|p| p.as_str()).unwrap_or(""),
                span.service_name.as_str(),
                span.operation_name,
                span.start_time,
                span.duration,
                if span.status.is_error() { "ERROR" } else { "OK" },
                serde_json::to_string(&span.attributes).unwrap_or_default(),
            ));
        }
        
        Ok(csv_output)
    }
    
    /// Export multiple traces as JSON.
    async fn export_traces_json(&self, traces: &[TraceInfo]) -> Result<String> {
        let mut all_traces = Vec::new();
        
        for trace_info in traces {
            let spans = self.storage.get_trace_spans(trace_info.trace_id.clone()).await?;
            all_traces.push(serde_json::json!({
                "trace_id": trace_info.trace_id.as_str(),
                "root_service": trace_info.root_service,
                "root_operation": trace_info.root_operation,
                "start_time": trace_info.start_time,
                "duration": trace_info.duration,
                "span_count": trace_info.span_count,
                "has_error": trace_info.has_error,
                "spans": spans,
            }));
        }
        
        serde_json::to_string_pretty(&all_traces)
            .map_err(|e| UrpoError::Serialization(e.to_string()))
    }
    
    /// Export multiple traces as Jaeger format.
    async fn export_traces_jaeger(&self, traces: &[TraceInfo]) -> Result<String> {
        let mut jaeger_traces = Vec::new();
        
        for trace_info in traces {
            let spans = self.storage.get_trace_spans(trace_info.trace_id.clone()).await?;
            jaeger_traces.push(convert_to_jaeger_format(&spans));
        }
        
        serde_json::to_string_pretty(&jaeger_traces)
            .map_err(|e| UrpoError::Serialization(e.to_string()))
    }
    
    /// Export multiple traces as OpenTelemetry format.
    async fn export_traces_otel(&self, traces: &[TraceInfo]) -> Result<String> {
        let mut otel_traces = Vec::new();
        
        for trace_info in traces {
            let spans = self.storage.get_trace_spans(trace_info.trace_id.clone()).await?;
            otel_traces.push(convert_to_otel_format(&spans));
        }
        
        serde_json::to_string_pretty(&otel_traces)
            .map_err(|e| UrpoError::Serialization(e.to_string()))
    }
    
    /// Export multiple traces as CSV.
    async fn export_traces_csv(&self, traces: &[TraceInfo]) -> Result<String> {
        let mut csv_output = String::new();
        
        // Header
        csv_output.push_str("trace_id,span_id,parent_span_id,service,operation,start_time,duration_us,status,attributes\n");
        
        for trace_info in traces {
            let spans = self.storage.get_trace_spans(trace_info.trace_id.clone()).await?;
            for span in spans {
                csv_output.push_str(&format!(
                    "{},{},{},{},{},{},{},{},\"{}\"\n",
                    span.trace_id.as_str(),
                    span.span_id.as_str(),
                    span.parent_span_id.as_ref().map(|p| p.as_str()).unwrap_or(""),
                    span.service_name.as_str(),
                    span.operation_name,
                    span.start_time,
                    span.duration,
                    if span.status.is_error() { "ERROR" } else { "OK" },
                    serde_json::to_string(&span.attributes).unwrap_or_default(),
                ));
            }
        }
        
        Ok(csv_output)
    }
    
    /// Write export to file or stdout.
    pub fn write_output(&self, content: &str, output: Option<&str>) -> Result<()> {
        match output {
            Some(path) => {
                let mut file = std::fs::File::create(path)
                    .map_err(|e| UrpoError::Storage(format!("Failed to create file: {}", e)))?;
                file.write_all(content.as_bytes())
                    .map_err(|e| UrpoError::Storage(format!("Failed to write file: {}", e)))?;
                Ok(())
            }
            None => {
                print!("{}", content);
                Ok(())
            }
        }
    }
}

/// Jaeger trace format.
#[derive(Debug, Serialize, Deserialize)]
struct JaegerTrace {
    #[serde(rename = "traceID")]
    trace_id: String,
    spans: Vec<JaegerSpan>,
    processes: HashMap<String, JaegerProcess>,
}

/// Jaeger span format.
#[derive(Debug, Serialize, Deserialize)]
struct JaegerSpan {
    #[serde(rename = "traceID")]
    trace_id: String,
    #[serde(rename = "spanID")]
    span_id: String,
    #[serde(rename = "parentSpanID", skip_serializing_if = "Option::is_none")]
    parent_span_id: Option<String>,
    #[serde(rename = "operationName")]
    operation_name: String,
    references: Vec<JaegerReference>,
    #[serde(rename = "startTime")]
    start_time: u64,
    duration: u64,
    tags: Vec<JaegerTag>,
    logs: Vec<JaegerLog>,
    #[serde(rename = "processID")]
    process_id: String,
}

/// Jaeger process info.
#[derive(Debug, Serialize, Deserialize)]
struct JaegerProcess {
    #[serde(rename = "serviceName")]
    service_name: String,
    tags: Vec<JaegerTag>,
}

/// Jaeger reference.
#[derive(Debug, Serialize, Deserialize)]
struct JaegerReference {
    #[serde(rename = "refType")]
    ref_type: String,
    #[serde(rename = "traceID")]
    trace_id: String,
    #[serde(rename = "spanID")]
    span_id: String,
}

/// Jaeger tag.
#[derive(Debug, Serialize, Deserialize)]
struct JaegerTag {
    key: String,
    #[serde(rename = "type")]
    tag_type: String,
    value: serde_json::Value,
}

/// Jaeger log entry.
#[derive(Debug, Serialize, Deserialize)]
struct JaegerLog {
    timestamp: u64,
    fields: Vec<JaegerTag>,
}

/// Convert Urpo spans to Jaeger format.
fn convert_to_jaeger_format(spans: &[Span]) -> JaegerTrace {
    let mut processes = HashMap::new();
    let mut jaeger_spans = Vec::new();
    
    for span in spans {
        let process_id = span.service_name.as_str();
        
        // Add process if not exists
        processes.entry(process_id.to_string()).or_insert(JaegerProcess {
            service_name: span.service_name.as_str().to_string(),
            tags: vec![],
        });
        
        // Convert span
        let mut tags = vec![];
        for (key, value) in &span.attributes {
            tags.push(JaegerTag {
                key: key.clone(),
                tag_type: "string".to_string(),
                value: serde_json::Value::String(value.clone()),
            });
        }
        
        // Add status tag
        if span.status.is_error() {
            tags.push(JaegerTag {
                key: "error".to_string(),
                tag_type: "bool".to_string(),
                value: serde_json::Value::Bool(true),
            });
        }
        
        let references = if let Some(parent_id) = &span.parent_span_id {
            vec![JaegerReference {
                ref_type: "CHILD_OF".to_string(),
                trace_id: span.trace_id.as_str().to_string(),
                span_id: parent_id.as_str().to_string(),
            }]
        } else {
            vec![]
        };
        
        jaeger_spans.push(JaegerSpan {
            trace_id: span.trace_id.as_str().to_string(),
            span_id: span.span_id.as_str().to_string(),
            parent_span_id: span.parent_span_id.as_ref().map(|p| p.as_str().to_string()),
            operation_name: span.operation_name.clone(),
            references,
            start_time: span.start_time / 1000, // Convert to microseconds
            duration: span.duration,
            tags,
            logs: vec![],
            process_id: process_id.to_string(),
        });
    }
    
    JaegerTrace {
        trace_id: spans.first().map(|s| s.trace_id.as_str().to_string()).unwrap_or_default(),
        spans: jaeger_spans,
        processes,
    }
}

/// Convert Urpo spans to OpenTelemetry format.
fn convert_to_otel_format(spans: &[Span]) -> serde_json::Value {
    // Group spans by service
    let mut services_map: HashMap<String, Vec<&Span>> = HashMap::new();
    for span in spans {
        services_map.entry(span.service_name.as_str().to_string())
            .or_insert_with(Vec::new)
            .push(span);
    }
    
    let mut resource_spans = Vec::new();
    
    for (service_name, service_spans) in services_map {
        let mut otel_spans = Vec::new();
        
        for span in service_spans {
            let mut attributes = Vec::new();
            for (key, value) in &span.attributes {
                attributes.push(serde_json::json!({
                    "key": key,
                    "value": {
                        "stringValue": value
                    }
                }));
            }
            
            otel_spans.push(serde_json::json!({
                "traceId": span.trace_id.as_str(),
                "spanId": span.span_id.as_str(),
                "parentSpanId": span.parent_span_id.as_ref().map(|p| p.as_str()),
                "name": span.operation_name,
                "kind": 1, // SPAN_KIND_SERVER
                "startTimeUnixNano": span.start_time.to_string(),
                "endTimeUnixNano": (span.start_time + span.duration * 1000).to_string(),
                "attributes": attributes,
                "status": {
                    "code": if span.status.is_error() { 2 } else { 1 }
                }
            }));
        }
        
        resource_spans.push(serde_json::json!({
            "resource": {
                "attributes": [
                    {
                        "key": "service.name",
                        "value": {
                            "stringValue": service_name
                        }
                    }
                ]
            },
            "scopeSpans": [
                {
                    "scope": {
                        "name": "urpo"
                    },
                    "spans": otel_spans
                }
            ]
        }));
    }
    
    serde_json::json!({
        "resourceSpans": resource_spans
    })
}