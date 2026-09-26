# Documentation Guidance For Agents

The current source-of-truth design document is `docs/design-document.md`.
If a change alters architecture, ownership boundaries, public contracts, or
milestone expectations, update the design document or add an ADR rather than
letting code and documentation drift apart.

- Treat hand-authored current documents and accepted ADRs as authoritative.
- Read accepted product requirements, security/privacy policy, and relevant
  quality constraints before changing externally meaningful behavior.
- Read relevant ADRs before adding a subsystem, changing ownership, or adding a
  dependency across an established boundary.
- Use Architectural Review records for unresolved architectural questions,
  evidence, alternatives, and deferred or rejected proposals.
- Treat `corpus/` entries as executable architectural evidence, not examples or
  accepted policy. Link each campaign to the design, plan, AR, or ADR claim that
  consumes its evidence.
- Keep ordinary tests, reviewed goldens, and corpus campaigns distinct. A test
  fixture does not become architectural evidence merely because a campaign
  references it.
- Do not let a plan silently override an ADR or current design document.
- Treat notes, conversations, incidents, and audits as evidence, not accepted
  policy.
- Keep guides task-oriented and reference pages factual. Do not hide new
  architecture or compatibility promises inside instructional prose.
- Keep tutorials learning-oriented and independently verifiable. Move stable
  facts into reference pages instead of duplicating them across tutorials.
- Treat runbooks and migration guides as safety-sensitive procedures: make
  targets, prerequisites, verification, rollback, and irreversible steps
  explicit.
- Keep archived documents for history; do not cite them as current policy
  unless a current document explicitly adopts their content.
- Keep `docs/README.md` indexes and links current when documents are added,
  moved, superseded, or archived.
- Use release records to coordinate evidence and communication; do not let a
  release checklist redefine product, architecture, security, or quality
  policy.
