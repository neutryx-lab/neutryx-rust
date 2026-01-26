//! FX option types for the Garman-Kohlhagen model.

/// FX option type (Call or Put).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxOptionType {
    /// Call option - right to buy the base currency.
    Call,
    /// Put option - right to sell the base currency.
    Put,
}

impl FxOptionType {
    /// Returns true if this is a call option.
    #[inline]
    #[must_use]
    pub fn is_call(&self) -> bool { matches!(self, FxOptionType::Call) }

    /// Returns true if this is a put option.
    #[inline]
    #[must_use]
    pub fn is_put(&self) -> bool { matches!(self, FxOptionType::Put) }
}
