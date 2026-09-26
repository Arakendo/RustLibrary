# {{PROJECT_NAME}} Testing Strategy

| Field | Value |
| --- | --- |
| Status | Draft |
| Authority | Current testing policy |
| Owner | Team or role |
| Last reviewed | YYYY-MM-DD |
| Review cadence | On material change |

## Purpose

Define what the project validates, at which boundaries, and with what evidence.
Connect validation layers to accepted
[requirements](product/requirements/), [quality attributes](quality/), and
[security constraints](security/).

## Principles

- Test at the narrowest boundary that honestly proves the behavior.
- Test public meaning and contracts, not incidental implementation.
- Treat failure behavior as part of the contract.
- Keep nondeterminism and environmental assumptions visible.

## Validation Layers

### Unit Tests

- Scope:
- Location:
- Command:

### Integration Tests

- Scope:
- Location:
- Command:

### End-To-End Or System Tests

- Scope:
- Location:
- Command:

### Contract, Compatibility, Or Corpus Tests

- Scope:
- Evidence:
- Command:

## Test Placement

| Behavior | Narrowest honest layer | Notes |
| --- | --- | --- |
| Example behavior | Unit / integration / system | |

## Fixtures, Golden Files, And Snapshots

State when these artifacts are appropriate, how they are reviewed, and how
intentional updates are distinguished from regressions.

## Execution Tiers

### Fast Local

```text
command
```

### Full Validation

```text
command
```

### Scheduled Or Target-Specific

```text
command
```

## Continuous Integration

Describe required checks, supported environments, and release gates.

## Known Gaps

- Gap:

## Admission Rule For New Test Infrastructure

Require a concrete behavior or risk that existing test layers cannot validate
clearly before adding a new framework, harness, or fixture system.
