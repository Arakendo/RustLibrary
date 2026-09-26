# rustlibrary-data-structures

Focused reusable structures, inspired by ClassLibrary's Text collection.
This is a first slice, not a parity port of its entire data-structures library.

- `GapBuffer<T>`: bounded element count, movable insertion gap, indexed reads,
  iteration and deletion on either side. Safe Rust; no Clone/Copy bound. Failed
  insertion returns the owned value. Moving the gap costs proportional to distance;
  growth moves elements and allocates. `Option<T>` slots have storage overhead.
- `TextRope`: Ropey-backed UTF-8 storage, shared clones, checked byte-range edits,
  chunks, line starts, and strict absolute UTF-16 offset conversion. Ranges are
  zero-based, end-exclusive and must land on Unicode scalar boundaries.
  Grapheme movement, bidi presentation and visual coordinates belong to consumers.

Line breaks are CR, LF and CRLF (one break). Empty text has one line; a trailing
break starts an empty final line. Other Unicode separators remain content.
No normalization or newline conversion occurs. Typed edit failures preserve text.
Zero limits admit empty data only. Limits bound logical bytes/elements, not total
heap use, retained clones or memory inside T. Rope allocation OOM is not recovered.
Debug output omits content. Flattening with `to_text` allocates the whole text.

```rust
use rustlibrary_data_structures::TextRope;
let mut text = TextRope::new("hello", 1024).unwrap();
let before = text.clone();
text.replace(0..5, "hello 🌍").unwrap();
assert_eq!(before.to_text(), "hello");
assert_eq!(text.utf16_to_byte(8).unwrap(), 10);
```

Run workspace tests, Clippy and formatting from RustLibrary. Five contract tests
cover deterministic Vec/String models, multi-chunk Unicode edits, clone isolation,
ownership/drop behavior, bounds, budgets and line/UTF-16 contracts. These are unit
and integration evidence, not a completed architectural corpus campaign or a
performance benchmark. Windows Rust 1.95 was exercised; other targets/MSRV were not.

Ropey 1.6.1 supplies tree mechanics (MIT licensed); upstream dependencies retain
their licenses. Gap-buffer implementation is independently authored, not copied
from ClassLibrary. Authored-code licensing remains undecided; `publish = false`.
Piece tables, generic ropes and wider collections remain consumer-driven follow-up.
