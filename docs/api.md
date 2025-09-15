# Urpo API Reference

Urpo provides a lightweight HTTP API for integration with external tools, dashboards, and alert systems.

## Base URL

```
http://localhost:8080
```

## Authentication

Currently, no authentication is required. This may change in future versions.

## Endpoints

### Health Check

Get system health and basic statistics.

```http
GET /health
```

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "trace_count": 50000,
  "service_count": 12
}
```

### Query Traces (TraceQL)

Execute TraceQL queries to find matching traces.

```http
GET /api/query?q=<query>&limit=<number>
```

**Parameters:**
- `q` (required): TraceQL query string (URL-encoded)
- `limit` (optional): Maximum results to return (default: 100, max: 10000)

**Examples:**

```bash
# Find API errors
curl "http://localhost:8080/api/query?q=service%3D%22api%22%20%26%26%20status%3Derror"

# Find slow requests
curl "http://localhost:8080/api/query?q=duration%20%3E%20100ms&limit=50"

# Complex query
curl "http://localhost:8080/api/query?q=service%3D%22frontend%22%20%26%26%20(status%3Derror%20%7C%7C%20duration%3E500ms)"
```

**Response:**
```json
{
  "trace_ids": [
    "1234567890abcdef1234567890abcdef",
    "fedcba0987654321fedcba0987654321"
  ],
  "total_matches": 42,
  "query_time_ms": 5,
  "limited": false
}
```

**Errors:**
- `400 Bad Request`: Invalid query syntax
- `500 Internal Server Error`: Query execution failed

### List Traces

List recent traces with basic filtering.

```http
GET /api/traces?service=<name>&start_time=<unix>&end_time=<unix>&limit=<number>&errors_only=<bool>&format=<format>
```

**Parameters:**
- `service` (optional): Filter by service name
- `start_time` (optional): Start time as Unix timestamp in seconds
- `end_time` (optional): End time as Unix timestamp in seconds
- `limit` (optional): Maximum results (default: 100, max: 1000)
- `errors_only` (optional): Only return traces with errors (default: false)
- `format` (optional): Export format - `json`, `jaeger`, `otel`, `csv`

**Examples:**

```bash
# Recent traces
curl "http://localhost:8080/api/traces"

# API traces from last hour
curl "http://localhost:8080/api/traces?service=api&start_time=1703980800"

# Only error traces
curl "http://localhost:8080/api/traces?errors_only=true"

