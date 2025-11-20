#!/usr/bin/env pwsh
# Build MCP binaries for multiple platforms with correct naming format for Tauri bundling

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Info { Write-Host "[$([char]0x2713)] $args" -ForegroundColor Green }
function Write-Step { Write-Host "==>" $args -ForegroundColor Cyan }
function Write-Error { Write-Host "[X] $args" -ForegroundColor Red }
function Write-Warn { Write-Host "[!] $args" -ForegroundColor Yellow }

Write-Host ""
Write-Host "=====================================" -ForegroundColor Magenta
Write-Host "  Ingat MCP Binaries Builder" -ForegroundColor Magenta
Write-Host "  Multi-Platform Build Script" -ForegroundColor Magenta
Write-Host "=====================================" -ForegroundColor Magenta
Write-Host ""

# Define target platforms
$TARGETS = @(
    @{ Name = "Windows (x64)"; Triple = "x86_64-pc-windows-msvc"; Extension = ".exe" },
    @{ Name = "Linux (x64)"; Triple = "x86_64-unknown-linux-gnu"; Extension = "" },
    @{ Name = "macOS (ARM64)"; Triple = "aarch64-apple-darwin"; Extension = "" }
)

$BINARIES_DIR = "binaries"

# Check if cross-compilation toolchains are available
Write-Step "Checking installed Rust targets..."
$installedTargets = rustup target list --installed
Write-Host ""

# Create binaries directory
Write-Step "Creating binaries directory..."
if (!(Test-Path $BINARIES_DIR)) {
    New-Item -ItemType Directory -Path $BINARIES_DIR | Out-Null
}
Write-Info "Directory ready: $BINARIES_DIR"
Write-Host ""

# Function to check and install target
function Ensure-Target {
    param (
        [string]$TargetTriple
    )

    if ($installedTargets -notcontains $TargetTriple) {
        Write-Warn "Target $TargetTriple is not installed. Installing..."
        try {
            rustup target add $TargetTriple
            Write-Info "Target installed: $TargetTriple"
        } catch {
            Write-Error "Failed to install target: $TargetTriple"
            return $false
        }
    } else {
        Write-Info "Target already installed: $TargetTriple"
    }
    return $true
}

# Function to build and copy binary for a specific target
function Build-Binary {
    param (
        [string]$BinaryName,
        [string]$Features,
        [string]$TargetTriple,
        [string]$Extension
    )

    Write-Step "Building $BinaryName for $TargetTriple..."
    Write-Host "   Features: $Features" -ForegroundColor Gray

    # Use tauri.build.conf.json to avoid circular dependency (no externalBin check)
    $env:TAURI_CONFIG = "tauri.build.conf.json"
    $buildCmd = "cargo build --release --bin $BinaryName --features $Features --target $TargetTriple"

    try {
        Invoke-Expression $buildCmd
        Write-Info "Build successful: $BinaryName ($TargetTriple)"
    } catch {
        Write-Error "Build failed for $BinaryName ($TargetTriple)"
        Remove-Item Env:\TAURI_CONFIG -ErrorAction SilentlyContinue
        return $false
    } finally {
        Remove-Item Env:\TAURI_CONFIG -ErrorAction SilentlyContinue
    }

    # Copy to binaries folder with correct naming
    $sourcePath = "target\$TargetTriple\release\$BinaryName$Extension"
    $destPath = "$BINARIES_DIR\$BinaryName-$TargetTriple$Extension"

    if (Test-Path $sourcePath) {
        Copy-Item -Path $sourcePath -Destination $destPath -Force
        Write-Info "Copied: $destPath"
    } else {
        Write-Error "Binary not found: $sourcePath"
        return $false
    }

    Write-Host ""
    return $true
}

# Build all binaries for all targets
$successCount = 0
$failureCount = 0
$builtBinaries = @()

