# RustEditor Campaign

| Field | Value |
| --- | --- |
| Status | Complete for the initial consumer workflow; retained for regression |
| Authority | Executable architectural evidence |
| Owner | RustLibrary maintainers |
| Opened / reviewed | 2026-10-07 |
| Governing record | [ADR-0001](../../../docs/adr/ADR-0001-resource-store.md) |
| Evidence state | Contract assertions plus bounded consumer observations |

## Architectural question

Can an editor own its unsaved documents and conflict decisions while composing
RustLibrary's resource store, text rope, and diff crates without reimplementing
their storage, edit, or comparison semantics?

## Boundary under pressure

This is a standalone, non-publishable workspace consumer. It imports only public
APIs through path dependencies on rustlibrary-resource-store,
rustlibrary-data-structures, and rustlibrary-diff. There is no source inclusion,
private API access, dependency on the actual RustEditor repository, or application
state added to the reusable crates.

OpenDocument is campaign-owned orchestration: stable entry ID, captured version,
and an unsaved TextRope. Save resolves the ID and uses CompareVersion. The consumer
keeps conflict handling and explicit reload decisions.

An executable package lives in this campaign because compilation as a separate
consumer is part of the evidence. Its tests execute the same workflow as its CLI.
Ordinary crate tests prove individual contracts; this campaign records what their
composition requires from an editor and what remains application-owned.

## Claims and controls

The CLI executes these controls under both case policies:

1. Bundle navigation and related-resource resolution.
2. Unsaved Unicode draft isolation from stored bytes.
3. Line comparison using the shared diff crate.
4. Successful version-checked save.
5. Immutable preview capture across save.
6. Rejection of a second tab's stale save, preserving its draft and stored content.
7. Open tabs finding renamed resources by stable ID.
8. Rename conflict followed by explicit reload and save.
9. Backup content equality with distinct entry identity.
10. Hidden navigation with direct reads still available.
11. Atomic rejection of deletion through read-only attributes.
12. Atomic rejection of a colliding batch import.
13. Atomic rejection of an oversized import.
14. Related-reference rejection at the logical root boundary.
15. A deleted tab refusing to overwrite a replacement at the same address.
16. Explicit case-policy behavior.

All are assertions of existing crate contracts. The finding that a small editor
can compose them is a consumer observation, not proof of full RustEditor parity.
A mismatch or unexpected error exits nonzero; the CLI emits a successful report
only after all 32 controls complete. Tests also verify repeatability and the
failure-reporting helper.

## Inputs and environment

- Inputs: [original document](fixtures/document.txt), [expected edit](fixtures/edited.txt),
  and small named binary/text values in the consumer source.
- Text is deterministic UTF-8 with LF line endings and a non-BMP globe character,
  exercising an editor's UTF-16 column mapping.
- No network, filesystem import/export, random values, clocks, UI, threads, or
  external application are needed at runtime.
- Building requires the workspace's dependencies in Cargo's cache for offline use.
- Only normal Cargo build artifacts are produced unless output is redirected.
  No source fixture is modified and no application cleanup is required.

## Execution

From the repository root:

    cargo run -p rustlibrary-corpus-rust-editor --offline
    cargo test -p rustlibrary-corpus-rust-editor --offline
    cargo clippy -p rustlibrary-corpus-rust-editor --all-targets --offline -- -D warnings

The package is a workspace member, so cargo test --workspace includes it.
The [workspace CI](../../../.github/workflows/rust.yml) also runs the CLI on
Windows/Linux with stable Rust and Rust 1.85, capturing fresh reports and
provenance as job artifacts without changing the committed observation.

The CLI writes TSV with columns campaign, case_policy, control, outcome.
Capture stdout if a new point-in-time observation is needed. Cargo diagnostics
go to stderr. No resource content is included in the report.

## Evidence inventory

| Evidence | Provenance | Review state | Meaning |
| --- | --- | --- | --- |
| src/main.rs | Campaign-owned executable consumer | Assertions exercised locally | Composition and positive/negative controls |
| fixtures/ | Campaign-owned deterministic inputs | Checked against byte/text contracts | Unicode editing input and expected output |
| observed.tsv | Actual CLI stdout captured 2026-10-07 on Windows x86_64 | Observational, not a reviewed golden | 32 passing controls across two case policies |

The initial Resource Store baseline is commit 983a6a4. Exact toolchain versions
used for the captured observation are in [toolchain.txt](toolchain.txt).
The executable assertions are the regression oracle; observed.tsv is historical
evidence and must not be silently refreshed to conceal a failing assertion.

## Findings and friction

