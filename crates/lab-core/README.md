# lab-core

The root of the Lab workspace, and the error contract every other crate
programs against.

`lab-core` sits at the bottom of the dependency graph: it depends on nothing
in the workspace, and everything in the workspace may depend on it. That
asymmetry is the definition of the layer, and it is enforced — `cargo xtask
check` asserts the crate has zero workspace dependencies. Because it is the
most-depended-on artifact in the ecosystem, it is kept deliberately small.

## The contract is a trait, not a type

There is no shared concrete error type — no `lab_core::Error` struct or enum
that every crate returns. A single foundational error type would freeze one
field set, and one opinion about failure, onto every crate beneath it. Instead
`lab-core` publishes a contract and each crate that can fail brings its own
type that meets it. This is the shape `serde` takes (`serde::de::Error` is a
trait), and it is why the contract lives at the crate root rather than in a
namespace: at this layer the contract *is* most of what the crate is.

```rust
pub trait Fail: std::error::Error {
    fn chain(&self) -> Chain<'_> { /* … */ }
    fn rendered(&self) -> Rendered<'_> { /* … */ }
}
```

Three things define `Fail`:

- **`std::error::Error` is a supertrait.** Every Lab error is, with no
  conversion, an ordinary member of the Rust error ecosystem — `?`-convertible,
  boxable as `Box<dyn Error>`, composable alongside errors from crates that have
  never heard of Lab. The cost of interoperability is paid once, here.
- **The causal chain is the populated `source()`.** A wrapping error returns its
  cause; a leaf returns `None` because it genuinely has none. `source()` is the
  real chain — never a stub that lies about its own depth.
- **`Display` renders one level only.** Each error prints its own message; the
  chain-spanning reading is assembled by `Rendered`, not by any level restating
  the levels beneath it. Because `Rendered` reproduces that message verbatim
  into logs and UI, it names the operation and must never carry a value unsafe
  to log — a secret, a credential, or personal data.

There is **no blanket implementation**. A blanket `impl Fail for T: Error` would
auto-enrol every error type and make the contract assert nothing. A type joins
by implementing it explicitly — `impl Fail for MyError {}`.

## Two views over one chain

`source()` already holds the causal chain. `lab-core` adds two ways to read it,
and they are the same chain seen twice — not two data structures.

- **`Chain<'a>`** — an iterator that walks `source()` links, outermost first.
  The first item is the error you started from; each next item is the previous
  one's cause. It borrows; it owns nothing.
- **`Rendered<'a>`** — a `Display` view that joins each link's own message with
  `": "` to produce the Go-style reading `outer: middle: leaf`. It does not
  re-walk the chain itself; it formats a `Chain`, so there is one walking
  algorithm and one source of truth.

Both are views, not error types: they implement no `Error`, hold no failure
data, and carry no opinion a crate inherits. They construct from any
`&dyn std::error::Error`, so they work over external errors too; `Fail` offers
`chain()` and `rendered()` as convenience over its implementors.

The walk is bounded by `MAX_DEPTH`. `source()` is supposed to be acyclic and
finite, but the adapters accept any error — including buggy or hostile external
ones — so a cyclic or unbounded chain cannot make them hang or allocate without
end: `Chain` stops at the cap, and `Rendered` ends with a visible `: …` marker.

## Implementing the contract

```rust
use std::error::Error;
use std::fmt;
use lab_core::Fail;

#[derive(Debug)]
struct OpenFailed {
    source: ConfigError, // some other error type
}

impl fmt::Display for OpenFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // This level only — name the operation per the message grammar.
        f.write_str("session.open failed")
    }
}

impl Error for OpenFailed {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source) // hold the cause honestly
    }
}

impl Fail for OpenFailed {}
```

Given that, `err.rendered().to_string()` reads
`session.open failed: config.load failed: file is missing`, and `err.chain()`
yields each link outermost first. Context is added by **wrapping** — a new error
that holds the old one as its `source()` — never by mutating a message string.

## What is deliberately absent

`lab-core` carries the contract and nothing application-shaped:

- no recovery classification (`Kind`) — that vocabulary is the application's,
  decided where recovery actually lives;
- no construction-site location capture (`#[track_caller]` / `Location`) — a
  mechanism whose precision degrades invisibly;
- no concrete `Error` type and no string-prepending `.context()`.

A propagation / wrap helper is a likely future addition, deferred to its own
decision rather than guessed at here.

## Design record

The shape comes from three ADRs at <https://adrs.rsb.sh>: *errors are values*
(failures are returned values with identity), *the error contract is a trait*
(this trait, the `source()` chain, the rendering as a view), and *error message
grammar* (`receiver.method failed: msg`, the content each level carries).

## Testing

Tests live in `tests/`, exercising the public contract the way a dependent
crate would; the source file carries only shipped code. See the
*tests are consumers of the contract* decision.
