# PACM Performance Benchmarking Guide

## Overview

I've successfully moved and enhanced the performance monitoring code from `pacm-core` to a comprehensive benchmark suite in `apps/benchmark`. The benchmark application now includes multiple categories of performance tests and advanced monitoring capabilities.

## What Was Done

1. **Enhanced Performance Monitor**: Moved from `pacm-core` to `apps/benchmark` with advanced features:
   - Multiple measurement support for statistical analysis
   - System resource monitoring (memory, CPU)
   - Performance expectations and thresholds
   - Detailed reporting with color-coded status

2. **New Benchmark Categories**:
   - **System Benchmarks**: Memory usage, CPU utilization
   - **Stress Tests**: High concurrent load testing
   - **Enhanced Test Scenarios**: More comprehensive package combinations

3. **Improved PowerShell Runner**: Enhanced `run-benchmarks.ps1` with new commands

## How to Use the Benchmark Suite

### Basic Commands

```powershell
# Run all benchmarks
.\run-benchmarks.ps1 all

# Run all with detailed output and more iterations
.\run-benchmarks.ps1 all -Detailed -Iterations 5

# Run specific benchmark categories
.\run-benchmarks.ps1 install
.\run-benchmarks.ps1 resolution
.\run-benchmarks.ps1 cache
.\run-benchmarks.ps1 download
.\run-benchmarks.ps1 system      # NEW: Memory and CPU benchmarks
.\run-benchmarks.ps1 stress      # NEW: High-load stress testing
```

### Advanced Usage

```powershell
# Test specific packages
.\run-benchmarks.ps1 install -Packages lodash,express,typescript

# Stress test with high concurrency
.\run-benchmarks.ps1 stress -ConcurrentOps 20 -Iterations 5

# Compare against other package managers
.\run-benchmarks.ps1 compare -Managers npm,yarn,pnpm

# Generate performance reports
.\run-benchmarks.ps1 report -Output benchmark_results.md

# Run statistical benchmarks with Criterion.rs
.\run-benchmarks.ps1 criterion
```

### Direct Cargo Commands

```powershell
# Build benchmark app
cargo build --release --bin pacm-benchmark

# Run benchmark app directly
cargo run --release --bin pacm-benchmark all --iterations 5 --detailed

# Run specific Criterion benchmarks
cargo bench --bench install_benchmarks
cargo bench --bench system_benchmarks    # NEW
cargo bench --bench resolution_benchmarks
cargo bench --bench cache_benchmarks
cargo bench --bench download_benchmarks
```

## Benchmark Categories Explained

### 1. Installation Benchmarks
Tests package installation performance across different sizes:
- **Small**: lodash, chalk, debug (~100KB-1MB)
- **Medium**: express, react, vue (~1-10MB)
- **Large**: typescript, webpack, @babel/core (~10-50MB)

### 2. Resolution Benchmarks
Tests dependency resolution algorithms:
- Simple scenarios (5 deps, depth 2)
- Complex scenarios (50+ deps, depth 6+)
- Conflict resolution scenarios

### 3. Cache Benchmarks
Tests cache system performance:
- Cache hits vs. misses
- Cache storage and retrieval
- Cache eviction algorithms

### 4. Download Benchmarks
Tests package download performance:
- Different file sizes
- Concurrent downloads
- Network utilization

### 5. System Benchmarks (NEW)
Tests system resource usage:
- Memory consumption during operations
- CPU utilization
- Concurrent operation performance

### 6. Stress Tests (NEW)
Tests performance under high load:
- Many concurrent operations
- Resource exhaustion scenarios
- Performance degradation analysis

## Understanding Results

### Performance Status Indicators
- 🟢 **Excellent**: Performance better than expected
- ✅ **Good**: Performance within expected range
- ⚠️ **Slow**: Performance 1-2x slower than expected
- 🐌 **Very Slow**: Performance >2x slower than expected

### Sample Output
```
🚀 PACM Performance Summary:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🟢 cache_hit_lodash              45ms (min:    38ms, max:    52ms, runs: 3)
   ⚡ Excellent (expected: 10-100ms)
✅ install_lodash               1.2s (min:   980ms, max:  1.4s, runs: 3)
   ✅ Good (expected: 100ms-2s)
⚠️  resolution_complex          8.5s (min:   7.2s, max:  9.8s, runs: 3)
   ⚠️  Slow (expected: 1s-5s)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⚡ Total Time:                   10.2s
```

## Performance Expectations

| Operation | Small | Medium | Large |
|-----------|-------|--------|-------|
| Install | 100ms - 2s | 500ms - 5s | 2s - 15s |
| Resolution | 50ms - 500ms | 200ms - 2s | 1s - 5s |
| Cache Hit | 10ms - 100ms | 50ms - 500ms | 200ms - 2s |
| Download | 100ms - 1s | 1s - 5s | 5s - 20s |

## Best Practices

1. **Use Release Builds**: Always benchmark with `--release` for accurate results
2. **Multiple Iterations**: Use `--iterations 5` for statistical significance
3. **Consistent Environment**: Close other applications during benchmarking
4. **Baseline Measurements**: Save baseline results before making optimizations
5. **System Monitoring**: Use system benchmarks to identify resource bottlenecks

## Integration with Development

The benchmark suite can be integrated into your development workflow:

1. **Performance Regression Testing**: Run benchmarks in CI/CD
2. **Optimization Validation**: Compare before/after performance
3. **Resource Planning**: Use system benchmarks for capacity planning
4. **Bottleneck Identification**: Use detailed reports to find slow operations

The enhanced benchmark suite provides comprehensive performance analysis tools to help optimize PACM's performance across all operations!
