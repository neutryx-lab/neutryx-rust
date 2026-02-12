//! CallableKernel: Block-structured IR for callable/Bermudan products.
//!
//! This module provides the `CallableKernel` structure that represents
//! callable products (Bermudan swaptions, callable bonds, etc.) as a
//! sequence of exercise blocks.
//!
//! # Design Principles
//!
//! - **Block Structure**: Products are divided by exercise dates into blocks
//! - **Forward/Backward Compatible**: Supports both accumulation and regression
//! - **Nested PricingKernel**: Each block contains core cashflows
//! - **Enzyme AD Compatible**: Only primitive types, no trait objects
//!
//! # Example
//!
//! ```
//! use pricer_core::kernel::{CallableKernel, CallableBlock, ExerciseDef, ExerciseStyle, PricingKernel};
//!
//! // Create a simple Bermudan swaption with 2 exercise dates
//! let kernel = CallableKernel::new(
//!     vec![
//!         CallableBlock {
//!             start_date: 19000,
//!             end_date: 19365,
//!             core_flows: PricingKernel::empty(),
//!             exercise: Some(ExerciseDef {
//!                 exercise_date: 19365,
//!                 exercise_cost: 0.0,
//!                 style: ExerciseStyle::Bermudan,
//!             }),
//!         },
//!         CallableBlock {
//!             start_date: 19365,
//!             end_date: 19730,
//!             core_flows: PricingKernel::empty(),
//!             exercise: Some(ExerciseDef {
//!                 exercise_date: 19730,
//!                 exercise_cost: 0.0,
//!                 style: ExerciseStyle::Bermudan,
//!             }),
//!         },
//!     ],
//!     0, // base currency
//! );
//!
//! assert_eq!(kernel.block_count(), 2);
//! assert_eq!(kernel.exercise_count(), 2);
//! ```

use super::PricingKernel;

/// Exercise style for callable products.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, strum::Display,
)]
pub enum ExerciseStyle {
    /// Bermudan: Exercise only at specific dates.
    Bermudan,
    /// American: Exercise at any time (approximated by frequent dates).
    American,
}

/// Exercise opportunity definition for a single decision point in a callable
/// product.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExerciseDef {
    /// Exercise date (days from Unix epoch).
    pub exercise_date: i32,

    /// Exercise cost (fee paid upon exercise, typically 0).
    pub exercise_cost: f64,

    /// Exercise style (Bermudan or American).
    pub style: ExerciseStyle,
}

impl ExerciseDef {
    /// Creates a new exercise definition.
    #[must_use]
    pub const fn new(exercise_date: i32, exercise_cost: f64, style: ExerciseStyle) -> Self {
        Self {
            exercise_date,
            exercise_cost,
            style,
        }
    }

    /// Creates a Bermudan exercise with zero cost.
    #[must_use]
    pub const fn bermudan(exercise_date: i32) -> Self {
        Self::new(exercise_date, 0.0, ExerciseStyle::Bermudan)
    }

    /// Creates a Bermudan exercise with specified cost.
    #[must_use]
    pub const fn bermudan_with_cost(exercise_date: i32, cost: f64) -> Self {
        Self::new(exercise_date, cost, ExerciseStyle::Bermudan)
    }

    /// Creates an American exercise with zero cost.
    #[must_use]
    pub const fn american(exercise_date: i32) -> Self {
        Self::new(exercise_date, 0.0, ExerciseStyle::American)
    }
}

/// A period between exercise dates with underlying cashflows and an optional
/// exercise opportunity.
#[derive(Clone, Debug)]
pub struct CallableBlock {
    /// Block start date (days from epoch).
    pub start_date: i32,

    /// Block end date (next exercise date or maturity).
    pub end_date: i32,

    /// Cashflows within this block (evaluated as a PricingKernel).
    pub core_flows: PricingKernel,

    /// Exercise opportunity at block end (None if no-call period).
    pub exercise: Option<ExerciseDef>,
}

