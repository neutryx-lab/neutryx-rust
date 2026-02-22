//! Binomial and Trinomial tree pricing methods for vanilla and American

pub mod bermudan_engine;
mod binomial;
pub(crate) mod common;
mod config;
pub mod gaussian;
mod method;
pub mod tarn_engine;
mod trinomial;

pub use bermudan_engine::{
    BermudanTreeConfig, BermudanTreeEngine, BermudanTreeResult, CouponInfo, ExerciseInfo,
};
pub use binomial::{BinomialTree, CrrParams};
pub use config::{TreeConfig, TreeConfigBuilder, TreeType};
pub use gaussian::{
    GaussianTree, GaussianTreeConfig, GaussianTreeSlice, GaussianTreeTransition,
};
pub use method::TreeMethod;
pub use tarn_engine::{
    TarnConfig, TarnCouponInfo, TarnExerciseInfo, TarnGrid, TarnTreeEngine, TarnTreeResult,
};
pub use trinomial::{TrinomialParams, TrinomialTree};