foreach ($target in $TARGETS) {
    Write-Host ""
    Write-Host "=====================================" -ForegroundColor Cyan
    Write-Host "  Building for $($target.Name)" -ForegroundColor Cyan
    Write-Host "=====================================" -ForegroundColor Cyan
    Write-Host ""

    # Ensure target is installed
    if (-not (Ensure-Target -TargetTriple $target.Triple)) {
        Write-Warn "Skipping $($target.Name) due to target installation failure"
        $failureCount += 3
        continue
    }

    Write-Host ""

    # Build each binary
    $binaries = @(
        @{ Name = "mcp-stdio"; Features = "mcp-server" },
        @{ Name = "mcp-bridge"; Features = "mcp-server" },
        @{ Name = "mcp-service"; Features = "mcp-server,tauri-plugin" }
    )

    foreach ($binary in $binaries) {
        $result = Build-Binary `
            -BinaryName $binary.Name `
            -Features $binary.Features `
            -TargetTriple $target.Triple `
            -Extension $target.Extension

        if ($result) {
            $successCount++
            $builtBinaries += "$($binary.Name)-$($target.Triple)$($target.Extension)"
        } else {
            $failureCount++
            Write-Warn "Failed to build $($binary.Name) for $($target.Name)"
        }
    }
}

# Summary
Write-Host ""
Write-Host "=====================================" -ForegroundColor Magenta
Write-Host "  Build Summary" -ForegroundColor Magenta
Write-Host "=====================================" -ForegroundColor Magenta
Write-Host ""

if ($failureCount -eq 0) {
    Write-Host "  Status: " -NoNewline
    Write-Host "ALL BUILDS SUCCESSFUL" -ForegroundColor Green
} else {
    Write-Host "  Status: " -NoNewline
    Write-Host "SOME BUILDS FAILED" -ForegroundColor Yellow
}

Write-Host "  Successful: $successCount" -ForegroundColor Green
if ($failureCount -gt 0) {
    Write-Host "  Failed: $failureCount" -ForegroundColor Red
}
Write-Host ""

if ($successCount -gt 0) {
    Write-Info "Binaries are ready in: $BINARIES_DIR"
    Write-Host ""
    Write-Host "Built binaries:" -ForegroundColor Cyan

    Get-ChildItem $BINARIES_DIR | Sort-Object Name | ForEach-Object {
        $size = [math]::Round($_.Length / 1MB, 2)
        Write-Host "  - $($_.Name) " -NoNewline -ForegroundColor White
        Write-Host "($size MB)" -ForegroundColor Gray
    }

    Write-Host ""
    Write-Info "To bundle these binaries with Tauri, ensure tauri.conf.json includes:"
    Write-Host '  "bundle": {' -ForegroundColor Gray
    Write-Host '    "resources": ["binaries/*"],' -ForegroundColor Gray
    Write-Host '    "externalBin": [' -ForegroundColor Gray
    Write-Host '      "binaries/mcp-stdio",' -ForegroundColor Gray
    Write-Host '      "binaries/mcp-bridge",' -ForegroundColor Gray
    Write-Host '      "binaries/mcp-service"' -ForegroundColor Gray
    Write-Host '    ]' -ForegroundColor Gray
    Write-Host '  }' -ForegroundColor Gray
    Write-Host ""
    Write-Info "Tauri will automatically select the correct binary for each platform."
}

Write-Host ""

if ($failureCount -gt 0) {
    Write-Warn "Some builds failed. This may be due to:"
    Write-Host "  - Missing cross-compilation toolchains" -ForegroundColor Gray
    Write-Host "  - Platform-specific dependencies" -ForegroundColor Gray
    Write-Host "  - Target not supported on current host" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Note: Cross-compiling to macOS from Windows/Linux requires special setup." -ForegroundColor Gray
    Write-Host "      Cross-compiling to Windows from macOS/Linux may require mingw-w64." -ForegroundColor Gray
    Write-Host "      Consider using CI/CD to build on native platforms." -ForegroundColor Gray
    Write-Host ""
    exit 1
}

Write-Host "=====================================" -ForegroundColor Green
Write-Host "  All Builds Complete!" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Green
Write-Host ""
