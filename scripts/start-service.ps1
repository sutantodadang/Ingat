# Ingat Unified Backend Service Startup Script (Windows)
# This script builds and starts the mcp-service for multi-client usage

param(
    [switch]$Build,
    [switch]$Rebuild,
    [int]$Port = 3200,
    [string]$Host = "127.0.0.1",
    [string]$LogLevel = "info",
    [string]$DataDir = "",
    [switch]$Help
)

# Colors for output
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

function Write-Info($message) {
    Write-ColorOutput Cyan $message
}

function Write-Success($message) {
    Write-ColorOutput Green $message
}

function Write-Warning($message) {
    Write-ColorOutput Yellow $message
}

function Write-Error($message) {
    Write-ColorOutput Red $message
}

# Show help
if ($Help) {
    Write-Output "Ingat Unified Backend Service Startup Script"
    Write-Output ""
    Write-Output "Usage: .\start-service.ps1 [OPTIONS]"
    Write-Output ""
    Write-Output "Options:"
    Write-Output "  -Build              Build before starting"
    Write-Output "  -Rebuild            Force rebuild (clean build)"
    Write-Output "  -Port <port>        Set service port (default: 3200)"
    Write-Output "  -Host <host>        Set bind address (default: 127.0.0.1)"
    Write-Output "  -LogLevel <level>   Set log level (default: info)"
    Write-Output "  -DataDir <path>     Set data directory"
    Write-Output "  -Help               Show this help"
    Write-Output ""
    Write-Output "Environment Variables:"
    Write-Output "  INGAT_SERVICE_PORT    Service port"
    Write-Output "  INGAT_SERVICE_HOST    Bind address"
    Write-Output "  INGAT_LOG             Log level"
    Write-Output "  INGAT_DATA_DIR        Data directory"
    Write-Output ""
    Write-Output "Examples:"
    Write-Output "  .\start-service.ps1                    # Start with defaults"
    Write-Output "  .\start-service.ps1 -Build             # Build then start"
    Write-Output "  .\start-service.ps1 -Port 3201         # Use custom port"
    Write-Output "  .\start-service.ps1 -LogLevel debug    # Enable debug logging"
    exit 0
}

# Script paths
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$TauriDir = Join-Path $ProjectRoot "src-tauri"
$BinaryPath = Join-Path $TauriDir "target\release\mcp-service.exe"

Write-Info "========================================"
Write-Info "  Ingat Unified Backend Service"
Write-Info "========================================"
Write-Output ""

# Override with environment variables if set
if ($env:INGAT_SERVICE_PORT) {
    $Port = [int]$env:INGAT_SERVICE_PORT
}
if ($env:INGAT_SERVICE_HOST) {
    $Host = $env:INGAT_SERVICE_HOST
}
if ($env:INGAT_LOG) {
    $LogLevel = $env:INGAT_LOG
}
if ($env:INGAT_DATA_DIR) {
    $DataDir = $env:INGAT_DATA_DIR
}

# Function to check if port is in use
function Test-PortInUse($port) {
    $connections = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
    return $connections.Count -gt 0
}

# Function to build the service
function Build-Service {
    Write-Info "Building mcp-service..."
    Push-Location $TauriDir

    try {
        $result = cargo build --release --bin mcp-service --features mcp-server,tauri-plugin 2>&1

        if ($LASTEXITCODE -eq 0) {
            Write-Success "✓ Build successful"
            Write-Output ""
            return $true
        } else {
            Write-Error "✗ Build failed"
            Write-Output $result
            return $false
        }
    } finally {
        Pop-Location
    }
}

# Function to start the service
function Start-Service {
    Write-Info "Starting service..."
    Write-Output "Configuration:"
    Write-Success "  Host:      $Host"
    Write-Success "  Port:      $Port"
    Write-Success "  Log Level: $LogLevel"

    if ($DataDir) {
        Write-Success "  Data Dir:  $DataDir"
    }
    Write-Output ""

    # Set environment variables
    $env:INGAT_SERVICE_HOST = $Host
    $env:INGAT_SERVICE_PORT = $Port
    $env:INGAT_LOG = $LogLevel

    if ($DataDir) {
        $env:INGAT_DATA_DIR = $DataDir
    }

    # Start the service
    Write-Success "Service starting..."
    Write-Output "Press Ctrl+C to stop"
    Write-Output ""
    Write-Info "========================================"
    Write-Output ""

    & $BinaryPath
}

# Main execution
try {
    # Check if binary exists
    if (-not (Test-Path $BinaryPath)) {
        Write-Warning "Binary not found. Building..."
        $Build = $true
    }

    # Build if requested
    if ($Rebuild) {
        Write-Warning "Forcing clean build..."
        Push-Location $TauriDir
        try {
            cargo clean --release --package ingat --bin mcp-service 2>$null
        } catch {
            # Ignore errors from clean
        } finally {
            Pop-Location
        }

        if (-not (Build-Service)) {
            exit 1
        }
    } elseif ($Build) {
        if (-not (Build-Service)) {
            exit 1
        }
    }

    # Check if port is already in use
    if (Test-PortInUse $Port) {
        Write-Warning "Warning: Port $Port is already in use!"
        Write-Output "Another service might be running. Stop it first or change the port:"
        Write-Output "  `$env:INGAT_SERVICE_PORT = 3201"
        Write-Output "Or use: .\start-service.ps1 -Port 3201"
        exit 1
    }

    # Start the service
    Start-Service

} catch {
    Write-Error "An error occurred: $_"
    exit 1
}
