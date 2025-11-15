#!/usr/bin/env pwsh
# Build MCP binaries with correct naming format for Tauri bundling

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Info { Write-Host "[$([char]0x2713)] $args" -ForegroundColor Green }
function Write-Step { Write-Host "==>" $args -ForegroundColor Cyan }
function Write-Error { Write-Host "[X] $args" -ForegroundColor Red }

Write-Host ""
Write-Host "=====================================" -ForegroundColor Magenta
Write-Host "  Ingat MCP Binaries Builder" -ForegroundColor Magenta
Write-Host "=====================================" -ForegroundColor Magenta
Write-Host ""

# Detect target triple
$TARGET = if ($env:CARGO_BUILD_TARGET) {
    $env:CARGO_BUILD_TARGET
} else {
    "x86_64-pc-windows-msvc"
}

$EXTENSION = if ($TARGET -match "windows") { ".exe" } else { "" }
$BINARIES_DIR = "binaries"

Write-Info "Target: $TARGET"
Write-Info "Extension: $EXTENSION"
Write-Host ""

# Create binaries directory
Write-Step "Creating binaries directory..."
if (!(Test-Path $BINARIES_DIR)) {
    New-Item -ItemType Directory -Path $BINARIES_DIR | Out-Null
}
Write-Info "Directory ready: $BINARIES_DIR"
Write-Host ""

# Function to build and copy binary
function Build-Binary {
    param (
        [string]$BinaryName,
        [string]$Features
    )

    Write-Step "Building $BinaryName..."
    Write-Host "   Features: $Features" -ForegroundColor Gray

    $buildCmd = "cargo build --release --bin $BinaryName --features $Features"

    try {
        Invoke-Expression $buildCmd
        Write-Info "Build successful: $BinaryName"
    } catch {
        Write-Error "Build failed for $BinaryName"
        throw
    }

    # Copy to binaries folder with correct naming
    $sourcePath = "target\release\$BinaryName$EXTENSION"
    $destPath = "$BINARIES_DIR\$BinaryName-$TARGET$EXTENSION"

    if (Test-Path $sourcePath) {
        Copy-Item -Path $sourcePath -Destination $destPath -Force
        Write-Info "Copied: $destPath"
    } else {
        Write-Error "Binary not found: $sourcePath"
        throw "Binary not found after build"
    }

    Write-Host ""
}

# Build all binaries
try {
    Build-Binary -BinaryName "mcp-stdio" -Features "mcp-server"
    Build-Binary -BinaryName "mcp-bridge" -Features "mcp-server"
    Build-Binary -BinaryName "mcp-service" -Features "mcp-server,tauri-plugin"

    Write-Host ""
    Write-Host "=====================================" -ForegroundColor Green
    Write-Host "  Build Complete!" -ForegroundColor Green
    Write-Host "=====================================" -ForegroundColor Green
    Write-Host ""

    Write-Info "Binaries are ready in: $BINARIES_DIR"
    Write-Host ""
    Write-Host "Built binaries:" -ForegroundColor Cyan
    Get-ChildItem $BINARIES_DIR | ForEach-Object {
        $size = [math]::Round($_.Length / 1MB, 2)
        Write-Host "  - $($_.Name) ($size MB)" -ForegroundColor White
    }

    Write-Host ""
    Write-Info "To bundle these binaries, update tauri.conf.json:"
    Write-Host '  "bundle": {' -ForegroundColor Gray
    Write-Host '    "resources": ["binaries/*"],' -ForegroundColor Gray
    Write-Host '    "externalBin": [' -ForegroundColor Gray
    Write-Host '      "binaries/mcp-stdio",' -ForegroundColor Gray
    Write-Host '      "binaries/mcp-bridge",' -ForegroundColor Gray
    Write-Host '      "binaries/mcp-service"' -ForegroundColor Gray
    Write-Host '    ]' -ForegroundColor Gray
    Write-Host '  }' -ForegroundColor Gray
    Write-Host ""

} catch {
    Write-Host ""
    Write-Error "Build process failed!"
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}