impl CallableBlock {
    /// Creates a new callable block.
    #[must_use]
    pub fn new(
        start_date: i32,
        end_date: i32,
        core_flows: PricingKernel,
        exercise: Option<ExerciseDef>,
    ) -> Self {
        Self {
            start_date,
            end_date,
            core_flows,
            exercise,
        }
    }

    /// Creates an empty block with no cashflows and no exercise.
    #[must_use]
    pub fn empty(start_date: i32, end_date: i32) -> Self {
        Self::new(start_date, end_date, PricingKernel::empty(), None)
    }

    /// Returns `true` if this block has an exercise opportunity.
    #[must_use]
    pub const fn has_exercise(&self) -> bool { self.exercise.is_some() }

    /// Returns the block duration in days.
    #[must_use]
    pub const fn duration_days(&self) -> i32 { self.end_date - self.start_date }

    /// Returns the number of cashflows in this block.
    #[must_use]
    pub fn cashflow_count(&self) -> usize { self.core_flows.len() }

    /// Returns the exercise date if present.
    #[must_use]
    pub fn exercise_date(&self) -> Option<i32> { self.exercise.as_ref().map(|e| e.exercise_date) }
}

/// Block-structured IR for callable/Bermudan products.
///
/// Represents products with early exercise features as a sequence of blocks
/// divided by exercise dates. Supports forward (cashflow accumulation) and
/// backward (LSMC regression) passes.
#[derive(Clone, Debug)]
pub struct CallableKernel {
    /// Sequence of exercise blocks.
    pub blocks: Vec<CallableBlock>,

    /// Base currency ID (for PV reporting).
    pub base_currency_id: u8,
}

impl CallableKernel {
    /// Creates a new `CallableKernel`.
    #[must_use]
    pub fn new(blocks: Vec<CallableBlock>, base_currency_id: u8) -> Self {
        Self {
            blocks,
            base_currency_id,
        }
    }

    /// Creates an empty `CallableKernel` with no blocks.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            blocks: Vec::new(),
            base_currency_id: 0,
        }
    }

    /// Returns the number of blocks.
    #[must_use]
    pub fn block_count(&self) -> usize { self.blocks.len() }

    /// Returns `true` if the kernel has no blocks.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.blocks.is_empty() }

    /// Returns the number of exercise opportunities.
    #[must_use]
    pub fn exercise_count(&self) -> usize {
        self.blocks.iter().filter(|b| b.has_exercise()).count()
    }

    /// Returns the total number of cashflows across all blocks.
    #[must_use]
    pub fn total_cashflow_count(&self) -> usize {
        self.blocks.iter().map(|b| b.cashflow_count()).sum()
    }

    /// Returns all exercise dates in chronological order.
    #[must_use]
    pub fn exercise_dates(&self) -> Vec<i32> {
        self.blocks
            .iter()
            .filter_map(|b| b.exercise_date())
            .collect()
    }

    /// Returns the first exercise date (if any).
    #[must_use]
    pub fn first_exercise_date(&self) -> Option<i32> {
        self.blocks.iter().find_map(|b| b.exercise_date())
    }

    /// Returns the last exercise date (if any).
    #[must_use]
    pub fn last_exercise_date(&self) -> Option<i32> {
        self.blocks.iter().rev().find_map(|b| b.exercise_date())
    }

    /// Returns the start date of the first block.
    #[must_use]
    pub fn start_date(&self) -> Option<i32> { self.blocks.first().map(|b| b.start_date) }

    /// Returns the end date of the last block (maturity).
    #[must_use]
    pub fn maturity(&self) -> Option<i32> { self.blocks.last().map(|b| b.end_date) }

    /// Returns an iterator over blocks.
    pub fn iter(&self) -> impl Iterator<Item = &CallableBlock> { self.blocks.iter() }

    /// Returns a mutable iterator over blocks.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut CallableBlock> {
        self.blocks.iter_mut()
    }

    /// Returns an iterator over blocks in reverse order (for backward pass).
    pub fn iter_backward(&self) -> impl Iterator<Item = &CallableBlock> { self.blocks.iter().rev() }

    /// Returns the total memory usage in bytes.
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>()
            + self
                .blocks
                .iter()
                .map(|b| std::mem::size_of::<CallableBlock>() + b.core_flows.memory_usage())
                .sum::<usize>()
    }
}

