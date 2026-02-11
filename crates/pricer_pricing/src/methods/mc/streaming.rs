//! Streaming Monte Carlo engine for memory-efficient path processing.

use pricer_core::math::rng::PricerRng;

use super::{aligned_buffer::AlignedPathBuffer, layout_config::StreamingConfig, paths::GbmParams};

/// Result of streaming Monte Carlo simulation.
#[derive(Clone, Debug, Default)]
pub struct StreamingResult {
    /// Mean of observed payoffs (undiscounted).
    pub mean: f64,
    /// Standard error of the mean.
    pub std_error: f64,
    /// Total steps processed.
    pub steps_processed: usize,
}

/// Trait for observers that can process streaming step data.
pub trait StreamingObserver: Send + Sync {
    /// Observes a batch of path values at the current step.
    fn observe_step(&mut self, step_idx: usize, values: &[f64]);

    /// Finalizes observation and returns aggregated payoffs.
    fn finalize(&mut self) -> Vec<f64>;

    /// Resets observer state for reuse.
    fn reset(&mut self);
}

/// Streaming Monte Carlo engine with double buffering.
pub struct StreamingEngine {
    /// Current step buffer (aligned for SIMD).
    current: AlignedPathBuffer<f64>,
    /// Previous step buffer (aligned for SIMD).
    previous: AlignedPathBuffer<f64>,
    /// Random number generator.
    rng: PricerRng,
    /// Streaming configuration.
    config: StreamingConfig,
    /// Number of paths.
    num_paths: usize,
    /// Number of steps.
    num_steps: usize,
    /// Current step index.
    current_step: usize,
    /// Alignment in bytes.
    alignment: usize,
}

impl StreamingEngine {
    /// Default alignment for AVX-512 cache lines.
    pub const DEFAULT_ALIGNMENT: usize = 64;

    /// Creates a new streaming engine.
    pub fn new(num_paths: usize, num_steps: usize, config: StreamingConfig, seed: u64) -> Self {
        Self::with_alignment(num_paths, num_steps, config, seed, Self::DEFAULT_ALIGNMENT)
    }

    /// Creates a new streaming engine with custom alignment.
    pub fn with_alignment(
        num_paths: usize,
        num_steps: usize,
        config: StreamingConfig,
        seed: u64,
        alignment: usize,
    ) -> Self {
        let current = AlignedPathBuffer::with_alignment(num_paths, alignment);
        let previous = AlignedPathBuffer::with_alignment(num_paths, alignment);
        let rng = PricerRng::from_seed(seed);

        Self {
            current,
            previous,
            rng,
            config,
            num_paths,
            num_steps,
            current_step: 0,
            alignment,
        }
    }

    /// Returns the number of paths.
    #[inline]
    pub fn num_paths(&self) -> usize { self.num_paths }

    /// Returns the number of steps.
    #[inline]
    pub fn num_steps(&self) -> usize { self.num_steps }

    /// Returns the streaming configuration.
    #[inline]
    pub fn config(&self) -> &StreamingConfig { &self.config }

    /// Returns the buffer alignment in bytes.
    #[inline]
    pub fn alignment(&self) -> usize { self.alignment }

    /// Returns current memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        self.current.memory_usage() + self.previous.memory_usage()
    }

    /// Swaps current and previous buffers.
    #[inline]
    fn swap_buffers(&mut self) { std::mem::swap(&mut self.current, &mut self.previous); }

    /// Runs streaming simulation with GBM model and observer.
    pub fn run<O>(&mut self, params: GbmParams, observer: &mut O) -> StreamingResult
    where
        O: StreamingObserver,
    {
        observer.reset();
        self.current_step = 0;

        let dt = params.maturity / self.num_steps as f64;
        let drift_dt = (params.rate - 0.5 * params.volatility * params.volatility) * dt;
        let vol_sqrt_dt = params.volatility * dt.sqrt();

        for val in self.current.as_mut_slice().iter_mut() {
            *val = params.spot;
        }

        observer.observe_step(0, self.current.as_slice());

        let mut step_randoms = vec![0.0; self.num_paths];

        for step in 0..self.num_steps {
            self.swap_buffers();

            self.rng.fill_normal(&mut step_randoms);

            let prev_slice = self.previous.as_slice();
            let curr_slice = self.current.as_mut_slice();

            for path_idx in 0..self.num_paths {
                let z = step_randoms[path_idx];
                let increment = drift_dt + vol_sqrt_dt * z;
                curr_slice[path_idx] = prev_slice[path_idx] * increment.exp();
            }

            observer.observe_step(step + 1, self.current.as_slice());
            self.current_step = step + 1;
        }

        let payoffs = observer.finalize();

        let sum: f64 = payoffs.iter().sum();
        let mean = sum / self.num_paths as f64;

        let variance: f64 =
            payoffs.iter().map(|&p| (p - mean).powi(2)).sum::<f64>() / (self.num_paths - 1) as f64;
        let std_dev = variance.sqrt();
        let std_error = std_dev / (self.num_paths as f64).sqrt();

        StreamingResult {
            mean,
            std_error,
            steps_processed: self.num_steps,
        }
    }

    /// Resets the engine for a new simulation.
    pub fn reset(&mut self) {
        self.current_step = 0;
        self.current.clear();
        self.previous.clear();
    }

    /// Resets the engine with a new seed.
    pub fn reset_with_seed(&mut self, seed: u64) {
        self.reset();
        self.rng = PricerRng::from_seed(seed);
    }
}

