# Development script for Neutryx with hot reloading
# Usage: dev [-clean] [-nobuild]
#   -clean   : Remove node_modules and reinstall dependencies
#   -nobuild : Skip Rust rebuild (use existing binary)

param(
    [switch]$clean,
    [switch]$nobuild
)

$ErrorActionPreference = "Stop"
$rootDir = $PWD
$frontendDir = Join-Path $rootDir "demo/gui/static"

Write-Host "======================================" -ForegroundColor Cyan
Write-Host " Neutryx Development Environment" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Clean frontend if requested
if ($clean) {
    Write-Host "[1/4] Cleaning frontend dependencies..." -ForegroundColor Yellow

    # Kill any node processes that might lock files
    Write-Host "      Stopping Node processes..." -ForegroundColor Gray
    Get-Process -Name "node" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1

    Set-Location $frontendDir
    if (Test-Path "node_modules") {
        # Use cmd /c rd for more reliable deletion on Windows
        cmd /c "rd /s /q node_modules" 2>$null
        if (Test-Path "node_modules") {
            # Fallback: try PowerShell method
            Remove-Item -Recurse -Force "node_modules" -ErrorAction SilentlyContinue
        }
    }
    if (Test-Path "package-lock.json") {
        Remove-Item -Force "package-lock.json"
    }
    Write-Host "      Cleaned!" -ForegroundColor Green
} else {
    Write-Host "[1/4] Skipping clean (use -clean to reinstall)" -ForegroundColor Gray
}

# Install frontend dependencies
Write-Host "[2/4] Installing frontend dependencies..." -ForegroundColor Yellow
Set-Location $frontendDir
if (-not (Test-Path "node_modules")) {
    npm install
    Write-Host "      Installed!" -ForegroundColor Green
} else {
    Write-Host "      Dependencies already installed" -ForegroundColor Gray
}

Set-Location $rootDir

# Start Rust server in background
Write-Host "[3/4] Starting Rust server..." -ForegroundColor Yellow
$rustJob = Start-Job -ScriptBlock {
    Set-Location $using:rootDir
    if ($using:nobuild) {
        # Run existing binary
        & "target/debug/neutryx-server.exe"
    } else {
        cargo run -p service_gateway --bin neutryx-server --features demo
    }
}

# Wait for Rust server to start
Write-Host "      Waiting for Rust server..." -ForegroundColor Gray
Start-Sleep -Seconds 5

# Check if Rust server started successfully
$jobState = Get-Job -Id $rustJob.Id
if ($jobState.State -eq "Failed") {
    Write-Host "      ERROR: Rust server failed to start!" -ForegroundColor Red
    Receive-Job $rustJob
    Remove-Job $rustJob -Force
    exit 1
}
Write-Host "      Rust server started!" -ForegroundColor Green

# Start Vite dev server
Write-Host "[4/4] Starting Vite dev server..." -ForegroundColor Yellow
Write-Host ""
Write-Host "======================================" -ForegroundColor Green
Write-Host " Ready! Access at: http://localhost:5173" -ForegroundColor Green
Write-Host "======================================" -ForegroundColor Green
Write-Host ""
Write-Host "  Frontend (Vite):  http://localhost:5173" -ForegroundColor White
Write-Host "  Backend (Rust):   http://localhost:8080" -ForegroundColor Gray
Write-Host "  API Proxy:        /api/* -> :8080" -ForegroundColor Gray
Write-Host ""
Write-Host "  Press Ctrl+C to stop both servers" -ForegroundColor Gray
Write-Host ""

try {
    Set-Location $frontendDir
    npm run dev
} finally {
    Write-Host ""
    Write-Host "Shutting down..." -ForegroundColor Yellow
    Stop-Job $rustJob -ErrorAction SilentlyContinue
    Remove-Job $rustJob -Force -ErrorAction SilentlyContinue
    Write-Host "Done!" -ForegroundColor Green
    Set-Location $rootDir
}
