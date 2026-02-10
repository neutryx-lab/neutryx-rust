//! Binomial and Trinomial tree pricing methods for vanilla and American
//! options.

mod binomial;
mod config;
mod method;
mod trinomial;

pub use binomial::{BinomialTree, CrrParams};
pub use config::{TreeConfig, TreeConfigBuilder, TreeType};
pub use method::TreeMethod;
pub use trinomial::{TrinomialParams, TrinomialTree};
