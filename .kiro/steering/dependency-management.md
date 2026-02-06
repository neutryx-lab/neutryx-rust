# Dependency Management

[Purpose: manage workspace dependencies, version unification, feature flags design]

## Philosophy

- **Single source of truth**: All shared dependencies defined in workspace `[workspace.dependencies]`
- **Version unification**: Prevent duplicate compilation from version mismatches
- **Feature minimisation**: Include only necessary features to reduce compile time and binary size
- **Optional by default**: Heavy dependencies gated behind feature flags

## Workspace Dependencies Pattern

All crates use workspace inheritance for shared dependencies:

```toml
# Cargo.toml (workspace root)
[workspace.dependencies]
tokio = { version = "1.42", features = ["rt-multi-thread", "macros", "sync", "time", "net", "io-util", "signal"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
serde = { version = "1.0", features = ["derive"] }

# crate/Cargo.toml
[dependencies]
tokio = { workspace = true }
tower-http = { workspace = true }  # Additional features: tower-http = { workspace = true, features = ["fs"] }
```

**Rules**:
1. Never specify version directly in crate Cargo.toml for workspace dependencies
2. Add new workspace dependencies to root Cargo.toml first
3. Use `{ workspace = true }` inheritance, extend with additional features as needed

## Feature Flags Architecture

### AD Mode Selection
```toml
[features]
default = []
enzyme-mode = []        # Production mode (LLVM Enzyme)
```

### Asset Classes (Additive)
```toml
[features]
default = ["equity"]
equity = []
rates = []
credit = []
fx = []
commodity = []
exotic = []
all = ["equity", "rates", "credit", "fx", "commodity", "exotic"]
```

### Integration Features (Propagating)
```toml
# Features propagate through dependency chain
l1l2-integration = ["dep:pricer_core", "dep:pricer_models"]
enzyme-ad = ["dep:llvm-sys", "pricer_risk/enzyme-ad"]
```

## Heavy Dependencies (Optional)

| Dependency | Crate | Feature | Impact |
|------------|-------|---------|--------|
| `llvm-sys` | pricer_risk | `enzyme-ad` | ~500KB, LLVM 18 required |
| `parquet` + `arrow` | adapter_loader | `parquet` | ~200KB, Arrow ecosystem |
| `sqlx` | infra_store | `postgres` | ~100KB, async SQL |
| `pyo3` | service_python | always | ~300KB, Python bindings |

**Pattern**: Gate heavy dependencies behind optional features:
```toml
[dependencies]
llvm-sys = { version = "180", optional = true }

[features]
enzyme-ad = ["dep:llvm-sys"]
```

## Version Conflict Resolution

**Detection**:
```bash
cargo tree --duplicates
```

**Common Conflicts**:
- `tower-http`: Ensure all crates use same version via workspace
- `base64`: Transitive from `config`/`reqwest` - monitor but acceptable
- `tokio`: Use workspace definition, avoid `features = ["full"]`

**Resolution Strategy**:
1. Add conflicting dependency to `[workspace.dependencies]`
2. Update all crates to use `{ workspace = true }`
3. Run `cargo tree --duplicates` to verify resolution

## Tokio Features Guideline

Avoid `features = ["full"]`. Specify needed features explicitly:

| Use Case | Features |
|----------|----------|
| Async runtime | `rt-multi-thread`, `macros` |
| Networking | `net`, `io-util` |
| Time/timers | `time` |
| Sync primitives | `sync` |
| Signal handling | `signal` |

## Builder Pattern & Enum Utilities

| Crate | Version | Purpose |
|-------|---------|---------|
| `bon` | 3.8 | Builder pattern auto-generation via `#[derive(bon::Builder)]` |
| `strum` | 0.26 | Enum utilities (Display, EnumIter, EnumString) |
| `enum_dispatch` | 0.3 | Zero-cost trait dispatch via enum wrappers |
| `derive_more` | 2 | Newtype derive utilities (From, Display, AsRef, Add, Mul) |

**bon usage pattern**:
```rust
use bon::Builder;

#[derive(Builder)]
pub struct Book {
    #[builder(into)]
    book_id: BookId,
    #[builder(into)]
    name: String,
    #[builder(into, default)]
    description: Option<String>,
    #[builder(default)]
    book_type: BookType,
}

// Usage: Book::builder().book_id("BOOK001").name("Trading").build()
```

**Key attributes**:
- `#[builder(into)]` — Accepts `impl Into<T>` for ergonomic API
- `#[builder(default)]` — Uses `Default::default()` if not specified
- `#[builder(default = expr)]` — Custom default value

**enum_dispatch usage pattern**:
```rust
use enum_dispatch::enum_dispatch;

#[enum_dispatch]
pub trait YieldCurve<T> {
    fn discount_factor(&self, t: T) -> Result<T, Error>;
}

#[enum_dispatch(YieldCurve<T>)]
pub enum CurveEnum<T> {
    Flat(FlatCurve<T>),
    Bootstrapped(BootstrappedCurve<T>),
}
// Now CurveEnum<T>::discount_factor() dispatches without vtable overhead
```

**When to use enum_dispatch**:
- Traits with multiple implementations (>2)
- Hot paths requiring zero-cost dispatch
- Enzyme AD compatibility (concrete types preferred)

**derive_more for newtypes**: See `ai_rules.md` for ID type and numeric type patterns.

## Adding New Dependencies

1. **Check workspace**: Does similar dependency exist?
2. **Evaluate size**: Run `cargo build --timings` before/after
3. **Feature gate**: If >50KB compiled, make optional
4. **Add to workspace**: Define in root Cargo.toml first
5. **Use inheritance**: All crates use `{ workspace = true }`

## Monitoring Commands

```bash
# Duplicate detection
cargo tree --duplicates

# Dependency size analysis
cargo bloat --release --crates

# Build timing
cargo build --timings

# Feature resolution
cargo tree -e features
```

---
_Created: 2026-01-19_
_Updated: 2026-01-31_ — Added enum_dispatch and derive_more patterns
_Patterns over lists; optimise for single compilation of each dependency_
