//! Criterion benchmarks for PricingKernel batch evaluation.
//!
//! Benchmarks cover:
//! - Single kernel pricing (baseline)
//! - Batch evaluation throughput (10, 100, 1000, 10000 trades)
//! - Linear scaling verification
//! - SIMD-friendly access patterns
//!
//! Requirements: 11.4 - バッチ評価スループット

#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pricer_core::ir::PricingKernel;
use pricer_pricing::kernel::{FlatCurveProvider, KernelContext, LinearEngine};

/// Creates a realistic IRS-like pricing kernel.
///
/// Generates a kernel with quarterly cashflows over 5 years (20 flows).
fn create_irs_kernel() -> PricingKernel {
    let num_flows = 20;
    let start_date = 19000; // Days from epoch

    let payment_dates: Vec<i32> = (0..num_flows)
        .map(|i| start_date + (i as i32 + 1) * 91) // Quarterly
        .collect();

    let fixing_dates: Vec<i32> = payment_dates
        .iter()
        .map(|&d| d - 2) // T-2 fixing
        .collect();

    let year_fractions: Vec<f64> = vec![0.25; num_flows]; // Quarterly
    let notionals: Vec<f64> = vec![1_000_000.0; num_flows];

    // Pay leg: fixed 3%
    let spreads: Vec<f64> = vec![0.03; num_flows];
    let gearings: Vec<f64> = vec![0.0; num_flows]; // Fixed

    let currency_ids: Vec<u8> = vec![0; num_flows]; // Base currency
    let discount_curve_ids: Vec<u8> = vec![1; num_flows];
    let fwd_index_ids: Vec<u16> = vec![0; num_flows]; // No forward index (fixed)
    let fx_index_ids: Vec<u16> = vec![0; num_flows]; // No FX

    PricingKernel::new(
        payment_dates,
        fixing_dates,
        year_fractions,
        notionals,
        spreads,
        gearings,
        currency_ids,
        discount_curve_ids,
        fwd_index_ids,
        fx_index_ids,
    )
    .unwrap()
}

/// Creates a floating leg kernel (requires forward rate lookups).
fn create_floating_kernel() -> PricingKernel {
    let num_flows = 20;
    let start_date = 19000;

    let payment_dates: Vec<i32> = (0..num_flows)
        .map(|i| start_date + (i as i32 + 1) * 91)
        .collect();

    let fixing_dates: Vec<i32> = payment_dates
        .iter()
        .map(|&d| d - 2)
        .collect();

    let year_fractions: Vec<f64> = vec![0.25; num_flows];
    let notionals: Vec<f64> = vec![1_000_000.0; num_flows];
    let spreads: Vec<f64> = vec![0.001; num_flows]; // 10bp spread
    let gearings: Vec<f64> = vec![1.0; num_flows]; // LIBOR * 1.0

    let currency_ids: Vec<u8> = vec![0; num_flows];
    let discount_curve_ids: Vec<u8> = vec![1; num_flows];
    let fwd_index_ids: Vec<u16> = vec![1; num_flows]; // Use forward curve
    let fx_index_ids: Vec<u16> = vec![0; num_flows];

    PricingKernel::new(
        payment_dates,
        fixing_dates,
        year_fractions,
        notionals,
        spreads,
        gearings,
        currency_ids,
        discount_curve_ids,
        fwd_index_ids,
        fx_index_ids,
    )
    .unwrap()
}

