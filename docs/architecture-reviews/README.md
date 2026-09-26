# Architectural Review Records

Architectural Review records preserve questions, evidence, analysis, findings,
dispositions, and reopening criteria. They sit between informal observations
and binding ADRs.

```text
Observation or repeated pressure
    -> Architectural Review
        -> incubate / defer / reject / no change
        -> accept architectural change -> ADR
```

A review does not override an ADR. If its findings require a binding change,
create or supersede an ADR and update the current design document.

## When To Open A Review

Open a review when ownership is unclear, a stable cross-component contract is
proposed, repeated implementation friction challenges a boundary, or a
proposal should be durably deferred or rejected.

Ordinary bug fixes and local refactors that preserve accepted contracts do not
need a review record.

## Naming And Status

Copy `TEMPLATE.md` to `AR-NNNN-short-title.md` and use the next unused number.
Valid statuses are Proposed, Under Review, Incubating, Accepted, Deferred,
Rejected, No Change, Superseded, and Reopened.

## Index

- No reviews recorded.
