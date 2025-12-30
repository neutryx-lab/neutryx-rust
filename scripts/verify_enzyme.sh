#!/usr/bin/env bash
#
# Verify Rust autodiff (Enzyme) integration
#
# This script:
# 1. Checks Rust nightly toolchain
# 2. Creates a simple test Rust function
# 3. Compiles with autodiff enabled (-Z autodiff=Enable)
# 4. Runs basic differentiation test
#
# Note: Modern Rust nightly has built-in Enzyme support via #![feature(autodiff)]
# The Enzyme backend component must be installed via rustup (this script handles it).

set -euo pipefail

echo "===================================="
echo "Rust Autodiff Verification Script"
echo "===================================="
echo ""

# Check Rust nightly
echo "[1/4] Checking Rust toolchain..."
if ! rustc +nightly --version &> /dev/null; then
    echo "✗ Nightly Rust not found"
    echo "Install with: rustup toolchain install nightly"
    exit 1
fi
echo "✓ Nightly Rust: $(rustc +nightly --version)"

# Check if autodiff feature is available
echo "[2/4] Checking autodiff support..."
if rustc +nightly -Z help 2>&1 | grep -q "autodiff"; then
    echo "✓ autodiff feature available"
else
    echo "⚠ autodiff may not be available in this nightly version"
    echo "  This is expected - autodiff is still experimental"
fi

# Check and install Enzyme/autodiff backend component
echo "[3/4] Checking Enzyme backend component..."
# The autodiff backend requires the llvm-tools component and rust-src
# Some nightlies may also require rustc-codegen-llvm-enzyme (when available)
COMPONENTS_NEEDED=""

if ! rustup +nightly component list --installed 2>/dev/null | grep -q "llvm-tools"; then
    COMPONENTS_NEEDED="$COMPONENTS_NEEDED llvm-tools"
fi

if ! rustup +nightly component list --installed 2>/dev/null | grep -q "rust-src"; then
    COMPONENTS_NEEDED="$COMPONENTS_NEEDED rust-src"
fi

if [ -n "$COMPONENTS_NEEDED" ]; then
    echo "Installing required components:$COMPONENTS_NEEDED"
    rustup +nightly component add $COMPONENTS_NEEDED || {
        echo "⚠ Failed to install some components, continuing anyway..."
    }
fi

# Check if enzyme-specific component exists (may not be available in all nightlies)
if rustup +nightly component list 2>/dev/null | grep -q "rustc-codegen-llvm-enzyme"; then
    if ! rustup +nightly component list --installed 2>/dev/null | grep -q "rustc-codegen-llvm-enzyme"; then
        echo "Installing rustc-codegen-llvm-enzyme component..."
        rustup +nightly component add rustc-codegen-llvm-enzyme || {
            echo "⚠ Failed to install enzyme codegen component"
            echo "  This component may not be available for your platform"
        }
    fi
    echo "✓ Enzyme codegen component available"
else
    echo "⚠ rustc-codegen-llvm-enzyme component not available in this nightly"
    echo "  Autodiff may still work with built-in LLVM Enzyme support"
fi

echo "✓ Required components checked"

# Create test project
echo "[4/4] Creating and testing autodiff project..."
TEST_DIR=$(mktemp -d)
cd $TEST_DIR

cat > Cargo.toml <<'EOF'
[package]
name = "autodiff_test"
version = "0.1.0"
edition = "2024"

[dependencies]
EOF

mkdir src
cat > src/lib.rs <<'EOF'
#![feature(autodiff)]
use std::autodiff::autodiff;

// Simple test: f(x) = x²
// Expected: f'(x) = 2x
#[autodiff(d_square, Reverse, Duplicated, Active)]
pub fn square(x: f64) -> f64 {
    x * x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autodiff_derivative() {
        let x = 3.0;
        let mut dx = 1.0;

        let result = d_square(x, &mut dx);

        // f(3) = 9
        assert!((result - 9.0).abs() < 1e-10);

        // f'(3) = 6
        assert!((dx - 6.0).abs() < 1e-10);
    }
}
EOF

# Build and test with autodiff enabled
# Note: -Z autodiff=Enable activates Rust's built-in Enzyme support
export RUSTFLAGS="-Z autodiff=Enable"

if cargo +nightly test 2>&1 | tee build.log; then
    echo ""
    echo "✓ Autodiff verification PASSED"
    echo ""
    echo "===================================="
    echo "Verification Complete!"
    echo "===================================="
    echo ""
    echo "Rust autodiff (Enzyme) is working correctly."
    echo ""
    # Cleanup
    cd /
    rm -rf $TEST_DIR
else
    echo ""
    echo "✗ Autodiff verification FAILED"
    echo ""
    echo "Build log saved to: $TEST_DIR/build.log"
    echo "Common issues:"
    echo "  - Enzyme backend not installed (run: rustup +nightly component add rustc-codegen-llvm-enzyme)"
    echo "  - autodiff feature not available in this nightly"
    echo "  - Missing #![feature(autodiff)] in source"
    echo "  - Rust nightly version too old (autodiff merged ~2024)"
    echo ""
    echo "Note: autodiff/Enzyme is experimental and requires:"
    echo "  1. A recent nightly toolchain with Enzyme support"
    echo "  2. The rustc-codegen-llvm-enzyme component (if available for your platform)"
    echo "  3. Platform support (primarily Linux x86_64)"
    echo ""
    exit 1
fi