The consumer opens, edits, compares, saves, snapshots, moves, and copies resources
through public shared APIs. Its application-owned glue consists of the open
document's ID, captured version, draft, and conflict/reload decisions.

Rename correctly preserves identity but advances version. Locating an entry by
ID therefore does not authorize saving an old draft. The consumer demonstrates
explicit reload before a subsequent save; automatic conflict merging is not proved.

Entry IDs remain store-local. This single-store consumer does not establish a
multi-store identity or workspace registry contract.

## Disposition and next action

The initial question is supported for this small in-memory workflow. Keep this
campaign in workspace regression checks and expand corpus-owned consumers.
Production RustEditor integration is deferred at the owner's request: its current
system remains separate. No production RustEditor changes were made.

Reopen for API/identity/version changes, multiple-store sessions, encoding changes,
filesystem persistence, or evidence of insufficient capacity.

## Non-goals

Actual RustEditor integration, UI fidelity, undo/redo, automatic merge, background
save races, native filesystem safety, production performance, and cross-platform
runtime certification. A save here commits to the in-memory store, not disk.

## Native adapter extension (2026-10-07)

Governing record: [ADR-0002](../../../docs/adr/ADR-0002-native-resource-adapter.md).
This extension asks whether the same editor-owned draft can compose the native
adapter without moving host paths or export failure policy into the resource core.

    cargo run -p rustlibrary-corpus-rust-editor --bin native --offline
    cargo test -p rustlibrary-corpus-rust-editor --bin native --offline

Unlike the original in-memory CLI, this executable creates an exclusively owned
temporary workspace. It writes a source fixture, imports it, edits through TextRope,
conditionally saves in memory, exports a captured snapshot, and reopens the output.
It also proves source disk bytes remain unchanged, empty folders survive export,
an existing output is rejected, and its temporary workspace is removed.

Six controls emit native TSV only after all checks and explicit cleanup succeed.
The checked-in native-observed.tsv is actual Windows CLI stdout captured on
2026-10-07 using the versions in native-toolchain.txt. It is point-in-time evidence,
not a golden. CI captures fresh native.tsv in every matrix artifact.

src/bin/native.rs owns this orchestration and imports the public
rustlibrary-resource-store-fs API. No production RustEditor code is changed.
The native extension's passing tests support this bounded caller-controlled
workflow only; they do not prove concurrency-safe confinement, atomic filesystem
publication, permission/attribute roundtripping, or crash durability.

## Deterministic workload extension (2026-10-07)

Governing record: [ADR-0001](../../../docs/adr/ADR-0001-resource-store.md).
The question is whether the shared contracts continue to compose across longer
mixed document lifecycles and retained preview captures, beyond the initial
single-document scenario. All new implementation is corpus-owned.

    cargo run -p rustlibrary-corpus-rust-editor --bin workloads --offline
    cargo test -p rustlibrary-corpus-rust-editor --bin workloads --offline

The standard workload seeds 256 Unicode/CRLF documents across eight folders,
then executes 512 deterministic operations for each case policy. The operation
cycle covers rope prefix edits and exact diff ranges, conditional saves and stale
save rejection, moves, copies, delete/recreate, colliding bulk imports, protected
subtree deletion, oversized writes, and explicit empty-directory creation.

An independent model owns Vec payloads and folder-name sets. Every step compares
the complete file/folder inventory, bytes, payload sizes, identity uniqueness,
tracked identity/version lifecycle, and total byte accounting. Expected bytes
come from independent string operations, not from reading results back from the
store. IDs/versions are opaque values tracked from successful calls, with separate
assertions for preservation, replacement, and advancement.

Each policy retains five immutable captures together with the expected state at
capture time. At completion all captures are compared against those historical
models. Rejected operations also check generation preservation. There are 256
expected rejections per policy and 518 full-state checks, including retained
captures. No source content appears in the report.

The three small-profile regression tests cover both policies, repeated-run
determinism, and negative controls that deliberately corrupt expected bytes,
identity, and accounting to prove the model checker rejects mismatches. The full
standard workload runs separately as a CI CLI step in all four matrix jobs.

workloads-observed.tsv is actual local Windows stdout captured 2026-10-07;
workloads-toolchain.txt records the toolchain used. CI writes fresh workloads.tsv
into its evidence artifact. These are observations, not goldens. No timing
threshold, memory measurement, concurrency result, or production-scale performance
claim is made. This is deterministic correctness evidence, not fuzzing or a
benchmark. The fixture workloads do not reproduce the actual RustEditor's system.

Current evidence supports the stated mixed-operation sequence for both policies.
Continue corpus-only validation for further concrete ownership questions; actual
production integration stays deferred.
