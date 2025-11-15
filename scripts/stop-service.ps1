# Stop the persistent Ingat MCP service (Windows)

param(
    [int]$Port = 3200,
    [switch]$Force,
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

function Write-ErrorMsg($message) {
    Write-ColorOutput Red $message
}

# Show help
if ($Help) {
    Write-Output "Stop Ingat MCP Service"
    Write-Output ""
    Write-Output "Usage: .\stop-service.ps1 [OPTIONS]"
    Write-Output ""
    Write-Output "Options:"
    Write-Output "  -Port <port>    Service port (default: 3200)"
    Write-Output "  -Force          Force kill without confirmation"
    Write-Output "  -Help           Show this help"
    Write-Output ""
    Write-Output "Examples:"
    Write-Output "  .\stop-service.ps1                # Stop service on default port"
    Write-Output "  .\stop-service.ps1 -Port 3201     # Stop service on custom port"
    Write-Output "  .\stop-service.ps1 -Force         # Force stop without confirmation"
    exit 0
}

# Override with environment variable if set
if ($env:INGAT_SERVICE_PORT) {
    $Port = [int]$env:INGAT_SERVICE_PORT
}

Write-Info "========================================"
Write-Info "  Stop Ingat MCP Service"
Write-Info "========================================"
Write-Output ""

# Find service process
Write-Info "Checking for running service..."

$processes = Get-Process -Name "mcp-service" -ErrorAction SilentlyContinue

if (-not $processes) {
    # Try to find by port
    try {
        $connection = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
        if ($connection) {
            $processId = $connection.OwningProcess
            $processes = Get-Process -Id $processId -ErrorAction SilentlyContinue
        }
    } catch {
        # Ignore errors
    }
}

if (-not $processes) {
    Write-Warning "No mcp-service process found"
    Write-Output ""
    Write-Output "The service is not running, or it's running under a different name."
    Write-Output ""
    Write-Info "To check manually:"
    Write-Output "  Get-Process -Name `"mcp-service`""
    Write-Output "  Get-NetTCPConnection -LocalPort $Port -State Listen"
    exit 0
}

Write-Success "Found mcp-service process(es):"
foreach ($process in $processes) {
    Write-Output "  PID: $($process.Id)"
}
Write-Output ""

# Confirm before stopping (unless -Force)
if (-not $Force) {
    $response = Read-Host "Stop the service? (y/n)"
    if ($response -notmatch '^[Yy]$') {
        Write-Output "Cancelled."
        exit 0
    }
}

# Stop each process
foreach ($process in $processes) {
    Write-Info "Stopping process $($process.Id)..."

    try {
        # Try graceful shutdown first
        $process.CloseMainWindow() | Out-Null

        # Wait up to 5 seconds for graceful shutdown
        $timeout = 5
        $stopped = $false

        for ($i = 0; $i -lt $timeout; $i++) {
            if ($process.HasExited) {
                Write-Success "✓ Process $($process.Id) stopped gracefully"
                $stopped = $true
                break
            }
            Start-Sleep -Seconds 1
            $process.Refresh()
        }

        # Force kill if still running
        if (-not $stopped) {
            Write-Warning "Process still running, forcing shutdown..."
            Stop-Process -Id $process.Id -Force -ErrorAction Stop
            Start-Sleep -Seconds 1

            # Check if stopped
            $checkProcess = Get-Process -Id $process.Id -ErrorAction SilentlyContinue
            if (-not $checkProcess) {
                Write-Success "✓ Process $($process.Id) force stopped"
            } else {
                Write-ErrorMsg "✗ Failed to stop process $($process.Id)"
            }
        }
    } catch {
        Write-ErrorMsg "✗ Failed to stop process $($process.Id): $($_.Exception.Message)"
        Write-Output "You may need to run this script as Administrator"
    }
}

Write-Output ""

# Verify service is stopped
Start-Sleep -Seconds 1
$stillRunning = Get-Process -Name "mcp-service" -ErrorAction SilentlyContinue

if ($stillRunning) {
    Write-Warning "⚠ Service may still be running"
    Write-Output "Check with: Get-Process -Name `"mcp-service`""
} else {
    Write-Success "========================================"
    Write-Success "✓ Service stopped successfully!"
    Write-Success "========================================"
}

Write-Output ""
Write-Info "To start the service again:"
Write-Output "  .\scripts\start-service.ps1"
Write-Output "  or: .\target\release\mcp-service.exe"
Write-Output ""

exit 0
