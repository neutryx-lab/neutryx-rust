# Research & Design Decisions

## Summary
- **Feature**: `stochastic-models-enzyme-ad`
- **Discovery Scope**: Complex Integration
- **Key Findings**:
  - QE (Quadratic Exponential) scheme by Andersen (2008) is the industry standard for Heston simulation
  - SABR Hagan formula has numerical stability issues near ATM; smooth approximations required
  - Enzyme Rust integration requires nightly toolchain with specific intrinsics API

## Research Log

### Heston Model QE Discretization Scheme

- **Context**: Requirement 2 specifies QE discretization for variance process simulation
- **Sources Consulted**:
  - [Andersen 2008 Paper](https://www.ressources-actuarielles.net/EXT/ISFA/1226.nsf/0/1826b88b152e65a7c12574b000347c74/$FILE/LeifAndersenHeston.pdf)
  - [UvA Efficient Simulation Paper](https://pure.uva.nl/ws/files/4291187/64392_302222.pdf)
  - [GitHub QE Implementation](https://github.com/Artemisia-DL/Heston-Model-Simulation)

- **Findings**:
  - QE scheme samples V(t+dt) given V(t) using switching rule based on psi ratio
  - When psi < psi_c (typically 1.5): use quadratic (squared Gaussian) approximation
  - When psi >= psi_c: use exponential approximation with moment matching
  - Asset price uses mid-point rule: U = (V(t) + V(t+dt)) / 2
  - Martingale correction required for drift-free simulation
  - Coefficients K0-K4 depend on gamma1, gamma2 averaging parameters

- **Implications**:
  - Switching rule requires smooth approximation for AD compatibility
  - Must implement both quadratic and exponential branches
  - Correlated Brownian motion generation needed (Cholesky decomposition)
  - Feller condition check determines variance floor behavior

### SABR Model Hagan Formula

- **Context**: Requirement 3 specifies Hagan formula for implied volatility calculation
- **Sources Consulted**:
  - [SABR Wikipedia](https://en.wikipedia.org/wiki/SABR_volatility_model)
  - [Hagan SABR Notes](http://www.frouah.com/finance%20notes/The%20SABR%20Model.pdf)
  - [Imperial College SABR Thesis](https://www.imperial.ac.uk/media/imperial-college/faculty-of-natural-sciences/department-of-mathematics/math-finance/Cheng_Luo-thesis.pdf)
  - [Learning Exact SABR (2025)](https://arxiv.org/html/2510.10343v1)

- **Findings**:
  - Standard Hagan formula: sigma_B = alpha / (F*K)^((1-beta)/2) * z/x(z) * (1 + higher_order_terms)
  - ATM formula simplifies when F = K
  - Numerical instability occurs for:
    - Small strikes (arbitrage in density)
    - F/K ratio very close to 1 (log singularity)
    - beta = 0 (Normal SABR mode)
  - Higher order terms include: (1-beta)^2/24 * alpha^2 / (FK)^(1-beta) + rho*beta*nu*alpha / 4*(FK)^((1-beta)/2) + (2-3*rho^2)/24 * nu^2

- **Implications**:
  - Need smooth_log for log(F/K) computation near ATM
  - ATM expansion formula required for |F-K| < epsilon threshold
  - beta = 0 and beta = 1 are special cases with simplified formulas
  - smooth_pow required for (F*K)^((1-beta)/2) computation

### Enzyme Rust Integration

- **Context**: Requirements 4-6 specify Enzyme AD context and path generation
- **Sources Consulted**:
  - [Enzyme AD Official](https://enzyme.mit.edu/)
  - [Rust autodiff intrinsic](https://doc.rust-lang.org/nightly/std/intrinsics/fn.autodiff.html)
  - [GSoC 2025 Enzyme Rust](https://blog.karanjanthe.me/posts/enzyme-autodiff-rust-gsoc/)
  - [Enzyme Limitations](https://enzyme.mit.edu/rust/limitations.html)

- **Findings**:
  - Rust nightly provides `#[autodiff_forward]` and `#[autodiff_reverse]` macros
  - Activity annotations: Const, Dual (forward), Active/Duplicated (reverse)
  - Forward mode: returns (primal, tangent) tuple
  - Reverse mode: returns adjoint values for active inputs
  - Checkpointing not yet exposed to user in Rust bindings
  - LLVM type analysis can be slow for complex Rust types
  - Thread-local storage supported via standard Rust mechanisms

- **Implications**:
  - EnzymeContext should use thread_local! for mode storage
  - Finite difference fallback required until Enzyme integration complete
  - Manual tangent/adjoint propagation needed for current phase
  - Checkpointing via recomputation strategy initially

### Memory Management for Adjoint Mode

- **Context**: Requirement 6.4 specifies checkpointing for memory management
- **Sources Consulted**:
  - [Enzyme Checkpointing](https://c.wsmoses.com/papers/enzymePar.pdf)
  - [Algorithm 799: revolve](https://dl.acm.org/doi/abs/10.1145/347837.347846)

- **Findings**:
  - Enzyme caches values needed in reverse pass automatically
  - For fixed-size loops, single allocation per instruction cached
  - Alias analysis determines recomputation vs caching
  - Optimal checkpointing is NP-hard; heuristics used
  - Binomial checkpointing (revolve) balances memory/compute

- **Implications**:
  - Initial implementation: store all intermediate states
  - Fallback: recomputation with fixed checkpoint intervals
  - Future: leverage Enzyme's built-in checkpointing when available

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Enum-based Static Dispatch | Single StochasticModelEnum wrapping all models | Zero-cost abstraction, Enzyme-compatible, compile-time monomorphization | Adding models requires enum variant | Aligns with steering principles |
| Generic Trait + Monomorphization | StochasticModel<T: Float> trait with generic functions | Flexible, type-safe | May generate large binaries | Current pricer_core pattern |
| State Machine Pattern | Model as state with transition functions | Clear state management | Overhead for simple evolve step | Not suitable for hot path |

**Selected**: Enum-based Static Dispatch combined with generic trait for Float compatibility.

## Design Decisions

### Decision: StochasticModelEnum for Static Dispatch

- **Context**: Requirement 1.6 prohibits Box<dyn Trait>; models must use static dispatch
- **Alternatives Considered**:
  1. Box<dyn StochasticModel> - dynamic dispatch, simple but incompatible with Enzyme
  2. Individual model structs without common interface - no polymorphism
  3. Enum wrapping all model variants - static dispatch, pattern matching
- **Selected Approach**: StochasticModelEnum<T: Float> with variants (GBM, Heston, SABR)
- **Rationale**: Matches existing Instrument enum pattern; Enzyme-compatible; zero-cost dispatch
- **Trade-offs**: Must update enum when adding new models; match exhaustiveness checked at compile time
- **Follow-up**: Verify Enzyme can optimize through match expressions

### Decision: Thread-Local EnzymeContext

- **Context**: Requirement 4.5 requires thread-safe AD mode management
- **Alternatives Considered**:
  1. Global static mutex - contention in parallel scenarios
  2. Thread-local storage - per-thread state, no contention
  3. Per-function parameter - clutters API, explicit passing
- **Selected Approach**: thread_local! macro with RefCell<EnzymeContext>
- **Rationale**: Zero-contention parallel access; natural per-thread mode switching
- **Trade-offs**: Cannot share context across threads; must set mode per thread
- **Follow-up**: Benchmark against explicit parameter passing

### Decision: Smooth Switching for QE Scheme

- **Context**: QE scheme switches between quadratic and exponential based on psi threshold
- **Alternatives Considered**:
  1. Hard if/else switch - creates discontinuity, AD-incompatible
  2. Smooth blend using smooth_indicator - differentiable everywhere
  3. Always use one branch - loses accuracy
- **Selected Approach**: smooth_indicator(psi - psi_c, epsilon) to blend branches
- **Rationale**: Maintains differentiability; converges to true QE as epsilon -> 0
- **Trade-offs**: Small epsilon increases numerical sensitivity; performance overhead
- **Follow-up**: Tune epsilon for accuracy vs stability trade-off

### Decision: Two-Phase AD Implementation

- **Context**: Full Enzyme integration not complete; need working AD now
- **Alternatives Considered**:
  1. Wait for Enzyme - blocks progress
  2. num-dual only - works but not LLVM-optimized
  3. Manual tangent/adjoint with Enzyme migration path
- **Selected Approach**: Manual tangent/adjoint propagation with Enzyme-ready API
- **Rationale**: Unblocks development; num-dual verification; clear migration path
- **Trade-offs**: Duplicate code for manual propagation; will be replaced
- **Follow-up**: Phase 4 replaces with #[autodiff_*] macros

### Decision: HestonWorkspace with 2D State

- **Context**: Heston requires 2D state (S, V) vs GBM's 1D state
- **Alternatives Considered**:
  1. Separate price and variance arrays - cache unfriendly
  2. Interleaved (S, V, S, V, ...) - complex indexing
  3. Two parallel arrays with same layout - simple extension of PathWorkspace
- **Selected Approach**: HestonWorkspace with parallel prices[] and variances[] arrays
- **Rationale**: Matches PathWorkspace pattern; simple indexing; cache-friendly per-array
- **Trade-offs**: Memory not strictly contiguous for (S,V) pairs
- **Follow-up**: Profile cache performance with real workloads

## Risks & Mitigations

- **Risk 1: Enzyme Rust compilation slowness** - Mitigation: Keep differentiated functions simple; use feature flag to enable/disable Enzyme
- **Risk 2: QE smooth switching accuracy** - Mitigation: Validate against exact QE with numerical tests; tune epsilon
- **Risk 3: SABR numerical instability at extreme strikes** - Mitigation: Implement ATM expansion; add boundary checks with clear errors
- **Risk 4: Checkpointing memory explosion for long paths** - Mitigation: Implement recomputation fallback; limit maximum steps

## References

- [Andersen (2008) QE Scheme Paper](https://www.ressources-actuarielles.net/EXT/ISFA/1226.nsf/0/1826b88b152e65a7c12574b000347c74/$FILE/LeifAndersenHeston.pdf) - Primary reference for Heston QE discretization
- [Hagan SABR Model Notes](http://www.frouah.com/finance%20notes/The%20SABR%20Model.pdf) - SABR formula derivation and special cases
- [Enzyme AD Documentation](https://enzyme.mit.edu/) - Official Enzyme project documentation
- [Rust autodiff Intrinsic](https://doc.rust-lang.org/nightly/std/intrinsics/fn.autodiff.html) - Rust nightly autodiff API reference
- [Enzyme Rust Limitations](https://enzyme.mit.edu/rust/limitations.html) - Current Enzyme-Rust integration limitations