/// Streaming observer for arithmetic average (Asian options).
#[derive(Clone, Debug)]
pub struct ArithmeticAverageObserver {
    /// Running sum for each path.
    sums: Vec<f64>,
    /// Observation count per path.
    counts: Vec<usize>,
    /// Terminal values.
    terminals: Vec<f64>,
    /// Strike price.
    strike: f64,
    /// Smoothing epsilon.
    epsilon: f64,
    /// True for call, false for put.
    is_call: bool,
}

impl ArithmeticAverageObserver {
    /// Creates a new arithmetic average observer for Asian options.
    pub fn new(num_paths: usize, strike: f64, epsilon: f64, is_call: bool) -> Self {
        Self {
            sums: vec![0.0; num_paths],
            counts: vec![0; num_paths],
            terminals: vec![0.0; num_paths],
            strike,
            epsilon,
            is_call,
        }
    }

    /// Creates an Asian call observer.
    pub fn asian_call(num_paths: usize, strike: f64, epsilon: f64) -> Self {
        Self::new(num_paths, strike, epsilon, true)
    }

    /// Creates an Asian put observer.
    pub fn asian_put(num_paths: usize, strike: f64, epsilon: f64) -> Self {
        Self::new(num_paths, strike, epsilon, false)
    }
}

impl StreamingObserver for ArithmeticAverageObserver {
    fn observe_step(&mut self, _step_idx: usize, values: &[f64]) {
        for (i, &val) in values.iter().enumerate() {
            self.sums[i] += val;
            self.counts[i] += 1;
            self.terminals[i] = val;
        }
    }

    fn finalize(&mut self) -> Vec<f64> {
        use super::payoff::soft_plus;

        self.sums
            .iter()
            .zip(&self.counts)
            .map(|(&sum, &count)| {
                let avg = if count > 0 { sum / count as f64 } else { 0.0 };
                if self.is_call {
                    soft_plus(avg - self.strike, self.epsilon)
                } else {
                    soft_plus(self.strike - avg, self.epsilon)
                }
            })
            .collect()
    }

    fn reset(&mut self) {
        self.sums.fill(0.0);
        self.counts.fill(0);
        self.terminals.fill(0.0);
    }
}

/// Streaming observer for barrier monitoring.
#[derive(Clone, Debug)]
pub struct BarrierObserver {
    /// Whether barrier was breached for each path.
    breached: Vec<bool>,
    /// Terminal values.
    terminals: Vec<f64>,
    /// Barrier level.
    barrier: f64,
    /// Strike price.
    strike: f64,
    /// Smoothing epsilon.
    epsilon: f64,
    /// True for up barrier, false for down.
    is_up: bool,
    /// True for knock-out, false for knock-in.
    is_out: bool,
    /// True for call, false for put.
    is_call: bool,
}

impl BarrierObserver {
    /// Creates a new barrier observer.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        num_paths: usize,
        strike: f64,
        barrier: f64,
        epsilon: f64,
        is_up: bool,
        is_out: bool,
        is_call: bool,
    ) -> Self {
        Self {
            breached: vec![false; num_paths],
            terminals: vec![0.0; num_paths],
            barrier,
            strike,
            epsilon,
            is_up,
            is_out,
            is_call,
        }
    }

    /// Creates an up-and-out call barrier observer.
    pub fn up_out_call(num_paths: usize, strike: f64, barrier: f64, epsilon: f64) -> Self {
        Self::new(num_paths, strike, barrier, epsilon, true, true, true)
    }

    /// Creates a down-and-out call barrier observer.
    pub fn down_out_call(num_paths: usize, strike: f64, barrier: f64, epsilon: f64) -> Self {
        Self::new(num_paths, strike, barrier, epsilon, false, true, true)
    }
}

