# Requirements Document

## Project Description (Input)
Implement missing stochastic volatility models (Heston, SABR) and complete Enzyme AD integration for the Rust quantitative finance library. This includes: 1) Creating a StochasticModel trait abstraction, 2) Implementing HestonModel with mean-reverting variance process and QE discretization, 3) Implementing SABRModel with Hagan implied volatility formula, 4) Completing EnzymeContext for AD mode management, 5) Adding tangent/adjoint path generation for Greeks computation, 6) Integrating models with MonteCarloPricer. All implementations must use smooth approximations (smooth_max, smooth_indicator) for Enzyme LLVM compatibility and support generic Float types for dual-mode verification with num-dual.

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->

