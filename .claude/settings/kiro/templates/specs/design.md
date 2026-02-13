# Technical Design: {{FEATURE_NAME}}

## Overview

**Purpose**: [1-2 sentences: what this feature delivers and to whom]

### Goals
- Goal 1
- Goal 2

## Architecture

> Reference `research.md` for background investigation. Keep design.md self-contained.

### Existing Architecture (if modifying)
- Current patterns and constraints
- Integration points to maintain

### Architecture Decisions
- Selected pattern and rationale
- Domain boundaries
- Steering compliance

**RECOMMENDED**: Include Mermaid diagram for complex features.

## Components

| Component | Layer | Intent | Requirements |
|-----------|-------|--------|--------------|
| Example | pricer_core | Brief purpose | 1.1, 1.2 |

### [Component Name]

**Responsibilities**: Brief description of what this component owns.

**Dependencies**: Inbound/outbound component relationships.

**Interface**:
```rust
pub trait ExampleTrait {
    fn method(&self) -> Result<Output, Error>;
}
```

**Key Decisions**: Rationale for non-obvious choices only.

_Repeat per component. Simple components need only a summary row in the table above._

## Data Model

Domain model only. Include Mermaid ER diagram for complex relationships.
- Aggregates and transactional boundaries
- Key entities and value objects
- Business invariants

_Add physical data model only for features requiring specific storage design._

## Error Handling

Error type and propagation pattern for this feature. Reference `error-handling` section in `tech.md` for project-wide patterns.

## Testing Strategy

- Unit: [2-3 key areas]
- Integration: [1-2 cross-component flows]
- Performance: [if applicable]

## References
- Links to papers, docs, or `research.md` sections
