<#
.SYNOPSIS
    Docker Local Test Script for Cloud Run Compatibility
.DESCRIPTION
    This script builds and runs the demo-web Docker container locally,
    simulating the Cloud Run environment to catch deployment issues early.
.PARAMETER BuildOnly
    Only build the image, don't run
.PARAMETER NoCache
    Build without Docker cache
.PARAMETER Port
    Use custom port (default: 8080)
.PARAMETER Debug
    Enable debug mode (FB_DEBUG_MODE=true)
.EXAMPLE
    .\scripts\docker_local_test.ps1
.EXAMPLE
    .\scripts\docker_local_test.ps1 -Port 3000 -Debug
#>

param(
    [switch]$BuildOnly,
    [switch]$NoCache,
    [int]$Port = 8080,
    [switch]$Debug
)

$ErrorActionPreference = "Stop"

# Configuration
$ImageName = "frictional-bank-local"

Write-Host "=============================================" -ForegroundColor Blue
Write-Host "  Docker Local Test for Cloud Run" -ForegroundColor Blue
Write-Host "=============================================" -ForegroundColor Blue
Write-Host ""

# Check if Docker is running
try {
    docker info 2>$null | Out-Null
} catch {
    Write-Host "Error: Docker is not running" -ForegroundColor Red
    exit 1
}

# Navigate to project root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
Set-Location $ProjectRoot

Write-Host "Building Docker image..." -ForegroundColor Yellow
Write-Host "  Image name: $ImageName"
Write-Host "  Dockerfile: Dockerfile.gui"
Write-Host ""

# Build the image
$buildArgs = @("-f", "Dockerfile.gui", "-t", $ImageName)
if ($NoCache) {
    $buildArgs += "--no-cache"
}
$buildArgs += "."

$buildResult = docker build @buildArgs
if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "Build failed!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Common issues:" -ForegroundColor Yellow
    Write-Host "  - Missing static files (check demo/gui/static/)"
    Write-Host "  - Missing data files (check demo/data/input/)"
    Write-Host "  - Case sensitivity: Linux is case-sensitive"
    exit 1
}

Write-Host ""
Write-Host "Build successful!" -ForegroundColor Green

if ($BuildOnly) {
    Write-Host ""
    Write-Host "Build-only mode: Image ready for deployment" -ForegroundColor Green
    exit 0
}

Write-Host ""
Write-Host "Starting container..." -ForegroundColor Yellow
Write-Host "  Port: $Port"
Write-Host "  Debug mode: $Debug"
Write-Host ""

# Build environment variables
$envVars = @(
    "-e", "PORT=$Port",
    "-e", "RUST_LOG=info",
    "-e", "FB_OPEN_BROWSER=false"
)

if ($Debug) {
    $envVars += @(
        "-e", "FB_DEBUG_MODE=true",
        "-e", "FB_LOG_LEVEL=DEBUG",
        "-e", "RUST_LOG=debug"
    )
}

# Run the container
Write-Host "=============================================" -ForegroundColor Blue
Write-Host "Container starting..." -ForegroundColor Green
Write-Host ""
Write-Host "  Access the dashboard at: " -NoNewline
Write-Host "http://localhost:$Port" -ForegroundColor Green
Write-Host ""
Write-Host "Testing checklist:" -ForegroundColor Yellow
Write-Host "  [ ] Dashboard loads correctly"
Write-Host "  [ ] All charts render"
Write-Host "  [ ] WebSocket connection establishes"
Write-Host "  [ ] Portfolio data loads"
Write-Host "  [ ] API endpoints respond"
Write-Host ""
Write-Host "Press Ctrl+C to stop the container" -ForegroundColor Yellow
Write-Host "=============================================" -ForegroundColor Blue
Write-Host ""

# Run interactively
$runArgs = @("run", "--rm", "-it", "-p", "${Port}:${Port}") + $envVars + @($ImageName)
docker @runArgs
