# Lab

A free, professional-grade photo editor for the desktop.

Lab is one Cargo workspace of small crates with enforced boundaries, ordered
from the most general to the most specific. Dependencies point only toward the
more general, and that direction is held mechanically — by Cargo and by `xtask`
checks — not by convention. The crates grow one at a time; each is earned, not
pre-created.

The foundation is an error model: failures are values, carried by a contract
trait that each crate implements and read as one legible chain. The decisions
behind the architecture precede the code and are public, recorded as ADRs:

- [adrs.rsb.sh](https://adrs.rsb.sh) — the decision log: the base coding
  standard, errors-are-values, the error contract, and the message grammar.
- [rsb.sh](https://rsb.sh) — the wider engineering record: decisions,
  standards, and the architecture.
- [app.rsb-lab.com](https://app.rsb-lab.com) — the product home.

## Layout

- `crates/` — the substrate: small, focused crates with enforced dependency
  direction, beginning with `lab-core` (the error contract at the workspace
  root).
- `apps/lab` — the application. Composition only.
- `xtask` — the checks that hold the architecture: dependency-cone assertions
  and module fences, arriving with the crates they guard.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

## Contributing

Contribution opens once the CLA is in place. Until then, the engineering
record at [rsb.sh](https://rsb.sh) is the best way to follow the work.
