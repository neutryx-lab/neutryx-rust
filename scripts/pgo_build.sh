#!/bin/bash
# Profile-Guided Optimization (PGO) Build Script
#
# This script automates the PGO workflow:
# 1. Build with instrumentation
# 2. Run benchmarks to collect profile data
# 3. Build optimized binary using profile data
#
# Requirements:
# - Rust nightly toolchain
# - cargo-pgo (install with: cargo install cargo-pgo)
#
# Usage:
#   ./scripts/pgo_build.sh [OPTIONS]
#
# Options:
#   --clean       Clean previous PGO data before building
#   --bench-only  Only run profiling benchmarks (skip instrumented build)
#   --optimize    Only run optimization build (skip instrumentation and profiling)
#   --help        Show this help message

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROFILE_DIR="target/pgo-profiles"
CRATES_TO_BUILD="pricer_core pricer_models pricer_risk"

# Parse arguments
CLEAN=false
BENCH_ONLY=false
OPTIMIZE_ONLY=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --clean)
            CLEAN=true
            shift
            ;;
        --bench-only)
            BENCH_ONLY=true
            shift
            ;;
        --optimize)
            OPTIMIZE_ONLY=true
            shift
            ;;
        --help)
            head -25 "$0" | tail -20
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}=== Neutryx PGO Build ===${NC}"
echo ""

# Check prerequisites
echo -n "Checking cargo-pgo... "
if ! command -v cargo-pgo &> /dev/null; then
    echo -e "${RED}NOT FOUND${NC}"
    echo "Install with: cargo install cargo-pgo"
    exit 1
fi
echo -e "${GREEN}OK${NC}"

echo -n "Checking nightly toolchain... "
if ! rustup run nightly rustc --version &> /dev/null; then
    echo -e "${RED}NOT FOUND${NC}"
    echo "Install with: rustup toolchain install nightly"
    exit 1
fi
echo -e "${GREEN}OK${NC}"

# Clean if requested
if [ "$CLEAN" = true ]; then
    echo -e "${YELLOW}Cleaning previous PGO data...${NC}"
    rm -rf "$PROFILE_DIR"
    cargo clean
fi

# Step 1: Instrumented Build
if [ "$OPTIMIZE_ONLY" = false ]; then
    echo ""
    echo -e "${BLUE}=== Step 1: Instrumented Build ===${NC}"
    echo "Building with profiling instrumentation..."

    # Build each stable crate with PGO instrumentation
    for crate in $CRATES_TO_BUILD; do
        echo -e "  Building ${GREEN}$crate${NC}..."
        cargo +nightly pgo build -p "$crate" 2>&1 | tail -5
    done

    echo -e "${GREEN}Instrumented build complete${NC}"
fi

# Step 2: Profile Collection
if [ "$OPTIMIZE_ONLY" = false ]; then
    echo ""
    echo -e "${BLUE}=== Step 2: Profile Collection ===${NC}"
    echo "Running benchmarks to collect profile data..."

    mkdir -p "$PROFILE_DIR"

    # Run benchmarks for each crate
    for crate in $CRATES_TO_BUILD; do
        BENCH_NAME="${crate#pricer_}_benchmarks"
        echo -e "  Profiling ${GREEN}$crate${NC} ($BENCH_NAME)..."

        # Check if benchmark exists
        if cargo +nightly bench -p "$crate" --bench "$BENCH_NAME" --no-run 2>/dev/null; then
            # Run benchmark with profiling
            cargo +nightly pgo run -p "$crate" -- bench --bench "$BENCH_NAME" -- --noplot 2>&1 | tail -3
        else
            echo -e "    ${YELLOW}No benchmark found, skipping${NC}"
        fi
    done

    echo -e "${GREEN}Profile collection complete${NC}"
fi

# Step 3: Optimized Build
if [ "$BENCH_ONLY" = false ]; then
    echo ""
    echo -e "${BLUE}=== Step 3: Optimized Build ===${NC}"
    echo "Building with collected profile data..."

    # Check if profile data exists
    if [ ! -d "$PROFILE_DIR" ] || [ -z "$(ls -A $PROFILE_DIR 2>/dev/null)" ]; then
        echo -e "${YELLOW}Warning: No profile data found. Running without PGO optimization.${NC}"
        echo "Run the full PGO workflow first, or use --bench-only to collect profiles."

        # Fall back to regular release build
        cargo build --release --workspace --exclude pricer_pricing
    else
        # Build with PGO optimization
        for crate in $CRATES_TO_BUILD; do
            echo -e "  Optimizing ${GREEN}$crate${NC}..."
            cargo +nightly pgo optimize -p "$crate" 2>&1 | tail -5
        done
    fi

    echo -e "${GREEN}Optimized build complete${NC}"
fi

# Summary
echo ""
echo -e "${BLUE}=== Summary ===${NC}"
echo "PGO build workflow completed."
echo ""
echo "Profile data location: $PROFILE_DIR"
echo "Optimized binaries: target/release/"
echo ""
echo "To verify optimization, compare benchmark results:"
echo "  cargo bench -p pricer_core --bench core_benchmarks -- --baseline pgo"
