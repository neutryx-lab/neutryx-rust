# Error Handling

[Purpose: standardise error type design, propagation patterns, and thiserror usage]

## Philosophy

- **Structured errors**: All errors are typed enums, not strings
- **thiserror everywhere**: Use `#[derive(Error)]` for all error types
- **Domain-specific**: Each layer has its own error types
- **Lossless propagation**: Preserve context through `#[from]` conversions

## Error Type Design

### Standard Pattern (thiserror)

```rust
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PricingError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Numerical instability: {0}")]
    NumericalInstability(String),

    #[error("Model failure: {0}")]
    ModelFailure(String),
}
```

**Rules**:
1. Always derive `Error` from thiserror
2. Include `Debug`, `Clone` for most errors
3. Add `PartialEq, Eq` for testable errors
4. Use `#[error("...")]` for Display implementation

### Structured Variants

For errors with structured data:

```rust
#[derive(Error, Debug, Clone, PartialEq)]
pub enum InterpolationError {
    #[error("Query point {x} outside valid domain [{min}, {max}]")]
    OutOfBounds { x: f64, min: f64, max: f64 },

    #[error("Insufficient data points: got {got}, need at least {need}")]
    InsufficientData { got: usize, need: usize },
}
```

## Layer-Specific Error Types

### Pricer Layer (Core)

| Error Type | Purpose | Location |
|------------|---------|----------|
| `PricingError` | General pricing failures | pricer_core |
| `DateError` | Date construction/parsing | pricer_core |
| `CurrencyError` | Currency parsing | pricer_core |
| `InterpolationError` | Interpolation domain issues | pricer_core |
| `SolverError` | Root-finding failures | pricer_core |
| `CalibrationError` | Model calibration failures | pricer_core |
| `ConfigError` | MC configuration validation | pricer_pricing |

### Adapter Layer

| Error Type | Purpose | Location |
|------------|---------|----------|
| `FeedError` | Market data feed issues | adapter_feeds |
| `FpmlError` | FpML parsing failures | adapter_fpml |
| `LoaderError` | File loading failures | adapter_loader |

### Service Layer

| Error Type | Purpose | Location |
|------------|---------|----------|
| `CliError` | CLI argument/config errors | service_cli |
| `GatewayError` | API request handling | service_gateway |

## Error Propagation

### From Conversions

Use `#[from]` for automatic conversion:

```rust
#[derive(Error, Debug)]
pub enum CalibrationError {
    #[error("Solver failed: {0}")]
    Solver(#[from] SolverError),

    #[error("Invalid constraint: {0}")]
    Constraint(String),
}

// Automatic conversion via ?
fn calibrate() -> Result<(), CalibrationError> {
    let root = solver.find_root()?;  // SolverError → CalibrationError
    Ok(())
}
```

### Manual Conversion

For complex mappings:

```rust
impl From<SolverError> for CalibrationError {
    fn from(err: SolverError) -> Self {
        match err {
            SolverError::MaxIterationsExceeded { iterations } =>
                CalibrationError::not_converged(iterations, f64::NAN),
            SolverError::NumericalInstability(msg) =>
                CalibrationError::numerical_instability(msg),
            // ...
        }
    }
}
```

## Rich Diagnostic Errors

For complex operations (e.g., calibration), include diagnostic data:

```rust
#[derive(Debug, Clone)]
pub struct CalibrationError {
    pub kind: CalibrationErrorKind,
    pub residual_ss: f64,
    pub iterations: usize,
    pub message: Option<String>,
    pub parameter_values: Option<Vec<f64>>,
}

impl CalibrationError {
    pub fn not_converged(iterations: usize, residual_ss: f64) -> Self { ... }
    pub fn with_parameters(mut self, params: Vec<f64>) -> Self { ... }
}
```

## Serde Support (Optional)

For errors that need serialisation:

```rust
#[derive(Error, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SolverError {
    #[error("Failed to converge after {iterations} iterations")]
    MaxIterationsExceeded { iterations: usize },
}
```

## Anti-Patterns

**Avoid**:
```rust
// String-based errors
Err("Something went wrong".to_string())

// Manual Display + Error impl (use thiserror instead)
impl fmt::Display for MyError { ... }
impl std::error::Error for MyError {}

// Overly generic errors
enum Error { Generic(String) }
```

**Prefer**:
```rust
// Typed, specific variants
#[derive(Error, Debug)]
enum MyError {
    #[error("Invalid parameter {name}: {reason}")]
    InvalidParameter { name: &'static str, reason: String },
}
```

## Testing Errors

```rust
#[test]
fn test_error_display() {
    let err = PricingError::InvalidInput("negative spot".to_string());
    assert_eq!(format!("{}", err), "Invalid input: negative spot");
}

#[test]
fn test_error_conversion() {
    let solver_err = SolverError::MaxIterationsExceeded { iterations: 100 };
    let calib_err: CalibrationError = solver_err.into();
    assert!(calib_err.is_not_converged());
}
```

---
_Created: 2026-01-19_
_thiserror is the standard; structured errors enable better debugging_
