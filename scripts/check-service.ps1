# Ingat Service Status Checker (Windows)
# Checks if the mcp-service is running and healthy

param(
    [int]$Port = 3200,
    [string]$Host = "127.0.0.1",
    [switch]$Verbose
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

function Write-Status($status, $message) {
    switch ($status) {
        "ok" { Write-Output "$(Write-ColorOutput Green '✓') $message" }
        "error" { Write-Output "$(Write-ColorOutput Red '✗') $message" }
        "warning" { Write-Output "$(Write-ColorOutput Yellow '⚠') $message" }
        default { Write-Output "$(Write-ColorOutput Cyan 'ℹ') $message" }
    }
}

# Override with environment variables if set
if ($env:INGAT_SERVICE_PORT) {
    $Port = [int]$env:INGAT_SERVICE_PORT
}
if ($env:INGAT_SERVICE_HOST) {
    $Host = $env:INGAT_SERVICE_HOST
}

$ServiceUrl = "http://${Host}:${Port}"

Write-Info "========================================"
Write-Info "  Ingat Service Status Checker"
Write-Info "========================================"
Write-Output ""

# Check 1: Process running
Write-Info "Checking process..."
$process = Get-Process -Name "mcp-service" -ErrorAction SilentlyContinue

if ($process) {
    Write-Status "ok" "Process is running (PID: $($process.Id))"
    $ProcessId = $process.Id
} else {
    Write-Status "error" "Process is NOT running"
    Write-Output ""
    Write-Warning "To start the service:"
    Write-Output "  .\scripts\start-service.ps1"
    Write-Output "  or manually: .\target\release\mcp-service.exe"
    exit 1
}

# Check 2: Port listening
Write-Output ""
Write-Info "Checking port ${Port}..."
$connection = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue

if ($connection) {
    Write-Status "ok" "Port ${Port} is listening"
} else {
    Write-Status "error" "Port ${Port} is NOT listening"
    Write-Output ""
    Write-Warning "The process is running but not listening on port ${Port}"
    Write-Output "Check the service logs for errors."
    exit 1
}

# Check 3: Health endpoint
Write-Output ""
Write-Info "Checking health endpoint..."

try {
    $healthResponse = Invoke-WebRequest -Uri "${ServiceUrl}/health" -Method GET -TimeoutSec 5 -ErrorAction Stop

    if ($healthResponse.StatusCode -eq 200) {
        Write-Status "ok" "Health endpoint responding (HTTP 200)"

        $healthData = $healthResponse.Content | ConvertFrom-Json
        if ($Verbose) {
            Write-Info "Response:"
            Write-Output "  Status:  $($healthData.status)"
            Write-Output "  Service: $($healthData.service)"
        } else {
            Write-Output "  $($healthResponse.Content)"
        }
    } else {
        Write-Status "error" "Health endpoint returned HTTP $($healthResponse.StatusCode)"
        exit 1
    }
} catch {
    Write-Status "error" "Health endpoint not responding"
    Write-Output "  Error: $($_.Exception.Message)"
    exit 1
}

# Check 4: Service statistics
Write-Output ""
Write-Info "Fetching service statistics..."

try {
    $statsResponse = Invoke-WebRequest -Uri "${ServiceUrl}/api/stats" -Method GET -TimeoutSec 5 -ErrorAction Stop

    if ($statsResponse.StatusCode -eq 200) {
        Write-Status "ok" "Statistics endpoint responding"

        $stats = $statsResponse.Content | ConvertFrom-Json
        Write-Output ""
        Write-Info "Service Details:"
        Write-Output "  Version:        $($stats.version)"
        Write-Output "  Total Contexts: $($stats.total_contexts)"
        Write-Output "  Data Directory: $($stats.data_dir)"
        Write-Output "  Uptime:         $($stats.uptime_seconds) seconds"
    } else {
        Write-Status "warning" "Statistics endpoint returned HTTP $($statsResponse.StatusCode)"
    }
} catch {
    Write-Status "warning" "Could not fetch statistics"
    if ($Verbose) {
        Write-Output "  Error: $($_.Exception.Message)"
    }
}

# Check 5: Recent contexts
Write-Output ""
Write-Info "Checking contexts endpoint..."

try {
    $contextsResponse = Invoke-WebRequest -Uri "${ServiceUrl}/api/contexts?limit=1" -Method GET -TimeoutSec 5 -ErrorAction Stop

    if ($contextsResponse.StatusCode -eq 200) {
        Write-Status "ok" "Contexts endpoint responding (HTTP 200)"
    } else {
        Write-Status "warning" "Contexts endpoint returned HTTP $($contextsResponse.StatusCode)"
    }
} catch {
    Write-Status "warning" "Contexts endpoint error"
    if ($Verbose) {
        Write-Output "  Error: $($_.Exception.Message)"
    }
}

# Test search endpoint
Write-Output ""
Write-Info "Checking search endpoint..."

try {
    $searchBody = @{
        prompt = "test"
        limit = 1
    } | ConvertTo-Json

    $searchResponse = Invoke-WebRequest -Uri "${ServiceUrl}/api/search" `
        -Method POST `
        -ContentType "application/json" `
        -Body $searchBody `
        -TimeoutSec 5 `
        -ErrorAction Stop

    if ($searchResponse.StatusCode -eq 200) {
        Write-Status "ok" "Search endpoint responding (HTTP 200)"
    } else {
        Write-Status "warning" "Search endpoint returned HTTP $($searchResponse.StatusCode)"
    }
} catch {
    Write-Status "warning" "Search endpoint error"
    if ($Verbose) {
        Write-Output "  Error: $($_.Exception.Message)"
    }
}

# Summary
Write-Output ""
Write-Info "========================================"
Write-Success "✓ Service is running and healthy!"
Write-Info "========================================"
Write-Output ""
Write-Info "Service URL:  ${ServiceUrl}"
Write-Output "  Health:     ${ServiceUrl}/health"
Write-Output "  Stats:      ${ServiceUrl}/api/stats"
Write-Output "  Contexts:   ${ServiceUrl}/api/contexts"
Write-Output "  Search:     ${ServiceUrl}/api/search"
Write-Output ""
Write-Info "To stop the service:"
Write-Output "  Stop-Process -Id ${ProcessId}"
Write-Output "  or press Ctrl+C in the service terminal"
Write-Output ""

# Additional tips
if (-not $Verbose) {
    Write-Output ""
    Write-Output "Tip: Use -Verbose flag for detailed output"
    Write-Output "  .\scripts\check-service.ps1 -Verbose"
}

Write-Output ""
exit 0
