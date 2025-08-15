# Rust Best Practices for Claude
**Essential Guidelines for Production-Ready Rust Code**

## Core Principles

### 1. **Complete Implementation Only**
- ✅ No `TODO!()`, `unimplemented!()`, or `panic!()` in production code
- ✅ Every function fully implemented and tested
- ✅ All error cases handled explicitly

### 2. **Error Handling First**
```rust
// ✅ ALWAYS use Result types and proper error handling
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {message}")]
    Parse { message: String },
}

pub type Result<T> = std::result::Result<T, AppError>;

// ✅ Never unwrap in library code
pub fn read_config(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)?;
    serde_json::from_str(&contents).map_err(|e| AppError::Parse {
        message: e.to_string(),
    })
}
```

### 3. **Performance Awareness**
```rust
// ✅ Avoid unnecessary allocations
pub fn process_items(items: &[Item]) -> Vec<&str> {
    items.iter()
        .filter(|item| item.is_valid())
        .map(|item| item.name.as_str())  // Borrow, don't clone
        .collect()
}

// ✅ Use Cow for flexible string handling
use std::borrow::Cow;

pub fn get_name(input: &str) -> Cow<'_, str> {
    if input.starts_with("prefix_") {
        Cow::Borrowed(&input[7..])
    } else {
        Cow::Owned(format!("default_{}", input))
    }
}
```

---

## Project Structure

```
your-project/
├── Cargo.toml          # Dependencies and metadata
├── src/
│   ├── main.rs         # Binary entry point
│   ├── lib.rs          # Library root
│   ├── cli/            # CLI commands
│   ├── core/           # Business logic
│   └── utils/          # Utilities
├── tests/              # Integration tests
└── examples/           # Usage examples
```

---

## Essential Patterns

### 1. **Builder Pattern for Complex Types**
```rust
#[derive(Debug)]
pub struct Config {
    pub timeout: Duration,
    pub retries: u32,
    pub endpoint: String,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    timeout: Option<Duration>,
    retries: Option<u32>,
    endpoint: Option<String>,
}

impl ConfigBuilder {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = Some(retries);
        self
    }

    pub fn build(self) -> Result<Config> {
        Ok(Config {
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
            retries: self.retries.unwrap_or(3),
            endpoint: self.endpoint.ok_or(AppError::Parse {
                message: "endpoint is required".to_string(),
            })?,
        })
    }
}
```

### 2. **Trait-Based Architecture**
```rust
// ✅ Define clear interfaces
pub trait DataSource {
    async fn fetch_data(&self) -> Result<Vec<Record>>;
    async fn health_check(&self) -> Result<()>;
}

// ✅ Implement for different sources
pub struct FileSource {
    path: PathBuf,
}

impl DataSource for FileSource {
    async fn fetch_data(&self) -> Result<Vec<Record>> {
        let content = tokio::fs::read_to_string(&self.path).await?;
        serde_json::from_str(&content).map_err(Into::into)
    }

    async fn health_check(&self) -> Result<()> {
        if self.path.exists() {
            Ok(())
        } else {
            Err(AppError::Parse {
                message: format!("File not found: {:?}", self.path),
            })
        }
    }
}
```

### 3. **Strong Typing with NewTypes**
```rust
// ✅ Use NewType pattern for domain concepts
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraceId(String);

impl UserId {
    pub fn new(id: String) -> Result<Self> {
        if id.is_empty() {
            return Err(AppError::Parse {
                message: "UserId cannot be empty".to_string(),
            });
        }
        Ok(UserId(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

---

## Memory and Performance

### 1. **Efficient String Handling**
```rust
// ✅ Use &str when possible, String when ownership needed
pub fn process_data(input: &str) -> String {
    input.trim().to_lowercase()
}

// ✅ Use format! for complex strings, avoid concatenation
pub fn create_message(user: &str, action: &str) -> String {
    format!("User {} performed {}", user, action)
}

