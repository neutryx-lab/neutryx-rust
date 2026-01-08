#!/bin/bash
# PGO Profile Collection Workload
#
# Runs representative workloads for PGO profile collection.
# This script defines the benchmark workloads that should be used
# to generate profile data for PGO optimization.
#
# The workloads are designed to exercise the hot paths:
# - Monte Carlo simulation
# - Greeks calculation (bump-and-revalue)
# - Portfolio XVA computation
# - Interpolation (yield curves, volatility surfaces)
#
# Usage:
#   ./scripts/pgo_profile.sh

set -euo pipefail

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== PGO Profile Collection Workload ===${NC}"
echo ""

# Ensure we're in the project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# Run pricer_core benchmarks (math operations, interpolation)
echo -e "${GREEN}Running pricer_core benchmarks...${NC}"
cargo +nightly bench -p pricer_core --bench core_benchmarks -- --noplot 2>&1 | grep -E "(bench_|time:)" | head -20

# Run pricer_models benchmarks (instrument pricing, stochastic models)
echo ""
echo -e "${GREEN}Running pricer_models benchmarks...${NC}"
cargo +nightly bench -p pricer_models --bench models_benchmarks -- --noplot 2>&1 | grep -E "(bench_|time:)" | head -20

# Run pricer_risk benchmarks (portfolio, XVA calculations)
echo ""
echo -e "${GREEN}Running pricer_risk benchmarks...${NC}"
cargo +nightly bench -p pricer_risk --bench xva_benchmarks -- --noplot 2>&1 | grep -E "(bench_|time:)" | head -20

echo ""
echo -e "${BLUE}=== Profile Collection Complete ===${NC}"
echo ""
echo "Profile data has been collected from representative workloads."
echo "Use this data with 'cargo +nightly pgo optimize' to build optimized binaries."
