# Flow (Tauri) — agent instructions

Flow is a calm, keyboard-first personal task manager (Inbox, Today, Upcoming,
Anytime, Someday, and a read-only glance at the user's own macOS Calendar via
EventKit). This is **the only active Flow app**. It used to live in its own
`flow-tauri-prototype` GitHub repo while an earlier GPUI/Rust desktop
implementation occupied this `flow` repo's root; the GPUI app was archived
(tag `gpui-app-archived-2026-08-20`) after this Tauri app reached functional
parity with it, and `flow-tauri-prototype` was then merged into this repo as
`apps/desktop/` (full commit history preserved via `git subtree`) once the
GPUI-era split no longer served a purpose. `wayfinder/tickets/
migrate-to-tauri.md` at the repo root has the full history — read it for
context on *why* things are the way they are before assuming something here
is accidental.

This directory is one package (`apps/desktop`) inside flow's root Bun
workspace (`apps/*`, alongside `apps/web`, `packages/flow-client`,
`website/`) — run commands from here (`cd apps/desktop && bun run dev`, not
from the repo root, whose own `package.json` is unrelated tooling (drizzle,
`website`/`apps/web`)).

## Stack

Tauri v2 + React 19 + TypeScript + Vite + Framer Motion (frontend), Rust +
`flow-data` crate + reqwest (backend). No test framework is wired up yet —
verification is `bunx tsc --noEmit` plus actually running the app and
checking behavior (see "Verifying changes" below), not a unit test suite.

## The `flow-data` dependency

`src-tauri/Cargo.toml` depends on `flow-data` via a same-repo path,
`{ path = "../../../crates/flow-data" }` (three levels up from `src-tauri/`
to the repo root, then into `crates/flow-data`) — it's also a member of the
root `Cargo.toml` workspace (`apps/desktop/src-tauri`), so `cargo check
--workspace` from the repo root exercises it too. `crates/flow-data` and
`crates/flow-core` are the one part of the old GPUI app's own footprint
that's still load-bearing — don't move or delete them.

## Dev workflow

`scripts/dev-app.ts` (run with `bun scripts/dev-app.ts` from inside
`apps/desktop/`) is the real dev loop: it rebuilds and relaunches a proper,
Spotlight-discoverable "Flow Debug.app" bundle on every source change under
`src/` or `src-tauri/src/`, rather than the bare unbundled binary `bun run
tauri dev` produces. It watches this directory with one recursive
`fs.watch` and filters by hand (see the file's own comments for why — a real
infinite-rebuild-loop bug was found and fixed there). It stops itself if the
app process it launched exits, so after manually killing the debug app (e.g.
to free the SQLite lock — see below) the watcher needs restarting too.

The release build lives at `/Applications/Flow.app` and shares the exact
same Cargo package/binary name (`flow-tauri-prototype` — kept as-is through
the repo merge; renaming it would orphan the app's existing on-disk data
directory, see below) as the debug bundle — `dev-app.ts` is careful to
`pkill -f` the full debug bundle path, not a bare process name, because a
bare match once killed the user's real installed release app. Don't loosen
that.

## Verifying changes for real

`bunx tsc --noEmit` catches type errors but not behavior. Before considering
a data-affecting change done, quit the running debug app (releases the
turso/SQLite lock) and query
`~/Library/Application Support/com.avi.flow-tauri-prototype/flow-tauri-dev.db`
directly with `sqlite3` to confirm real invariants hold — e.g. no orphaned
subtasks, no completed parent with open children (PRD §6.2/§11). Relaunch
the debug app (or restart `dev-app.ts`) afterward.

## The AI feature architecture ("shiny blocks")

Every AI-labeled feature in Settings is a self-contained component gated on
a master AI toggle, living wherever it's contextually relevant in the app
(not one central AI panel), each with its own independent per-feature
Off / Manual / Auto toggle (`useAiFeatureState` in `lib/aiConfig.ts`):

- **Off** — feature invisible.
- **Manual** — user explicitly triggers it, sees a preview before anything
  writes.
- **Auto** — runs itself. For a feature that only *displays* something
  (Today briefing, Draft from task) this just means "generate without being
  asked." For a feature that *writes* task data (Checklist expansion,
  Overdue batch reschedule, Smart scheduling), Auto writes with **no
  confirmation step** — a deliberate, explicit product decision, not an
  oversight. Keep that distinction when adding a new block.

Model calls go through Rust (`src-tauri/src/ai.rs`, via `reqwest`) and are
invoked from the frontend via `lib/ai.ts`, never a frontend `fetch()` —
most third-party OpenAI-compatible endpoints don't send permissive CORS
headers for an arbitrary app origin, and the Tauri webview enforces CORS
regardless of `security.csp`.

Not every "AI feature" in Settings is actually a model call — Duplicate
detection (`lib/similarity.ts`) is a local Jaccard word-overlap check, and
several write-mode blocks (Overdue batch reschedule, Smart scheduling)
deliberately ask the model only for a small judgment call (a day offset, an
index into a pre-computed list) rather than trusting it with arithmetic the
code can do exactly. Follow that split for new blocks: let the model judge,
let code compute.

## Where the durable docs live

Product/design source of truth (`docs/PRODUCT_REQUIREMENTS.md`,
`docs/DESIGN_DIRECTION.md`, `PRODUCT.md`, `CONTEXT.md`) and the full
migration/decision history (`wayfinder/`) live at this repo's own root, one
level up from `apps/desktop/` — check there before assuming a behavior is
unspecified.