impl StreamingObserver for BarrierObserver {
    fn observe_step(&mut self, _step_idx: usize, values: &[f64]) {
        for (i, &val) in values.iter().enumerate() {
            if self.is_up {
                if val >= self.barrier {
                    self.breached[i] = true;
                }
            } else if val <= self.barrier {
                self.breached[i] = true;
            }
            self.terminals[i] = val;
        }
    }

    fn finalize(&mut self) -> Vec<f64> {
        use super::payoff::soft_plus;

        self.terminals
            .iter()
            .zip(&self.breached)
            .map(|(&terminal, &breached)| {
                let vanilla = if self.is_call {
                    soft_plus(terminal - self.strike, self.epsilon)
                } else {
                    soft_plus(self.strike - terminal, self.epsilon)
                };

                if self.is_out {
                    if breached {
                        0.0
                    } else {
                        vanilla
                    }
                } else {
                    if breached {
                        vanilla
                    } else {
                        0.0
                    }
                }
            })
            .collect()
    }

    fn reset(&mut self) {
        self.breached.fill(false);
        self.terminals.fill(0.0);
    }
}

/// Streaming observer for lookback options.
#[derive(Clone, Debug)]
pub struct LookbackObserver {
    /// Running maximum for each path.
    maxs: Vec<f64>,
    /// Running minimum for each path.
    mins: Vec<f64>,
    /// Terminal values.
    terminals: Vec<f64>,
    /// Strike price (for fixed strike).
    strike: Option<f64>,
    /// Smoothing epsilon.
    epsilon: f64,
    /// True for call, false for put.
    is_call: bool,
    /// True for floating strike, false for fixed strike.
    is_floating: bool,
}

impl LookbackObserver {
    /// Creates a new lookback observer.
    pub fn new(
        num_paths: usize,
        strike: Option<f64>,
        epsilon: f64,
        is_call: bool,
        is_floating: bool,
    ) -> Self {
        Self {
            maxs: vec![f64::NEG_INFINITY; num_paths],
            mins: vec![f64::INFINITY; num_paths],
            terminals: vec![0.0; num_paths],
            strike,
            epsilon,
            is_call,
            is_floating,
        }
    }

    /// Creates a fixed strike lookback call observer.
    pub fn fixed_call(num_paths: usize, strike: f64, epsilon: f64) -> Self {
        Self::new(num_paths, Some(strike), epsilon, true, false)
    }

    /// Creates a fixed strike lookback put observer.
    pub fn fixed_put(num_paths: usize, strike: f64, epsilon: f64) -> Self {
        Self::new(num_paths, Some(strike), epsilon, false, false)
    }

    /// Creates a floating strike lookback call observer.
    pub fn floating_call(num_paths: usize, epsilon: f64) -> Self {
        Self::new(num_paths, None, epsilon, true, true)
    }

    /// Creates a floating strike lookback put observer.
    pub fn floating_put(num_paths: usize, epsilon: f64) -> Self {
        Self::new(num_paths, None, epsilon, false, true)
    }
}

impl StreamingObserver for LookbackObserver {
    fn observe_step(&mut self, _step_idx: usize, values: &[f64]) {
        for (i, &val) in values.iter().enumerate() {
            self.maxs[i] = self.maxs[i].max(val);
            self.mins[i] = self.mins[i].min(val);
            self.terminals[i] = val;
        }
    }

    fn finalize(&mut self) -> Vec<f64> {
        use super::payoff::soft_plus;

        (0..self.terminals.len())
            .map(|i| {
                if self.is_floating {
                    if self.is_call {
                        soft_plus(self.terminals[i] - self.mins[i], self.epsilon)
                    } else {
                        soft_plus(self.maxs[i] - self.terminals[i], self.epsilon)
                    }
                } else {
                    let strike = self.strike.unwrap_or(0.0);
                    if self.is_call {
                        soft_plus(self.maxs[i] - strike, self.epsilon)
                    } else {
                        soft_plus(strike - self.mins[i], self.epsilon)
                    }
                }
            })
            .collect()
    }

    fn reset(&mut self) {
        self.maxs.fill(f64::NEG_INFINITY);
        self.mins.fill(f64::INFINITY);
        self.terminals.fill(0.0);
    }
}

/// Streaming observer for European options (terminal only).
#[derive(Clone, Debug)]
pub struct EuropeanObserver {
    /// Terminal values.
    terminals: Vec<f64>,
    /// Strike price.
    strike: f64,
    /// Smoothing epsilon.
    epsilon: f64,
    /// True for call, false for put.
    is_call: bool,
}

