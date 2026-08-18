---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-0]
status: closed
assignee: unassigned
---

# Clean up the Cargo workspace and package metadata

## Scope

Exclusive ownership of:

- Top-level `Cargo.toml` `[package]` and `[dependencies]` sections (the
  `[workspace]` member list itself belongs to `strip-daemon-and-runtime`;
  this ticket only removes now-unused dependency entries after that ticket
  lands)
- `Cargo.lock` (regenerate via `cargo build`, do not hand-edit)
- `package.json`, `bun.lock`, `tsconfig.json`, `drizzle.config.ts` if they
  reference removed daemon/JS-runtime tooling

No other ticket touches these paths. Do not edit files outside this list.

## Goal

Once the daemon, session/transcript, and provider-tooling tickets land, several
top-level dependencies become dead weight: `alacritty_terminal` (terminal),
`rquickjs` (daemon JS runtime), and any others only reachable from deleted
code. `docs/PRODUCT_REQUIREMENTS.md` Milestone 0 wants a shell with "no
external backend dependency" — a slim, honest dependency list is part of that.

## Definition of done

- `cargo machete` (or a manual `rg` sweep per crate) confirms every remaining
  dependency in `[dependencies]` has a real caller in the post-strip
  `src/` tree.
- `alacritty_terminal` and `rquickjs` are removed unless a surviving caller is
  found and named in the PR description.
- `Cargo.lock` is regenerated and committed.
- `cargo build` succeeds from a clean checkout.
- `package.json`/`bun.lock` reference only tooling Flow's frontend build
  (if any survives Milestone 0) actually needs.

## Dependencies

Depends on `strip-daemon-and-runtime`, `strip-session-transcript-composer`,
and `strip-provider-and-tooling-ui` landing first — their deletions are what
make dependencies here provably dead. Run last among the strip tickets,
before `verify-milestone-0-exit`.
