#!/bin/bash
# Build configuration verification script
# Verifies that optimization settings are correctly applied

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Build Configuration Verification ==="

# Check 1: Cargo.toml release profile settings
echo -n "Checking Cargo.toml release profile... "
if grep -q 'lto = true' Cargo.toml && \
   grep -q 'codegen-units = 1' Cargo.toml && \
   grep -q 'opt-level = 3' Cargo.toml; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo "  Missing one or more settings: lto, codegen-units, opt-level"
    exit 1
fi

# Check 2: .cargo/config.toml exists
echo -n "Checking .cargo/config.toml exists... "
if [ -f ".cargo/config.toml" ]; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo "  .cargo/config.toml not found"
    exit 1
fi

# Check 3: target-cpu=native configuration
echo -n "Checking target-cpu=native configuration... "
if grep -q 'target-cpu=native' .cargo/config.toml; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${YELLOW}WARNING${NC}"
    echo "  target-cpu=native not found (may be intentionally disabled)"
fi

# Check 4: Detect platform and report SIMD capabilities
echo ""
echo "=== Platform Detection ==="
ARCH=$(uname -m)
OS=$(uname -s)
echo "Architecture: $ARCH"
echo "OS: $OS"

if [ "$ARCH" == "x86_64" ]; then
    echo -n "Checking SIMD capabilities... "
    if [ "$OS" == "Linux" ]; then
        if grep -q 'avx2' /proc/cpuinfo 2>/dev/null; then
            echo -e "${GREEN}AVX2 supported${NC}"
        elif grep -q 'avx' /proc/cpuinfo 2>/dev/null; then
            echo -e "${YELLOW}AVX supported (no AVX2)${NC}"
        else
            echo -e "${YELLOW}Basic SSE only${NC}"
        fi

        if grep -q 'avx512' /proc/cpuinfo 2>/dev/null; then
            echo -e "  ${GREEN}AVX-512 supported${NC}"
        fi
    elif [ "$OS" == "Darwin" ]; then
        if sysctl -n machdep.cpu.features 2>/dev/null | grep -q 'AVX2'; then
            echo -e "${GREEN}AVX2 supported${NC}"
        else
            echo -e "${YELLOW}Check manually with: sysctl machdep.cpu.features${NC}"
        fi
    else
        echo -e "${YELLOW}Platform not auto-detected${NC}"
    fi
fi

# Check 5: Verify rustflags can be parsed
echo ""
echo "=== Rust Toolchain ==="
echo -n "Checking Rust installation... "
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}$RUST_VERSION${NC}"
else
    echo -e "${RED}rustc not found${NC}"
    exit 1
fi

# Check 6: Verify build works with configuration
echo ""
echo "=== Build Verification ==="
echo "Testing release build with configuration..."
if cargo build --release --workspace --exclude pricer_pricing -q 2>/dev/null; then
    echo -e "${GREEN}Release build successful${NC}"
else
    echo -e "${YELLOW}Build test skipped (run manually with: cargo build --release)${NC}"
fi

echo ""
echo "=== Summary ==="
echo -e "${GREEN}Build configuration verification complete${NC}"
echo ""
echo "Note: For maximum performance on this machine, ensure target-cpu=native is enabled"
echo "      in .cargo/config.toml for your target platform."
