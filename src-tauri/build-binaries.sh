#!/usr/bin/env bash
# Build MCP binaries with correct naming format for Tauri bundling

set -e

# Colors for output
GREEN='\033[0;32m'
CYAN='\033[0;36m'
RED='\033[0;31m'
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

echo ""
echo -e "${MAGENTA}=====================================${NC}"
echo -e "${MAGENTA}  Ingat MCP Binaries Builder${NC}"
echo -e "${MAGENTA}=====================================${NC}"
echo ""

# Detect target triple
if [ -n "$CARGO_BUILD_TARGET" ]; then
    TARGET="$CARGO_BUILD_TARGET"
else
    # Auto-detect based on platform
    case "$(uname -s)" in
        Darwin)
            if [ "$(uname -m)" = "arm64" ]; then
                TARGET="aarch64-apple-darwin"
            else
                TARGET="x86_64-apple-darwin"
            fi
            ;;
        Linux)
            TARGET="x86_64-unknown-linux-gnu"
            ;;
        *)
            error "Unsupported platform: $(uname -s)"
            exit 1
            ;;
    esac
fi

EXTENSION=""
if [[ "$TARGET" == *"windows"* ]]; then
    EXTENSION=".exe"
fi

BINARIES_DIR="binaries"

info "Target: $TARGET"
info "Extension: $EXTENSION"
echo ""

# Create binaries directory
step "Creating binaries directory..."
mkdir -p "$BINARIES_DIR"
info "Directory ready: $BINARIES_DIR"
echo ""

# Function to build and copy binary
build_binary() {
    local binary_name="$1"
    local features="$2"

    step "Building $binary_name..."
    echo -e "   ${GRAY}Features: $features${NC}"

    if cargo build --release --bin "$binary_name" --features "$features"; then
        info "Build successful: $binary_name"
    else
        error "Build failed for $binary_name"
        exit 1
    fi

    # Copy to binaries folder with correct naming
    local source_path="target/release/${binary_name}${EXTENSION}"
    local dest_path="${BINARIES_DIR}/${binary_name}-${TARGET}${EXTENSION}"

    if [ -f "$source_path" ]; then
        cp "$source_path" "$dest_path"
        chmod +x "$dest_path"
        info "Copied: $dest_path"
    else
        error "Binary not found: $source_path"
        exit 1
    fi

    echo ""
}

# Build all binaries
build_binary "mcp-stdio" "mcp-server"
build_binary "mcp-bridge" "mcp-server"
build_binary "mcp-service" "mcp-server,tauri-plugin"

echo ""
echo -e "${GREEN}=====================================${NC}"
echo -e "${GREEN}  Build Complete!${NC}"
echo -e "${GREEN}=====================================${NC}"
echo ""

info "Binaries are ready in: $BINARIES_DIR"
echo ""
echo -e "${CYAN}Built binaries:${NC}"
for file in "$BINARIES_DIR"/*; do
    if [ -f "$file" ]; then
        size=$(du -h "$file" | cut -f1)
        echo -e "  ${WHITE}- $(basename "$file") ($size)${NC}"
    fi
done

echo ""
info "To bundle these binaries, update tauri.conf.json:"
echo -e "${GRAY}  \"bundle\": {${NC}"
echo -e "${GRAY}    \"resources\": [\"binaries/*\"],${NC}"
echo -e "${GRAY}    \"externalBin\": [${NC}"
echo -e "${GRAY}      \"binaries/mcp-stdio\",${NC}"
echo -e "${GRAY}      \"binaries/mcp-bridge\",${NC}"
echo -e "${GRAY}      \"binaries/mcp-service\"${NC}"
echo -e "${GRAY}    ]${NC}"
echo -e "${GRAY}  }${NC}"
echo ""
