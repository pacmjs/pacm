use colored::{Color, *};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    start_times: HashMap<String, Instant>,
    durations: HashMap<String, Vec<Duration>>,
    metadata: HashMap<String, OperationMetadata>,
    system_metrics: SystemMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetadata {
    pub category: String,
    pub description: String,
    pub expected_range: Option<(u64, u64)>, // min, max in milliseconds
}

#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    pub memory_usage_start: u64,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            start_times: HashMap::new(),
            durations: HashMap::new(),
            metadata: HashMap::new(),
            system_metrics: SystemMetrics::default(),
        }
    }

    pub fn start_timer(&mut self, operation: &str) {
        self.start_times
            .insert(operation.to_string(), Instant::now());

        self.system_metrics.memory_usage_start = 0;
    }

    pub fn stop_timer(&mut self, operation: &str) -> Option<Duration> {
        if let Some(start_time) = self.start_times.remove(operation) {
            let duration = start_time.elapsed();

            self.durations
                .entry(operation.to_string())
                .or_insert_with(Vec::new)
                .push(duration);

            Some(duration)
        } else {
            None
        }
    }

    pub fn add_metadata(&mut self, operation: &str, metadata: OperationMetadata) {
        self.metadata.insert(operation.to_string(), metadata);
    }

    pub fn get_average_duration(&self, operation: &str) -> Option<Duration> {
        if let Some(durations) = self.durations.get(operation) {
            if durations.is_empty() {
                return None;
            }

            let total_nanos: u64 = durations.iter().map(|d| d.as_nanos() as u64).sum();
            let avg_nanos = total_nanos / durations.len() as u64;
            Some(Duration::from_nanos(avg_nanos))
        } else {
            None
        }
    }

    pub fn print_summary(&self) {
        println!("\n{}", "🚀 PACM Performance Summary:".bright_cyan().bold());
        println!("{}", "━".repeat(80).bright_black());

        let mut operations: Vec<_> = self.durations.iter().collect();
        operations.sort_by_key(|(_, durations)| {
            durations.iter().sum::<Duration>() / durations.len() as u32
        });

        for (operation, durations) in operations {
            let avg_duration = self.get_average_duration(operation).unwrap_or_default();
            let default_duration = Duration::default();
            let min_duration = durations.iter().min().unwrap_or(&default_duration);
            let max_duration = durations.iter().max().unwrap_or(&default_duration);

            let ms = avg_duration.as_millis();
            let (color, status) = self.get_performance_status(operation, ms);

            println!(
                "{} {:<35} {:>8}ms (min: {:>6}ms, max: {:>6}ms, runs: {})",
                status,
                operation.bright_white(),
                ms.to_string().color(color),
                min_duration.as_millis(),
                max_duration.as_millis(),
                durations.len()
            );

            if let Some(metadata) = self.metadata.get(operation) {
                if let Some((min_expected, max_expected)) = metadata.expected_range {
                    let performance_indicator = if ms < min_expected as u128 {
                        "⚡ Excellent".bright_green()
                    } else if ms <= max_expected as u128 {
                        "✅ Good".bright_green()
                    } else if ms <= (max_expected * 2) as u128 {
                        "⚠️  Slow".bright_yellow()
                    } else {
                        "🐌 Very Slow".bright_red()
                    };

                    println!(
                        "    {} (expected: {}-{}ms)",
                        performance_indicator, min_expected, max_expected
                    );
                }
            }
        }

        if let Some(total) = self.calculate_total_time() {
            println!("{}", "━".repeat(80).bright_black());
            println!(
                "{} {:<35} {:>8}ms",
                "⚡".bright_yellow(),
                "Total Time:".bright_white().bold(),
                total.as_millis().to_string().bright_cyan().bold()
            );
        }

        self.print_performance_insights();
    }

    fn get_performance_status(&self, operation: &str, ms: u128) -> (Color, &str) {
        if let Some(metadata) = self.metadata.get(operation) {
            if let Some((min_expected, max_expected)) = metadata.expected_range {
                return if ms < min_expected as u128 {
                    (Color::BrightGreen, "⚡")
                } else if ms <= max_expected as u128 {
                    (Color::Green, "✅")
                } else if ms <= (max_expected * 2) as u128 {
                    (Color::Yellow, "⚠️")
                } else {
                    (Color::Red, "🐌")
                };
            }
        }

        if ms < 100 {
            (Color::BrightGreen, "🟢")
        } else if ms < 1000 {
            (Color::Yellow, "🟡")
        } else {
            (Color::Red, "🔴")
        }
    }

    fn print_performance_insights(&self) {
        println!("\n{}", "💡 Performance Insights:".bright_blue().bold());
        println!("{}", "-".repeat(50).bright_black());

        let metrics = self.get_metrics();

        if let Some(fastest) = metrics.fastest_operation {
            println!("🏃 Fastest operation: {}ms", fastest.as_millis());
        }

        if let Some(slowest) = metrics.slowest_operation {
            println!("🐌 Slowest operation: {}ms", slowest.as_millis());
        }

        if let Some(avg) = metrics.average_operation_time {
            println!("📊 Average operation: {}ms", avg.as_millis());
        }

        let bottlenecks: Vec<_> = self
            .durations
            .iter()
            .filter(|(_, durations)| {
                let avg = durations.iter().sum::<Duration>() / durations.len() as u32;
                avg.as_millis() > 1000
            })
            .collect();

        if !bottlenecks.is_empty() {
            println!("\n{}", "🚨 Performance Bottlenecks:".bright_red().bold());
            for (operation, _) in bottlenecks {
                println!("   • {}", operation.bright_red());
            }
        }
    }

    fn calculate_total_time(&self) -> Option<Duration> {
        let total_nanos: u64 = self
            .durations
            .values()
            .flat_map(|durations| durations.iter())
            .map(|d| d.as_nanos() as u64)
            .sum();

        if total_nanos > 0 {
            Some(Duration::from_nanos(total_nanos))
        } else {
            None
        }
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        let all_durations: Vec<Duration> = self
            .durations
            .values()
            .flat_map(|durations| durations.iter())
            .copied()
            .collect();

        PerformanceMetrics {
            total_operations: self.durations.len(),
            total_measurements: all_durations.len(),
            fastest_operation: all_durations.iter().min().copied(),
            slowest_operation: all_durations.iter().max().copied(),
            average_operation_time: self.calculate_average_time(),
            total_time: self.calculate_total_time(),
            operations_summary: self.get_operations_summary(),
        }
    }

    fn calculate_average_time(&self) -> Option<Duration> {
        let all_durations: Vec<Duration> = self
            .durations
            .values()
            .flat_map(|durations| durations.iter())
            .copied()
            .collect();

        if all_durations.is_empty() {
            return None;
        }

        let total_nanos: u64 = all_durations.iter().map(|d| d.as_nanos() as u64).sum();
        let avg_nanos = total_nanos / all_durations.len() as u64;
        Some(Duration::from_nanos(avg_nanos))
    }

    fn get_operations_summary(&self) -> HashMap<String, OperationSummary> {
        self.durations
            .iter()
            .map(|(operation, durations)| {
                let avg = durations.iter().sum::<Duration>() / durations.len() as u32;
                let min = *durations.iter().min().unwrap_or(&Duration::default());
                let max = *durations.iter().max().unwrap_or(&Duration::default());

                (
                    operation.clone(),
                    OperationSummary {
                        runs: durations.len(),
                        average: avg,
                        min,
                        max,
                        total: durations.iter().sum(),
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_operations: usize,
    pub total_measurements: usize,
    pub fastest_operation: Option<Duration>,
    pub slowest_operation: Option<Duration>,
    pub average_operation_time: Option<Duration>,
    pub total_time: Option<Duration>,
    pub operations_summary: HashMap<String, OperationSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSummary {
    pub runs: usize,
    pub average: Duration,
    pub min: Duration,
    pub max: Duration,
    pub total: Duration,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[macro_export]
macro_rules! time_operation {
    ($monitor:expr, $operation:expr, $code:block) => {{
        $monitor.start_timer($operation);
        let result = $code;
        $monitor.stop_timer($operation);
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();

        monitor.start_timer("test_operation");
        thread::sleep(Duration::from_millis(100));
        let duration = monitor.stop_timer("test_operation");

        assert!(duration.is_some());
        assert!(duration.unwrap().as_millis() >= 100);

        let metrics = monitor.get_metrics();
        assert_eq!(metrics.total_operations, 1);
    }

    #[test]
    fn test_multiple_measurements() {
        let mut monitor = PerformanceMonitor::new();

        for _ in 0..3 {
            monitor.start_timer("repeated_operation");
            thread::sleep(Duration::from_millis(50));
            monitor.stop_timer("repeated_operation");
        }

        let avg = monitor.get_average_duration("repeated_operation").unwrap();
        assert!(avg.as_millis() >= 50);
    }
}
