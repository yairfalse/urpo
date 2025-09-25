//! High-performance telemetry system for Tauri app monitoring.

use atomic_float::AtomicF64 as AtomicFloat;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use sysinfo::System;
use tokio::sync::RwLock;

/// Global telemetry state for ultra-high performance monitoring
pub struct TelemetryState {
    pub command_latencies: DashMap<String, AtomicFloat>,
    pub heap_usage_mb: AtomicFloat,
    pub cpu_usage_percent: AtomicFloat,
    pub memory_pressure: AtomicFloat,
    pub cold_fetch_latency_ms: AtomicFloat,
    pub tier_status: DashMap<String, String>,
    pub free_space_mb: AtomicFloat,
    pub system_info: RwLock<Option<System>>,
}

impl TelemetryState {
    pub fn new() -> Self {
        Self {
            command_latencies: DashMap::new(),
            heap_usage_mb: AtomicFloat::new(0.0),
            cpu_usage_percent: AtomicFloat::new(0.0),
            memory_pressure: AtomicFloat::new(0.0),
            cold_fetch_latency_ms: AtomicFloat::new(0.0),
            tier_status: DashMap::new(),
            free_space_mb: AtomicFloat::new(0.0),
            system_info: RwLock::new(None),
        }
    }

    /// Record command latency with exponential moving average for ultra-low latency
    #[inline(always)]
    pub fn record_command_latency(&self, command: &str, latency_ms: f64) {
        if let Some(existing) = self.command_latencies.get(command) {
            let current = existing.load(Ordering::Relaxed);
            let alpha = 0.1; // Exponential moving average
            let new_value = alpha * latency_ms + (1.0 - alpha) * current;
            existing.store(new_value, Ordering::Relaxed);
        } else {
            self.command_latencies
                .insert(command.to_string(), AtomicFloat::new(latency_ms));
        }
    }

    /// Update system metrics with minimal overhead (cached updates)
    pub async fn update_system_metrics(&self) {
        let mut system_guard = self.system_info.write().await;
        if system_guard.is_none() {
            *system_guard = Some(System::new_all());
        }

        if let Some(ref mut system) = system_guard.as_mut() {
            system.refresh_memory();
            system.refresh_cpu_all();

            // Memory pressure calculation (0.0 = no pressure, 1.0 = maximum pressure)
            let used_memory = system.used_memory() as f64;
            let total_memory = system.total_memory() as f64;
            let pressure = used_memory / total_memory;
            self.memory_pressure.store(pressure, Ordering::Relaxed);

            // Heap usage in MB
            let heap_mb = used_memory / (1024.0 * 1024.0);
            self.heap_usage_mb.store(heap_mb, Ordering::Relaxed);

            // CPU usage percentage
            let cpu_usage = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>()
                / system.cpus().len() as f32;
            self.cpu_usage_percent
                .store(cpu_usage as f64, Ordering::Relaxed);

            // Available disk space
            let available_space = system.available_memory() as f64 / (1024.0 * 1024.0);
            self.free_space_mb.store(available_space, Ordering::Relaxed);
        }
    }

    /// Get command latencies as HashMap for serialization
    #[inline]
    pub fn get_command_latencies(&self) -> HashMap<String, f64> {
        self.command_latencies
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
            .collect()
    }

    /// Get current memory pressure
    #[inline(always)]
    pub fn get_memory_pressure(&self) -> f64 {
        self.memory_pressure.load(Ordering::Relaxed)
    }

    /// Get CPU usage percentage
    #[inline(always)]
    pub fn get_cpu_usage(&self) -> f64 {
        self.cpu_usage_percent.load(Ordering::Relaxed)
    }

    /// Get heap usage in MB
    #[inline(always)]
    pub fn get_heap_usage_mb(&self) -> f64 {
        self.heap_usage_mb.load(Ordering::Relaxed)
    }
}
