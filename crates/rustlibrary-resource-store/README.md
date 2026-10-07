# rustlibrary-resource-store

A dependency-free, in-memory resource store for reusable document bundles, assets,
generated output, and application resources. No filesystem, async runtime, or
format parser is required. Rust 1.85+, edition 2024.

## Consume

Add this crate as a path dependency in a sibling project:

    [dependencies]
    rustlibrary-resource-store = { path = "../RustLibrary/crates/rustlibrary-resource-store" }

It is also usable as a Git dependency selecting this package from the workspace.
Like the other current RustLibrary crates, registry publishing is disabled.

    use rustlibrary_resource_store::{ResourceStore, WriteMode};

    let mut store = ResourceStore::default();
    let entry = store.write_text("docs/readme.txt", "Hello", WriteMode::Create)?;
    let snapshot = store.snapshot();

    store.write_text(
        "docs/readme.txt", "Updated", WriteMode::CompareVersion(entry.version),
    )?;
    store.move_entry("docs/readme.txt", "published/readme.txt")?;
    assert_eq!(store.entry_by_id(entry.id)?.path, "published/readme.txt");
    assert_eq!(snapshot.read_text("docs/readme.txt")?, "Hello");
    # Ok::<(), rustlibrary_resource_store::Error>(())

Run the complete example from the workspace:

    cargo run -p rustlibrary-resource-store --example document_bundle

## Contracts

- Bytes are immutable shared allocations. Writes copy caller input; reads, copies,
  streams, and snapshots never expose mutable stored bytes.
- File creation, replacement, upsert, and version-checked replacement are explicit
  WriteMode variants. Replacing a file preserves its ID and attributes.
- EntryId is stable within one store instance. Moves preserve IDs for the entire
  subtree. Copies and delete/recreate allocate new IDs. IDs and version numbers
  are local to a store; applications must qualify them with their own store identity.
- Every successful transaction advances generation once. Directly changed entries
  receive that generation as their version. Ancestors are not version-bumped by
  child mutations; generation detects changes anywhere in the store. Entry versions
  are not timestamps or globally unique identity. Exhaustion returns an error.
- Missing removal and creating an already existing directory are idempotent and do
  not advance generation. Empty batches, clear, and unchanged writes/attribute
  assignments still count as transactions.
- All mutations stage changes before commit. Any returned error leaves content,
  metadata, IDs, generation, and hierarchy unchanged. Bulk writes are processed
  in order and must stay within limits at each step. An unwinding producer panic
  also leaves the live store unchanged.
- Explicit directories include empty folders. Missing parents are created during
  writes, directory creation, and transfers. Files cannot be directory ancestors.
  Removing the last file leaves its parent directories.
- Copy and move accept either a file or a complete subtree. Destinations must be
  absent; neither overwrite nor merge is implicit. Root, same-address transfers,
  and transfers inside the source subtree are rejected.
- Listings are sorted by canonical path, exclude the requested directory itself,
  and default to immediate visible children. Recursive/include-hidden options
  are explicit. Prefix checks respect segment boundaries.
- Hidden and read-only tags inherit through ancestors. Hidden entries are still
  directly readable. Read-only descendants prevent recursive deletion, move, or
  clear atomically. Copying read-only sources is allowed. Transfers preserve own
  tags; inherited tags are recomputed at the destination. An owner may clear an
  entry's own read-only tag unless an ancestor is read-only.
- Attributes are application conventions, not access control or encryption.
  The permanent root can have attributes but cannot be removed.
- UTF-8 helpers are strict. No BOM stripping, Unicode normalization, newline
  rewriting, or encoding guessing occurs. Use rustlibrary-text-codec for other
  encodings and retain the resulting bytes through this crate.

## Addresses

Paths are relative to a single explicitly selected store, separated by forward
slashes. The empty string denotes root. Absolute paths, drive/URI syntax,
backslashes, percent escapes, query/fragment syntax, control characters, repeated
separators, trailing separators, and dot segments are rejected on normal APIs.

Sensitive is the default case policy. AsciiInsensitive lowercases ASCII letters
and stores that canonical spelling; non-ASCII characters remain distinct. The
policy cannot change after construction.

The resolve helper separately allows dot and parent segments in a reference
relative to a base file's parent, while rejecting escape above root. It validates
even segments subsequently removed by parent traversal. Resolution does not
require either address to exist and performs no URI decoding or host I/O.

Logical paths are not sanitized native filenames. A future native adapter must
independently handle platform naming rules, symlinks, and filesystem confinement.

## Budgets and ownership

Default limits are 10,000 entries (files plus directories, excluding root),
64 MiB logical payload total, 16 MiB per resource, 4,096 UTF-8 bytes per path,
and 128 path segments. Callers can choose different limits at construction,
including zero. Copies count toward logical byte budgets even though they share
an allocation.

Limits bound retained store content, not process memory, caller buffers, or
snapshot lifetime. Retained snapshots/read handles can keep old bytes alive after
overwrite or removal. Release them when no longer needed; deletion is not secure
erasure. Allocation failure follows standard Rust allocation behavior.

Mutation uses an O(n) staged metadata-map clone to guarantee atomic commit;
snapshots also copy the metadata index while sharing payloads. Lookup by path is
O(log n); listing, statistics, and ID lookup scan entries (inherited attributes
also walk ancestors). These are deliberate initial tradeoffs for bounded bundles,
not a disk-scale VFS or a benchmark-backed throughput claim.

Mutations require exclusive access. ResourceStore and Snapshot are Send + Sync;
applications can use an RwLock for shared mutation, or distribute snapshots for
independent readers. No global registry, singleton, thread, or hidden lock exists.

## Scope and provenance

Designed after inspecting ClassLibrary/MemoryStore, RustEditor's resource-space
crate, and Tokimu's corpus/lib/resource-space. This is a new implementation; it
does not import their source or change those projects.

The shared document-bundle workflow is preserved, with explicit collision policy,
bounded retention, immutable data, stable local IDs, and stale-write detection.
This is not C# API or behavioral parity: C#'s arbitrary URI handling and always
case-insensitive overwrite policy are deliberately not adopted.

The separate [native adapter](../rustlibrary-resource-store-fs/README.md) provides
bounded directory import and new-directory snapshot export.

JSON/XML parsing, MIME inference, hashing, archive import/export, persistence,
watchers, provider mounts, and editor undo/save state remain separate concerns.
Consumers can parse or hash returned bytes without duplicating store semantics.
No public storage-backend abstraction is frozen before a second implementation
establishes its requirements.

## Validation

    cargo test -p rustlibrary-resource-store
    cargo clippy -p rustlibrary-resource-store --all-targets -- -D warnings
    cargo doc -p rustlibrary-resource-store --no-deps

Contract tests cover the document-bundle workflow, atomic failures, path validation,
hierarchy, limits, attributes, identity, version conflicts, retained snapshots,
stream seeking, cross-thread reads, and a deterministic mutation/reference-model
comparison. Internal tests exercise counter exhaustion.
