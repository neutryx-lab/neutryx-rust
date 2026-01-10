# Product Overview

A production-grade **derivatives pricing library** for Tier-1 banks, delivering comprehensive multi-asset class coverage (Rates, FX, Equity, Credit, Commodity) with cutting-edge performance through Enzyme automatic differentiation.

## Architecture: A-I-P-S Stream

The workspace enforces a strict unidirectional data flow that mirrors alphabetical order:

1. **A**dapter: Ingestion and normalisation of external data (The Raw Inputs)
2. **I**nfra: System-wide definitions, persistence, and configuration (The Foundation)
3. **P**ricer: Mathematical modelling, optimisation, and risk computation (The Kernel)
4. **S**ervice: Execution environments and interfaces (The Outputs)

## Core Capabilities

- **Multi-Asset Class Instruments**: Rates (IRS, Swaption, Cap/Floor), FX (options, barriers, forwards), Equity (vanilla, exotic), Credit (CDS), Commodity
- **High-Performance Greeks**: Enzyme LLVM-level AD for C++-competitive differentiation
- **Dual-Mode Verification**: Parallel Enzyme and num-dual backends for correctness validation
- **Monte Carlo Pricing**: Path-dependent options with workspace buffers and checkpointing
- **Analytical Solutions**: Black-Scholes, Garman-Kohlhagen, Kemna-Vorst, barrier formulas
- **XVA & Risk Analytics**: CVA, DVA, FVA calculations with exposure metrics (EE, EPE, PFE, EEPE, ENE)
- **Market Data Infrastructure**: AD-compatible yield curves and volatility surfaces with interpolation
- **Model Calibration**: Heston, SABR, Hull-White calibration with Levenberg-Marquardt optimisation
- **Interest Rate Models**: Hull-White, Cox-Ingersoll-Ross (CIR) with mean reversion
- **Portfolio Analytics**: Parallelised portfolio-level computations with SoA optimisation

## Target Use Cases

- **Trading Desks**: Real-time pricing and risk management for derivatives trading (Rates, FX, Equity, Credit, Commodity)
- **Risk Management**: Portfolio-level XVA, exposure calculations, and regulatory metrics
- **Quantitative Analysis**: Model calibration, validation, and performance benchmarking
- **Research & Validation**: Dual-mode verification enables academic/production validation

## Value Proposition

- **Tier-1 Bank Ready**: Comprehensive asset class coverage for production trading desks
- **Unidirectional Data Flow**: A-I-P-S architecture ensures clear separation of concerns
- **Isolation of Experimental Code**: Enzyme confined to pricer_pricing, keeping 75% of codebase production-stable
- **Correctness First**: Built-in verification through dual AD backends (Enzyme + num-dual)
- **Differentiability by Design**: Smooth approximations replace discontinuities throughout
- **Performance Without Compromise**: Static dispatch and LLVM optimisation for zero-cost abstractions

---
_Created: 2025-12-29_
_Updated: 2026-01-09_ — Pivoted from XVA-focused to comprehensive bank derivatives pricer
_Focus on patterns and purpose, not exhaustive feature lists_
