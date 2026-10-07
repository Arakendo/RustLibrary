# ADR-0002: Native resource directory adapter

- Status: Accepted for the initial adapter implementation
- Scope: rustlibrary-resource-store-fs
- Owner: RustLibrary maintainers
- Reviewed: 2026-10-07
- Refines: [ADR-0001](ADR-0001-resource-store.md)

## Context and decision

Consumers need native import/export without duplicating resource semantics or
adding filesystem dependencies to the in-memory core. Introduce a separate
workspace crate using only the standard library and rustlibrary-resource-store.

Import builds a private new store under explicit logical budgets and returns it
only on success. Preserve regular bytes and explicit directory shape. Reject links,
Windows reparse points, special files, unsupported native names, and canonical
address collisions. Native permissions and logical attributes remain distinct.

Export takes an immutable Snapshot and requires an absent destination beneath
an existing caller-owned parent. Validate names before writes; create directories
and files exclusively. No merge or overwrite mode is introduced.

## Failure and trust boundaries

Import errors expose no partially built store. Native export is not atomic:
after destination creation, an I/O failure can leave partial output. A structured
ExportError records whether this call created the destination. The library never
recursively deletes output to conceal a failure.

The adapter is for caller-controlled, quiescent directories. Path-based metadata
checks cannot defend against entry replacement between checking and opening.
Link checks cover the final source root, enumerated descendants, and immediate
destination parent, not all ancestors of caller-supplied host paths.

A future adversarial confinement boundary would need handle-relative operations
and platform-specific guarantees. Neither this adapter nor passing corpus evidence
establishes that boundary or filesystem crash durability.

## Consequences and evidence

Consumers reuse the core for bytes, hierarchy, budgets, snapshots, and conditional
writes while retaining ownership of host paths and any partial-output cleanup.
Cross-platform conservative names can reject names accepted by an individual OS.
Unicode aliases and platform path limits can still fail during native export.

The [adapter README](../../crates/rustlibrary-resource-store-fs/README.md) specifies
the full initial contract. Its external tests exercise bounded import, snapshots,
empty folders, collisions, partial errors, and platform link/special-file handling.

The [RustEditor native campaign](../../corpus/campaigns/rust-editor/README.md)
uses a temporary source directory and composes import -> rope edit -> conditional
store save -> captured export -> reopen. This is adapter composition evidence,
not production RustEditor integration.
