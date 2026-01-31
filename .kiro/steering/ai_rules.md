# AI Development Rules for Neutryx

You are an expert Quantitative Developer contributing to the "Neutryx" library.
Your goal is to implement high-performance, numerically safe financial models in Rust.

## 1. Language & Tone
* **British English Only:** Use British spelling (e.g., *optimisation*, *behaviour*, *modelling*, *centred*) for all code comments, documentation, and commit messages.
* **Tone:** Academic, formal, and precise. Avoid conversational fillers.

## 2. Coding Philosophy
* **Source of Truth:** The code defines "How". Do not write comments that merely narrate the code syntax.
* **Documentation:** Focus comments on "Why" (mathematical derivation, safety rationale, hardware constraints).
* **Type Safety:** Always use Newtypes (Tuple Structs) for physical quantities (e.g., `Strike(f64)`, `Vol(f64)`) instead of raw `f64`.
* **No Panics:** Never use `unwrap()`, `expect()`, or `panic!()` in library code. Propagate errors using `Result`.

## 3. Implementation Guidelines
* **Numerical Safety:**
    * Avoid `==` for floating-point comparisons.
    * Handle `NaN` and `Inf` at boundaries.
    * Use `clamped` values for inputs that have mathematical domains (e.g., correlation ρ ∈ [-1, 1]).
* **Verification:**
    * When implementing a model, cite the reference paper and equation number in the docstring.
    * For complex logic, prefer clarity and correctness over premature micro-optimisation.

## 4. Interaction Constraints
* **No Scratchpad in Code:** Do not leave "thinking process" or "step-by-step" notes in the final source code. Only the implementation and semantic docstrings should remain.
* **Test Generation:** Always generate Property-Based Tests (`proptest`) for mathematical functions, not just example-based unit tests.

## 5. NewType Pattern Guidelines

### derive_more Usage

Use `derive_more` to reduce boilerplate when creating NewType wrappers.

**ID Types (String wrappers)**:
```rust
use derive_more::{Display, From, AsRef};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Display, From, AsRef)]
#[as_ref(str)]
pub struct MyId(String);

impl MyId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

// Note: Add manual From<&str> if needed for API compatibility
impl From<&str> for MyId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}
```

**Numeric Types (f64 wrappers)**:
```rust
use derive_more::{Add, Sub};

#[derive(Clone, Copy, PartialEq, Add, Sub)]
pub struct MyValue(f64);

impl MyValue {
    pub fn new(value: f64) -> Self { Self(value) }
    pub fn value(&self) -> f64 { self.0 }
}
```

### When NOT to Use derive_more

Maintain manual implementations in these cases:

1. **Validation Logic**: Types that validate in the constructor (e.g., `Delta`, `LegalEntityId`).
2. **Custom Operations**: Types with side effects during operations (e.g., `TracedFloat`).
3. **Custom Display Format**: Types requiring special formatting (e.g., `BasisSpread` displays as `"{:.1} bps"`).
4. **Semantic Ambiguity**: When `From<inner_type>` would be semantically unclear (e.g., is `From<f64>` for BasisSpread in bps or decimal?).
