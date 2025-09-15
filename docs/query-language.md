# Urpo Query Language (TraceQL)

Urpo implements a powerful query language inspired by Grafana Tempo's TraceQL, optimized for fast trace searching and filtering.

## Quick Start

```sql
-- Find all traces from the API service
service = "api"

-- Find slow requests
duration > 100ms

-- Find errors
status = error

-- Combine conditions
service = "api" && duration > 100ms && status = error
```

## Query Syntax

### Basic Structure

Queries consist of filter expressions that can be combined with logical operators:

```
<field> <operator> <value>
<filter> && <filter>  -- AND
<filter> || <filter>  -- OR
(<filter>)           -- Grouping
```

### Fields

| Field | Description | Example |
|-------|-------------|---------|
| `service` | Service name | `service = "api-gateway"` |
| `name` | Operation/span name | `name = "GET /users"` |
| `operation` | Alias for name | `operation =~ "GET.*"` |
| `duration` | Span duration | `duration > 100ms` |
| `status` | Span status | `status = error` |
| `trace_id` | Trace identifier | `trace_id = "abc123..."` |
| `span_id` | Span identifier | `span_id = "def456..."` |
| `parent_span_id` | Parent span ID | `parent_span_id = "xyz789..."` |
| `span.kind` | Span kind | `span.kind = "server"` |
| *attributes* | Any attribute | `http.status_code = 500` |

### Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `=` | Equals | `service = "api"` |
| `!=` | Not equals | `status != ok` |
| `>` | Greater than | `duration > 100ms` |
| `>=` | Greater than or equal | `http.status_code >= 400` |
| `<` | Less than | `duration < 1s` |
| `<=` | Less than or equal | `response.size <= 1000` |
| `=~` | Regex match | `name =~ "GET /users/.*"` |
| `contains` | Contains substring | `error.message contains "timeout"` |

### Values

#### Strings
```sql
service = "api"              -- Quoted string
service = api                -- Unquoted (simple identifiers)
name = "GET /users/{id}"     -- Quoted (with special chars)
```

#### Durations
```sql
duration > 100ns   -- Nanoseconds
duration > 100us   -- Microseconds (or μs)
duration > 100ms   -- Milliseconds
duration > 10s     -- Seconds
duration > 5m      -- Minutes
```

#### Status Values
```sql
status = ok
status = error
status = unknown
```

#### Numbers
```sql
http.status_code = 500
retry.count > 3
```

#### Booleans
```sql
cache.hit = true
error.retryable = false
```

### Logical Operators

#### AND (`&&`)
Both conditions must be true:
```sql
service = "api" && duration > 100ms
```

#### OR (`||`)
At least one condition must be true:
```sql
status = error || duration > 1s
```

#### Grouping with Parentheses
Control evaluation order:
```sql
service = "frontend" && (status = error || duration > 500ms)
```

## Common Query Patterns

### Find Slow Requests
```sql
-- All slow requests
duration > 500ms

-- Slow API requests
service = "api" && duration > 1s

-- Extremely slow requests from any service
duration > 5s
```

### Find Errors
```sql
-- All errors
status = error

-- API errors
service = "api" && status = error

-- HTTP 5xx errors
http.status_code >= 500

-- Database errors
db.system = "postgresql" && status = error
```

### Service-Specific Queries
```sql
-- All traces from frontend
service = "frontend"

-- Frontend requests to user endpoints
service = "frontend" && name =~ ".*user.*"

-- Inter-service communication
service = "api" && span.kind = "client"
```

### Complex Queries
```sql
-- Slow or failed API calls
service = "api" && (duration > 1s || status = error)

-- Critical user-facing errors
service = "frontend" && status = error && http.route =~ "/checkout.*"

-- Database queries taking too long
db.system = "mysql" && db.operation = "SELECT" && duration > 100ms
```

## Attribute Queries

Query any span attribute using dot notation:

