#!/bin/bash

# PACM Benchmark Runner Script for Bash
# This script provides easy commands to run different benchmark suites

# Default values for parameters
COMMAND="help"
ITERATIONS=3
CONCURRENT_OPS=10
PACKAGES=()
MANAGERS=()
OUTPUT=""
DETAILED=false
DEBUG_MODE=false

# Function to display help message
show_help() {
    echo -e "\e[36m🚀 PACM Benchmark Runner\e[0m"
    echo -e "\e[36m=========================\e[0m"
    echo ""
    echo -e "\e[33mUsage:\e[0m"
    echo -e "  ./run-benchmarks.sh <command> [options]"
    echo ""
    echo -e "\e[33mCommands:\e[0m"
    echo -e "  all         - Run all benchmark categories"
    echo -e "  install     - Run installation benchmarks"
    echo -e "  resolution  - Run dependency resolution benchmarks"
    echo -e "  cache       - Run cache performance benchmarks"
    echo -e "  download    - Run download performance benchmarks"
    echo -e "  system      - Run system resource benchmarks (memory, CPU)"
    echo -e "  stress      - Run stress tests with high concurrent load"
    echo -e "  compare     - Compare against other package managers"
    echo -e "  report      - Generate performance report"
    echo -e "  criterion   - Run Criterion.rs statistical benchmarks"
    echo ""
    echo -e "\e[33mOptions:\e[0m"
    echo -e "  -i, --iterations <n>     - Number of iterations (default: 3)"
    echo -e "  -c, --concurrent-ops <n> - Concurrent operations for stress tests (default: 10)"
    echo -e "  -p, --packages <list>    - Specific packages to test (comma-separated)"
    echo -e "  -m, --managers <list>    - Package managers to compare against (comma-separated)"
    echo -e "  -o, --output <path>      - Output file path for reports"
    echo -e "  -d, --detailed           - Show detailed performance metrics"
    echo -e "  --debug                  - Enable debug output"
    echo ""
    echo -e "\e[33mExamples:\e[0m"
    echo -e "  ./run-benchmarks.sh all --iterations 5 --detailed"
    echo -e "  ./run-benchmarks.sh install --packages lodash,express"
    echo -e "  ./run-benchmarks.sh system --iterations 5"
    echo -e "  ./run-benchmarks.sh stress --concurrent-ops 20 --iterations 3"
    echo -e "  ./run-benchmarks.sh compare --managers npm,yarn,pnpm"
    echo -e "  ./run-benchmarks.sh criterion"
}

# Function to start a benchmark
start_benchmark() {
    local bench_command="$1"
    shift
    local arguments=("$@")

    local build_args=("run" "--bin" "pacm-benchmark")
    if [ "$DEBUG_MODE" = true ]; then
        export RUST_LOG="debug"
    fi

    build_args+=("$bench_command")
    build_args+=("${arguments[@]}")

    echo -e "\e[33m🔄 Running: cargo ${build_args[*]}\e[0m"
    cargo "${build_args[@]}"
}

# Function to start Criterion.rs benchmarks
start_criterion() {
    echo -e "\e[36m📊 Running Criterion.rs benchmarks...\e[0m"

    echo -e "\e[33m🔄 Building benchmarks...\e[0m"
    cargo build --benches

    echo -e "\e[33m🔄 Running installation benchmarks...\e[0m"
    cargo bench --bench install_benchmarks

    echo -e "\e[33m🔄 Running resolution benchmarks...\e[0m"
    cargo bench --bench resolution_benchmarks

    echo -e "\e[33m🔄 Running cache benchmarks...\e[0m"
    cargo bench --bench cache_benchmarks

    echo -e "\e[33m🔄 Running download benchmarks...\e[0m"
    cargo bench --bench download_benchmarks

    echo -e "\e[32m✅ Criterion benchmarks completed!\e[0m"
    echo -e "\e[36m📈 View HTML reports in target/criterion/\e[0m"
}

