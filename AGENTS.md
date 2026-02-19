# Agent Guidelines

<!-- Do not restructure or delete sections. Update individual values in-place when they change. -->

## Core Principles

- Think in English, generate responses in Japanese.
- **British English** in code: `optimiser`, `serialisation`, `modelling`, `visualisation`.
- **Do NOT maintain backward compatibility** unless explicitly requested. Break things boldly.
- Follow the user's instructions precisely, and within that scope act autonomously.

## Commands

<!-- Update commands when workflows change. -->

- **Stable (L1/L2)**: `cargo clippy --workspace --exclude pricer_pricing --exclude pricer_risk -- -D warnings && cargo test --workspace --exclude pricer_pricing --exclude pricer_risk`
- **Nightly (L3/L4)**: `cargo +nightly test -p pricer_pricing && cargo +nightly test -p pricer_risk`
- **Full integration**: `cargo +nightly test --workspace`

## Architecture

<!-- Rewrite this section when major architectural changes occur. -->

**A-I-P-S** (strict unidirectional): Adapter → Infra → Pricer (L1–L4) → Service. Violations fail CI.
**Dependency rules**: S→any | P→never S,A | I→never P,S | A→only I,P

## Gotchas

- **No `Box<dyn Trait>`** — Enzyme AD requires static dispatch. Use enum + `enum_dispatch`.
- **Smoothing is mandatory** — all discontinuous ops must use `smooth_max` / `smooth_indicator` for Enzyme compatibility.
- **Enzyme = nightly** — `pricer_pricing` and `pricer_risk` require `cargo +nightly`.

## Maintenance Notes

<!-- This section is permanent. Do not delete. -->

**Keep this file lean and current:**

1. **Review regularly** — stale instructions poison the agent's context
2. **CRITICAL: Keep total under 20-30 lines of instructions** — move detailed docs to `.claude/steering/`
3. **Update commands immediately** when workflows change
4. **Rewrite Architecture section** when major architectural changes occur
5. **Delete anything the agent can infer** from the code
