---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-0]
status: open
assignee: unassigned
---

# Remove or repurpose the orphaned daemon-era web apps

## Context

Found via a dead-code sweep continuing the same audit that flagged
`src/browser.rs` and `src/analytics.rs` — this one is larger and more
clear-cut than either.

`apps/web/` (99 files, ~21,500 lines, a TanStack Start browser client) and
`packages/flow-client/` (99 files, ~1,350 lines, its generated WebSocket
protocol client) both exist entirely to connect to a `flow-daemon` binary.
[Strip daemon and agent runtime](strip-daemon-and-runtime.md) deleted
`crates/flow-daemon`, `crates/flow-client`, and `crates/flow-protocol`
during Milestone 0 — that ticket's own scope was Rust-crate-only
(`Exclusive ownership of: crates/flow-daemon/, crates/flow-client/,
crates/flow-protocol/...`) and never mentions `apps/web` or
`packages/flow-client` at all. Both are still listed as active bun
workspace members (root `package.json`'s `"workspaces": ["website",
"apps/*", "packages/*"]`), but neither can actually function: `apps/web`'s
own README instructs `cargo run -p flow-daemon --bin flow-daemon`, a
binary that no longer exists anywhere in this repository.

Unlike `browser.rs`/`analytics.rs` (unused but still functional, kept for
a plausible future), these two packages are **not merely unused — they
are broken**, referencing deleted infrastructure with no path to ever
working again short of rebuilding the daemon they were strip-deleted
specifically to remove.

## Why not deleted here

Same reasoning as the other two dead-code findings, at larger scale: a
~22,850-line removal across two whole packages deserves an explicit call
rather than a solo autonomous-loop decision, even at higher confidence
than either single-file candidate. Also not currently causing any active
harm — `bun ./scripts/dev.ts` (the only script actually run every session)
never touches either package, and no CI workflow references them, so this
is dormant weight, not a live breakage.

## What to check before deciding

- Whether `website/` (the separate marketing/landing site, also a bun
  workspace member) has any real dependency on `packages/flow-client` —
  a quick grep during this same sweep found none, but worth confirming
  before removing `flow-client` out from under it.
- Whether `apps/web`'s own UI/design work (a real browser client for
  *something*) is worth salvaging as a starting point for a genuinely
  different future Flow surface (e.g. a companion web view once Turso
  Sync exists — PRODUCT.md's own Operating Context already names
  self-hosting on k3s as a later phase), versus a clean deletion now with
  a fresh start whenever that need is real.