/// Builder for constructing `CallableKernel` incrementally.
#[derive(Debug, Default)]
pub struct CallableKernelBuilder {
    blocks: Vec<CallableBlock>,
    base_currency_id: u8,
}

impl CallableKernelBuilder {
    /// Creates a new empty builder.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Sets the base currency ID.
    #[must_use]
    pub fn base_currency(mut self, currency_id: u8) -> Self {
        self.base_currency_id = currency_id;
        self
    }

    /// Adds a block to the kernel.
    #[must_use]
    pub fn add_block(mut self, block: CallableBlock) -> Self {
        self.blocks.push(block);
        self
    }

    /// Adds a block with the given parameters.
    #[must_use]
    pub fn add_block_with(
        mut self,
        start_date: i32,
        end_date: i32,
        core_flows: PricingKernel,
        exercise: Option<ExerciseDef>,
    ) -> Self {
        self.blocks.push(CallableBlock::new(
            start_date, end_date, core_flows, exercise,
        ));
        self
    }

    /// Adds an empty block (no cashflows, no exercise).
    #[must_use]
    pub fn add_empty_block(mut self, start_date: i32, end_date: i32) -> Self {
        self.blocks.push(CallableBlock::empty(start_date, end_date));
        self
    }

    /// Returns the number of blocks added.
    #[must_use]
    pub fn block_count(&self) -> usize { self.blocks.len() }

