# Code Quality

[Purpose: enforce consistent code style, lint configuration, dead code management]

## Philosophy

- **Production-grade**: Clippy pedantic enabled, warnings treated as errors in CI
- **Mathematical conventions**: Single-letter variables allowed for math code
- **Incremental improvement**: Panic lints as warnings, fix incrementally
- **No unsafe**: Blocked by default, explicit allow only for Enzyme bindings

## Workspace Lint Configuration

### Cargo.toml

```toml
[workspace.lints.rust]
missing_docs = "warn"       # Require documentation for public items
unsafe_code = "deny"        # Block unsafe code (allow explicitly where needed)

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }  # Enable pedantic checks
unwrap_used = "warn"        # Discourage panic-inducing methods
expect_used = "warn"
panic = "warn"
float_cmp = "warn"          # Warn on direct float equality
suboptimal_flops = "warn"   # Catch mathematically inefficient operations
doc_markdown = "warn"       # Enforce markdown in docs
```

### Crate Inheritance

All crates inherit workspace lints:

```toml
[lints]
workspace = true
```

## Clippy Configuration

### clippy.toml

```toml
# Allow mathematical single-letter variables (x, y, t, r, v, etc.)
single-char-binding-names-threshold = 10
```

### Common Pedantic Overrides

When necessary, override at module or function level:

```rust
#[allow(clippy::similar_names)]  // For sorted_xs, sorted_ys
fn interpolate(sorted_xs: &[f64], sorted_ys: &[f64]) { ... }

#[allow(clippy::too_many_arguments)]  // For complex model constructors
fn new(spot: T, vol: T, rate: T, ...) { ... }
```

## Code Style

### Formatting

```bash
cargo fmt --all -- --check  # Verify formatting
cargo fmt --all             # Apply formatting
```

Configuration via `rustfmt.toml`:
```toml
edition = "2021"
max_width = 100
use_field_init_shorthand = true
group_imports = "StdExternalCrate"
imports_granularity = "Crate"
```

### Naming Conventions

| Item | Convention | Example |
|------|------------|---------|
| Types | PascalCase | `HestonModel` |
| Functions | snake_case | `calculate_greeks` |
| Constants | SCREAMING_SNAKE | `MAX_ITERATIONS` |
| Math variables | Single letter OK | `x`, `t`, `sigma` |

### British English

Use British spelling in identifiers and documentation:
- `optimiser` (not optimizer)
- `serialisation` (not serialization)
- `modelling` (not modeling)

## Dead Code Management

### #[allow(dead_code)] Usage

Use sparingly for:
1. **Future API**: Public API not yet used internally
2. **Test infrastructure**: Helper code for tests
3. **Demo layer**: Example code intentionally unused

```rust
#[allow(dead_code)]
pub struct CliConfig {
    // Future configuration, not yet used
    pub general: GeneralConfig,
}
```

### Review Checklist

Before adding `#[allow(dead_code)]`:
1. Is this truly needed for future use?
2. Can it be feature-gated instead?
3. Should it be removed entirely?

## Panic Handling

### Incremental Fix Strategy

Clippy warns on panic-inducing methods (`unwrap`, `expect`, `panic!`). Fix incrementally:

```rust
// Before (warns)
let value = map.get(&key).unwrap();

// After (proper error handling)
let value = map.get(&key).ok_or(MyError::KeyNotFound)?;
```

### Acceptable Panic Locations

- Test code (`#[cfg(test)]`)
- Build scripts (`build.rs`)
- Infallible operations with proof (add comment)

```rust
// Infallible: regex is compile-time constant
let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
```

## Documentation

### Module Documentation

```rust
//! Brief description of module purpose.
//!
//! This module provides:
//! - Item 1
//! - Item 2
```

### Public Item Documentation

```rust
/// Brief description.
///
/// More detailed explanation if needed.
///
/// # Examples
/// ```
/// use crate::MyType;
/// let x = MyType::new();
/// ```
///
/// # Errors
/// Returns `MyError` if...
pub fn my_function() -> Result<(), MyError> { ... }
```

### Doc Markdown

Wrap code references in backticks:

```rust
/// Uses the `HestonModel` for pricing.  // Correct
/// Uses the HestonModel for pricing.    // Warns (doc_markdown)
```

## CI Quality Gates

```bash
# Format check
cargo fmt --all -- --check

# Lint check (warnings = errors)
cargo clippy --workspace -- -D warnings

# Test with all features
cargo test --workspace --all-features

# Documentation
cargo doc --workspace --no-deps
```

## Common Lint Issues

### float_cmp

```rust
// Warns
if price == 0.0 { ... }

// Fix
if price.abs() < f64::EPSILON { ... }
// Or use approx crate
use approx::relative_eq;
if relative_eq!(price, 0.0) { ... }
```

### uninlined_format_args

```rust
// Warns
format!("Value: {}", x)

// Fix
format!("Value: {x}")
```

### doc_markdown

```rust
// Warns
/// Uses service_gateway for API

// Fix
/// Uses `service_gateway` for API
```

---
_Created: 2026-01-19_
_Quality is non-negotiable; conventions serve clarity_
