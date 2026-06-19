//! Compute budget: hardware-aware depth control.
//!
//! Samples OS-level metrics (CPU load, memory pressure, available threads)
//! and produces a [`DepthLevel`] that gates how deep the pipeline computes.
//! This is the "frequency scaling" analogy: high system load → shallow compute,
//! idle → deep compute.
//!
//! On Linux, reads `/proc/loadavg` and `/proc/meminfo`. On all other platforms,
//! returns a conservative default (`Standard` depth, `cpu_load = 0.5`).

use serde::{Deserialize, Serialize};

// ── DepthLevel ────────────────────────────────────────────────────

/// How deep the pipeline should compute.
///
/// Ordered from cheapest to most expensive. Higher levels enable more
/// optional stages (attention scheduling, self-knowledge inference,
/// phase tracing, reflexive verification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[repr(u8)]
pub enum DepthLevel {
    /// TAND + arbitrate only. No extensions.
    Minimal = 1,
    /// + SafeFallback. No attention/self_knowledge/phase_trace.
    Reduced = 2,
    /// + attention, self_knowledge, phase_trace. Standard operation.
    Standard = 3,
    /// + reflexive verify. Deeper introspection.
    Deep = 4,
    /// Reserved for future multi-path TAND comparison. Currently same as Deep.
    Exhaustive = 5,
}

impl DepthLevel {
    /// Returns true if the given depth level enables optional extensions
    /// (attention scheduling, self-knowledge inference, phase tracing).
    pub fn has_extensions(self) -> bool {
        self >= DepthLevel::Standard
    }

    /// Returns true if reflexive verification should run.
    pub fn has_reflexive_verify(self) -> bool {
        self >= DepthLevel::Deep
    }
}

impl Default for DepthLevel {
    fn default() -> Self {
        DepthLevel::Standard
    }
}

// ── ComputeBudget ─────────────────────────────────────────────────

/// Hardware-aware compute budget sampled from OS metrics.
///
/// # Fallback
///
/// On platforms where OS metrics cannot be read (non-Linux), `sample()`
/// returns `Standard` depth with `cpu_load = 0.5` — a conservative
/// assumption that neither over-commits nor starves the pipeline.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ComputeBudget {
    /// Current compute depth level.
    pub depth_level: DepthLevel,
    /// OS CPU load average (1-minute), normalized to [0.0, 1.0].
    pub cpu_load: f64,
    /// Memory pressure estimate in [0.0, 1.0].
    pub mem_pressure: f64,
    /// Number of available (idle) hardware threads.
    pub available_threads: u32,
}

impl ComputeBudget {
    /// Sample OS metrics and produce a compute budget.
    ///
    /// On Linux, reads `/proc/loadavg` and `/proc/meminfo`. On other
    /// platforms, returns a conservative default.
    pub fn sample() -> Self {
        let (cpu_load, mem_pressure, available_threads) = sample_os_metrics();

        let depth_level = if cpu_load > 0.9 || mem_pressure > 0.9 {
            DepthLevel::Minimal
        } else if cpu_load > 0.7 || mem_pressure > 0.7 {
            DepthLevel::Reduced
        } else if cpu_load < 0.2 && mem_pressure < 0.3 {
            DepthLevel::Deep
        } else {
            DepthLevel::Standard
        };

        ComputeBudget {
            depth_level,
            cpu_load,
            mem_pressure,
            available_threads,
        }
    }

    /// Create a budget with explicit values (for testing).
    pub fn new(
        depth_level: DepthLevel,
        cpu_load: f64,
        mem_pressure: f64,
        available_threads: u32,
    ) -> Self {
        ComputeBudget {
            depth_level,
            cpu_load: cpu_load.clamp(0.0, 1.0),
            mem_pressure: mem_pressure.clamp(0.0, 1.0),
            available_threads,
        }
    }

    /// Create a conservative default budget (Standard depth, moderate load).
    pub fn conservative() -> Self {
        ComputeBudget {
            depth_level: DepthLevel::Standard,
            cpu_load: 0.5,
            mem_pressure: 0.5,
            available_threads: num_cpus::get() as u32,
        }
    }
}

impl Default for ComputeBudget {
    fn default() -> Self {
        Self::conservative()
    }
}

// ── OS sampling (platform-specific) ───────────────────────────────

