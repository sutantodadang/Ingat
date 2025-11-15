#!/usr/bin/env bash
# Build MCP binaries for multiple platforms with correct naming format for Tauri bundling

set -e

# Colors for output
GREEN='\033[0;32m'
CYAN='\033[0;36m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
GRAY='\033[0;90m'
MAGENTA='\033[0;35m'
WHITE='\033[0;97m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[✓]${NC} $1"
}

step() {
    echo -e "${CYAN}==>${NC} $1"
}

error() {
    echo -e "${RED}[X]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[!]${NC} $1"
}

echo ""
echo -e "${MAGENTA}=====================================${NC}"
echo -e "${MAGENTA}  Ingat MCP Binaries Builder${NC}"
echo -e "${MAGENTA}  Multi-Platform Build Script${NC}"
echo -e "${MAGENTA}=====================================${NC}"
echo ""

# Define target platforms
declare -a TARGETS=(
    "x86_64-pc-windows-msvc:.exe:Windows (x64)"
    "x86_64-unknown-linux-gnu::Linux (x64)"
    "aarch64-apple-darwin::macOS (ARM64)"
)

BINARIES_DIR="binaries"

# Check if cross-compilation toolchains are available
step "Checking installed Rust targets..."
INSTALLED_TARGETS=$(rustup target list --installed)
echo ""

# Create binaries directory
step "Creating binaries directory..."
mkdir -p "$BINARIES_DIR"
info "Directory ready: $BINARIES_DIR"
echo ""

# Function to check and install target
ensure_target() {
    local target_triple="$1"

    if ! echo "$INSTALLED_TARGETS" | grep -q "^${target_triple}$"; then
        warn "Target $target_triple is not installed. Installing..."
        if rustup target add "$target_triple" 2>/dev/null; then
            info "Target installed: $target_triple"
            return 0
        else
            error "Failed to install target: $target_triple"
            return 1
        fi
    else
        info "Target already installed: $target_triple"
        return 0
    fi
}

# Function to build and copy binary for a specific target
build_binary() {
    local binary_name="$1"
    local features="$2"
    local target_triple="$3"
    local extension="$4"

    step "Building $binary_name for $target_triple..."
    echo -e "   ${GRAY}Features: $features${NC}"

    if cargo build --release --bin "$binary_name" --features "$features" --target "$target_triple" 2>&1; then
        info "Build successful: $binary_name ($target_triple)"
    else
        error "Build failed for $binary_name ($target_triple)"
        return 1
    fi

    # Copy to binaries folder with correct naming
    local source_path="target/${target_triple}/release/${binary_name}${extension}"
    local dest_path="${BINARIES_DIR}/${binary_name}-${target_triple}${extension}"

    if [ -f "$source_path" ]; then
        cp "$source_path" "$dest_path"
        chmod +x "$dest_path" 2>/dev/null || true
        info "Copied: $dest_path"
    else
        error "Binary not found: $source_path"
        return 1
    fi

    echo ""
    return 0
}

# Build all binaries for all targets
SUCCESS_COUNT=0
FAILURE_COUNT=0
declare -a BUILT_BINARIES=()

for target_info in "${TARGETS[@]}"; do
    IFS=':' read -r target_triple extension platform_name <<< "$target_info"

    echo ""
    echo -e "${CYAN}=====================================${NC}"
    echo -e "${CYAN}  Building for $platform_name${NC}"
    echo -e "${CYAN}=====================================${NC}"
    echo ""

    # Ensure target is installed
    if ! ensure_target "$target_triple"; then
        warn "Skipping $platform_name due to target installation failure"
        FAILURE_COUNT=$((FAILURE_COUNT + 3))
        continue
    fi

    echo ""

    # Build each binary
    declare -a BINARIES=(
        "mcp-stdio:mcp-server"
        "mcp-bridge:mcp-server"
        "mcp-service:mcp-server,tauri-plugin"
    )

    for binary_info in "${BINARIES[@]}"; do
        IFS=':' read -r binary_name features <<< "$binary_info"

        if build_binary "$binary_name" "$features" "$target_triple" "$extension"; then
            SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
            BUILT_BINARIES+=("${binary_name}-${target_triple}${extension}")
        else
            FAILURE_COUNT=$((FAILURE_COUNT + 1))
            warn "Failed to build $binary_name for $platform_name"
        fi
    done
done

# Summary
echo ""
echo -e "${MAGENTA}=====================================${NC}"
echo -e "${MAGENTA}  Build Summary${NC}"
echo -e "${MAGENTA}=====================================${NC}"
echo ""

if [ $FAILURE_COUNT -eq 0 ]; then
    echo -e "  Status: ${GREEN}ALL BUILDS SUCCESSFUL${NC}"
else
    echo -e "  Status: ${YELLOW}SOME BUILDS FAILED${NC}"
fi

echo -e "  Successful: ${GREEN}$SUCCESS_COUNT${NC}"
if [ $FAILURE_COUNT -gt 0 ]; then
    echo -e "  Failed: ${RED}$FAILURE_COUNT${NC}"
fi
echo ""

if [ $SUCCESS_COUNT -gt 0 ]; then
    info "Binaries are ready in: $BINARIES_DIR"
    echo ""
    echo -e "${CYAN}Built binaries:${NC}"

    for file in "$BINARIES_DIR"/*; do
        if [ -f "$file" ]; then
            if command -v du &> /dev/null; then
                size=$(du -h "$file" | cut -f1)
            else
                size=$(ls -lh "$file" | awk '{print $5}')
            fi
            echo -e "  ${WHITE}- $(basename "$file")${NC} ${GRAY}($size)${NC}"
        fi
    done

    echo ""
    info "To bundle these binaries with Tauri, ensure tauri.conf.json includes:"
    echo -e "${GRAY}  \"bundle\": {${NC}"
    echo -e "${GRAY}    \"resources\": [\"binaries/*\"],${NC}"
    echo -e "${GRAY}    \"externalBin\": [${NC}"
    echo -e "${GRAY}      \"binaries/mcp-stdio\",${NC}"
    echo -e "${GRAY}      \"binaries/mcp-bridge\",${NC}"
    echo -e "${GRAY}      \"binaries/mcp-service\"${NC}"
    echo -e "${GRAY}    ]${NC}"
    echo -e "${GRAY}  }${NC}"
    echo ""
    info "Tauri will automatically select the correct binary for each platform."
fi

echo ""

if [ $FAILURE_COUNT -gt 0 ]; then
    warn "Some builds failed. This may be due to:"
    echo -e "  ${GRAY}- Missing cross-compilation toolchains${NC}"
    echo -e "  ${GRAY}- Platform-specific dependencies${NC}"
    echo -e "  ${GRAY}- Target not supported on current host${NC}"
    echo ""
    echo -e "${GRAY}Note: Cross-compiling to macOS from Windows/Linux requires special setup.${NC}"
    echo -e "${GRAY}      Cross-compiling to Windows from macOS/Linux may require mingw-w64.${NC}"
    echo -e "${GRAY}      Consider using CI/CD to build on native platforms.${NC}"
    echo ""
    exit 1
fi

echo -e "${GREEN}=====================================${NC}"
echo -e "${GREEN}  All Builds Complete!${NC}"
echo -e "${GREEN}=====================================${NC}"
echo ""
