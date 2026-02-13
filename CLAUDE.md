# AI-DLC and Spec-Driven Development

Kiro-style Spec Driven Development implementation on AI-DLC (AI Development Life Cycle)

## Project Context

**Neutryx** is a production-grade **derivatives pricing library** for Tier-1 banks, featuring multi-asset class coverage (Rates, FX, Equity, Credit, Commodity), Enzyme automatic differentiation for high-performance Greeks, and integrated XVA/risk analytics.

### Architecture: A-I-P-S Stream

The workspace enforces a strict unidirectional data flow:

```text
A: Adapter   → adapter_feeds, adapter_loader (incl. fpml feature)
I: Infra     → infra_config, infra_domain, infra_store
P: Pricer    → pricer_core (L1), pricer_models (L2), pricer_pricing (L3), pricer_risk (L4)
S: Service   → service_cli, service_gateway, service_python
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

### Paths
- Steering: `.kiro/steering/`
- Specs: `.kiro/specs/`

### Steering vs Specification

**Steering** (`.kiro/steering/`) - Guide AI with project-wide rules and context
**Specs** (`.kiro/specs/`) - Formalise development process for individual features

### Active Specifications
- Check `.kiro/specs/` for active specifications
- Use `/kiro:spec-status [feature-name]` to check progress

## Development Guidelines
- Think in English, generate responses in Japanese. All Markdown content written to project files (e.g., requirements.md, design.md, tasks.md, research.md, validation reports) MUST be written in the target language configured for this specification (see spec.json.language).
- **British English**: Use `optimiser`, `serialisation`, `visualisation`, `modelling`

## Minimal Workflow
- Phase 0 (optional): `/kiro:steering`, `/kiro:steering-custom`
- Phase 1 (Specification):
  - `/kiro:spec-init "description"`
  - `/kiro:spec-requirements {feature}`
  - `/kiro:validate-gap {feature}` (optional: for existing codebase)
  - `/kiro:spec-design {feature} [-y]`
  - `/kiro:validate-design {feature}` (optional: design review)
  - `/kiro:spec-tasks {feature} [-y]`
- Phase 2 (Implementation): `/kiro:spec-impl {feature} [tasks]`
  - `/kiro:validate-impl {feature}` (optional: after implementation)
- Progress check: `/kiro:spec-status {feature}` (use anytime)

## Development Rules
- 3-phase approval workflow: Requirements → Design → Tasks → Implementation
- Human review required each phase; use `-y` only for intentional fast-track
- Keep steering current and verify alignment with `/kiro:spec-status`
- Follow the user's instructions precisely, and within that scope act autonomously: gather the necessary context and complete the requested work end-to-end in this run, asking questions only when essential information is missing or the instructions are critically ambiguous.

## Steering Configuration
- Load entire `.kiro/steering/` as project memory
- Default files: `product.md`, `tech.md`, `structure.md`, `roadmap.md`
- Custom files are supported (managed via `/kiro:steering-custom`)

### Roadmap Maintenance

- `roadmap.md` tracks implementation status and future development items
- **Reference** when: planning new features, checking current state, prioritising work
- **Update** when: completing specs, changing priorities, adding/removing development items
- Run `/kiro:spec-status` to verify alignment between specs and roadmap
