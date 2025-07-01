# PACM Benchmark Suite

A performance benchmarking suite for the PACM package manager.

## Overview

This benchmark suite provides detailed performance analysis across multiple categories:

- **Installation Benchmarks**: Test package installation performance
- **Resolution Benchmarks**: Test dependency resolution speed
- **Cache Benchmarks**: Test cache lookup and storage performance  
- **Download Benchmarks**: Test package download performance
- **System Benchmarks**: Test memory usage, CPU utilization, and system resource consumption
- **Stress Benchmarks**: Test performance under high load and concurrent operations

## Quick Start

### Building the Benchmark Suite

From the workspace root:
```powershell
# Build the benchmark application
cargo build --bin pacm-benchmark

# Or build in release mode for accurate performance measurements
cargo build --release --bin pacm-benchmark
```

### Running Basic Benchmarks

```powershell
# Run all benchmarks
cargo run --bin pacm-benchmark all

# Run all benchmarks with detailed output
cargo run --bin pacm-benchmark all --detailed

# Run with custom iterations for better statistical accuracy
cargo run --bin pacm-benchmark all --iterations 5
```

## Benchmark Categories

### 1. Installation Benchmarks

Test package installation performance across different package sizes:

```powershell
# Run installation benchmarks
cargo run --bin pacm-benchmark install

# Test specific packages
cargo run --bin pacm-benchmark install --packages lodash express typescript

# Test with multiple iterations
cargo run --bin pacm-benchmark install --iterations 5
```

**Package Categories:**
- **Small packages**: lodash, chalk, debug (< 1MB)
- **Medium packages**: express, react, vue (1-10MB)  
- **Large packages**: typescript, webpack, @babel/core (10-50MB)

### 2. Resolution Benchmarks

Test dependency resolution algorithms:

```powershell
# Run resolution benchmarks
cargo run --bin pacm-benchmark resolution --iterations 3
```

**Test Scenarios:**
- Simple resolution (5 deps, depth 2)
- Medium resolution (20 deps, depth 4)
- Complex resolution (50 deps, depth 6)
- Conflict resolution scenarios

### 3. Cache Benchmarks

Test cache performance:

```powershell
# Run cache benchmarks
cargo run --bin pacm-benchmark cache --iterations 3
```

**Cache Operations:**
- Cache hits vs. misses
- Cache storage performance
- Cache eviction algorithms
- Concurrent cache access

### 4. Download Benchmarks

Test package download performance:

```powershell
# Run download benchmarks
cargo run --bin pacm-benchmark download --iterations 3
```

**Download Scenarios:**
- Small files (~100KB)
- Medium files (~2.5MB)
- Large files (~15MB)
- Concurrent downloads

### 5. System Benchmarks

Test system resource usage:

```powershell
# Run system performance benchmarks
cargo run --bin pacm-benchmark system --iterations 3
```

**System Metrics:**
- Memory usage during operations
- CPU utilization
- File system I/O
- Network bandwidth usage

### 6. Stress Testing

Test performance under high load:

```powershell
# Run stress tests
cargo run --bin pacm-benchmark stress --concurrent-operations 10 --iterations 3

# High-load stress test
cargo run --bin pacm-benchmark stress --concurrent-operations 50 --iterations 5
```

**Stress Scenarios:**
- Concurrent package installations
- High-frequency cache operations
- Multiple resolution requests
- Resource exhaustion testing

## Advanced Usage

### Comparison with Other Package Managers

```powershell
# Compare against npm, yarn, and pnpm
cargo run --bin pacm-benchmark compare --managers npm yarn pnpm --iterations 3
```

### Generate Performance Reports

```powershell
# Generate markdown report
cargo run --bin pacm-benchmark report --output benchmark_report.md

# Generate JSON report for analysis
cargo run --bin pacm-benchmark report --output benchmark_data.json
```

## Criterion.rs Benchmarks

For detailed statistical analysis using Criterion.rs:

```powershell
# Run all Criterion benchmarks
cargo bench

# Run specific benchmark suites
cargo bench --bench install_benchmarks
cargo bench --bench resolution_benchmarks
cargo bench --bench cache_benchmarks
cargo bench --bench download_benchmarks
cargo bench --bench system_benchmarks

# Generate HTML reports
cargo bench -- --output-format html

# Save results for comparison
cargo bench -- --save-baseline before_optimization
# ... make changes ...
cargo bench -- --baseline before_optimization
```

## Understanding Results

### Performance Expectations

| Operation | Small | Medium | Large |
|-----------|-------|--------|-------|
| Install | 100ms - 2s | 500ms - 5s | 2s - 15s |
| Resolution | 50ms - 500ms | 200ms - 2s | 1s - 5s |
| Cache Hit | 10ms - 100ms | 50ms - 500ms | 200ms - 2s |
| Download | 100ms - 1s | 1s - 5s | 5s - 20s |

### Performance Indicators

- 🟢 **Excellent**: Below minimum expected time
- ✅ **Good**: Within expected range
- ⚠️ **Slow**: 1-2x expected time
- 🐌 **Very Slow**: >2x expected time

### Sample Output

