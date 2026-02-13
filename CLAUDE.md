# Neutryx Development Guide

## Project Context

**Neutryx** is a production-grade **derivatives pricing library** for Tier-1 banks, featuring multi-asset class coverage (Rates, FX, Equity, Credit, Commodity), Enzyme automatic differentiation for high-performance Greeks, and integrated XVA/risk analytics.

### Architecture: A-I-P-S Stream

The workspace enforces a strict unidirectional data flow:

```text
A: Adapter   → adapter_feeds, adapter_loader (incl. fpml feature)
I: Infra     → infra_config, infra_domain, infra_store
P: Pricer    → pricer_core (L1), pricer_models (L2), pricer_pricing (L3), pricer_risk (L4)
S: Service   → service_gateway (unified: REST API + CLI feature + Python feature)
```

**Dependency Rules**:
1. **S**ervices may depend on any **P**, **I**, or **A** crate.
2. **P**ricer crates must never depend on **S** or **A** crates.
3. **I**nfra crates must never depend on **P** or **S** crates.
4. **A**dapter crates depend only on **I** (for definitions) or **P** (for target types), never on **S**.

## Tier 1 Minimalist Architecture (CRITICAL)

To achieve Tier 1 diversity (Exotics, SABR/Heston, etc.) with **minimal code**, strict adherence to these patterns is required:

1.  **Enum Dispatch over Boilerplate**:
    - Do NOT create isolated structs for every new model or product.
    - **Extend existing Enums** (e.g., `VolatilitySurface`, `ProductType`) and use Rust's `enum_dispatch` or `match` patterns.
    - Polymorphism should happen at the Enum level to simplify serialization/deserialization boundaries with the GUI.

2.  **Data-Driven over Hard-Coding**:
    - **Exotic Products** (TARF, Autocallables) must be implemented via the **Script Engine** (`pricer_core::script_kernel`) or configuration structs, NOT by writing new Rust structs for each payout type.
    - Payoff logic should be composed, not duplicated.

3.  **Strict Reuse**:
    - Reuse `pricer_core` math primitives (interpolators, solvers, random number generators).
    - Never duplicate Monte Carlo path generation logic; extend the `Process` trait instead.

4.  **GUI-First Integration**:
    - Any structural change to `infra_domain` (Enums/Structs) MUST be immediately reflected in `service_gateway` (JSON DTOs) and verified against `demo/gui` (Vue.js).
    - Breaking the GUI is a critical failure.

## Agent Team Protocols (Experimental)

When operating as an Agent Team:

1.  **Phase 1: Architecture Consensus**:
    - Before writing code, the **Architect Agent** must propose the Enum/Trait abstraction.
    - Other agents must validate that this abstraction covers their use cases (e.g., "Does this Enum support Heston parameters?").
    - **NO CODING** until consensus is reached via `SendMessage`.

2.  **Phase 2: Parallel Implementation**:
    - Agents work on assigned crates defined in the A-I-P-S stream.
    - Code comments must be in **British English**.

3.  **Phase 3: Integration & Verification**:
    - The **Integration Agent** (usually assigned to `S` layer) acts as the gatekeeper.
    - They must verify that `cargo run --bin service_gateway` serves the new data correctly to the GUI.

## Development Guidelines

- Think in English, generate responses in Japanese.
- **British English** in code: Use `optimiser`, `serialisation`, `visualisation`, `modelling`
- Follow the user's instructions precisely, and within that scope act autonomously.

## Project Knowledge

- **Steering**: `.kiro/steering/` — project-wide context (product.md, tech.md, structure.md, roadmap.md). Load as needed.
- **Roadmap**: `.kiro/steering/roadmap.md` — source of truth for project status. Reference when planning, update when completing work.

## Spec Workflow (use selectively)

Specs (`.kiro/specs/`) formalise the development process. Use them **only for large features** that require architectural decisions across multiple crates. For small/medium changes, skip specs and implement directly.

**When to use a spec**: New asset class, new pricing engine, cross-layer refactoring, new GUI feature with backend changes.
**When NOT to use a spec**: Bug fixes, single-crate changes, migrations, small refactorings, dependency updates.

When using specs:
- `/kiro:spec-init` → `/kiro:spec-requirements` → `/kiro:spec-tasks` → `/kiro:spec-impl`
- Skip `/kiro:spec-design` unless genuine architectural trade-offs exist
- 53 completed specs are deleted (git history preserved). Summary in `.kiro/steering/roadmap.md`