// ✅ Use Vec::with_capacity when size is known
pub fn collect_items(size_hint: usize) -> Vec<String> {
    let mut items = Vec::with_capacity(size_hint);
    // ... fill items
    items
}
```

### 2. **Async Best Practices**
```rust
// ✅ Use async/await properly
pub async fn fetch_multiple_sources(
    sources: Vec<Box<dyn DataSource>>,
) -> Result<Vec<Vec<Record>>> {
    let futures: Vec<_> = sources
        .iter()
        .map(|source| source.fetch_data())
        .collect();

    let results = futures::future::try_join_all(futures).await?;
    Ok(results)
}

// ✅ Use channels for producer/consumer patterns
use tokio::sync::mpsc;

pub async fn process_stream(
    mut receiver: mpsc::Receiver<Data>,
) -> Result<()> {
    while let Some(data) = receiver.recv().await {
        // Process data
        process_item(data).await?;
    }
    Ok(())
}
```

---

## Testing Standards

### 1. **Unit Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_creation() {
        let valid_id = UserId::new("user123".to_string());
        assert!(valid_id.is_ok());

        let invalid_id = UserId::new("".to_string());
        assert!(invalid_id.is_err());
    }

    #[tokio::test]
    async fn test_file_source() {
        use tempfile::NamedTempFile;
        use std::io::Write;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"[{"id": "1", "name": "test"}]"#).unwrap();

        let source = FileSource {
            path: temp_file.path().to_path_buf(),
        };

        let result = source.fetch_data().await;
        assert!(result.is_ok());
    }
}
```

### 2. **Integration Tests**
```rust
// tests/integration_tests.rs
use your_crate::{Config, DataSource, FileSource};

#[tokio::test]
async fn test_end_to_end_processing() {
    let config = Config::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("valid config");

    // Test with real data
    let source = FileSource::new("tests/fixtures/sample.json");
    let data = source.fetch_data().await.expect("fetch should succeed");

    assert!(!data.is_empty());
}
```

---

## Documentation

### 1. **Public API Documentation**
```rust
/// Processes trace data and extracts performance insights.
///
/// This function analyzes OpenTelemetry traces to identify bottlenecks
/// and error patterns in distributed systems.
///
/// # Arguments
///
/// * `traces` - A slice of trace data to analyze
/// * `config` - Analysis configuration including thresholds
///
/// # Returns
///
/// Returns a `Result` containing analysis insights or an error if
/// processing fails.
///
/// # Examples
///
/// ```rust
/// use your_crate::{analyze_traces, Config, Trace};
///
/// let traces = vec![/* trace data */];
/// let config = Config::default();
/// let insights = analyze_traces(&traces, &config)?;
/// ```
///
/// # Errors
///
/// Returns `AppError::Parse` if trace data is malformed.
/// Returns `AppError::Io` if temporary files cannot be created.
pub fn analyze_traces(
    traces: &[Trace],
    config: &Config,
) -> Result<Vec<Insight>> {
    // Implementation
}
```

---

## Cargo.toml Best Practices

```toml
[package]
name = "urpo"
version = "0.1.0"
edition = "2021"
rust-version = "1.70"
license = "MIT OR Apache-2.0"
description = "Fast OTEL trace analysis"
repository = "https://github.com/user/urpo"
keywords = ["observability", "tracing"]
categories = ["command-line-utilities"]

[dependencies]
# Core
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["rt-multi-thread", "fs", "net"] }
thiserror = "1.0"

# CLI
clap = { version = "4.0", features = ["derive"] }

[dev-dependencies]
tempfile = "3.0"
wiremock = "0.5"

[features]
default = []

[[bin]]
name = "urpo"
path = "src/main.rs"
```

---

## CI/CD Integration

### 1. **Essential Checks**
```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Check formatting
        run: cargo fmt --check

      - name: Lint
        run: cargo clippy -- -D warnings

      - name: Test
        run: cargo test

      - name: Build
        run: cargo build --release
```

---

## Key Rules Summary

1. **No panics in library code** - Always use `Result` types
2. **Complete implementations** - No TODOs or stubs
3. **Proper error handling** - Use `thiserror` for custom errors
4. **Performance awareness** - Avoid unnecessary allocations
5. **Strong typing** - Use NewTypes for domain concepts
6. **Comprehensive testing** - Unit and integration tests
7. **Clear documentation** - Document all public APIs
8. **Consistent formatting** - Use `rustfmt` and `clippy`

**Remember**: Write code that's maintainable, performant, and correct from day one!