impl EuropeanObserver {
    /// Creates a new European observer.
    pub fn new(num_paths: usize, strike: f64, epsilon: f64, is_call: bool) -> Self {
        Self {
            terminals: vec![0.0; num_paths],
            strike,
            epsilon,
            is_call,
        }
    }

    /// Creates a European call observer.
    pub fn call(num_paths: usize, strike: f64, epsilon: f64) -> Self {
        Self::new(num_paths, strike, epsilon, true)
    }

    /// Creates a European put observer.
    pub fn put(num_paths: usize, strike: f64, epsilon: f64) -> Self {
        Self::new(num_paths, strike, epsilon, false)
    }
}

impl StreamingObserver for EuropeanObserver {
    fn observe_step(&mut self, _step_idx: usize, values: &[f64]) {
        self.terminals.copy_from_slice(values);
    }

    fn finalize(&mut self) -> Vec<f64> {
        use super::payoff::soft_plus;

        self.terminals
            .iter()
            .map(|&terminal| {
                if self.is_call {
                    soft_plus(terminal - self.strike, self.epsilon)
                } else {
                    soft_plus(self.strike - terminal, self.epsilon)
                }
            })
            .collect()
    }

    fn reset(&mut self) { self.terminals.fill(0.0); }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_streaming_engine_new() {
        let config = StreamingConfig::enabled();
        let engine = StreamingEngine::new(1000, 100, config, 42);

        assert_eq!(engine.num_paths(), 1000);
        assert_eq!(engine.num_steps(), 100);
        assert!(engine.config().is_enabled());
    }

    #[test]
    fn test_streaming_engine_memory_usage() {
        let config = StreamingConfig::enabled();
        let engine = StreamingEngine::new(1000, 100, config, 42);

        let mem = engine.memory_usage();

        let expected = 2 * 1000 * std::mem::size_of::<f64>();
        assert_eq!(mem, expected);
    }

    #[test]
    fn test_streaming_engine_memory_independent_of_steps() {
        let config = StreamingConfig::enabled();

        let engine_10 = StreamingEngine::new(1000, 10, config, 42);
        let engine_1000 = StreamingEngine::new(1000, 1000, config, 42);

        assert_eq!(engine_10.memory_usage(), engine_1000.memory_usage());
    }

