#!/bin/bash
# Ingat Service Status Checker
# Checks if the mcp-service is running and healthy

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SERVICE_PORT="${INGAT_SERVICE_PORT:-3200}"
SERVICE_HOST="${INGAT_SERVICE_HOST:-127.0.0.1}"
SERVICE_URL="http://${SERVICE_HOST}:${SERVICE_PORT}"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Ingat Service Status Checker${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Function to print status
print_status() {
    local status=$1
    local message=$2

    if [ "$status" = "ok" ]; then
        echo -e "${GREEN}✓${NC} $message"
    elif [ "$status" = "error" ]; then
        echo -e "${RED}✗${NC} $message"
    elif [ "$status" = "warning" ]; then
        echo -e "${YELLOW}⚠${NC} $message"
    else
        echo -e "${BLUE}ℹ${NC} $message"
    fi
}

# Check 1: Process running
echo -e "${BLUE}Checking process...${NC}"
if pgrep -f "mcp-service" > /dev/null 2>&1; then
    PID=$(pgrep -f "mcp-service")
    print_status "ok" "Process is running (PID: $PID)"
else
    print_status "error" "Process is NOT running"
    echo ""
    echo -e "${YELLOW}To start the service:${NC}"
    echo -e "  ./scripts/start-service.sh"
    echo -e "  or manually: ./target/release/mcp-service"
    exit 1
fi

# Check 2: Port listening
echo ""
echo -e "${BLUE}Checking port ${SERVICE_PORT}...${NC}"
if command -v lsof &> /dev/null; then
    if lsof -Pi :${SERVICE_PORT} -sTCP:LISTEN -t >/dev/null 2>&1; then
        print_status "ok" "Port ${SERVICE_PORT} is listening"
    else
        print_status "error" "Port ${SERVICE_PORT} is NOT listening"
        exit 1
    fi
elif command -v netstat &> /dev/null; then
    if netstat -tuln | grep ":${SERVICE_PORT}" > /dev/null 2>&1; then
        print_status "ok" "Port ${SERVICE_PORT} is listening"
    else
        print_status "error" "Port ${SERVICE_PORT} is NOT listening"
        exit 1
    fi
elif command -v ss &> /dev/null; then
    if ss -tuln | grep ":${SERVICE_PORT}" > /dev/null 2>&1; then
        print_status "ok" "Port ${SERVICE_PORT} is listening"
    else
        print_status "error" "Port ${SERVICE_PORT} is NOT listening"
        exit 1
    fi
else
    print_status "warning" "Cannot check port (lsof/netstat/ss not available)"
fi

# Check 3: Health endpoint
echo ""
echo -e "${BLUE}Checking health endpoint...${NC}"
if command -v curl &> /dev/null; then
    HEALTH_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "${SERVICE_URL}/health" 2>/dev/null || echo "000")

    if [ "$HEALTH_RESPONSE" = "200" ]; then
        print_status "ok" "Health endpoint responding (HTTP 200)"

        # Get the actual health data
        HEALTH_DATA=$(curl -s "${SERVICE_URL}/health" 2>/dev/null)
        if [ -n "$HEALTH_DATA" ]; then
            echo -e "${BLUE}Response:${NC} $HEALTH_DATA"
        fi
    else
        print_status "error" "Health endpoint not responding (HTTP $HEALTH_RESPONSE)"
        exit 1
    fi
else
    print_status "warning" "Cannot check health (curl not available)"
fi

# Check 4: Service statistics
echo ""
echo -e "${BLUE}Fetching service statistics...${NC}"
if command -v curl &> /dev/null; then
    STATS_RESPONSE=$(curl -s "${SERVICE_URL}/api/stats" 2>/dev/null)

    if [ -n "$STATS_RESPONSE" ]; then
        print_status "ok" "Statistics endpoint responding"

        # Try to parse JSON if jq is available
        if command -v jq &> /dev/null; then
            echo ""
            echo -e "${BLUE}Service Details:${NC}"
            echo "$STATS_RESPONSE" | jq -r '
                "  Version:        " + .version,
                "  Total Contexts: " + (.total_contexts | tostring),
                "  Data Directory: " + .data_dir
            '
        else
            echo -e "${BLUE}Raw stats:${NC} $STATS_RESPONSE"
            echo -e "${YELLOW}Tip: Install jq for formatted output${NC}"
        fi
    else
        print_status "warning" "Could not fetch statistics"
    fi
fi

# Check 5: Recent contexts
echo ""
echo -e "${BLUE}Checking contexts endpoint...${NC}"
if command -v curl &> /dev/null; then
    CONTEXTS_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "${SERVICE_URL}/api/contexts?limit=1" 2>/dev/null || echo "000")

    if [ "$CONTEXTS_RESPONSE" = "200" ]; then
        print_status "ok" "Contexts endpoint responding (HTTP 200)"
    else
        print_status "warning" "Contexts endpoint returned HTTP $CONTEXTS_RESPONSE"
    fi
fi

# Summary
echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}✓ Service is running and healthy!${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${BLUE}Service URL:${NC} ${SERVICE_URL}"
echo -e "${BLUE}Health:${NC}      ${SERVICE_URL}/health"
echo -e "${BLUE}Stats:${NC}       ${SERVICE_URL}/api/stats"
echo -e "${BLUE}Contexts:${NC}    ${SERVICE_URL}/api/contexts"
echo -e "${BLUE}Search:${NC}      ${SERVICE_URL}/api/search"
echo ""
echo -e "${BLUE}To stop the service:${NC}"
echo -e "  kill $PID"
echo -e "  or press Ctrl+C in the service terminal"
echo ""

exit 0
