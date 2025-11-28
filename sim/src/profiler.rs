//! Simple profiling utility for measuring system performance.
//!
//! This module provides lightweight profiling tools for measuring
//! the execution time of ECS systems and system groups.
//!
//! ## Usage
//!
//! Enable profiling with the `profile` feature:
//! ```bash
//! cargo test --release --features profile
//! ```
//!
//! Or use the `StressProfiler` directly in stress tests.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A simple profiler for measuring named sections of code.
///
/// Collects timing data for named sections and provides
/// aggregated statistics.
#[derive(Default)]
pub struct Profiler {
    /// Accumulated time per section
    sections: HashMap<String, SectionStats>,
    /// Current section being timed (if any)
    current_section: Option<(String, Instant)>,
    /// Total ticks profiled
    tick_count: u64,
}

/// Statistics for a profiled section
#[derive(Default, Clone)]
pub struct SectionStats {
    pub total_time: Duration,
    pub call_count: u64,
    pub min_time: Option<Duration>,
    pub max_time: Option<Duration>,
}

impl SectionStats {
    pub fn avg_time(&self) -> Duration {
        if self.call_count == 0 {
            Duration::ZERO
        } else {
            self.total_time / self.call_count as u32
        }
    }
}

impl Profiler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start timing a named section.
    /// Call `end_section` to stop timing.
    pub fn begin_section(&mut self, name: &str) {
        self.current_section = Some((name.to_string(), Instant::now()));
    }

    /// End the current section and record its duration.
    pub fn end_section(&mut self) {
        if let Some((name, start)) = self.current_section.take() {
            let elapsed = start.elapsed();
            let stats = self.sections.entry(name).or_default();
            stats.total_time += elapsed;
            stats.call_count += 1;
            stats.min_time = Some(stats.min_time.map_or(elapsed, |m| m.min(elapsed)));
            stats.max_time = Some(stats.max_time.map_or(elapsed, |m| m.max(elapsed)));
        }
    }

    /// Time a section using a closure.
    pub fn time_section<F, R>(&mut self, name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.begin_section(name);
        let result = f();
        self.end_section();
        result
    }

    /// Increment the tick counter.
    pub fn tick(&mut self) {
        self.tick_count += 1;
    }

    /// Get the number of ticks profiled.
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Get statistics for a specific section.
    pub fn get_section(&self, name: &str) -> Option<&SectionStats> {
        self.sections.get(name)
    }

    /// Get all section names.
    pub fn section_names(&self) -> Vec<&str> {
        self.sections.keys().map(|s| s.as_str()).collect()
    }

    /// Print a summary of all profiled sections.
    pub fn print_summary(&self) {
        println!("\n=== Profiler Summary ({} ticks) ===", self.tick_count);
        
        // Sort sections by total time (descending)
        let mut sections: Vec<_> = self.sections.iter().collect();
        sections.sort_by(|a, b| b.1.total_time.cmp(&a.1.total_time));
        
        // Calculate total time across all sections
        let total: Duration = sections.iter().map(|(_, s)| s.total_time).sum();
        
        println!("{:<25} {:>10} {:>10} {:>10} {:>10} {:>8}",
                 "Section", "Total", "Avg/tick", "Min", "Max", "% Time");
        println!("{}", "-".repeat(78));
        
        for (name, stats) in &sections {
            let avg_per_tick = if self.tick_count > 0 {
                stats.total_time / self.tick_count as u32
            } else {
                Duration::ZERO
            };
            
            let pct = if total.as_nanos() > 0 {
                (stats.total_time.as_nanos() as f64 / total.as_nanos() as f64) * 100.0
            } else {
                0.0
            };
            
            println!("{:<25} {:>10.2?} {:>10.2?} {:>10.2?} {:>10.2?} {:>7.1}%",
                     name,
                     stats.total_time,
                     avg_per_tick,
                     stats.min_time.unwrap_or(Duration::ZERO),
                     stats.max_time.unwrap_or(Duration::ZERO),
                     pct);
        }
        
        println!("{}", "-".repeat(78));
        println!("{:<25} {:>10.2?}", "TOTAL", total);
        
        if self.tick_count > 0 {
            let avg_tick = total / self.tick_count as u32;
            let effective_fps = if avg_tick.as_secs_f64() > 0.0 {
                1.0 / avg_tick.as_secs_f64()
            } else {
                0.0
            };
            println!("{:<25} {:>10.2?} ({:.1} FPS)", "Avg per tick", avg_tick, effective_fps);
        }
        println!();
    }

    /// Reset all profiling data.
    pub fn reset(&mut self) {
        self.sections.clear();
        self.current_section = None;
        self.tick_count = 0;
    }
}

/// A stress test profiler that wraps SimWorld and measures system groups.
///
/// This is designed for use in stress tests to identify performance bottlenecks.
pub struct StressProfiler {
    pub profiler: Profiler,
    pub total_time: Duration,
}

impl StressProfiler {
    pub fn new() -> Self {
        Self {
            profiler: Profiler::new(),
            total_time: Duration::ZERO,
        }
    }

    /// Record a tick's total time.
    pub fn record_tick(&mut self, duration: Duration) {
        self.total_time += duration;
        self.profiler.tick();
    }

    /// Print final summary.
    pub fn print_summary(&self, unit_count: usize) {
        let ticks = self.profiler.tick_count();
        let avg_tick = if ticks > 0 {
            self.total_time / ticks as u32
        } else {
            Duration::ZERO
        };
        
        println!("\n=== Stress Test Summary ===");
        println!("Units: {}", unit_count);
        println!("Ticks: {}", ticks);
        println!("Total time: {:?}", self.total_time);
        println!("Avg per tick: {:?} ({:.2} ms)", avg_tick, avg_tick.as_secs_f64() * 1000.0);
        
        let effective_fps = if avg_tick.as_secs_f64() > 0.0 {
            1.0 / avg_tick.as_secs_f64()
        } else {
            0.0
        };
        println!("Effective FPS: {:.1}", effective_fps);
        
        // Print per-system breakdown if available
        if !self.profiler.sections.is_empty() {
            self.profiler.print_summary();
        }
    }
}

impl Default for StressProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_profiler_basic() {
        let mut profiler = Profiler::new();
        
        profiler.time_section("test_section", || {
            sleep(Duration::from_millis(10));
        });
        
        profiler.tick();
        
        let stats = profiler.get_section("test_section").unwrap();
        assert!(stats.total_time >= Duration::from_millis(10));
        assert_eq!(stats.call_count, 1);
    }

    #[test]
    fn test_profiler_multiple_sections() {
        let mut profiler = Profiler::new();
        
        for _ in 0..5 {
            profiler.time_section("fast", || {
                sleep(Duration::from_millis(1));
            });
            profiler.time_section("slow", || {
                sleep(Duration::from_millis(5));
            });
            profiler.tick();
        }
        
        assert_eq!(profiler.tick_count(), 5);
        
        let fast = profiler.get_section("fast").unwrap();
        let slow = profiler.get_section("slow").unwrap();
        
        assert_eq!(fast.call_count, 5);
        assert_eq!(slow.call_count, 5);
        assert!(slow.total_time > fast.total_time);
    }
}