```
🚀 PACM Performance Summary:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🟢 cache_hit_lodash                      45ms (min:    38ms, max:    52ms, runs: 3)
   ⚡ Excellent (expected: 10-100ms)
✅ install_lodash                       1.2s (min:   980ms, max:  1.4s, runs: 3)
   ✅ Good (expected: 100ms-2s)
⚠️  install_typescript                  8.5s (min:   7.2s, max:  9.8s, runs: 3)
   ⚠️  Slow (expected: 2s-15s)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⚡ Total Time:                         10.2s
```

## Configuration

### Environment Variables

```powershell
# Set custom registry for testing
$env:PACM_REGISTRY_URL = "https://registry.npmjs.org/"

# Set cache directory
$env:PACM_CACHE_DIR = "C:\temp\pacm-cache"

# Enable verbose logging
$env:PACM_LOG_LEVEL = "debug"
```

### Custom Test Scenarios

You can modify `src/test_scenarios.rs` to add custom package combinations for testing specific use cases.

## Troubleshooting

### Common Issues

1. **Slow initial runs**: First runs are slower due to cache warming
2. **Network-dependent results**: Download benchmarks vary with network conditions
3. **System resource contention**: Close other applications for accurate results

### Best Practices

1. **Run multiple iterations**: Use `--iterations 5` for statistical significance
2. **Use release builds**: Always benchmark with `--release` flag
3. **Consistent environment**: Run benchmarks on the same system configuration
4. **Baseline measurements**: Save baseline results before making changes

## Contributing

To add new benchmarks:

1. Add test scenarios in `src/test_scenarios.rs`
2. Implement benchmark in appropriate `benches/*.rs` file
3. Add command-line options in `src/main.rs`
4. Update this README with usage instructions

## Performance Optimization

Use benchmark results to identify bottlenecks:

1. **Installation**: Focus on dependency resolution and download parallelization
2. **Resolution**: Optimize algorithm complexity and caching
3. **Cache**: Improve lookup performance and memory usage
4. **Downloads**: Implement better compression and parallel downloads

Tests dependency resolution performance:
- Simple dependency trees
- Complex dependency trees
- Scoped packages (@types/node, @babel/core)
- Multiple dependency resolution
- Version range resolution

**Expected Performance Ranges:**
- Simple resolution: 50ms - 500ms
- Complex resolution: 200ms - 2s

### Cache Benchmarks

Tests cache system performance:
- Cache lookup operations
- Cache storage operations
- Cache index building
- Batch cache operations

**Expected Performance Ranges:**
- Cache lookup: 1ms - 50ms
- Cache storage: 10ms - 200ms

### Download Benchmarks

Tests package download performance:
- Small package downloads
- Large package downloads
- Parallel downloads
- Different concurrency levels
- Retry mechanisms

**Expected Performance Ranges:**
- Small packages: 100ms - 1s
- Large packages: 500ms - 5s

## Test Scenarios

The benchmark suite includes predefined test scenarios:

### Small Packages
- lodash
- is-number  
- chalk
- debug
- ms

### Medium Packages
- express
- react
- vue
- axios
- moment

### Large Packages
- typescript
- webpack
- @babel/core
- eslint
- jest

### Complex Scenarios
- **React App**: react, react-dom, react-router-dom, axios, styled-components
- **Node Server**: express, cors, helmet, morgan, dotenv, jsonwebtoken
- **Full-stack Dev**: typescript, webpack, babel, eslint, prettier, jest
- **Vue App**: vue, vue-router, vuex, @vue/cli-service, vite

## Performance Insights

The benchmark suite provides performance insights including:

- **Operation timing** with color-coded status indicators
- **Statistical analysis** (min, max, mean, median, percentiles)
- **Performance bottleneck identification**
- **Comparison against expected ranges**
- **System resource usage**

## Output Example

```
🚀 PACM Performance Summary:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⚡ install_cached_lodash              85ms (min:    76ms, max:   94ms, runs: 3)
✅ install_fresh_lodash              1247ms (min:  1156ms, max: 1338ms, runs: 3)  
🟡 install_fresh_express             3456ms (min:  3201ms, max: 3712ms, runs: 3)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⚡ Total Time:                        4788ms

💡 Performance Insights:
──────────────────────────────────────────────────
🏃 Fastest operation: 76ms
🐌 Slowest operation: 3712ms  
📊 Average operation: 1596ms
```

## System Requirements

- Rust 1.85+
- Internet connection (for download benchmarks)
- ~1GB free disk space (for package cache)

## Configuration

The benchmark suite can be configured through:

- **Command line arguments**: iterations, package selection, output format
- **Environment variables**: PACM_STORE_PATH, PACM_CACHE_SIZE
- **Configuration files**: benchmark_config.toml (future)

## Contributing

To add new benchmarks:

1. Add benchmark functions to appropriate modules in `src/benchmarks.rs`
2. Add Criterion benchmarks to `benches/` directory  
3. Update test scenarios in `src/test_scenarios.rs`
4. Update expected performance ranges in metadata

## Troubleshooting

### Common Issues

**High download times**: Check internet connection and try using a different registry mirror.

**Cache permission errors**: Ensure PACM has write permissions to the cache directory.

**Out of disk space**: Clear the PACM cache or increase available disk space.

### Debug Mode

Run benchmarks with debug output:
```bash
RUST_LOG=debug cargo run --bin pacm-benchmark all
```

### Performance Tips

- Run benchmarks on a dedicated machine without other CPU-intensive tasks
- Use SSD storage for better cache performance
- Ensure stable internet connection for download benchmarks
- Run multiple iterations for statistical significance
