# rustlibrary-ulid

Reusable typed ULIDs; first consumer: RustEditor Resource Space. Version 0.1.0
is a repository dependency, not a crates.io release or a stable serialization API
for any application's domain wrappers.

```rust
use rustlibrary_ulid::{Generator, Ulid};
let id = Generator::new().generate()?;
let decoded: Ulid = id.to_string().parse()?;
assert_eq!(id, decoded);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The independently implemented format follows the [ULID specification](https://github.com/ulid/spec):
48 timestamp bits, 80 randomness bits, network-order bytes and 26-character
Crockford Base32. Display emits uppercase; parsing accepts lowercase, rejects
ambiguous letters I/L/O/U, whitespace, non-ASCII, wrong lengths and 130-bit overflow.
No upstream implementation code was copied.

`Generator` owns its sequence. On equal milliseconds it increments randomness;
on a later millisecond it requests fresh entropy. Clock regression, pre-epoch
time, timestamp overflow, random-component exhaustion and entropy failure are
typed errors. Failures preserve state; there is no waiting, rollover or weak RNG
fallback. `generate_with` permits explicit host clock/entropy injection. Its
callback must supply secure entropy outside deterministic tests.

Share a generator through a mutex when callers need one monotonic sequence.
Independent generators have probabilistic uniqueness and no global ordering
guarantee. IDs reveal time and are not secrets or authorization capabilities.
Consumers still enforce workspace ownership, version checks and stale completion
rules. RustEditor deliberately generates independent IDs rather than relying on
ULID ordering for document versions.

Dependency: `getrandom` 0.3.4 (MIT OR Apache-2.0), locked with `cfg-if` 1.0.5
(MIT OR Apache-2.0). Other platform dependencies are retained in Cargo.lock;
inspect `cargo tree --target all` for the resolved closure. This crate does not
depend on Servo, Lucide, Tokimu, Tosumu or RustEditor. Licensing of newly authored
RustLibrary code remains to be designated before external redistribution;
`publish = false` prevents an accidental registry release.

Run from RustLibrary:

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -p rustlibrary-ulid --example generate
```

Validated on Windows with Rust 1.95.0: canonical vectors, binary layout, strict
parsing, deterministic round trips/order, monotonic carry, failure recovery,
overflow, concurrent shared generation and an OS-entropy smoke check. This is
focused contract evidence, not a statistical proof of uniqueness. Other OSes and
WASM have not been validated; no browser entropy backend is selected.
