# Build Optimisation

[Purpose: configure Cargo profiles, reduce binary size, optimise compile times]

## Philosophy

- **Release builds**: Maximum runtime performance, acceptable compile time
- **Dev builds**: Balance between compile speed and runtime performance
- **Test builds**: Fast execution, reasonable compile time
- **Binary size**: Strip symbols, minimise features

## Cargo Profiles

### Release Profile

```toml
[profile.release]
lto = true              # Link-time optimisation (cross-module inlining)
codegen-units = 1       # Single codegen unit (max optimisation)
opt-level = 3           # Maximum optimisation
strip = "symbols"       # Remove debug symbols (30-50% size reduction)
```

**Trade-offs**:
- Compile time: ~3-5x slower than debug
- Binary size: Minimal with strip
- Runtime: Maximum performance

### Development Profile

```toml
[profile.dev]
opt-level = 1           # Light optimisation (faster runtime)

[profile.dev.package."*"]
opt-level = 2           # Optimise dependencies more (they change less)
```

**Rationale**:
- `opt-level = 1`: ~50% faster runtime vs `opt-level = 0`
- Dependencies at `opt-level = 2`: One-time cost, benefits every build

### Test Profile

```toml
[profile.test]
opt-level = 1           # Faster test execution
```

**Rationale**:
- Tests run faster with light optimisation
- Especially beneficial for numerical/mathematical tests

### Bench Profile

```toml
[profile.bench]
inherits = "release"    # Same as release for accurate benchmarks
```

### PGO Profiles (Advanced)

```toml
[profile.pgo-generate]
inherits = "release"

[profile.pgo-use]
inherits = "release"
```

Usage with `cargo-pgo`:
```bash
cargo +nightly pgo build
cargo +nightly pgo optimize
```

## Target-Specific Optimisation

### .cargo/config.toml

```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-cpu=native"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "target-cpu=native"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "target-cpu=native"]
```

**Warning**: `target-cpu=native` produces non-portable binaries. Use only for local development and benchmarking.

## Binary Size Reduction

### Strategies (Cumulative)

| Strategy | Size Reduction | Profile Setting |
|----------|---------------|-----------------|
| Strip symbols | 30-50% | `strip = "symbols"` |
| LTO | 10-20% | `lto = true` |
| Single codegen unit | 5-10% | `codegen-units = 1` |
| Feature minimisation | varies | Disable unused features |

### Panic Handling (Optional)

```toml
[profile.release]
panic = "abort"         # Removes unwinding code (~10% smaller)
```

**Trade-off**: No stack unwinding on panic, less graceful error handling.

## Compile Time Optimisation

### Workspace-Level

1. **Avoid `features = ["full"]`**: Specify needed features explicitly
2. **Use workspace dependencies**: Single version resolution
3. **Minimise proc-macros**: Each adds compile time overhead

### Per-Crate

1. **Optional heavy deps**: Gate behind features
2. **Avoid deep generic nesting**: Causes monomorphisation bloat
3. **Use `#[inline]` sparingly**: Compiler usually knows better

### Incremental Compilation

```toml
[profile.dev]
incremental = true      # Default, but explicit
```

For CI (disable for reproducibility):
```bash
CARGO_INCREMENTAL=0 cargo build
```

## Monitoring Commands

```bash
# Build timing analysis
cargo build --timings

# Binary size breakdown (requires cargo-bloat)
cargo bloat --release --crates

# Dependency compilation time
cargo build --timings --release 2>&1 | grep "Compiling"

# Feature impact
cargo build --features "all" --timings
cargo build --features "equity" --timings
```

## CI/CD Considerations

### Reproducible Builds

```bash
CARGO_INCREMENTAL=0 cargo build --release
```

### Cache Optimisation

```yaml
# GitHub Actions example
- uses: Swatinem/rust-cache@v2
  with:
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

### Conditional Feature Testing

```bash
# Test minimal features
cargo test --no-default-features

# Test all features
cargo test --all-features

# Test specific combinations
cargo test --features "rates,credit"
```

## Enzyme-Specific Builds

```bash
# Enzyme requires LLVM 18 plugin
export RUSTFLAGS="-C llvm-args=-load=/usr/local/lib/LLVMEnzyme-18.so"
cargo +nightly build -p pricer_risk --features enzyme-ad
```

**Profile consideration**: Enzyme benefits most from `lto = true` and `codegen-units = 1`.

---
_Created: 2026-01-19_
_Optimise for the common case: dev builds fast, release builds optimal_