# Parse command-line arguments
if [ -n "$1" ]; then
    COMMAND="$1"
    shift
fi

while (( "$#" )); do
    case "$1" in
    -i|--iterations)
        ITERATIONS="$2"
        shift 2
        ;;
    -c|--concurrent-ops)
        CONCURRENT_OPS="$2"
        shift 2
        ;;
    -p|--packages)
        IFS=',' read -r -a PACKAGES <<< "$2"
        shift 2
        ;;
    -m|--managers)
        IFS=',' read -r -a MANAGERS <<< "$2"
        shift 2
        ;;
    -o|--output)
        OUTPUT="$2"
        shift 2
        ;;
    -d|--detailed)
        DETAILED=true
        shift
        ;;
    --debug)
        DEBUG_MODE=true
        shift
        ;;
    --) # end argument parsing
        shift
        break
        ;;
    -*|--*=) # unsupported flags
        echo -e "\e[31mError: Unsupported flag $1\e[0m" >&2
        show_help
        exit 1
        ;;
    *) # preserve positional arguments
        PARAMS="$PARAMS $1"
        shift
        ;;
    esac
done

# Change to the benchmark directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
BENCHMARK_DIR="${SCRIPT_DIR}/apps/benchmark"

if [ -d "$BENCHMARK_DIR" ]; then
    cd "$BENCHMARK_DIR" || { echo -e "\e[31mError: Could not change to directory $BENCHMARK_DIR\e[0m"; exit 1; }
else
    echo -e "\e[31mError: Benchmark directory not found: $BENCHMARK_DIR\e[0m"
    exit 1
fi

# Execute the command
case "$COMMAND" in
    "help")
        show_help
        ;;
    "all")
        bench_args=("--iterations" "$ITERATIONS")
        if [ "$DETAILED" = true ]; then
            bench_args+=("--detailed")
        fi
        start_benchmark "all" "${bench_args[@]}"
        ;;
    "install")
        bench_args=("--iterations" "$ITERATIONS")
        if [ "${#PACKAGES[@]}" -gt 0 ]; then
            bench_args+=("--packages" "$(IFS=,; echo "${PACKAGES[*]}")")
        fi
        start_benchmark "install" "${bench_args[@]}"
        ;;
    "resolution")
        bench_args=("--iterations" "$ITERATIONS")
        start_benchmark "resolution" "${bench_args[@]}"
        ;;
    "cache")
        bench_args=("--iterations" "$ITERATIONS")
        start_benchmark "cache" "${bench_args[@]}"
        ;;
    "download")
        bench_args=("--iterations" "$ITERATIONS")
        start_benchmark "download" "${bench_args[@]}"
        ;;
    "system")
        bench_args=("--iterations" "$ITERATIONS")
        start_benchmark "system" "${bench_args[@]}"
        ;;
    "stress")
        bench_args=("--iterations" "$ITERATIONS")
        if [ "$CONCURRENT_OPS" -ne 10 ]; then
            bench_args+=("--concurrent-operations" "$CONCURRENT_OPS")
        fi
        start_benchmark "stress" "${bench_args[@]}"
        ;;
    "compare")
        bench_args=("--iterations" "$ITERATIONS")
        if [ "${#MANAGERS[@]}" -gt 0 ]; then
            bench_args+=("--managers" "$(IFS=,; echo "${MANAGERS[*]}")")
        fi
        start_benchmark "compare" "${bench_args[@]}"
        ;;
    "report")
        bench_args=()
        if [ -n "$OUTPUT" ]; then
            bench_args+=("--output" "$OUTPUT")
        fi
        start_benchmark "report" "${bench_args[@]}"
        ;;
    "criterion")
        start_criterion
        ;;
    *)
        echo -e "\e[31mError: Unknown command: $COMMAND\e[0m" >&2
        show_help
        exit 1
        ;;
esac