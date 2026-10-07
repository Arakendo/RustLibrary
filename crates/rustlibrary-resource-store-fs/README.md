# rustlibrary-resource-store-fs

Native directory import/export for rustlibrary-resource-store. Rust 1.85+;
the only dependency is the shared resource-store crate.

## Usage

    use rustlibrary_resource_store_fs::{
        import_directory, export_directory, ImportOptions, ExportOptions,
    };

    let store = import_directory("input", ImportOptions::default())?;
    let report = export_directory(
        &store.snapshot(), "new-output", ExportOptions::default(),
    )?;

The input must be an existing directory. The output must not exist, and its
parent must already exist. Relative caller paths are resolved by the OS against
the process working directory. These APIs do not change that directory.

For a complete executable consumer:

    cargo run -p rustlibrary-corpus-rust-editor --bin native

## Import contract

- Return a new ResourceStore only after all entries are imported successfully.
  Failure returns an error, not partially imported state. Source data is never
  changed by the adapter.
- Preserve regular file bytes, empty files, and explicit empty directories.
- Use supplied Limits and CasePolicy. Default limits are inherited from the core.
  Count discovered files/directories before retaining a directory enumeration,
  validate logical depth/path length, and bound each actual file read by the
  smaller remaining total-byte and per-resource budget (plus one sentinel byte).
- Enumerate each folder in sorted canonical-name order. The traversal is iterative.
  Reject normalized name collisions rather than merging folders or replacing files.
- Reject links at the input root and traversed children. Windows reparse points,
  including junctions, are rejected. Sockets, devices, and other special files
  are rejected. Hard links are admitted as independent regular-file byte copies.
- Reject non-UTF-8 names and names outside the component rules below.
- Native hidden/read-only flags, permissions, ownership, timestamps, extended
  attributes, and links are not imported. Dotfiles are ordinary included files.
  The resulting resource tags are the core defaults.

Metadata sizes provide an early check, but actual reads are also bounded if a
file grows. A metadata budget rejection is a Store LimitExceeded error; growth
past the read budget is an Io error with InvalidData. A file changed during
reading is not guaranteed to represent one point in time.

## Export contract

- Export the complete selected Snapshot under a new directory. Default options
  include hidden entries; include_hidden=false uses inherited logical visibility.
- Preflight all selected names and ASCII case aliases before creating output.
  Directories sort before their descendants under canonical path ordering.
- Create the destination with create_dir and files with create_new; never merge,
  truncate, or replace an existing destination, including an empty directory.
- Copy bytes and folder shape. Logical read-only tags do not prevent reads for
  export, and logical tags/IDs/versions are not translated into native metadata.
- ExportReport counts files, directories excluding root, and payload bytes.

Export is not a filesystem transaction. A native error after creating the root
can leave partial files/directories. ExportError.destination_created records
whether this call created that root. No automatic deletion or rollback is attempted.
The caller decides what to do with partial output and must not delete a destination
it does not own. Successful export does not promise crash durability.

## Names and filesystem trust

Every path component must satisfy the core logical path contract and these
additional conservative rules on every platform: no Windows device basenames
(including device names with extensions), trailing dot/space, or native reserved
punctuation. Percent and fragment syntax are also rejected. Export rejects ASCII
case aliases even on case-sensitive filesystems.

This policy is intentionally stricter than some native filesystems. It is not a
complete model of Unicode equivalence, short-name aliases, volume rules, native
path length limits, or every filesystem's case behavior. Such remaining conflicts
fail through native operations; they may produce a partial export.

Caller-selected input roots and output parents must be trusted, quiescent trees.
The final input root, traversed children, and immediate output parent are checked
for links/reparse points. Ancestors of caller-selected roots may already traverse
links. Path-based checks and later opens are not atomic. Another process can
replace an entry between those operations.

These APIs are convenience adapters, not an adversarial filesystem sandbox.
Do not use them as a confinement boundary for concurrently writable/untrusted
trees. Handle-relative, platform-specific operations would require a separate
contract and implementation. No archive extraction or persistence backend is
implied by this adapter.

## Validation

    cargo test -p rustlibrary-resource-store-fs
    cargo clippy -p rustlibrary-resource-store-fs --all-targets -- -D warnings

Tests use exclusively created temporary roots. Coverage includes byte/empty-folder
roundtrips, budget failures, captured snapshots, name preflight, destination
collisions, partial failures, Unix links/sockets/non-UTF-8 names, and Windows
junction rejection. Platform-specific tests run on their corresponding CI hosts.

The [native RustEditor consumer](../../corpus/campaigns/rust-editor/README.md)
exercises import, rope edits, conditional store save, snapshot export, reopen,
overwrite rejection, and owned temporary-directory cleanup.

## References

- [ADR-0002](../../docs/adr/ADR-0002-native-resource-adapter.md)
- [Rust create_new semantics](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.create_new)
- [Rust symlink_metadata](https://doc.rust-lang.org/std/fs/fn.symlink_metadata.html)
- [Windows metadata extensions](https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html)
- [Microsoft filename conventions](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file)