### HTTP Attributes
```sql
http.method = "POST"
http.status_code >= 400
http.url contains "/api/v2"
http.route = "/users/{id}"
```

### Database Attributes
```sql
db.system = "redis"
db.operation = "SET"
db.statement contains "SELECT * FROM users"
```

### Custom Attributes
```sql
user.id = "12345"
tenant.name = "acme-corp"
feature.flag = "new-checkout"
correlation.id = "abc-123-def"
```

## API Usage

### REST API

```bash
# Basic query
curl "http://localhost:8080/api/query?q=service%3D%22api%22"

# With limit
curl "http://localhost:8080/api/query?q=status%3Derror&limit=100"

# URL-encoded complex query
curl "http://localhost:8080/api/query?q=service%3D%22api%22%20%26%26%20duration%20%3E%20100ms"
```

### Response Format

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

## Performance Tips

1. **Use specific service names** when possible:
   ```sql
   -- Fast: service is indexed
   service = "api" && duration > 100ms

   -- Slower: requires full scan
   duration > 100ms
   ```

2. **Put most selective conditions first**:
   ```sql
   -- Better: rare condition first
   http.status_code = 503 && service = "api"

   -- Worse: common condition first
   service = "api" && http.status_code = 503
   ```

3. **Use exact matches over regex** when possible:
   ```sql
   -- Fast: exact match
   name = "GET /users"

   -- Slower: regex evaluation
   name =~ "GET /users.*"
   ```

## Limitations

Current implementation limitations (to be improved):

- No aggregation functions yet (`count()`, `avg()`, etc.)
- No trace-level operations (all queries are span-level)
- Limited attribute indexing (only high-value attributes are indexed)
- No support for complex data types in attributes

## Examples by Use Case

### Debugging Production Issues

```sql
-- Find timeout errors
error.message contains "timeout" || error.type = "TimeoutError"

-- Find cascading failures
service = "api" && status = error && duration < 10ms

-- Find retried operations
retry.count > 0
```

### Performance Analysis

```sql
-- Find P99 latency outliers (if most requests are <100ms)
service = "api" && duration > 1s

-- Find slow database queries
db.system = "postgresql" && duration > 500ms

-- Find large responses
http.response.size > 1000000
```

### Security and Compliance

```sql
-- Find unauthorized access attempts
http.status_code = 401 || http.status_code = 403

-- Find admin operations
user.role = "admin" && http.method != "GET"

-- Find sensitive data access
db.statement contains "credit_card" || db.statement contains "ssn"
```

## Future Enhancements

Planned query language improvements:

- **Aggregations**: `{ service = "api" } | count() by endpoint`
- **Percentiles**: `{ service = "api" } | histogram(duration)`
- **Time ranges**: `service = "api" && timestamp > now() - 1h`
- **Trace context**: `{ .trace | any(.spans | status = error) }`
- **Saved queries**: Named, reusable query templates
- **Query optimization**: Query planner and execution optimization

## Integration with UI

The query language is integrated with Urpo's web UI:

1. **Query Bar**: Type queries directly in the search bar
2. **Autocomplete**: Get suggestions for fields and values
3. **Query Builder**: Visual query builder for complex queries
4. **Query History**: Recent queries are saved and searchable
5. **Saved Queries**: Save frequently used queries with names

## Troubleshooting

### Query Not Returning Results

1. Check service name spelling: `service = "api-gateway"` (exact match)
2. Verify time range: Queries search recent data by default
3. Check attribute names: Use exact attribute names from spans

### Query Too Slow

1. Add service filter: `service = "api" && <other conditions>`
2. Reduce time range: Limit search to recent data
3. Use indexed fields: service, status, and operation are fastest

### Parse Errors

1. Check quotes: Strings with special characters need quotes
2. Check operators: Use `&&` not `AND`, `||` not `OR`
3. Check parentheses: Ensure they're balanced

---

*For more examples and updates, see the [Urpo GitHub repository](https://github.com/yairturpo/urpo)*