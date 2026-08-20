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

**Active development now happens in `apps/desktop/`** (Tauri v2 + React +
TypeScript + Vite + Framer Motion, its own `.agent/README.md`). It reached
functional parity with the archived GPUI app first, while living in a
separate `flow-tauri-prototype` GitHub repo; that repo was then merged into
this one as `apps/desktop/` — full commit history preserved via `git
subtree` — once the GPUI-era repo split no longer served a purpose. This
`flow` repo is now the single canonical home for Flow: the app, its shared
crates, and its product/planning docs.

## Repo layout

- `apps/desktop/` — the active Tauri app (see its own `.agent/README.md` for
  its dev workflow, verification practice, and AI feature architecture). One
  package inside this repo's root Bun workspace.
- `crates/flow-data` — Flow's shared local task store (SQLite/turso, NLP
  date parsing, EventKit calendar bindings), depended on by `apps/desktop`
  via a same-repo path (`../../../crates/flow-data` from `src-tauri/`), and
  a member of the root `Cargo.toml` workspace. **Do not move or delete this
  directory.** This is the one piece of the archived GPUI app that's still
  load-bearing.
- `crates/flow-core` — lower-level shared primitives `flow-data` itself
  depends on.
- `docs/PRODUCT_REQUIREMENTS.md`, `docs/DESIGN_DIRECTION.md`, `PRODUCT.md`,
  `CONTEXT.md` — the durable product/design source of truth. These describe
  Flow the product, not any one implementation; drift between them and
  `apps/desktop`'s actual behavior should be fixed there, not treated as
  stale.
- `wayfinder/flow-map.md` and `wayfinder/tickets/*.md` — planning history,
  most importantly `migrate-to-tauri.md`'s full record of the GPUI archive,
  the Tauri app's build-out, and this repo consolidation itself.
- `apps/web`, `packages/flow-client`, `db/`, `website/` — broader
  infrastructure not specific to Flow's desktop app; left untouched by the
  GPUI archive and this consolidation, scope not audited here.

## Working in `crates/flow-data`/`crates/flow-core`

`cargo check --workspace` / `cargo test --workspace` from the repo root now
also exercises `apps/desktop/src-tauri` (it's a workspace member). If a
change here needs verifying against real app behavior, actually run
`apps/desktop` (`bun scripts/dev-app.ts` from inside that directory) rather
than trusting a clean `cargo check` alone.
