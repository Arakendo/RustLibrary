# Documentation Policy

| Field | Value |
| --- | --- |
| Status | Draft |
| Authority | Project governance |
| Owner | Documentation owner or maintainers |
| Last reviewed | YYYY-MM-DD |
| Review cadence | Project cadence |

## Objectives

- Current behavior and current documentation should agree.
- Readers should be able to identify authority, ownership, and freshness.
- Historical evidence should remain distinguishable from current policy.

## Required Metadata

Use status, authority, owner, and last-reviewed fields when a document can
become stale or carries a durable contract.

## Ownership And Review

| Document group | Owner | Review trigger | Stale after |
| --- | --- | --- | --- |
| Product and requirements | | Material product change | |
| Design and ADRs | | Architectural change | |
| Guides and tutorials | | Supported workflow change | |
| Reference | | Public contract change | |
| Operations | | Incident or environment change | |

## Change Rules

- Update documentation in the same change as the behavior it describes.
- Update indexes when documents are created, moved, superseded, or archived.
- Link replacements in both directions where historical context matters.
- Do not resolve conflicting documents by silently deleting evidence.

## Health Checks

- [ ] Relative links resolve.
- [ ] Indexed current documents exist.
- [ ] No placeholder remains in an adopted project.
- [ ] Owners and review dates are current.
- [ ] Guides, tutorials, runbooks, and migrations were verified in supported
      environments.
- [ ] Accepted decisions are reflected in current design documents.

## Exceptions

Record who approved an exception, why normal maintenance is not proportionate,
and which event should end the exception.
