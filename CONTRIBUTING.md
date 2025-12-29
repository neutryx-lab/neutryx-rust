### 1. Development Protocol

**File:** `CONTRIBUTING.md`

# Neutryx Development Protocol: Spec-Driven Development

## 1. Objective

To ensure mathematical consistency, reproducibility, and rigorous documentation across the Neutryx codebase by strictly adhering to a Specification-Driven Development (SDD) workflow.

## 2. Standards & Conventions

* **Documentation Language:** Japanese or British English.
* **Code & Comments:** British English.
* **Comment Style:** Minimal, essential, and strictly formal.
* **Verification:** All numerical implementations must include verifiable test cases (e.g., checking positive definiteness, convergence rates).

## 3. Workflow

### Phase I: Context Synchronisation

Execute upon session initialisation or significant architectural changes to align the AI agent's context.

```bash
> /kiro:steering
```

* **Action:** Verify that `tech.md` accurately reflects the latest JAX/Rust crate versions.

### Phase II: Specification & Design

**Do not commence coding without an approved design.**

1. **Initialisation:** Define the scope of the new module or feature.
```bash
> /kiro:spec-init "Brief description of the feature"
```


2. **Requirements Definition:** Define acceptance criteria and mathematical constraints.
```bash
> /kiro:spec-requirements <feature-name>
```


3. **Technical Design:** Generate the architecture and interface definitions.
```bash
> /kiro:spec-design <feature-name>
```


* **Mandate:** Review `design.md` to ensure variable naming conventions and interface signatures are logically consistent.


4. **Task Breakdown:** Decompose the design into atomic implementation steps.
```bash
> /kiro:spec-tasks <feature-name>
```



### Phase III: Implementation (TDD)

Execute implementation tasks sequentially.

```bash
> /kiro:spec-impl <feature-name> <task-id>
```

* **Constraint:** If the AI suggests deviations from `design.md`, reject the change and enforce the original specification.

### Phase IV: Status & Verification

Monitor progress and sign off on artefacts.

```bash
> /kiro:spec-status <feature-name>
```
