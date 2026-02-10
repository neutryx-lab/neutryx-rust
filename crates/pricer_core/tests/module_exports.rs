//! Integration tests verifying cross-crate trait implementation.

use pricer_core::{
    traits::priceable::{Differentiable, Priceable},
    types::error::PricingError,
};

/// Verify that external crates can implement Priceable + Differentiable.
#[test]
fn test_external_trait_implementation() {
    struct TestInstrument(f64);

    impl Priceable<f64> for TestInstrument {
        fn price(&self) -> Result<f64, PricingError> { Ok(self.0) }
    }

    impl Differentiable for TestInstrument {}

    let instrument = TestInstrument(100.0);
    assert_eq!(instrument.price().unwrap(), 100.0);
}
