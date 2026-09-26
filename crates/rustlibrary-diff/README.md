# rustlibrary-diff

Pure exact line comparison with explicit resource budgets. No dependencies,
filesystem access, patch application, Git operations or UI rendering.

`diff_lines(old, new, Limits)` returns contiguous changed regions with old/new
line and UTF-8 byte ranges. All ranges are zero-based, end-exclusive. Empty old
ranges represent insertion; empty new ranges represent deletion. Adjacent insert
and delete operations form replacements. Unchanged regions are implicit.
Input strings remain untouched, and output contains offsets rather than text.

CR, LF and CRLF terminate tokens and remain part of those tokens. Missing final
newline and line-ending changes are differences. Empty text has zero diff lines;
a trailing terminator does not create a phantom data line. This differs from an
editor's caret-line count: map a zero-width EOF anchor to the document's EOF byte.
Whitespace, Unicode normalization, combining marks and bidi order are unchanged.

The initial backend is a full longest-common-subsequence table with deterministic
delete-first ties. It uses quadratic table space/time in line counts and is only
admitted for small bounded comparisons. It is not a large-file diff engine.

Default limits (provisional): 1 MiB combined input, 4,096 combined lines,
1,000,000 table cells, 16,000,000 work units and 4,096 changed regions.
Work charges input bytes, initialized cells, equality tests and their compared
bytes, and traceback steps; it is conservative bookkeeping, not elapsed time.
The table uses usize cells (up to 8 MB on 64-bit hosts), plus line-boundary vectors
and output. Limits are not total process-memory guarantees. Vec allocation uses
fallible reservation. Zero is a finite limit; even empty input needs one table cell.
Any exceeded limit returns a typed error, never approximate or partial clean output.
Calls are synchronous with finite work; cancellation/worker scheduling belongs to
consumers and is not implemented here.

Run `cargo test -p rustlibrary-diff` in RustLibrary. Six contract tests cover
reconstruction across fixed/generated documents, exact anchors, repeated-line ties,
all logical budgets and long-line work. Allocation failure injection is not tested.
Run `cargo run -p rustlibrary-diff --release --example diff_probe` for a synthetic
timing probe. These checks are not a completed architectural corpus campaign.

Implemented independently from ClassLibrary's conceptual diff/hunk precedent.
No C# source or third-party algorithm code was copied. Authored-code licensing
remains undecided and registry publishing is disabled (`publish = false`).