    /// Builds the `CallableKernel`.
    #[must_use]
    pub fn build(self) -> CallableKernel { CallableKernel::new(self.blocks, self.base_currency_id) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exercise_style_display() {
        assert_eq!(format!("{}", ExerciseStyle::Bermudan), "Bermudan");
        assert_eq!(format!("{}", ExerciseStyle::American), "American");
    }

    #[test]
    fn test_exercise_style_equality() {
        assert_eq!(ExerciseStyle::Bermudan, ExerciseStyle::Bermudan);
        assert_ne!(ExerciseStyle::Bermudan, ExerciseStyle::American);
    }

    #[test]
    fn test_exercise_style_copy() {
        let style = ExerciseStyle::Bermudan;
        let copied = style;
        assert_eq!(style, copied);
    }

    #[test]
    fn test_exercise_def_new() {
        let exercise = ExerciseDef::new(19365, 100.0, ExerciseStyle::Bermudan);
        assert_eq!(exercise.exercise_date, 19365);
        assert!((exercise.exercise_cost - 100.0).abs() < 1e-10);
        assert_eq!(exercise.style, ExerciseStyle::Bermudan);
    }

    #[test]
    fn test_exercise_def_bermudan() {
        let exercise = ExerciseDef::bermudan(19365);
        assert_eq!(exercise.exercise_date, 19365);
        assert!(exercise.exercise_cost.abs() < 1e-10);
        assert_eq!(exercise.style, ExerciseStyle::Bermudan);
    }

    #[test]
    fn test_exercise_def_bermudan_with_cost() {
        let exercise = ExerciseDef::bermudan_with_cost(19365, 50.0);
        assert_eq!(exercise.exercise_date, 19365);
        assert!((exercise.exercise_cost - 50.0).abs() < 1e-10);
        assert_eq!(exercise.style, ExerciseStyle::Bermudan);
    }

    #[test]
    fn test_exercise_def_american() {
        let exercise = ExerciseDef::american(19365);
        assert_eq!(exercise.exercise_date, 19365);
        assert!(exercise.exercise_cost.abs() < 1e-10);
        assert_eq!(exercise.style, ExerciseStyle::American);
    }

    #[test]
    fn test_exercise_def_clone() {
        let exercise = ExerciseDef::bermudan(19365);
        let cloned = exercise.clone();
        assert_eq!(exercise, cloned);
    }

    #[test]
    fn test_callable_block_new() {
        let block = CallableBlock::new(
            19000,
            19365,
            PricingKernel::empty(),
            Some(ExerciseDef::bermudan(19365)),
        );

        assert_eq!(block.start_date, 19000);
        assert_eq!(block.end_date, 19365);
        assert!(block.has_exercise());
    }

    #[test]
    fn test_callable_block_empty() {
        let block = CallableBlock::empty(19000, 19365);

        assert_eq!(block.start_date, 19000);
        assert_eq!(block.end_date, 19365);
        assert!(!block.has_exercise());
        assert_eq!(block.cashflow_count(), 0);
    }

    #[test]
    fn test_callable_block_duration_days() {
        let block = CallableBlock::empty(19000, 19365);
        assert_eq!(block.duration_days(), 365);
    }

    #[test]
    fn test_callable_block_exercise_date() {
        let with_exercise = CallableBlock::new(
            19000,
            19365,
            PricingKernel::empty(),
            Some(ExerciseDef::bermudan(19365)),
        );

        let without_exercise = CallableBlock::empty(19000, 19365);

        assert_eq!(with_exercise.exercise_date(), Some(19365));
        assert_eq!(without_exercise.exercise_date(), None);
    }

    #[test]
    fn test_callable_block_cashflow_count() {
        let kernel = PricingKernel::new(
            vec![19000],
            vec![18900],
            vec![0.5],
            vec![1_000_000.0],
            vec![0.05],
            vec![0.0],
            vec![0],
            vec![0],
            vec![0],
            vec![0],
        )
        .unwrap();

        let block = CallableBlock::new(19000, 19365, kernel, None);
        assert_eq!(block.cashflow_count(), 1);
    }

    #[test]
    fn test_callable_kernel_new() {
        let blocks = vec![
            CallableBlock::new(
                19000,
                19365,
                PricingKernel::empty(),
                Some(ExerciseDef::bermudan(19365)),
            ),
            CallableBlock::new(
                19365,
                19730,
                PricingKernel::empty(),
                Some(ExerciseDef::bermudan(19730)),
            ),
        ];

        let kernel = CallableKernel::new(blocks, 0);

        assert_eq!(kernel.block_count(), 2);
        assert_eq!(kernel.exercise_count(), 2);
        assert_eq!(kernel.base_currency_id, 0);
    }

    #[test]
    fn test_callable_kernel_empty() {
        let kernel = CallableKernel::empty();

        assert!(kernel.is_empty());
        assert_eq!(kernel.block_count(), 0);
        assert_eq!(kernel.exercise_count(), 0);
    }

    #[test]
    fn test_callable_kernel_exercise_dates() {
        let blocks = vec![
            CallableBlock::new(
                19000,
                19365,
                PricingKernel::empty(),
                Some(ExerciseDef::bermudan(19365)),
            ),
            CallableBlock::new(19365, 19500, PricingKernel::empty(), None), // No exercise
            CallableBlock::new(
                19500,
                19730,
                PricingKernel::empty(),
                Some(ExerciseDef::bermudan(19730)),
            ),
        ];

        let kernel = CallableKernel::new(blocks, 0);

        let dates = kernel.exercise_dates();
        assert_eq!(dates.len(), 2);
        assert_eq!(dates[0], 19365);
        assert_eq!(dates[1], 19730);
    }

    #[test]
    fn test_callable_kernel_first_last_exercise() {
        let blocks = vec![
            CallableBlock::new(
                19000,
                19365,
                PricingKernel::empty(),
                Some(ExerciseDef::bermudan(19365)),
            ),
            CallableBlock::new(
                19365,
                19730,
                PricingKernel::empty(),
                Some(ExerciseDef::bermudan(19730)),
            ),
        ];

        let kernel = CallableKernel::new(blocks, 0);

        assert_eq!(kernel.first_exercise_date(), Some(19365));
        assert_eq!(kernel.last_exercise_date(), Some(19730));
    }

    #[test]
    fn test_callable_kernel_start_maturity() {
        let blocks = vec![
            CallableBlock::new(19000, 19365, PricingKernel::empty(), None),
            CallableBlock::new(19365, 19730, PricingKernel::empty(), None),
        ];

        let kernel = CallableKernel::new(blocks, 0);

        assert_eq!(kernel.start_date(), Some(19000));
        assert_eq!(kernel.maturity(), Some(19730));
    }

    #[test]
    fn test_callable_kernel_total_cashflow_count() {
        let kernel1 = PricingKernel::new(
            vec![19100, 19200],
            vec![19000, 19100],
            vec![0.25, 0.25],
            vec![1_000_000.0, 1_000_000.0],
            vec![0.05, 0.05],
            vec![0.0, 0.0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
        )
        .unwrap();

        let kernel2 = PricingKernel::new(
            vec![19500],
            vec![19400],
            vec![0.25],
            vec![1_000_000.0],
            vec![0.05],
            vec![0.0],
            vec![0],
            vec![0],
            vec![0],
            vec![0],
        )
        .unwrap();

        let blocks = vec![
            CallableBlock::new(19000, 19365, kernel1, None),
            CallableBlock::new(19365, 19730, kernel2, None),
        ];

        let kernel = CallableKernel::new(blocks, 0);

        assert_eq!(kernel.total_cashflow_count(), 3);
    }

    #[test]
    fn test_callable_kernel_iter() {
        let blocks = vec![
            CallableBlock::new(19000, 19365, PricingKernel::empty(), None),
            CallableBlock::new(19365, 19730, PricingKernel::empty(), None),
        ];

        let kernel = CallableKernel::new(blocks, 0);

        let dates: Vec<_> = kernel.iter().map(|b| b.start_date).collect();
        assert_eq!(dates, vec![19000, 19365]);
    }

    #[test]
    fn test_callable_kernel_iter_backward() {
        let blocks = vec![
            CallableBlock::new(19000, 19365, PricingKernel::empty(), None),
            CallableBlock::new(19365, 19730, PricingKernel::empty(), None),
        ];

        let kernel = CallableKernel::new(blocks, 0);

        let dates: Vec<_> = kernel.iter_backward().map(|b| b.start_date).collect();
        assert_eq!(dates, vec![19365, 19000]);
    }

    #[test]
    fn test_callable_kernel_memory_usage() {
        let kernel = CallableKernel::new(
            vec![CallableBlock::new(
                19000,
                19365,
                PricingKernel::empty(),
                None,
            )],
            0,
        );

        assert!(kernel.memory_usage() > 0);
    }

    #[test]
    fn test_builder_new() {
        let builder = CallableKernelBuilder::new();
        assert_eq!(builder.block_count(), 0);
    }

    #[test]
    fn test_builder_add_block() {
        let kernel = CallableKernelBuilder::new()
            .add_block(CallableBlock::empty(19000, 19365))
            .build();

        assert_eq!(kernel.block_count(), 1);
    }

    #[test]
    fn test_builder_add_block_with() {
        let kernel = CallableKernelBuilder::new()
            .add_block_with(
                19000,
                19365,
                PricingKernel::empty(),
                Some(ExerciseDef::bermudan(19365)),
            )
            .build();

        assert_eq!(kernel.block_count(), 1);
    }

    #[test]
    fn test_builder_add_empty_block() {
        let kernel = CallableKernelBuilder::new()
            .add_empty_block(19000, 19365)
            .build();

        assert_eq!(kernel.block_count(), 1);
        assert!(!kernel.blocks[0].has_exercise());
    }

    #[test]
    fn test_builder_base_currency() {
        let kernel = CallableKernelBuilder::new().base_currency(1).build();

        assert_eq!(kernel.base_currency_id, 1);
    }

    #[test]
    fn test_builder_chaining() {
        let kernel = CallableKernelBuilder::new()
            .base_currency(0)
            .add_block(CallableBlock::new(
                19000,
                19365,
                PricingKernel::empty(),
                Some(ExerciseDef::bermudan(19365)),
            ))
            .add_block(CallableBlock::new(
                19365,
                19730,
                PricingKernel::empty(),
                Some(ExerciseDef::bermudan(19730)),
            ))
            .build();

        assert_eq!(kernel.block_count(), 2);
        assert_eq!(kernel.exercise_count(), 2);
    }

    #[test]
    fn test_builder_build_empty() {
        let kernel = CallableKernelBuilder::new().build();
        assert!(kernel.is_empty());
    }
}
