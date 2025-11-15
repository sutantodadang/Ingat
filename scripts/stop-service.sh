#!/bin/bash
# Stop the persistent Ingat MCP service

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SERVICE_PORT="${INGAT_SERVICE_PORT:-3200}"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Stop Ingat MCP Service${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Function to find service process
find_service_pid() {
    if pgrep -f "mcp-service" > /dev/null 2>&1; then
        pgrep -f "mcp-service"
        return 0
    fi
    return 1
}

# Function to check if port is in use
check_port() {
    if command -v lsof &> /dev/null; then
        lsof -Pi :${SERVICE_PORT} -sTCP:LISTEN -t 2>/dev/null
    elif command -v netstat &> /dev/null; then
        netstat -tuln 2>/dev/null | grep ":${SERVICE_PORT}" | awk '{print $7}' | cut -d'/' -f1
    elif command -v ss &> /dev/null; then
        ss -tuln 2>/dev/null | grep ":${SERVICE_PORT}" | grep -o 'pid=[0-9]*' | cut -d'=' -f2
    fi
}

# Check if service is running
echo -e "${BLUE}Checking for running service...${NC}"

PID=$(find_service_pid)
if [ -z "$PID" ]; then
    # Try to find by port
    PID=$(check_port)
fi

if [ -z "$PID" ]; then
    echo -e "${YELLOW}No mcp-service process found${NC}"
    echo ""
    echo "The service is not running, or it's running under a different name."
    echo ""
    echo -e "${BLUE}To check manually:${NC}"
    echo "  ps aux | grep mcp-service"
    echo "  lsof -i :${SERVICE_PORT}"
    exit 0
fi

echo -e "${GREEN}Found mcp-service process(es): ${PID}${NC}"
echo ""

# Confirm before stopping
read -p "Stop the service? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 0
fi

# Stop each process
for pid in $PID; do
    echo -e "${BLUE}Stopping process ${pid}...${NC}"

    # Try graceful shutdown first (SIGTERM)
    if kill -TERM $pid 2>/dev/null; then
        echo "Sent SIGTERM to process ${pid}"

        # Wait up to 5 seconds for graceful shutdown
        for i in {1..5}; do
            if ! kill -0 $pid 2>/dev/null; then
                echo -e "${GREEN}✓ Process ${pid} stopped gracefully${NC}"
                break
            fi
            sleep 1
        done

        # Force kill if still running
        if kill -0 $pid 2>/dev/null; then
            echo -e "${YELLOW}Process still running, forcing shutdown...${NC}"
            kill -9 $pid 2>/dev/null || true
            sleep 1
            if ! kill -0 $pid 2>/dev/null; then
                echo -e "${GREEN}✓ Process ${pid} force stopped${NC}"
            else
                echo -e "${RED}✗ Failed to stop process ${pid}${NC}"
            fi
        fi
    else
        echo -e "${RED}✗ Failed to send signal to process ${pid}${NC}"
        echo "You may need to run this script with sudo"
    fi
done

echo ""

# Verify service is stopped
sleep 1
if pgrep -f "mcp-service" > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠ Service may still be running${NC}"
    echo "Check with: ps aux | grep mcp-service"
else
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}✓ Service stopped successfully!${NC}"
    echo -e "${GREEN}========================================${NC}"
fi

echo ""
echo -e "${BLUE}To start the service again:${NC}"
echo "  ./scripts/start-service.sh"
echo "  or: ./target/release/mcp-service"
echo ""

exit 0
