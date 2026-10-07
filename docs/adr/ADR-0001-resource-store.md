# ADR-0001: Reusable in-memory resource store

- Status: Accepted for the initial crate implementation
- Scope: rustlibrary-resource-store
- Owner: RustLibrary maintainers
- Reviewed: 2026-09-26

## Context

ClassLibrary's MemoryStore provides a productive byte-oriented document-bundle
workflow, with folder operations and inherited attributes. RustEditor provides
immutable captures, resource identity, and stale-write checks. Tokimu's incubating
resource-space demonstrates explicit case/collision policies and retention budgets.

RustLibrary needs a reusable core that does not depend on those applications,
their kernels, host filesystem APIs, or their format parsers.

## Decision

Introduce rustlibrary-resource-store as an independently consumable, dependency-free
workspace crate. Own one logical root, byte resources, explicit folders, inherited
attributes, store-local entry IDs, generations, limits, and coherent snapshots.
Use strict root-relative addresses and an immutable construction-time case policy.

Require explicit write collision intent. Preserve IDs on moves and replacement;
allocate new IDs on copies and recreation. Permit conditional file replacement by
entry version. Preserve failed-operation atomicity by staging metadata and shared
payloads before publishing a transaction.

Expose strict UTF-8 and seekable read conveniences. Keep encoding libraries,
serialization, cryptographic fingerprints, filesystem adapters, durable storage,
global store registries, editor working-copy state, and watchers outside this core.
A read-only Snapshot supplies the same read API as a ResourceStore.

The precise initial contracts and capacities are documented in the
[crate README](../../crates/rustlibrary-resource-store/README.md), with executable
[contract tests](../../crates/rustlibrary-resource-store/tests/contracts.rs).

## Consequences

Consumers reuse storage semantics and compose adapters around immutable bytes.
They explicitly qualify local IDs with their own store identity and own any locks
needed for shared mutations. No existing RustEditor or Tokimu dependency is changed.

Metadata staging is O(n) per mutation, and snapshots clone metadata while sharing
payloads. Logical budgets are not process-memory quotas. Snapshots can retain removed
data. These costs and retention semantics are documented rather than obscured by a
generic VFS abstraction.

Cross-language parity, filesystem security, on-disk persistence, globally unique
resource identities, and registry publishing are not claimed by this implementation.

## Evidence

Source inspection included the current C# ResourceHierarchy and ResourceTransfers,
RustEditor's resource identity/version/snapshot implementation, and Tokimu's
resource-space address/limit/content contracts and MemoryStore comparison note.
The comparison note predates C#'s explicit folder support, so current C# source took
precedence for hierarchy behavior. No source files were copied from those projects.
