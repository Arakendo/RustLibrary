# Corpus Campaigns

This directory groups sustained executable evidence by architectural campaign.
Each campaign should align with an Architectural Review, ADR, plan, roadmap
item, or current design claim and answer one bounded question.

Create a descriptive lowercase directory and copy `TEMPLATE.md` into it as
`README.md`. Add structured metadata only when automation needs it; do not
invent a schema before repeated campaigns require the same fields.

## Statuses

- **Proposed** — scoped but not started.
- **Active** — evidence collection is in progress.
- **Awaiting Review** — evidence is ready for architectural judgment.
- **Parked** — intentionally dormant with a reopening trigger.
- **Complete** — the campaign's evidence and disposition are recorded.
- **Superseded** — another named campaign owns the question.

## Index

- [RustEditor Campaign](rust-editor/README.md): editor-owned drafts and save conflicts composed with shared resource, rope, and diff crates.
