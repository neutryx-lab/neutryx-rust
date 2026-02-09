#!/bin/bash
# =============================================================================
# Docker Local Test Script for Cloud Run Compatibility
# =============================================================================
# This script builds and runs the demo-web Docker container locally,
# simulating the Cloud Run environment to catch deployment issues early.
#
# Usage:
#   ./scripts/docker_local_test.sh [options]
#
# Options:
#   --build-only    Only build the image, don't run
#   --no-cache      Build without Docker cache
#   --port PORT     Use custom port (default: 8080)
#   --debug         Enable debug mode (FB_DEBUG_MODE=true)
#   --help          Show this help message
# =============================================================================

set -e

# Configuration
IMAGE_NAME="frictional-bank-local"
DEFAULT_PORT=8080
BUILD_ONLY=false
NO_CACHE=""
DEBUG_MODE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --build-only)
            BUILD_ONLY=true
            shift
            ;;
        --no-cache)
            NO_CACHE="--no-cache"
            shift
            ;;
        --port)
            DEFAULT_PORT="$2"
            shift 2
            ;;
        --debug)
            DEBUG_MODE=true
            shift
            ;;
        --help)
            head -n 20 "$0" | tail -n +2 | sed 's/^# //'
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Colours for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=============================================${NC}"
echo -e "${BLUE}  Docker Local Test for Cloud Run${NC}"
echo -e "${BLUE}=============================================${NC}"
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}Error: Docker is not running${NC}"
    exit 1
fi

# Navigate to project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

echo -e "${YELLOW}Building Docker image...${NC}"
echo -e "  Image name: ${IMAGE_NAME}"
echo -e "  Dockerfile: docker/Dockerfile.gui"
echo ""

# Build the image
if docker build -f docker/Dockerfile.gui -t "$IMAGE_NAME" $NO_CACHE . ; then
    echo ""
    echo -e "${GREEN}Build successful!${NC}"
else
    echo ""
    echo -e "${RED}Build failed!${NC}"
    echo ""
    echo -e "${YELLOW}Common issues:${NC}"
    echo "  - Missing static files (check demo/gui/static/)"
    echo "  - Missing data files (check demo/data/input/)"
    echo "  - Case sensitivity: Linux is case-sensitive"
    exit 1
fi

if [ "$BUILD_ONLY" = true ]; then
    echo ""
    echo -e "${GREEN}Build-only mode: Image ready for deployment${NC}"
    exit 0
fi

echo ""
echo -e "${YELLOW}Starting container...${NC}"
echo -e "  Port: ${DEFAULT_PORT}"
echo -e "  Debug mode: ${DEBUG_MODE}"
echo ""

# Build environment variables
ENV_VARS="-e PORT=${DEFAULT_PORT}"
ENV_VARS="$ENV_VARS -e RUST_LOG=info"
ENV_VARS="$ENV_VARS -e FB_OPEN_BROWSER=false"

if [ "$DEBUG_MODE" = true ]; then
    ENV_VARS="$ENV_VARS -e FB_DEBUG_MODE=true"
    ENV_VARS="$ENV_VARS -e FB_LOG_LEVEL=DEBUG"
    ENV_VARS="$ENV_VARS -e RUST_LOG=debug"
fi

# Run the container (simulating Cloud Run environment)
echo -e "${BLUE}=============================================${NC}"
echo -e "${GREEN}Container starting...${NC}"
echo ""
echo -e "  Access the dashboard at: ${GREEN}http://localhost:${DEFAULT_PORT}${NC}"
echo ""
echo -e "${YELLOW}Testing checklist:${NC}"
echo "  [ ] Dashboard loads correctly"
echo "  [ ] All charts render"
echo "  [ ] WebSocket connection establishes"
echo "  [ ] Portfolio data loads"
echo "  [ ] API endpoints respond"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop the container${NC}"
echo -e "${BLUE}=============================================${NC}"
echo ""

# Run interactively so user can see logs
docker run --rm -it \
    -p "${DEFAULT_PORT}:${DEFAULT_PORT}" \
    $ENV_VARS \
    "$IMAGE_NAME"