/// Sample OS metrics. Returns (cpu_load, mem_pressure, available_threads).
fn sample_os_metrics() -> (f64, f64, u32) {
    let available_threads = num_cpus::get() as u32;

    #[cfg(target_os = "linux")]
    {
        let cpu_load = read_linux_loadavg().unwrap_or(0.5);
        let mem_pressure = read_linux_meminfo().unwrap_or(0.5);
        (cpu_load, mem_pressure, available_threads)
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Conservative fallback for non-Linux platforms.
        // CPU load is estimated from available threads vs total.
        // Memory pressure is unknown → assume moderate.
        (0.5, 0.5, available_threads)
    }
}

#[cfg(target_os = "linux")]
fn read_linux_loadavg() -> Option<f64> {
    let content = std::fs::read_to_string("/proc/loadavg").ok()?;
    // Format: "0.15 0.20 0.18 1/1234 56789"
    let first_field = content.split_whitespace().next()?;
    let load: f64 = first_field.parse().ok()?;
    // Normalize: divide by available CPUs. load=1.0 on a 4-core system = 0.25.
    let ncpus = num_cpus::get() as f64;
    Some((load / ncpus.max(1.0)).clamp(0.0, 1.0))
}

#[cfg(target_os = "linux")]
fn read_linux_meminfo() -> Option<f64> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut mem_total: Option<u64> = None;
    let mut mem_available: Option<u64> = None;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            mem_total = parse_kb_value(line);
        } else if line.starts_with("MemAvailable:") {
            mem_available = parse_kb_value(line);
        }
        if mem_total.is_some() && mem_available.is_some() {
            break;
        }
    }

    let total = mem_total?;
    let available = mem_available?;
    if total == 0 {
        return None;
    }
    // Pressure = 1.0 - (available / total). 0.0 = all free, 1.0 = all used.
    Some((1.0 - (available as f64 / total as f64)).clamp(0.0, 1.0))
}

#[cfg(target_os = "linux")]
fn parse_kb_value(line: &str) -> Option<u64> {
    // Lines look like: "MemTotal:       16384000 kB"
    line.split_whitespace().nth(1)?.parse().ok()
}

// ── num_cpus polyfill ─────────────────────────────────────────────

/// Minimal polyfill for `std::thread::available_parallelism`.
/// In Rust 1.96, `std::thread::available_parallelism` is stable,
/// so we use it directly.
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    }
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_level_ordering() {
        assert!(DepthLevel::Exhaustive > DepthLevel::Deep);
        assert!(DepthLevel::Deep > DepthLevel::Standard);
        assert!(DepthLevel::Standard > DepthLevel::Reduced);
        assert!(DepthLevel::Reduced > DepthLevel::Minimal);
    }

    #[test]
    fn depth_level_has_extensions() {
        assert!(!DepthLevel::Minimal.has_extensions());
        assert!(!DepthLevel::Reduced.has_extensions());
        assert!(DepthLevel::Standard.has_extensions());
        assert!(DepthLevel::Deep.has_extensions());
        assert!(DepthLevel::Exhaustive.has_extensions());
    }

    #[test]
    fn depth_level_has_reflexive_verify() {
        assert!(!DepthLevel::Minimal.has_reflexive_verify());
        assert!(!DepthLevel::Reduced.has_reflexive_verify());
        assert!(!DepthLevel::Standard.has_reflexive_verify());
        assert!(DepthLevel::Deep.has_reflexive_verify());
        assert!(DepthLevel::Exhaustive.has_reflexive_verify());
    }

    #[test]
    fn conservative_budget_is_standard() {
        let budget = ComputeBudget::conservative();
        assert_eq!(budget.depth_level, DepthLevel::Standard);
        assert_eq!(budget.cpu_load, 0.5);
        assert_eq!(budget.mem_pressure, 0.5);
        assert!(budget.available_threads > 0);
    }

    #[test]
    fn explicit_budget_clamps_values() {
        let budget = ComputeBudget::new(DepthLevel::Minimal, 1.5, -0.5, 8);
        assert_eq!(budget.cpu_load, 1.0);
        assert_eq!(budget.mem_pressure, 0.0);
    }

    #[test]
    fn sample_produces_valid_budget() {
        let budget = ComputeBudget::sample();
        assert!(budget.depth_level as u8 >= 1);
        assert!(budget.depth_level as u8 <= 5);
        assert!((0.0..=1.0).contains(&budget.cpu_load));
        assert!((0.0..=1.0).contains(&budget.mem_pressure));
        assert!(budget.available_threads > 0);
    }

    #[test]
    fn default_budget_is_conservative() {
        let budget = ComputeBudget::default();
        assert_eq!(budget.depth_level, DepthLevel::Standard);
    }
}
