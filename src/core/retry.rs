//! Retry and recovery utilities for transient errors.

use crate::core::{Result, UrpoError};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier (e.g., 2.0 for exponential backoff)
    pub multiplier: f64,
    /// Add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Retry policy for determining if an error is retryable
pub trait RetryPolicy {
    /// Check if the error should trigger a retry
    fn should_retry(&self, error: &UrpoError, attempt: u32) -> bool;
}

/// Default retry policy that retries on recoverable errors
pub struct DefaultRetryPolicy;

impl RetryPolicy for DefaultRetryPolicy {
    fn should_retry(&self, error: &UrpoError, attempt: u32) -> bool {
        error.is_recoverable() && attempt < 3
    }
}

/// Execute an operation with retry logic
pub async fn retry_with_config<F, Fut, T>(config: RetryConfig, operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut backoff = config.initial_backoff;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                if !error.is_recoverable() || attempt >= config.max_attempts {
                    tracing::error!("Operation failed after {} attempts: {}", attempt, error);
                    return Err(error);
                }

                // Calculate next backoff duration
                if attempt > 1 {
                    backoff = Duration::from_secs_f64(backoff.as_secs_f64() * config.multiplier);
                    if backoff > config.max_backoff {
                        backoff = config.max_backoff;
                    }
                }

                // Add jitter if configured
                let actual_backoff = if config.jitter {
                    let jitter_ms = rand::random::<f64>() * backoff.as_millis() as f64 * 0.1;
                    backoff + Duration::from_millis(jitter_ms as u64)
                } else {
                    backoff
                };

                tracing::warn!(
                    "Attempt {} failed: {}. Retrying in {:?}...",
                    attempt,
                    error,
                    actual_backoff
                );

                sleep(actual_backoff).await;
            },
        }
    }
}

/// Simple retry with default configuration
pub async fn retry<F, Fut, T>(operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    retry_with_config(RetryConfig::default(), operation).await
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    /// Failure threshold before opening
    failure_threshold: u32,
    /// Success threshold before closing
    success_threshold: u32,
    /// Timeout for half-open state
    timeout: Duration,
    /// Current failure count
    failures: std::sync::atomic::AtomicU32,
    /// Current success count
    successes: std::sync::atomic::AtomicU32,
    /// Circuit state
    state: std::sync::Arc<tokio::sync::RwLock<CircuitState>>,
    /// Last state change time
    last_change: std::sync::Arc<tokio::sync::RwLock<std::time::Instant>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            failures: std::sync::atomic::AtomicU32::new(0),
            successes: std::sync::atomic::AtomicU32::new(0),
            state: std::sync::Arc::new(tokio::sync::RwLock::new(CircuitState::Closed)),
            last_change: std::sync::Arc::new(tokio::sync::RwLock::new(std::time::Instant::now())),
        }
    }

    /// Execute an operation through the circuit breaker
    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        use std::sync::atomic::Ordering;

        // Check circuit state
        let mut state = self.state.write().await;
        let mut last_change = self.last_change.write().await;

        match *state {
            CircuitState::Open => {
                // Check if we should transition to half-open
                if last_change.elapsed() >= self.timeout {
                    *state = CircuitState::HalfOpen;
                    *last_change = std::time::Instant::now();
                    tracing::info!("Circuit breaker transitioning to half-open");
                } else {
                    return Err(UrpoError::network("Circuit breaker is open"));
                }
            },
            CircuitState::HalfOpen | CircuitState::Closed => {},
        }

        let current_state = *state;
        drop(state);
        drop(last_change);

        // Execute the operation
        match operation().await {
            Ok(result) => {
                self.successes.fetch_add(1, Ordering::Relaxed);

                if current_state == CircuitState::HalfOpen {
                    let successes = self.successes.load(Ordering::Relaxed);
                    if successes >= self.success_threshold {
                        let mut state = self.state.write().await;
                        if *state == CircuitState::HalfOpen {
                            *state = CircuitState::Closed;
                            self.failures.store(0, Ordering::Relaxed);
                            self.successes.store(0, Ordering::Relaxed);
                            tracing::info!("Circuit breaker closed after {} successes", successes);
                        }
                    }
                }

                Ok(result)
            },
            Err(error) => {
                self.failures.fetch_add(1, Ordering::Relaxed);

                let failures = self.failures.load(Ordering::Relaxed);
                if failures >= self.failure_threshold {
                    let mut state = self.state.write().await;
                    if *state != CircuitState::Open {
                        *state = CircuitState::Open;
                        *self.last_change.write().await = std::time::Instant::now();
                        self.failures.store(0, Ordering::Relaxed);
                        self.successes.store(0, Ordering::Relaxed);
                        tracing::error!("Circuit breaker opened after {} failures", failures);
                    }
                }

                Err(error)
            },
        }
    }

    /// Get the current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }
}

/// Rate limiter for preventing overload
pub struct RateLimiter {
    /// Maximum requests per second
    max_rps: f64,
    /// Token bucket
    tokens: std::sync::Arc<tokio::sync::Mutex<f64>>,
    /// Last refill time
    last_refill: std::sync::Arc<tokio::sync::Mutex<std::time::Instant>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_rps: f64) -> Self {
        Self {
            max_rps,
            tokens: std::sync::Arc::new(tokio::sync::Mutex::new(max_rps)),
            last_refill: std::sync::Arc::new(tokio::sync::Mutex::new(std::time::Instant::now())),
        }
    }

    /// Check if a request can proceed
    pub async fn check(&self) -> Result<()> {
        let mut tokens = self.tokens.lock().await;
        let mut last_refill = self.last_refill.lock().await;

        // Refill tokens based on elapsed time
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        let tokens_to_add = elapsed * self.max_rps;

        *tokens = (*tokens + tokens_to_add).min(self.max_rps);
        *last_refill = now;

        // Check if we have tokens available
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            Ok(())
        } else {
            Err(UrpoError::network("Rate limit exceeded"))
        }
    }

    /// Execute an operation with rate limiting
    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        self.check().await?;
        operation().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = retry(move || {
            let attempts = attempts_clone.clone();
            async move {
                let count = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                if count < 3 {
                    Err(UrpoError::network("temporary failure"))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_non_recoverable() {
        let result: Result<i32> =
            retry(|| async { Err(UrpoError::config("permanent failure")) }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let cb = CircuitBreaker::new(2, 2, Duration::from_millis(100));

        // First failure
        let _: Result<i32> = cb
            .call(|| async { Err(UrpoError::network("failure 1")) })
            .await;

        // Second failure - should open circuit
        let _: Result<i32> = cb
            .call(|| async { Err(UrpoError::network("failure 2")) })
            .await;

        // Circuit should be open
        assert_eq!(cb.state().await, CircuitState::Open);

        // Should fail immediately
        let result = cb.call(|| async { Ok(42) }).await;
        assert!(result.is_err());

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open and allow test
        let result = cb.call(|| async { Ok(42) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10.0); // 10 requests per second

        // Should allow first request
        assert!(limiter.check().await.is_ok());

        // Rapid requests should be limited
        for _ in 0..15 {
            let _ = limiter.check().await;
        }

        // Wait to refill tokens
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Should allow more requests
        assert!(limiter.check().await.is_ok());
    }
}
