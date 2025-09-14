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
}
