# Contributing

Thanks for your interest. The project is pre-MVP and moving fast; the roadmap in `README.md` is
the source of truth for what is being worked on.

## Ground rules

- **Hash stability is sacred.** Anything that changes the bytes produced by
  `scion_types::canonical` changes every id in every repository. Such a change requires a
  `FORMAT_VERSION` bump, regenerated golden fixtures, and a section in `docs/DATA_MODEL.md`.
- **No I/O in `scion-types` or `scion-core`.** Algorithms are written against the store traits
  and tested with `MemoryStore`.
- **Every store implementation passes the conformance suite** (`scion_store::conformance_tests!`).
- **Server-side refs are only `main` and `release/*`.** Do not add branch-like features.
- Everything merged to `main` passes `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test --workspace` and `cargo deny check`.

## Workflow

1. Open an issue or RFC (`docs/rfcs/`) for anything that touches the data model, the wire
   protocol or the cache tiering.
2. Branch from `main`, keep PRs small, and describe *why* in the PR body.
3. CI must be green. Reviews look at intent and invariants first, lines second.

## Code style

`rustfmt.toml` and the workspace lints in `Cargo.toml` are authoritative. Pedantic clippy is on;
allow a lint locally with a comment explaining why rather than disabling it globally.