/// Creates a swap kernel (pay fixed, receive floating).
fn create_swap_kernel() -> PricingKernel {
    let num_flows = 40; // 20 pay + 20 receive
    let start_date = 19000;

    // Pay leg (fixed)
    let mut payment_dates: Vec<i32> = (0..20)
        .map(|i| start_date + (i as i32 + 1) * 91)
        .collect();

    // Receive leg (floating)
    payment_dates.extend((0..20).map(|i| start_date + (i as i32 + 1) * 91));

    let fixing_dates: Vec<i32> = payment_dates
        .iter()
        .map(|&d| d - 2)
        .collect();

    let year_fractions: Vec<f64> = vec![0.25; num_flows];
    let notionals: Vec<f64> = vec![1_000_000.0; num_flows];

    // Fixed leg: 3%, Floating leg: spread of 10bp
    let mut spreads: Vec<f64> = vec![0.03; 20];
    spreads.extend(vec![0.001; 20]);

    // Fixed leg: gearing 0, Floating leg: gearing 1
    let mut gearings: Vec<f64> = vec![0.0; 20];
    gearings.extend(vec![1.0; 20]);

    let currency_ids: Vec<u8> = vec![0; num_flows];
    let discount_curve_ids: Vec<u8> = vec![1; num_flows];

    // Fixed leg: no forward index, Floating leg: forward index 1
    let mut fwd_index_ids: Vec<u16> = vec![0; 20];
    fwd_index_ids.extend(vec![1; 20]);

    let fx_index_ids: Vec<u16> = vec![0; num_flows];

    PricingKernel::new(
        payment_dates,
        fixing_dates,
        year_fractions,
        notionals,
        spreads,
        gearings,
        currency_ids,
        discount_curve_ids,
        fwd_index_ids,
        fx_index_ids,
    )
    .unwrap()
}

// =============================================================================
// Single Kernel Benchmarks
// =============================================================================

/// Benchmark single kernel pricing (baseline).
fn bench_single_kernel(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_kernel");
    group.sample_size(100);

    let provider = FlatCurveProvider::new(0.03, 0.03);
    let context = KernelContext::new(&provider);

    // Fixed leg
    group.bench_function("fixed_leg_20_flows", |b| {
        let kernel = create_irs_kernel();
        b.iter(|| black_box(LinearEngine::price(&kernel, &context)));
    });

    // Floating leg
    group.bench_function("floating_leg_20_flows", |b| {
        let kernel = create_floating_kernel();
        b.iter(|| black_box(LinearEngine::price(&kernel, &context)));
    });

    // Full swap
    group.bench_function("swap_40_flows", |b| {
        let kernel = create_swap_kernel();
        b.iter(|| black_box(LinearEngine::price(&kernel, &context)));
    });

    group.finish();
}

// =============================================================================
// Batch Evaluation Benchmarks
// =============================================================================

