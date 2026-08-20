# Flow development guidance

Flow is a calm, keyboard-first personal task manager (Inbox, Today, Upcoming,
Anytime, Someday, and a read-only glance at the user's own macOS Calendar via
EventKit). **The GPUI desktop app that used to live in this repo's `src/` and
`resources/` has been archived** (explicit user decision, 2026-08-20 — see
`wayfinder/tickets/migrate-to-tauri.md` for the full migration history and
the decision record). Its complete source is preserved at the git tag
`gpui-app-archived-2026-08-20` (`git checkout gpui-app-archived-2026-08-20`
in this repo restores it exactly as it last shipped); nothing was deleted
from history, only removed from the working tree.

**Active development now happens in
`/Users/avi/Developer/vibe/flow-tauri-prototype`** (Tauri v2 + React +
TypeScript + Vite + Framer Motion), a separate repository with its own git
history and its own CLAUDE.md/AGENTS.md. That app reached functional parity
with the archived GPUI app before this archive happened — see the migration
ticket for the full gap-closing history.

## What still lives in this repo

- `crates/flow-data` — Flow's shared local task store (SQLite/turso, NLP
  date parsing, EventKit calendar bindings). **Do not move or delete this
  directory** — the Tauri app depends on it via a path dependency
  (`../../flow/crates/flow-data` in its own `Cargo.toml`), and moving it
  would break that build. This is the one piece of the archived GPUI app
  that's still load-bearing.
- `crates/flow-core` — lower-level shared primitives `flow-data` itself
  depends on.
- `docs/PRODUCT_REQUIREMENTS.md`, `docs/DESIGN_DIRECTION.md`, `PRODUCT.md`,
  `CONTEXT.md` — the durable product/design source of truth. These describe
  Flow the product, not the GPUI implementation specifically, and still
  apply to the Tauri app; drift between them and the Tauri app's actual
  behavior should be fixed there, not treated as stale just because the
  original implementation moved.
- `wayfinder/flow-map.md` and `wayfinder/tickets/*.md` — planning history,
  most importantly `migrate-to-tauri.md`'s full record of what was ported,
  what was found and fixed along the way, and this archive decision itself.
- `apps/web`, `packages/flow-client`, `db/`, `website/` — broader
  infrastructure not specific to the GPUI app; left untouched by this
  archive, scope not audited here.

## Working in `crates/flow-data`/`crates/flow-core`

Both build and test independently of the rest of this repo now (`cargo check
--workspace`, `cargo test --workspace` from the repo root). Changes here
affect the Tauri app directly and the archived GPUI app not at all (it no
longer builds from this repo). If a change here needs verifying against real
app behavior, do that from the Tauri app's own repo.
