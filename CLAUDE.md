# AI-DLC and Spec-Driven Development

Kiro-style Spec Driven Development implementation on AI-DLC (AI Development Life Cycle)

## Project Context

**Neutryx** is a production-grade **derivatives pricing library** for Tier-1 banks, featuring multi-asset class coverage (Rates, FX, Equity, Credit, Commodity), Enzyme automatic differentiation for high-performance Greeks, and integrated XVA/risk analytics.

### Architecture: A-I-P-S Stream

The workspace enforces a strict unidirectional data flow:

```text
A: Adapter   → adapter_feeds, adapter_fpml, adapter_loader
I: Infra     → infra_config, infra_master, infra_store
P: Pricer    → pricer_core (L1), pricer_models (L2), pricer_optimiser (L2.5), pricer_pricing (L3), pricer_risk (L4)
S: Service   → service_cli, service_gateway, service_python
```

**Dependency Rules**:
1. **S**ervices may depend on any **P**, **I**, or **A** crate.
2. **P**ricer crates must never depend on **S** or **A** crates.
3. **I**nfra crates must never depend on **P** or **S** crates.
4. **A**dapter crates depend only on **I** (for definitions) or **P** (for target types), never on **S**.

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
