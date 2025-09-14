use thiserror::Error;

#[derive(Error, Debug)]
pub enum UrpoError {
    #[error("OTEL protocol error: {0}")]
    Protocol(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("UI rendering error: {0}")]
    Render(String),

    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Trace not found: {0}")]
    TraceNotFound(String),

    #[error("Invalid span data: {0}")]
    InvalidSpan(String),

    #[error("Memory limit exceeded: current {current}MB, limit {limit}MB")]
    MemoryLimitExceeded { current: usize, limit: usize },

    #[error("Sampling rate must be between 0.0 and 1.0, got {0}")]
    InvalidSamplingRate(f64),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("GRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("Terminal UI error: {0}")]
    Terminal(String),

    #[error("Async task join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("Channel send error")]
    ChannelSend,

    #[error("Channel receive error")]
    ChannelReceive,

    #[error("Timeout error: operation took longer than {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Parse error: {message}")]
    Parse { message: String },

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Buffer full: cannot store more items")]
    BufferFull,
}

/// Result type alias for Urpo operations
pub type Result<T> = std::result::Result<T, UrpoError>;

impl UrpoError {
    /// Creates a new protocol error
    pub fn protocol<S: Into<String>>(msg: S) -> Self {
        Self::Protocol(msg.into())
    }

    /// Creates a new storage error
    pub fn storage<S: Into<String>>(msg: S) -> Self {
        Self::Storage(msg.into())
    }

    /// Creates a new configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }

    /// Creates a new network error
    pub fn network<S: Into<String>>(msg: S) -> Self {
        Self::Network(msg.into())
    }

    /// Creates a new parse error
    pub fn parse<S: Into<String>>(msg: S) -> Self {
        Self::Parse {
            message: msg.into(),
        }
    }

    /// Creates a new render error
    pub fn render<S: Into<String>>(msg: S) -> Self {
        Self::Render(msg.into())
    }

    /// Creates a new terminal error
    pub fn terminal<S: Into<String>>(msg: S) -> Self {
        Self::Terminal(msg.into())
    }

    /// Returns true if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Network(_) => true,
            Self::Timeout { .. } => true,
            Self::ChannelSend | Self::ChannelReceive => true,
            Self::Grpc(status) => {
                matches!(status.code(), tonic::Code::Unavailable | tonic::Code::DeadlineExceeded)
            },
            _ => false,
        }
    }

    /// Returns the error category for metrics/logging
    pub fn category(&self) -> &'static str {
        match self {
            Self::Protocol(_) => "protocol",
            Self::Storage(_) => "storage",
            Self::Config(_) => "config",
            Self::Render(_) | Self::Terminal(_) => "ui",
            Self::ServiceNotFound(_) | Self::TraceNotFound(_) | Self::NotFound(_) => "not_found",
            Self::InvalidSpan(_) | Self::InvalidSamplingRate(_) => "validation",
            Self::MemoryLimitExceeded { .. } => "resource",
            Self::Io(_) => "io",
            Self::Serialization(_) | Self::SerializationError(_) | Self::Parse { .. } => {
                "serialization"
            },
            Self::Grpc(_) | Self::Network(_) => "network",
            Self::Join(_) => "async",
            Self::ChannelSend | Self::ChannelReceive => "channel",
            Self::Timeout { .. } => "timeout",
            Self::Auth(_) => "auth",
            Self::BufferFull => "buffer",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = UrpoError::protocol("test message");
        assert_eq!(err.to_string(), "OTEL protocol error: test message");
        assert_eq!(err.category(), "protocol");
    }

    #[test]
    fn test_error_recoverability() {
        assert!(UrpoError::network("connection failed").is_recoverable());
        assert!(!UrpoError::config("invalid config").is_recoverable());
        assert!(UrpoError::Timeout { timeout_ms: 5000 }.is_recoverable());
    }

    #[test]
    fn test_memory_limit_error() {
        let err = UrpoError::MemoryLimitExceeded {
            current: 2048,
            limit: 1024,
        };
        assert_eq!(err.to_string(), "Memory limit exceeded: current 2048MB, limit 1024MB");
        assert_eq!(err.category(), "resource");
    }
}