/// Benchmark batch evaluation throughput.
///
/// Target: 10,000 trades should complete in < 100ms.
fn bench_batch_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_evaluation");
    group.sample_size(50);

    let provider = FlatCurveProvider::new(0.03, 0.03);
    let context = KernelContext::new(&provider);

    // Test different batch sizes
    for batch_size in [10, 100, 1_000, 10_000] {
        // Pre-create kernels
        let kernels: Vec<PricingKernel> = (0..batch_size)
            .map(|_| create_swap_kernel())
            .collect();

        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("swap_batch", batch_size),
            &kernels,
            |b, kernels| {
                b.iter(|| {
                    let sum: f64 = kernels
                        .iter()
                        .map(|k| LinearEngine::price(k, &context))
                        .sum();
                    black_box(sum)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark linear scaling verification.
///
/// Verifies O(n) scaling where n = number of trades.
fn bench_linear_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("linear_scaling");
    group.sample_size(30);

    let provider = FlatCurveProvider::new(0.03, 0.03);
    let context = KernelContext::new(&provider);

    // Fixed leg kernels
    for batch_size in [100, 500, 1_000, 5_000, 10_000] {
        let kernels: Vec<PricingKernel> = (0..batch_size)
            .map(|_| create_irs_kernel())
            .collect();

        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("fixed_leg", batch_size),
            &kernels,
            |b, kernels| {
                b.iter(|| {
                    let sum: f64 = kernels
                        .iter()
                        .map(|k| LinearEngine::price(k, &context))
                        .sum();
                    black_box(sum)
                });
            },
        );
    }

    // Floating leg kernels (more computation)
    for batch_size in [100, 500, 1_000, 5_000, 10_000] {
        let kernels: Vec<PricingKernel> = (0..batch_size)
            .map(|_| create_floating_kernel())
            .collect();

        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("floating_leg", batch_size),
            &kernels,
            |b, kernels| {
                b.iter(|| {
                    let sum: f64 = kernels
                        .iter()
                        .map(|k| LinearEngine::price(k, &context))
                        .sum();
                    black_box(sum)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Kernel Size Benchmarks
// =============================================================================

/// Benchmark different kernel sizes (flow counts).
fn bench_kernel_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("kernel_sizes");
    group.sample_size(100);

    let provider = FlatCurveProvider::new(0.03, 0.03);
    let context = KernelContext::new(&provider);

    // Create kernels with different flow counts
    for num_flows in [5, 10, 20, 40, 80, 120] {
        let start_date = 19000;

        let payment_dates: Vec<i32> = (0..num_flows)
            .map(|i| start_date + (i as i32 + 1) * 91)
            .collect();

        let fixing_dates: Vec<i32> = payment_dates
            .iter()
            .map(|&d| d - 2)
            .collect();

        let year_fractions: Vec<f64> = vec![0.25; num_flows];
        let notionals: Vec<f64> = vec![1_000_000.0; num_flows];
        let spreads: Vec<f64> = vec![0.03; num_flows];
        let gearings: Vec<f64> = vec![0.0; num_flows];
        let currency_ids: Vec<u8> = vec![0; num_flows];
        let discount_curve_ids: Vec<u8> = vec![1; num_flows];
        let fwd_index_ids: Vec<u16> = vec![0; num_flows];
        let fx_index_ids: Vec<u16> = vec![0; num_flows];

        let kernel = PricingKernel::new(
            payment_dates,
            fixing_dates,
            year_fractions,
            notionals,
            spreads,
            gearings,
            currency_ids,
            discount_curve_ids,
            fwd_index_ids,
            fx_index_ids,
        )
        .unwrap();

        group.bench_with_input(
            BenchmarkId::new("flows", num_flows),
            &kernel,
            |b, kernel| {
                b.iter(|| black_box(LinearEngine::price(kernel, &context)));
            },
        );
    }

    group.finish();
}

// =============================================================================
// Decomposed Pricing Benchmarks
// =============================================================================

/// Benchmark decomposed pricing (detailed output).
fn bench_decomposed_pricing(c: &mut Criterion) {
    let mut group = c.benchmark_group("decomposed_pricing");
    group.sample_size(100);

    let provider = FlatCurveProvider::new(0.03, 0.03);
    let context = KernelContext::new(&provider);

    // Single kernel
    group.bench_function("swap_decomposed", |b| {
        let kernel = create_swap_kernel();
        b.iter(|| black_box(LinearEngine::price_decomposed(&kernel, &context)));
    });

    // Compare with simple price
    group.bench_function("swap_simple", |b| {
        let kernel = create_swap_kernel();
        b.iter(|| black_box(LinearEngine::price(&kernel, &context)));
    });

    group.finish();
}

// =============================================================================
// Memory Access Pattern Benchmarks
// =============================================================================

/// Benchmark sequential vs random access patterns.
///
/// Linear pricing should benefit from sequential memory access.
fn bench_access_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("access_patterns");
    group.sample_size(50);

    let provider = FlatCurveProvider::new(0.03, 0.03);
    let context = KernelContext::new(&provider);

    let batch_size = 1_000;
    let kernels: Vec<PricingKernel> = (0..batch_size)
        .map(|_| create_swap_kernel())
        .collect();

    // Sequential access
    group.bench_function("sequential", |b| {
        b.iter(|| {
            let sum: f64 = kernels
                .iter()
                .map(|k| LinearEngine::price(k, &context))
                .sum();
            black_box(sum)
        });
    });

    // Random access (simulate cache misses)
    let indices: Vec<usize> = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        (0..batch_size)
            .map(|i| {
                let mut hasher = DefaultHasher::new();
                i.hash(&mut hasher);
                (hasher.finish() as usize) % batch_size
            })
            .collect()
    };

    group.bench_function("random_order", |b| {
        b.iter(|| {
            let sum: f64 = indices
                .iter()
                .map(|&i| LinearEngine::price(&kernels[i], &context))
                .sum();
            black_box(sum)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_kernel,
    bench_batch_evaluation,
    bench_linear_scaling,
    bench_kernel_sizes,
    bench_decomposed_pricing,
    bench_access_patterns,
);
criterion_main!(benches);
