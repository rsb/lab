# xtask

The enforcement arm of the Lab workspace. The architecture's laws are held
per-repo and mechanically — "responsibility moved from author discipline into
structure." Cargo enforces some of it natively (it rejects dependency cycles),
but two of `lab-core`'s rules Cargo cannot see, so this binary holds them. Each
check arrives with the crate it guards.

## Usage

```sh
cargo xtask check
```

`cargo xtask` is an alias (`.cargo/config.toml`) for
`cargo run --package xtask --`, so `cargo xtask check` compiles and runs this
binary with `check` as its argument. The xtask pattern is just "a workspace
binary that runs project tasks" — no external task-runner dependency.

Exit codes are distinct on purpose, so CI can tell misuse from a real
violation:

- `0` — all checks passed.
- `1` — a check failed (an architecture rule was violated).
- `2` — the command was invoked incorrectly.

## Dispatch

```rust
fn main() {
  match std::env::args().nth(1).as_deref() {
    Some("check") => run_check(),
    Some(other) => { /* unknown command → exit 2 */ }
    None => { /* usage → exit 2 */ }
  }
}
```

The dispatch is a deliberate three-arm match, not a silent default. `check`
runs the suite; an unknown subcommand and no subcommand each print guidance and
exit `2`, kept separate from the `1` a failed check returns.

## The harness

```rust
type Check = fn(&Path) -> Result<(), String>;

fn run_check() {
  let checks: [(&str, Check); 2] = [
    ("lab-core has zero ecosystem dependencies", /* … */),
    ("lab-core root does not reference crate::report", /* … */),
  ];
  // run each; print `ok:`/`FAIL:`; exit 1 if any failed
}
```

- **`workspace_root()`** derives the root from `env!("CARGO_MANIFEST_DIR")` — a
  compile-time constant pointing at `…/lab/xtask`, whose parent is the workspace
  root. The checks therefore don't depend on the current working directory;
  `cargo xtask check` works the same from anywhere in the tree. Its one
  `.expect(...)` is the correct place to panic: a manifest dir with no parent is
  an impossible state, not an anticipated failure (the errors-are-values panic
  boundary — bugs panic, anticipated failures return).
- **`type Check`** makes every check a function taking the workspace root and
  returning `Ok(())` or `Err(reason)`. The suite is then a table, so adding a
  guard later is one array entry — checks are uniform, named, and listed in one
  place.
- **The runner accumulates rather than short-circuits.** It runs all checks and
  reports each, exiting `1` only after running them all if any failed. You see
  every violation in one run, not just the first.

## Guard 1 — zero ecosystem dependencies

```rust
Command::new(cargo)
  .current_dir(root)
  .args([
    "tree", "--package", "lab-core",
    "--edges", "normal", "--prefix", "none",
  ])
```

- **It shells out to `cargo tree`** rather than pulling in a metadata-parsing
  crate — keeping `xtask` itself dependency-free, fitting for the tool that
  enforces minimal dependencies. The `CARGO` env var (set by cargo when it runs
  the alias) locates the right toolchain's cargo, falling back to `cargo` on
  `PATH`.
- **`--edges normal`** restricts to normal dependencies (ignoring dev- and
  build-deps); **`--prefix none`** flattens the tree to a deduped list of
  `name version (path)` lines.
- **The discrimination is the key.** `cargo tree` annotates path/workspace
  members with their filesystem path in parens — `(…/lab/crates/…)` — while
  external crates print no such path. The rule is "zero *ecosystem* (workspace)
  dependencies"; external deps are out of scope. So the check skips `lab-core`'s
  own line, then treats any remaining line whose path is under the workspace
  root as a workspace dependency, and therefore an offender. External crates
  pass because they carry no workspace path. This is the dependency rules' "an
  edge's *absence* is a checkable fact."

## Guard 2 — root never references `crate::report`

```rust
let src = std::fs::read_to_string(root.join("crates/lab-core/src/lib.rs"))?;
// flag any line containing "crate::report"
```

- **A purely textual check** on `lib.rs`. It flags any line containing
  `crate::report`, guarding Rule 1's fractal application inside the crate: the
  parent (the root) may *declare* its child (`pub mod report;` is fine —
  administrative knowledge) but must never *reference* it (`crate::report::…`),
  which would invert the dependency direction the architecture rests on.
- **Why textual and not AST-based:** module direction isn't compiler-enforced
  (sibling modules may reference each other freely), so this lightweight
  grep-in-Rust is the right tool — and it's honest about being textual.
- **It targets `crate::report` specifically**, so it won't fire on the eventual
  `pub mod report;` declaration. Today `report.rs` doesn't exist, so the check
  passes; the guard is in place *before* the code it guards, so the rule can't
  be violated even once.

Together these two functions are the entire mechanical enforcement of
`lab-core`'s position: it depends on nothing in the workspace, and its root
never reaches down into its child.

## Adding a check

Write a `fn(&Path) -> Result<(), String>` that takes the workspace root and
returns `Err(reason)` on a violation, then add it to the `checks` table in
`run_check` — alongside the crate it guards.
