# rustlibrary-text-codec

Strict, bounded file-text conversion for RustLibrary consumers. RustEditor is the
first consumer. No UI, workspace, document-version, filesystem or Save policy is
owned here. Version 0.1.0 is a repository dependency, not a registry release.

```rust
use rustlibrary_text_codec::{decode, encode, Encoding, Limits};
let limits = Limits { input_bytes: 1024, output_bytes: 4096 };
let decoded = decode(b"caf\xe9\r\n", Encoding::for_label("cp1252")?, limits)?;
assert_eq!(decoded.text, "café\r\n");
assert_eq!(encode(&decoded.text, decoded.format, limits)?, b"caf\xe9\r\n");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Encoding coverage

| Family | Accepted labels |
| --- | --- |
| Unicode | UTF-8/utf8, UTF-16LE, UTF-16BE, UTF-32LE, UTF-32BE |
| Literal single-byte | ASCII/us-ascii (7-bit), ISO-8859-1/latin1/latin-1 (literal 0–255 mapping) |
| Windows | windows-1250 through windows-1258, also cp1250 through cp1258; windows-874 |
| DOS/OEM | cp437, cp720, cp737, cp775, cp850, cp852, cp855, cp857, cp858, cp860, cp861, cp862, cp863, cp864, cp865, cp866, cp869, cp874 |
| International | Backend canonical names: Shift_JIS, EUC-JP, ISO-2022-JP, GBK, gb18030, Big5, EUC-KR, IBM866, KOI8-R, KOI8-U, macintosh, x-mac-cyrillic, ISO-8859-2/3/4/5/6/7/8/8-I/10/13/14/15/16 |

Labels are case-insensitive and trimmed, with a 40-byte admission limit.
Unlisted aliases are deliberately not forwarded as browser aliases. `ansi`,
`oem`, `utf-16` and `utf-32` are ambiguous and fail. UTF-7, EBCDIC and other
unlisted formats remain unsupported. Windows/Asian backend mappings follow
encoding_rs, not a claim of exact compatibility with every historical Windows
code-page implementation. CP437 controls remain controls, not DOS screen glyphs.

## Fidelity and detection

Callers select an encoding. `detect_bom` reports only a Unicode signature; it
does not guess BOM-less content or rank encodings. A single byte can have several
valid meanings across code pages. Applications must obtain that choice from a
user, manifest or format-specific declaration and resolve conflicts themselves.

For explicitly selected Unicode, decode removes one matching leading BOM and
returns `Format` with the observed BOM choice. Contradictory signatures fail.
UTF-32 signatures take precedence over UTF-16 prefixes (including the inherently
ambiguous UTF-16LE BOM followed by NUL). Legacy decoders treat signature-looking
bytes as ordinary data. Encode emits only the requested BOM; a leading literal
U+FEFF without a signature remains ambiguous on subsequent file decoding.

Malformed input, unmappable text and output exceeding the caller budget fail
with typed errors and no partial success. No replacement characters, best-fit
fallback, question marks, HTML/XML numeric entities or Unicode normalization are
introduced. Legacy encode verifies decoded text equals the input, rejecting
accepted-but-changing mappings such as Shift_JIS yen/backslash. Mixed CR/LF/CRLF,
NUL, DOS EOF characters and combining sequences are preserved as text.

Valid legacy input can have multiple byte representations of the same text.
Decode/encode therefore does not promise byte-identical output for all encodings,
especially stateful formats. Consumers retain original bytes for unchanged files.
This crate does not parse XML encoding declarations or rewrite them on conversion.

## Budgets and errors

`Limits` requires explicit input and output byte budgets; zero means zero.
For decode, output is UTF-8 bytes. For encode, input is UTF-8 bytes and output
includes BOM. Input is checked before processing, output before growth. Internally
the backend uses fixed 4096-byte scratch chunks. Output allocation requests grow
geometrically up to the selected bound; allocator metadata/granularity is outside
that logical budget. Allocation failure is reported when reservation fails.
Legacy encode also retains a verification decode bounded by the input limit.
Budget these buffers together in the caller. No public streaming/file API is
selected yet. `Decoded` Debug reports byte length and format, never text contents.

## Dependencies, provenance and validation

Unicode scalar/byte handling is original RustLibrary code using Rust's standard
library. Existing mapping implementations are dependencies, not copied tables:

- [encoding_rs](https://docs.rs/encoding_rs/0.8.35/encoding_rs/) 0.8.35, MIT OR Apache-2.0;
  pinned for consistent mappings and the consumer's Rust 1.85 baseline. Backend
  upgrades require the same fidelity tests. Its notices include mapping provenance.
- [oem_cp](https://docs.rs/oem_cp/2.1.2/oem_cp/) 2.1.2, MIT, with PHF lookup dependencies.
- Cargo.lock and `cargo tree -p rustlibrary-text-codec` record the resolved closure.

RustLibrary's authored-code license remains undesignated; registry publication is
disabled. This does not alter dependency licensing. No upstream source was copied.

Run `cargo test -p rustlibrary-text-codec`, workspace Clippy with `-D warnings`,
and `cargo fmt --all -- --check`. The eight public contract tests use inline
synthetic fixtures: known Windows-1252/Latin-1 distinctions, CP437/850 character
vectors, all 256 bytes for each admitted OEM page, representative scripts,
Unicode/BOM/newlines, malformed inputs, bounds, and multi-chunk/stateful conversion.
Round-trip tests establish consistency, not independent verification of every
backend mapping. Validation ran on Windows with Rust 1.95.0; no cross-platform,
minimum-toolchain, heuristic detection or full historical code-page conformance
claim is made.
