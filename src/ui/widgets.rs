//! Custom widgets and UI helpers for Urpo.
//!
//! This module provides reusable UI components and helper functions
//! for creating a beautiful terminal interface.

use ratatui::style::Color;

/// Get health symbol and color based on error rate.
pub fn health_symbol(error_rate: f64) -> (&'static str, Color) {
    let error_pct = error_rate * 100.0;
    if error_pct > 5.0 {
        ("✖", Color::Red)
    } else if error_pct > 1.0 {
        ("⚠", Color::Yellow)
    } else {
        ("●", Color::Green)
    }
}

/// Create a sparkline trend string from values.
pub fn sparkline_trend(values: &[f64]) -> String {
    if values.is_empty() {
        return "     ".to_string();
    }

    let spark_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = values
        .iter()
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
        .max(1.0);
    
    let last_n = 5;
    let chars: Vec<char> = values
        .iter()
        .rev()
        .take(last_n)
        .rev()
        .map(|&v| {
            let normalized = ((v / max) * 7.0) as usize;
            spark_chars[normalized.min(7)]
        })
        .collect();

    // Pad with spaces if we have fewer than expected values
    let mut result = String::new();
    for _ in 0..(last_n - chars.len()) {
        result.push(' ');
    }
    for c in chars {
        result.push(c);
    }
    
    result
}

/// Get color based on latency in milliseconds.
pub fn latency_color(ms: u128) -> Color {
    if ms > 1000 {
        Color::Red
    } else if ms > 500 {
        Color::Yellow
    } else if ms > 200 {
        Color::Cyan
    } else {
        Color::Green
    }
}

/// Format duration in a human-readable way.
pub fn format_duration(duration: std::time::Duration) -> String {
    let ms = duration.as_millis();
    if ms >= 10000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else if ms >= 1000 {
        format!("{:.2}s", ms as f64 / 1000.0)
    } else {
        format!("{}ms", ms)
    }
}

/// Format a percentage with appropriate color.
pub fn format_percentage_colored(value: f64) -> (String, Color) {
    let pct = value * 100.0;
    let text = format!("{:.1}%", pct);
    let color = if pct > 10.0 {
        Color::Red
    } else if pct > 5.0 {
        Color::Yellow
    } else if pct > 1.0 {
        Color::Cyan
    } else {
        Color::Green
    };
    (text, color)
}

/// Create a progress bar string.
pub fn progress_bar(value: f64, width: usize) -> String {
    let filled = (value * width as f64) as usize;
    let empty = width.saturating_sub(filled);
    
    let mut bar = String::new();
    for _ in 0..filled {
        bar.push('█');
    }
    for _ in 0..empty {
        bar.push('░');
    }
    
    bar
}

/// Get status icon for different states.
pub fn status_icon(is_active: bool, is_healthy: bool) -> (&'static str, Color) {
    match (is_active, is_healthy) {
        (true, true) => ("●", Color::Green),
        (true, false) => ("●", Color::Red),
        (false, _) => ("○", Color::DarkGray),
    }
}

/// Format large numbers with units (K, M, B).
pub fn format_count(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Format bytes with appropriate units.
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1}GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1}MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}

/// Format a trace ID for display (shortened).
pub fn format_trace_id(trace_id: &str) -> String {
    if trace_id.len() > 8 {
        format!("{}...", &trace_id[..8])
    } else {
        trace_id.to_string()
    }
}

/// Format a span ID for display (shortened).
pub fn format_span_id(span_id: &str) -> String {
    if span_id.len() > 8 {
        format!("{}...", &span_id[..8])
    } else {
        span_id.to_string()
    }
}

/// Get tree connector characters for span hierarchy.
pub fn get_tree_connectors(is_last: bool, has_children: bool) -> (&'static str, &'static str) {
    match (is_last, has_children) {
        (false, true) => ("├─", "│ "),
        (false, false) => ("├─", "│ "),
        (true, true) => ("└─", "  "),
        (true, false) => ("└─", "  "),
    }
}

/// Format span attributes for display.
pub fn format_attributes(attrs: &std::collections::HashMap<String, String>, max_width: usize) -> String {
    if attrs.is_empty() {
        return String::new();
    }
    
    let mut result = String::new();
    let mut current_len = 0;
    
    for (key, value) in attrs.iter().take(3) {
        let attr = format!("{}={}", key, value);
        if current_len + attr.len() > max_width && current_len > 0 {
            result.push_str("...");
            break;
        }
        if !result.is_empty() {
            result.push_str(", ");
            current_len += 2;
        }
        result.push_str(&attr);
        current_len += attr.len();
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_symbol() {
        assert_eq!(health_symbol(0.001), ("●", Color::Green));
        assert_eq!(health_symbol(0.02), ("⚠", Color::Yellow));
        assert_eq!(health_symbol(0.1), ("✖", Color::Red));
    }

    #[test]
    fn test_sparkline_trend() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let trend = sparkline_trend(&values);
        assert_eq!(trend.len(), 5);
        
        let empty: Vec<f64> = vec![];
        assert_eq!(sparkline_trend(&empty), "     ");
    }

    #[test]
    fn test_format_duration() {
        use std::time::Duration;
        
        assert_eq!(format_duration(Duration::from_millis(50)), "50ms");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
        assert_eq!(format_duration(Duration::from_millis(12345)), "12.3s");
    }

    #[test]
    fn test_format_count() {
        assert_eq!(format_count(123), "123");
        assert_eq!(format_count(1234), "1.2K");
        assert_eq!(format_count(12345), "12K");
        assert_eq!(format_count(1234567), "1.2M");
        assert_eq!(format_count(1234567890), "1.2B");
    }

    #[test]
    fn test_progress_bar() {
        assert_eq!(progress_bar(0.5, 10), "█████░░░░░");
        assert_eq!(progress_bar(0.0, 5), "░░░░░");
        assert_eq!(progress_bar(1.0, 5), "█████");
    }
}