# RustLibrary Testing Strategy

| Field | Value |
| --- | --- |
| Status | Active for the current workspace |
| Authority | Current testing policy |
| Owner | RustLibrary maintainers |
| Last reviewed | 2026-10-07 |

## Validation boundaries

Crate unit tests cover internal failure boundaries such as counter exhaustion.
External contract tests under each crate's tests directory exercise public
behavior, including invalid inputs and failed-operation atomicity. Rustdoc examples
must compile and run. Runnable examples must build as workspace targets.

The [RustEditor campaign](../corpus/campaigns/rust-editor/README.md) is a separate
consumer package. It supplies architectural evidence that public resource, rope,
and diff APIs compose without moving editor state into reusable crates. Its
orchestration tests and CLI controls remain distinct from crate-level tests.

## Local checks

From the repository root:

    cargo test --workspace --all-targets --locked
    cargo test --workspace --doc --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo fmt --all -- --check
    cargo doc --workspace --no-deps --locked
    cargo run -p rustlibrary-corpus-rust-editor --locked

Set RUSTDOCFLAGS to "-D warnings" to match CI's documentation check.
Use --offline in addition when dependencies are already cached. A missing cache
entry is an environment failure, not evidence that a crate is broken.

For minimum-version verification, add +1.85.0 immediately after cargo in the test
and campaign commands. Stable Rust owns lint and formatting checks; old compiler
lint behavior does not define current formatting or style policy.

## Continuous integration

[The Rust workspace workflow](../.github/workflows/rust.yml) runs on pushes to main,
pull requests, and manual dispatch.

| Job | Environment | Checks |
| --- | --- | --- |
| Tests | Windows and Ubuntu; stable and Rust 1.85.0 | All workspace targets, doctests, campaign executable |
| Quality | Ubuntu; stable | Formatting, Clippy with denied warnings, API docs with denied warnings |

Dependencies use the committed Cargo.lock via --locked. The test matrix exercises
the workspace's current minimum-version baseline; individual manifests continue to
own their declared version requirements. A matrix entry is a test target, not a
blanket support promise for all other platforms or target triples.

Vendor submodules are not checked out or built. Official checkout and artifact
actions are pinned to reviewed commit SHAs, with read-only repository permissions
and no persisted checkout credentials. No secrets, publication, deployment, or
automatic source changes are part of this workflow.

The workflow defines checks but does not configure repository branch protection.
Remote results exist only after the workflow has been pushed and run on GitHub.

## Evidence and fixtures

A successful matrix job uploads its in-memory, native, and model-workload campaign TSV reports, verbose toolchain version, and
checkout commit ID as a uniquely named artifact retained for 14 days. Unexpected
campaign errors terminate the job. The CLI emits a success report only after all
controls pass; a failed run is not reported as successful evidence.

Committed corpus observations are point-in-time records, not golden files.
CI writes fresh evidence under target/corpus-evidence and does not rewrite the
campaign's historical observed.tsv. Assertions in the executable are the regression
oracle. Fixture changes must describe the intended public behavior they exercise.

## Known gaps

This workflow does not measure performance, test real RustEditor integration,
certify filesystem confinement under concurrent mutation, test WASM/macOS, or publish registry packages.
Add a new test layer when a concrete consumer or failure mode requires it; do not
treat a passing in-memory campaign as evidence for those untested boundaries.

Native adapter tests and the native corpus executable create isolated temporary
workspaces and remove only their owned roots. Unix-specific link/socket/name tests
and Windows junction tests run in their corresponding matrix jobs. The native
campaign TSV is saved beside the in-memory report in each evidence artifact.

The corpus workload CLI seeds 256 documents and applies 512 deterministic mixed
operations per case policy. It verifies every live state and five retained captures
against independently owned expected bytes/folders. Full runs are correctness
evidence; no performance threshold or production-editor compatibility is implied.
