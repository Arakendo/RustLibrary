# {{PROJECT_NAME}} Documentation

This directory separates current project truth, accepted decisions, active
work, supporting evidence, and historical material. A document's location
signals how it should be interpreted.

## Source Of Truth

These documents define current architecture and project policy:

- [Product Vision And Requirements](product/)
- [Design Document](design-document.md)
- [Subsystem Designs](design/)
- [Roadmap](roadmap.md)
- [Testing Strategy](testing-strategy.md)
- [Architecture Decision Records](adr/)
- [Project Governance](governance/)
- [Security And Privacy](security/)
- [Quality Attributes](quality/)
- [Support Policy](support/)

Add other cross-cutting authoritative documents to this list as the project
needs them.

## Active Work

- [Architectural Reviews](architecture-reviews/) preserve evidence and
  findings for architectural questions. They do not override accepted ADRs.
- [Plans](plans/) describe executable work. A plan does not change architecture
  by itself.
- [Architectural Corpus](../corpus/) contains optional executable evidence that
  pressures a specific boundary or supported claim. Corpus evidence does not
  override current design or accepted ADRs.
- [Notes](notes/) contain investigations, observations, and validation results.
- [Change Management](changes/) contains supported migrations and deprecation
  policy. This category is optional.
- [Releases](releases/) coordinates delivery readiness, evidence, compatibility,
  and communication.

## Evidence And History

- [Conversations](conversations/) contain source discussions and research
  inputs. Extract durable conclusions into a current document.
- [Audits](audits/) record observed conformance, defects, and risks at a point
  in time.
- [Operations](operations/) contains runbooks and incident records for deployed
  systems. This category is optional.
- [Archive](archive/) contains superseded material and is not current policy.

## User And Developer Documentation

- [Guides](guides/) provide tested, task-oriented instructions.
- [Tutorials](tutorials/) provide learning-oriented, end-to-end experiences.
- [Reference](reference/) provides stable factual material, compatibility
  details, and shared terminology.

## Authority Model

| Document type | Authority | Primary job |
| --- | --- | --- |
| Current design or policy | Binding within its declared scope | Describe current intent and constraints |
| Accepted product requirement | Binding within its declared scope | Define an externally meaningful outcome or constraint |
| Accepted ADR | Binding until superseded | Record an accepted architectural decision |
| Architectural Review | Analytical evidence | Evaluate an architectural question |
| Plan | Actionable intent | Sequence implementation and validation |
| Corpus campaign | Executable evidence | Test an architectural claim or boundary |
| Guide, tutorial, or runbook | Instructional procedure | Help a reader learn or safely complete a task |
| Reference or deprecation record | Factual or compatibility contract | Describe supported behavior and terms |
| Release record | Coordination | Assemble evidence, readiness, and communication |
| Note, audit, conversation, or incident | Observational evidence | Preserve what was learned or observed |
| Archive | Historical | Preserve superseded context |

## Placement Rules

```text
Accepted architectural decision?      -> docs/adr/
Architecture under review?            -> docs/architecture-reviews/
Executable implementation work?       -> docs/plans/
Executable architectural evidence?     -> corpus/
Observation or investigation?         -> docs/notes/
Source conversation or research?       -> docs/conversations/
Product intent or requirement?         -> docs/product/
Subsystem's current architecture?      -> docs/design/
Task-oriented instructions?            -> docs/guides/
Learning-oriented walkthrough?         -> docs/tutorials/
Stable facts, configuration, or terms? -> docs/reference/
Observed conformance, defects, risks?  -> docs/audits/
Ownership or decision process?         -> docs/governance/
Security, privacy, or threat model?     -> docs/security/
Cross-cutting quality expectation?     -> docs/quality/
Runbook or incident learning?          -> docs/operations/
Migration or deprecation?              -> docs/changes/
Release readiness or result?           -> docs/releases/
Supported scope or escalation?         -> docs/support/
Superseded historical material?        -> docs/archive/
Current architecture or policy?        -> docs/
```

## Document Lifecycle

```text
Research, conversation, incident, audit, or observation
        -> Note, requirement, Plan, or Corpus campaign
        -> Design or Architectural Review, when boundaries are in question
        -> ADR, if an architectural change is accepted
        -> Current policy, design, guide, tutorial, reference, or runbook
        -> Release, validation, and support
        -> Archive, when superseded
```

This lifecycle branches and is optional. Research may become a requirement
without changing architecture; an audit may lead directly to a plan; a local
plan may need no architectural review; and an accepted ADR should normally
produce a current design update.

## Maintenance Rules

- Link every current document from this index or a child index.
- Prefer updating current truth over adding a second competing description.
- Mark a replacement explicitly and link both the old and new records.
- Archive superseded prose; never rewrite historical evidence as if it were
  always known.
- Include status, authority, owner, and last-reviewed metadata where staleness
  or document ownership matters.
