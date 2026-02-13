//! Configuration types for the pricing engine.

/// Calculation mode for Greeks computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GreeksMode {
    /// Bump-and-revalue using finite differences.
    #[default]
    BumpRevalue,
    /// Enzyme LLVM-level automatic differentiation.
    #[cfg(feature = "enzyme-ad")]
    EnzymeAAD,
}
