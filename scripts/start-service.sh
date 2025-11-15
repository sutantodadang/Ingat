#!/bin/bash
# Ingat Unified Backend Service Startup Script
# This script builds and starts the mcp-service for multi-client usage

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
SERVICE_PORT="${INGAT_SERVICE_PORT:-3200}"
SERVICE_HOST="${INGAT_SERVICE_HOST:-127.0.0.1}"
LOG_LEVEL="${INGAT_LOG:-info}"
DATA_DIR="${INGAT_DATA_DIR:-}"

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TAURI_DIR="$PROJECT_ROOT/src-tauri"
BINARY_PATH="$TAURI_DIR/target/release/mcp-service"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Ingat Unified Backend Service${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Function to check if service is already running
check_existing_service() {
    if command -v lsof &> /dev/null; then
        if lsof -Pi :$SERVICE_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
            echo -e "${YELLOW}Warning: Port $SERVICE_PORT is already in use!${NC}"
            echo -e "Another service might be running. Stop it first or change the port:"
            echo -e "  export INGAT_SERVICE_PORT=3201"
            exit 1
        fi
    fi
}

# Function to build the service
build_service() {
    echo -e "${BLUE}Building mcp-service...${NC}"
    cd "$TAURI_DIR"

    if cargo build --release --bin mcp-service --features mcp-server,tauri-plugin; then
        echo -e "${GREEN}✓ Build successful${NC}"
        echo ""
    else
        echo -e "${RED}✗ Build failed${NC}"
        exit 1
    fi
}

# Function to start the service
start_service() {
    echo -e "${BLUE}Starting service...${NC}"
    echo -e "Configuration:"
    echo -e "  Host:      ${GREEN}$SERVICE_HOST${NC}"
    echo -e "  Port:      ${GREEN}$SERVICE_PORT${NC}"
    echo -e "  Log Level: ${GREEN}$LOG_LEVEL${NC}"
    if [ -n "$DATA_DIR" ]; then
        echo -e "  Data Dir:  ${GREEN}$DATA_DIR${NC}"
    fi
    echo ""

    # Set environment variables
    export INGAT_SERVICE_HOST="$SERVICE_HOST"
    export INGAT_SERVICE_PORT="$SERVICE_PORT"
    export INGAT_LOG="$LOG_LEVEL"

    if [ -n "$DATA_DIR" ]; then
        export INGAT_DATA_DIR="$DATA_DIR"
    fi

    # Start the service
    echo -e "${GREEN}Service starting...${NC}"
    echo -e "Press ${YELLOW}Ctrl+C${NC} to stop"
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo ""

    "$BINARY_PATH"
}

# Main execution
main() {
    # Parse command line arguments
    BUILD=false
    FORCE_BUILD=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --build|-b)
                BUILD=true
                shift
                ;;
            --rebuild|-r)
                FORCE_BUILD=true
                BUILD=true
                shift
                ;;
            --port|-p)
                SERVICE_PORT="$2"
                shift 2
                ;;
            --host|-h)
                SERVICE_HOST="$2"
                shift 2
                ;;
            --log|-l)
                LOG_LEVEL="$2"
                shift 2
                ;;
            --data-dir|-d)
                DATA_DIR="$2"
                shift 2
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --build, -b           Build before starting"
                echo "  --rebuild, -r         Force rebuild (clean build)"
                echo "  --port, -p PORT       Set service port (default: 3200)"
                echo "  --host, -h HOST       Set bind address (default: 127.0.0.1)"
                echo "  --log, -l LEVEL       Set log level (default: info)"
                echo "  --data-dir, -d DIR    Set data directory"
                echo "  --help                Show this help"
                echo ""
                echo "Environment Variables:"
                echo "  INGAT_SERVICE_PORT    Service port"
                echo "  INGAT_SERVICE_HOST    Bind address"
                echo "  INGAT_LOG             Log level"
                echo "  INGAT_DATA_DIR        Data directory"
                echo ""
                echo "Examples:"
                echo "  $0                       # Start with defaults"
                echo "  $0 --build               # Build then start"
                echo "  $0 --port 3201           # Use custom port"
                echo "  $0 --log debug           # Enable debug logging"
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done

    # Check if binary exists
    if [ ! -f "$BINARY_PATH" ]; then
        echo -e "${YELLOW}Binary not found. Building...${NC}"
        BUILD=true
    fi

    # Build if requested or necessary
    if [ "$FORCE_BUILD" = true ]; then
        echo -e "${YELLOW}Forcing clean build...${NC}"
        cd "$TAURI_DIR"
        cargo clean --release --package ingat --bin mcp-service 2>/dev/null || true
        build_service
    elif [ "$BUILD" = true ]; then
        build_service
    fi

    # Check for existing service
    check_existing_service

    # Start the service
    start_service
}

# Run main function
main "$@"
