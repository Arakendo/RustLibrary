# {{PROJECT_NAME}} Design Document

## Status

| Field | Value |
| --- | --- |
| Status | Draft |
| Authority | Current project design |
| Owner | Team or role |
| Created | YYYY-MM-DD |
| Last reviewed | YYYY-MM-DD |
| Review cadence | On material change |

## Purpose

State what this system is and why this document exists.

## Goals

- Goal:

## Non-Goals

- Non-goal:

## Guiding Principles

- Principle:

## Context And Users

Describe the operating environment, primary users, and important constraints.
Link relevant [product requirements](product/requirements/) rather than
restating them as architecture.

## Architecture Overview

Describe the system at a level that makes ownership and data flow clear.

```text
[Component] -> [Component] -> [External system]
```

## Components And Ownership

### Component Name

- Responsibilities:
- State owned:
- Dependencies:
- Must not own:

## Dependency Rules

- Allowed direction:
- Forbidden direction:

## Data And State

Describe the source of truth, lifecycle, persistence, consistency, and
migration expectations.

## External Interfaces

Describe public APIs, events, protocols, files, or user-facing contracts.

## Security And Privacy

Describe trust boundaries, sensitive data, authorization, retention, and
threats relevant to the design.

## Reliability And Diagnostics

Describe failure behavior, observability, recovery, and operational limits.

## Performance And Scale

Record required capacity, latency, resource, and cost constraints. Omit
speculative targets.

## Design Invariants

- Invariant:

## Testing And Validation

Link to [Testing Strategy](testing-strategy.md) and record design-specific
evidence requirements.

## Open Questions

- Question:

## Definition Of Done

- [ ] Architecture and ownership are documented.
- [ ] Acceptance criteria are testable.
- [ ] Relevant ADRs are linked.
- [ ] Known unsupported cases are explicit.

## Decision Summary

Link the ADRs that constrain this design.

- None yet.
