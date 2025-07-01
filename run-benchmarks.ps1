#!/usr/bin/env pwsh

# PACM Benchmark Runner Script for Windows PowerShell
# This script provides easy commands to run different benchmark suites

param(
    [Parameter(Position = 0)]
    [ValidateSet("all", "install", "resolution", "cache", "download", "system", "stress", "compare", "report", "criterion", "help")]
    [string]$Command = "help",
    
    [int]$Iterations = 3,
    [int]$ConcurrentOps = 10,
    [string[]]$Packages = @(),
    [string[]]$Managers = @(),
    [string]$Output = "",
    [switch]$Detailed,
    [switch]$DebugMode
)

function Show-Help {
    Write-Host "🚀 PACM Benchmark Runner" -ForegroundColor Cyan
    Write-Host "=========================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor Yellow
    Write-Host "  .\run-benchmarks.ps1 <command> [options]" -ForegroundColor White
    Write-Host ""
    Write-Host "Commands:" -ForegroundColor Yellow
    Write-Host "  all         - Run all benchmark categories" -ForegroundColor Green
    Write-Host "  install     - Run installation benchmarks" -ForegroundColor Green
    Write-Host "  resolution  - Run dependency resolution benchmarks" -ForegroundColor Green
    Write-Host "  cache       - Run cache performance benchmarks" -ForegroundColor Green
    Write-Host "  download    - Run download performance benchmarks" -ForegroundColor Green
    Write-Host "  system      - Run system resource benchmarks (memory, CPU)" -ForegroundColor Green
    Write-Host "  stress      - Run stress tests with high concurrent load" -ForegroundColor Green
    Write-Host "  compare     - Compare against other package managers" -ForegroundColor Green
    Write-Host "  report      - Generate performance report" -ForegroundColor Green
    Write-Host "  criterion   - Run Criterion.rs statistical benchmarks" -ForegroundColor Green
    Write-Host ""
    Write-Host "Options:" -ForegroundColor Yellow
    Write-Host "  -Iterations <n>  - Number of iterations (default: 3)" -ForegroundColor White
    Write-Host "  -ConcurrentOps <n> - Concurrent operations for stress tests (default: 10)" -ForegroundColor White
    Write-Host "  -Packages <list> - Specific packages to test" -ForegroundColor White
    Write-Host "  -Managers <list> - Package managers to compare against" -ForegroundColor White
    Write-Host "  -Output <path>   - Output file path for reports" -ForegroundColor White
    Write-Host "  -Detailed        - Show detailed performance metrics" -ForegroundColor White
    Write-Host "  -DebugMode       - Enable debug output" -ForegroundColor White
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor Yellow
    Write-Host "  .\run-benchmarks.ps1 all -Iterations 5 -Detailed" -ForegroundColor Cyan
    Write-Host "  .\run-benchmarks.ps1 install -Packages lodash,express" -ForegroundColor Cyan
    Write-Host "  .\run-benchmarks.ps1 system -Iterations 5" -ForegroundColor Cyan
    Write-Host "  .\run-benchmarks.ps1 stress -ConcurrentOps 20 -Iterations 3" -ForegroundColor Cyan
    Write-Host "  .\run-benchmarks.ps1 compare -Managers npm,yarn,pnpm" -ForegroundColor Cyan
    Write-Host "  .\run-benchmarks.ps1 criterion" -ForegroundColor Cyan
}

function Start-Benchmark {
    param($BenchCommand, $Arguments)
    
    $BuildArgs = @("run", "--bin", "pacm-benchmark")
    if ($Debug) {
        $env:RUST_LOG = "debug"
    }
    
    $BuildArgs += $BenchCommand
    $BuildArgs += $Arguments
    
    Write-Host "🔄 Running: cargo $($BuildArgs -join ' ')" -ForegroundColor Yellow
    & cargo @BuildArgs
}

function Start-Criterion {
    Write-Host "📊 Running Criterion.rs benchmarks..." -ForegroundColor Cyan
    
    Write-Host "🔄 Building benchmarks..." -ForegroundColor Yellow
    cargo build --benches
    
    Write-Host "🔄 Running installation benchmarks..." -ForegroundColor Yellow
    cargo bench --bench install_benchmarks
    
    Write-Host "🔄 Running resolution benchmarks..." -ForegroundColor Yellow
    cargo bench --bench resolution_benchmarks
    
    Write-Host "🔄 Running cache benchmarks..." -ForegroundColor Yellow
    cargo bench --bench cache_benchmarks
    
    Write-Host "🔄 Running download benchmarks..." -ForegroundColor Yellow
    cargo bench --bench download_benchmarks
    
    Write-Host "✅ Criterion benchmarks completed!" -ForegroundColor Green
    Write-Host "📈 View HTML reports in target/criterion/" -ForegroundColor Cyan
}

# Change to the benchmark directory
$BenchmarkDir = Join-Path $PSScriptRoot "apps\benchmark"
if (Test-Path $BenchmarkDir) {
    Set-Location $BenchmarkDir
} else {
    Write-Error "Benchmark directory not found: $BenchmarkDir"
    exit 1
}

switch ($Command) {
    "help" {
        Show-Help
    }
    
    "all" {
        $benchArgs = @("--iterations", $Iterations)
        if ($Detailed) { $benchArgs += "--detailed" }
        Start-Benchmark "all" $benchArgs
    }
    
    "install" {
        $benchArgs = @("--iterations", $Iterations)
        if ($Packages.Count -gt 0) {
            $benchArgs += "--packages"
            $benchArgs += ($Packages -join ",")
        }
        Start-Benchmark "install" $benchArgs
    }
    
    "resolution" {
        $benchArgs = @("--iterations", $Iterations)
        Start-Benchmark "resolution" $benchArgs
    }
    
    "cache" {
        $benchArgs = @("--iterations", $Iterations)
        Start-Benchmark "cache" $benchArgs
    }
    
    "download" {
        $benchArgs = @("--iterations", $Iterations)
        Start-Benchmark "download" $benchArgs
    }
    
    "system" {
        $benchArgs = @("--iterations", $Iterations)
        Start-Benchmark "system" $benchArgs
    }
    
    "stress" {
        $benchArgs = @("--iterations", $Iterations)
        if ($ConcurrentOps -ne 10) {
            $benchArgs += "--concurrent-operations"
            $benchArgs += $ConcurrentOps
        }
        Start-Benchmark "stress" $benchArgs
    }
    
    "compare" {
        $benchArgs = @("--iterations", $Iterations)
        if ($Managers.Count -gt 0) {
            $benchArgs += "--managers"
            $benchArgs += ($Managers -join ",")
        }
        Start-Benchmark "compare" $benchArgs
    }
    
    "report" {
        $benchArgs = @()
        if ($Output) {
            $benchArgs += "--output"
            $benchArgs += $Output
        }
        Start-Benchmark "report" $benchArgs
    }
    
    "criterion" {
        Start-Criterion
    }
    
    default {
        Write-Error "Unknown command: $Command"
        Show-Help
        exit 1
    }
}