# Export as Jaeger format
curl "http://localhost:8080/api/traces?format=jaeger"
```

**Response (JSON format):**
```json
[
  {
    "trace_id": "1234567890abcdef1234567890abcdef",
    "start_time": "2024-01-15T10:30:00Z",
    "end_time": "2024-01-15T10:30:01.234Z",
    "duration_ms": 1234,
    "span_count": 15,
    "service_count": 3,
    "root_service": "frontend",
    "root_operation": "GET /checkout",
    "has_error": false
  }
]
```

### Get Single Trace

Get detailed information about a specific trace.

```http
GET /api/traces/{trace_id}
```

**Parameters:**
- `trace_id`: 32-character hex trace ID

**Example:**
```bash
curl "http://localhost:8080/api/traces/1234567890abcdef1234567890abcdef"
```

**Response:**
```json
{
  "trace_id": "1234567890abcdef1234567890abcdef",
  "spans": [
    {
      "span_id": "fedcba0987654321",
      "parent_span_id": null,
      "service_name": "frontend",
      "operation_name": "GET /checkout",
      "start_time": "2024-01-15T10:30:00Z",
      "end_time": "2024-01-15T10:30:01.234Z",
      "duration_ms": 1234,
      "status": "ok",
      "tags": {
        "http.method": "GET",
        "http.status_code": "200",
        "http.url": "/checkout?user=123"
      }
    }
  ],
  "total_spans": 15,
  "duration_ms": 1234,
  "services": ["frontend", "api", "database"]
}
```

**Errors:**
- `404 Not Found`: Trace ID not found

### List Services

Get list of services with basic metrics.

```http
GET /api/services
```

**Response:**
```json
[
  {
    "name": "frontend",
    "trace_count": 1234,
    "span_count": 5678,
    "error_rate": 0.02,
    "avg_duration_ms": 145.6,
    "p95_duration_ms": 450.2,
    "last_seen": "2024-01-15T10:30:00Z"
  },
  {
    "name": "api",
    "trace_count": 2345,
    "span_count": 12345,
    "error_rate": 0.01,
    "avg_duration_ms": 89.3,
    "p95_duration_ms": 234.1,
    "last_seen": "2024-01-15T10:30:00Z"
  }
]
```

### Get Service Map

Get service dependency graph.

```http
GET /api/service-map
```

**Response:**
```json
{
  "services": [
    {
      "name": "frontend",
      "metrics": {
        "request_rate": 150.5,
        "error_rate": 0.02,
        "avg_latency_ms": 145.6
      }
    }
  ],
  "connections": [
    {
      "source": "frontend",
      "target": "api",
      "metrics": {
        "request_rate": 120.3,
        "error_rate": 0.01,
        "avg_latency_ms": 89.3
      }
    }
  ]
}
```

### Search (Legacy)

Simple text-based search for spans.

```http
GET /api/search?q=<text>&service=<name>&attribute_key=<key>&limit=<number>
```

**Parameters:**
- `q` (required): Search text
- `service` (optional): Filter by service
- `attribute_key` (optional): Search within specific attribute
- `limit` (optional): Maximum results (default: 100, max: 1000)

**Example:**
```bash
curl "http://localhost:8080/api/search?q=timeout&service=api"
```

**Response:**
```json
{
  "query": "timeout",
  "count": 5,
  "spans": [
    {
      "trace_id": "1234567890abcdef1234567890abcdef",
      "span_id": "fedcba0987654321",
      "service_name": "api",
      "operation_name": "GET /users/timeout",
      "start_time": "2024-01-15T10:30:00Z",
      "duration_ms": 5000,
      "tags": {
        "error.message": "Request timeout after 5s"
      }
    }
  ]
}
```

## Error Responses

All endpoints return consistent error responses:

```json
{
  "error": "Detailed error message",
  "code": 400
}
```

**Common HTTP Status Codes:**
- `200 OK`: Request successful
- `400 Bad Request`: Invalid parameters or query syntax
- `404 Not Found`: Resource not found
- `500 Internal Server Error`: Server error
- `503 Service Unavailable`: Storage backend unavailable

## Rate Limiting

Currently no rate limiting is implemented. This may be added in future versions.

## CORS

CORS is enabled by default for all origins. This can be disabled in the configuration.

## Configuration

API server configuration options:

```yaml
api:
  port: 8080              # Port to listen on
  enable_cors: true       # Enable CORS headers
  max_results: 1000       # Maximum results per query
```

## Client Libraries

### cURL Examples

```bash
# Basic health check
curl http://localhost:8080/health

# Query for errors
curl -G http://localhost:8080/api/query \
  --data-urlencode "q=status = error" \
  --data-urlencode "limit=50"

# Get trace details
curl http://localhost:8080/api/traces/1234567890abcdef1234567890abcdef

# Export traces as CSV
curl "http://localhost:8080/api/traces?format=csv" > traces.csv
```

### Python Example

```python
import requests
import urllib.parse

base_url = "http://localhost:8080"

# Query for slow API calls
query = "service = 'api' && duration > 100ms"
encoded_query = urllib.parse.quote(query)

response = requests.get(f"{base_url}/api/query?q={encoded_query}")
if response.status_code == 200:
    result = response.json()
    print(f"Found {len(result['trace_ids'])} traces")
    for trace_id in result['trace_ids']:
        print(f"Trace: {trace_id}")
else:
    print(f"Error: {response.text}")
```

### JavaScript Example

```javascript
const baseUrl = "http://localhost:8080";

async function queryTraces(query, limit = 100) {
  const encodedQuery = encodeURIComponent(query);
  const url = `${baseUrl}/api/query?q=${encodedQuery}&limit=${limit}`;

  try {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${await response.text()}`);
    }

    const result = await response.json();
    console.log(`Found ${result.trace_ids.length} traces in ${result.query_time_ms}ms`);
    return result;
  } catch (error) {
    console.error("Query failed:", error);
    throw error;
  }
}

// Usage
queryTraces("service = 'frontend' && status = error")
  .then(result => console.log(result))
  .catch(error => console.error(error));
```

## Integration Examples

### Grafana Dashboard

Create a JSON API data source pointing to `http://localhost:8080/api/query` for custom Grafana panels.

### Prometheus AlertManager

Use the API in AlertManager webhook receivers to query related traces when alerts fire.

### CI/CD Integration

Query for errors in deployment verification:

```bash
#!/bin/bash
# Check for errors in the last 5 minutes after deployment
errors=$(curl -s "http://localhost:8080/api/query?q=status%3Derror&limit=1" | jq '.total_matches')
if [ "$errors" -gt 0 ]; then
  echo "Deployment verification failed: $errors errors found"
  exit 1
fi
```

---

*For more details on query syntax, see [Query Language Documentation](query-language.md)*