    #[test]
    fn test_european_observer_call() {
        let mut observer = EuropeanObserver::call(4, 100.0, 1e-6);

        observer.observe_step(0, &[100.0, 100.0, 100.0, 100.0]);
        observer.observe_step(1, &[110.0, 90.0, 120.0, 80.0]);

        let payoffs = observer.finalize();

        assert_relative_eq!(payoffs[0], 10.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[1], 0.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[2], 20.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[3], 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_european_observer_put() {
        let mut observer = EuropeanObserver::put(4, 100.0, 1e-6);

        observer.observe_step(0, &[100.0, 100.0, 100.0, 100.0]);
        observer.observe_step(1, &[110.0, 90.0, 120.0, 80.0]);

        let payoffs = observer.finalize();

        assert_relative_eq!(payoffs[0], 0.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[1], 10.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[2], 0.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[3], 20.0, epsilon = 0.01);
    }

    #[test]
    fn test_arithmetic_average_observer() {
        let mut observer = ArithmeticAverageObserver::asian_call(2, 100.0, 1e-6);

        observer.observe_step(0, &[100.0, 100.0]);
        observer.observe_step(1, &[110.0, 90.0]);
        observer.observe_step(2, &[120.0, 80.0]);

        let payoffs = observer.finalize();

        assert_relative_eq!(payoffs[0], 10.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[1], 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_barrier_observer_up_out() {
        let mut observer = BarrierObserver::up_out_call(3, 100.0, 120.0, 1e-6);

        observer.observe_step(0, &[100.0, 100.0, 100.0]);
        observer.observe_step(1, &[110.0, 125.0, 95.0]);
        observer.observe_step(2, &[115.0, 110.0, 90.0]);

        let payoffs = observer.finalize();

        assert_relative_eq!(payoffs[0], 15.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[1], 0.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[2], 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_lookback_fixed_call() {
        let mut observer = LookbackObserver::fixed_call(2, 100.0, 1e-6);

        observer.observe_step(0, &[100.0, 100.0]);
        observer.observe_step(1, &[130.0, 90.0]);
        observer.observe_step(2, &[110.0, 95.0]);

        let payoffs = observer.finalize();

        assert_relative_eq!(payoffs[0], 30.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[1], 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_lookback_floating_call() {
        let mut observer = LookbackObserver::floating_call(2, 1e-6);

        observer.observe_step(0, &[100.0, 100.0]);
        observer.observe_step(1, &[105.0, 80.0]);
        observer.observe_step(2, &[110.0, 90.0]);

        let payoffs = observer.finalize();

        assert_relative_eq!(payoffs[0], 10.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[1], 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_observer_reset() {
        let mut observer = ArithmeticAverageObserver::asian_call(2, 100.0, 1e-6);

        observer.observe_step(0, &[100.0, 100.0]);
        observer.observe_step(1, &[110.0, 90.0]);

        observer.reset();

        observer.observe_step(0, &[200.0, 200.0]);
        let payoffs = observer.finalize();

        assert_relative_eq!(payoffs[0], 100.0, epsilon = 0.01);
        assert_relative_eq!(payoffs[1], 100.0, epsilon = 0.01);
    }

    #[test]
    fn test_streaming_engine_run_european() {
        let config = StreamingConfig::enabled();
        let mut engine = StreamingEngine::new(10_000, 50, config, 42);
        let mut observer = EuropeanObserver::call(10_000, 100.0, 1e-6);

        let params = GbmParams::default();
        let result = engine.run(params, &mut observer);

        assert!(result.mean > 0.0);
        assert!(result.std_error > 0.0);
        assert_eq!(result.steps_processed, 50);
    }

    #[test]
    fn test_streaming_engine_run_asian() {
        let config = StreamingConfig::enabled();
        let mut engine = StreamingEngine::new(10_000, 50, config, 42);
        let mut observer = ArithmeticAverageObserver::asian_call(10_000, 100.0, 1e-6);

        let params = GbmParams::default();
        let result = engine.run(params, &mut observer);

        assert!(result.mean > 0.0);
        assert!(result.std_error > 0.0);
    }

    #[test]
    fn test_streaming_engine_reproducibility() {
        let config = StreamingConfig::enabled();

        let mut engine1 = StreamingEngine::new(1000, 20, config, 42);
        let mut observer1 = EuropeanObserver::call(1000, 100.0, 1e-6);
        let result1 = engine1.run(GbmParams::default(), &mut observer1);

        let mut engine2 = StreamingEngine::new(1000, 20, config, 42);
        let mut observer2 = EuropeanObserver::call(1000, 100.0, 1e-6);
        let result2 = engine2.run(GbmParams::default(), &mut observer2);

        assert_eq!(result1.mean, result2.mean);
        assert_eq!(result1.std_error, result2.std_error);
    }

    #[test]
    fn test_streaming_vs_batch_consistency() {

        use pricer_core::math::rng::PricerRng;

        use super::super::{
            layout_config::PathLayout, paths::generate_gbm_paths_generic,
            workspace_enum::WorkspaceEnum, workspace_trait::PathWorkspaceTrait,
        };

        let n_paths = 10_000;
        let n_steps = 20;
        let seed = 42;
        let params = GbmParams::default();

        let config = StreamingConfig::enabled();
        let mut engine = StreamingEngine::new(n_paths, n_steps, config, seed);
        let mut observer = EuropeanObserver::call(n_paths, 100.0, 1e-6);
        let streaming_result = engine.run(params, &mut observer);

        let mut workspace = WorkspaceEnum::new(PathLayout::TimeStepFirst, n_paths, n_steps);
        let mut rng = PricerRng::from_seed(seed);
        rng.fill_normal(workspace.randoms_mut());
        generate_gbm_paths_generic(&mut workspace, params);

        let terminals = super::super::paths::terminal_prices_generic(&workspace);
        let batch_sum: f64 = terminals
            .iter()
            .map(|&t| super::super::payoff::soft_plus(t - 100.0, 1e-6))
            .sum();
        let batch_mean = batch_sum / n_paths as f64;

        let diff_ratio = (streaming_result.mean - batch_mean).abs() / batch_mean;
        assert!(
            diff_ratio < 0.05,
            "Streaming ({}) vs Batch ({}) diff ratio: {}",
            streaming_result.mean,
            batch_mean,
            diff_ratio
        );
    }

    #[test]
    fn test_streaming_engine_large_simulation() {
        let config = StreamingConfig::enabled();
        let n_paths = 100_000;
        let n_steps = 252;

        let engine = StreamingEngine::new(n_paths, n_steps, config, 42);

        let mem = engine.memory_usage();
        let expected_batch_mem = n_paths * n_steps * std::mem::size_of::<f64>();
        let expected_streaming_mem = 2 * n_paths * std::mem::size_of::<f64>();

        assert_eq!(mem, expected_streaming_mem);
        assert!(
            mem < expected_batch_mem / 100,
            "Streaming mem ({}) should be << batch mem ({})",
            mem,
            expected_batch_mem
        );
    }
}
