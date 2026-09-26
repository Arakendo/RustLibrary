# RustLibrary

A collection of focused, reusable Rust crates for other projects to consume.
Inspired by the toolkit-first organization of the C# `ClassLibrary` project.

This repository is a Cargo workspace with independently consumable crates.
Its first crate is [rustlibrary-ulid](crates/rustlibrary-ulid/README.md), used by
RustEditor for identifiers. [rustlibrary-text-codec](crates/rustlibrary-text-codec/README.md)
provides strict bounded Unicode and legacy text conversion. Run `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings` and
`cargo fmt --all -- --check` here. Vendor source is not part of the workspace.

## Third-party source references

[Servo](https://github.com/servo/servo) is pinned as a Git submodule at
`vendor/Servo` for evaluation by consumers such as RustEditor. It remains
upstream-owned source, not a RustLibrary crate or a selected application engine.
No wrapper, build integration or compatibility claim is introduced by adding it.
Keep upstream notices and licensing intact; evaluate the selected dependency
closure and target requirements before integrating it.

Fetch the pinned source with `git submodule update --init vendor/Servo`.
The initial checkout is shallow; deepen it explicitly if history is needed.
Follow the pinned upstream build instructions for any additional dependencies.
No Servo build or runtime validation has been performed for this addition.

[Lucide](https://github.com/lucide-icons/lucide) is pinned at `vendor/Lucide`
as upstream icon source for consumer evaluation. Adding it does not select a UI
framework, create a RustLibrary icon crate, or integrate icons into RustEditor.
Preserve upstream licenses and notices when using or adapting assets or packages.

Fetch it with `git submodule update --init vendor/Lucide`. The checkout is
shallow; deepen it explicitly if history is needed. No packages were installed,
built or tested as part of this source addition.

## Documentation Scaffold

The documentation fixture below is retained as a starting point. Its placeholders
and sample policies have not yet been adopted as RustLibrary project policy.

## Standard Project Documentation Fixture

This fixture provides a blank documentation system for `{{PROJECT_NAME}}`.

Copy the `docs` directory into the target project. Copy the optional `corpus`
directory when the project uses executable architectural evidence. Replace
every `{{...}}` placeholder, and delete any optional document that the project
will not maintain. Merge `AGENTS.documentation.md` into an existing project-level
`AGENTS.md` instead of overwriting project-specific instructions.
Use [`PROFILES.md`](PROFILES.md) to choose a smaller adoption set without
maintaining duplicate copies of the templates.

The optional [`root-files/`](root-files/) pack contains conventional repository
documents. Review and merge these files individually because existing projects
often already have equivalents.

## Included Documents

| Path | Purpose |
| --- | --- |
| `docs/README.md` | Documentation map and authority rules |
| `docs/product/` | Product intent and accepted requirements |
| `docs/design-document.md` | Current intended architecture and constraints |
| `docs/design/` | Current subsystem architecture and contracts |
| `docs/roadmap.md` | Milestone sequence and status |
| `docs/testing-strategy.md` | Validation policy and test layers |
| `docs/adr/` | Accepted, binding architectural decisions |
| `docs/architecture-reviews/` | Evidence and findings for open architectural questions |
| `docs/plans/` | Actionable implementation work |
| `docs/notes/` | Investigations, observations, and validation results |
| `docs/conversations/` | Preserved source discussions and research inputs |
| `docs/audits/` | Point-in-time conformance, defects, and risk evidence |
| `docs/guides/` | Task-oriented instructions |
| `docs/tutorials/` | Learning-oriented, end-to-end walkthroughs |
| `docs/reference/` | Stable factual descriptions and shared terminology |
| `docs/governance/` | Ownership, decision rights, and documentation policy |
| `docs/security/` | Security, privacy, data handling, and threat models |
| `docs/quality/` | Cross-cutting quality requirements and budgets |
| `docs/operations/` | Optional runbooks and incident records |
| `docs/changes/` | Optional migrations and deprecations |
| `docs/releases/` | Release readiness and delivery records |
| `docs/support/` | Supported scope, channels, and escalation |
| `docs/archive/` | Superseded historical material |
| `corpus/` | Optional executable evidence for architectural claims and reviews |

Start small. A project can adopt only the documentation index, design document,
and ADR template, then add the other artifact types when their distinction
becomes useful.

All groups are modular. In particular, remove `operations/`, `changes/`,
`releases/`, or `support/` when the project has no deployed environments,
compatibility windows, coordinated releases, or support commitment.

The top-level `corpus/` package is also modular. Adopt it when ordinary unit,
integration, and golden tests do not fully capture sustained evidence about
ownership boundaries, compatibility surfaces, or architectural review
questions.
