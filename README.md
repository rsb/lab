# Lab

Lab: a free, professional-grade photo editor for the desktop.

Lab is built as one Cargo workspace of small crates with enforced boundaries,
ordered from the most general to the most specific. The decisions behind the
architecture — and the standards the code is held to — are public:

- [rsb.sh](https://rsb.sh) — the engineering record: decisions, standards,
  and the architecture.
- [app.rsb-lab.com](https://app.rsb-lab.com) — the product home.

## Status

Scaffolding. The architecture is decided and public; the code is beginning.
Nothing here is released, and no crate is published.

## Layout

- `crates/` — the substrate: small, focused crates with enforced dependency
  direction. None exist yet; each is earned, not pre-created.
- `apps/lab` — the application. Composition only.
- `xtask` — the checks that hold the architecture: dependency-cone assertions
  and module fences, arriving with the crates they guard.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

## Contributing

Contribution opens once the CLA is in place. Until then, the engineering
record at [rsb.sh](https://rsb.sh) is the best way to follow the work.
