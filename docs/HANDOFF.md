# Flow — handoff

Status as of 2026-08-18. Written for another agent (or a future session)
picking up this repo cold. Read [AGENTS.md](../AGENTS.md) first for the
standing development rules (dev watcher, performance, accessibility) — this
document is project state and decisions, not those rules.

## What Flow is

A calm, keyboard-first personal task manager: Inbox, Today, Upcoming,
Anytime, Someday, and a read-only Google Calendar glance. Native Rust/GPUI
desktop app (macOS + Linux). Not a coding-agent tool, not a project-management
suite. See [PRODUCT.md](../PRODUCT.md) for the full north star and
[docs/PRODUCT_REQUIREMENTS.md](PRODUCT_REQUIREMENTS.md) for the PRD.

## Repo / git state

- `main` is a **fresh, orphan branch** — root commit `98d1b69`, no shared
  history with anything else. It was deliberately detached from the Waku
  repo it was forked from (github.com/egoist/waku); the user wants this
  history-free, remote-free, and pushed to a **new GitHub repo they will
  create themselves**. No `origin` remote is configured. Don't create or
  push to a remote unasked; the user said they plan to do that step
  themselves.
- Two other local branches exist purely as an archival safety net, not for
  active work: `archive/waku-upstream` (the pre-detachment Waku-based
  history) and `milestone-0-strip` (the working branch used during the
  strip, before detachment). Neither should be merged into `main`.
- Current commits on `main`:
  1. `98d1b69` — initial detached snapshot (Milestone 0 strip + Waku→Flow
     rename, squashed into one commit).
  2. `789344a` — untracked local Claude Code plugin/index scaffolding that
     had accidentally been swept into the first commit.
  3. `1d61bca` — retokened `theme.rs` to match `docs/DESIGN_DIRECTION.md`,
     rebuilt the sidebar (Tasks/Calendar mode switch + task list + pinned
     Settings), added several missing icons.
- **Uncommitted as of this handoff**: the Turso database wiring (`src/db.rs`,
  `Cargo.toml`/`Cargo.lock` changes, `docs/turso.md`, an `AGENTS.md` doc-map
  addition). `src/db.rs`'s smoke test (`db::tests::opens_a_database_and_answers_a_ping`)
  has been confirmed passing (`cargo test --package flow --lib db::` → 1
  passed) — the connection genuinely opens, applies the placeholder schema,
  and answers a query end to end, not just "compiles."

## What's built (Milestone 0 — done)

Per `wayfinder/flow-map.md` and its closed tickets: the Waku coding-agent
product (daemon, agent sessions/transcript/composer, provider/git/tooling
UI, the entire `waku-core` agent-provider backend — ~37,000 lines across
`crates/waku-core` plus `terminal.rs`/`computer_use.rs`/`daemon.rs`/
`js_repl.rs`) is deleted, not hidden. What survives: the GPUI window
lifecycle/chrome, `theme.rs`, `browser.rs` (generic WKWebView, kept for a
future Calendar OAuth flow), `input.rs` (generic text-input widget, its only
real caller of `src/md/`'s syntax highlighting), and a slimmed
`crates/flow-core` (just `i18n`/`identity`, recovered from the deleted
`waku-protocol` crate's source).

`cargo check --workspace` and `cargo test --workspace` are clean (148 tests
passing as of the Milestone 0 commit; more added since for `db.rs`).

## Current UI state

- Sidebar (`src/app/sidebar.rs`, 252px, matches
  `docs/DESIGN_DIRECTION.md`'s navigation-rail spec):
  - "Flow" wordmark, a "+ Capture" button (inert — no composer exists yet).
  - A **Tasks/Calendar segmented mode switch** — this is a deliberate
    departure from the original PRD's flat 7-destination sidebar list,
    added per explicit user request in this session. Tasks mode lists
    Inbox/Today/Upcoming/Anytime/Someday; Calendar mode shows no list (the
    main pane just shows the Calendar placeholder). **`docs/PRODUCT_REQUIREMENTS.md`
    section 5's IA diagram has not been updated to reflect this change yet**
    — that's a known doc-drift gap, not a decision to revert.
  - Settings is a single row pinned to the bottom via a flex spacer,
    reachable from either mode, not part of either mode.
  - All rows are icon + label, monochrome (no per-item color — the user
    explicitly rejected a colorful Things-3-style treatment and kept the
    existing focus-blue-only system). New icons authored this session:
    `inbox`, `calendar`, `layers`, `archive`, `home` (in
    `assets/icons/*.svg`, Lucide-style, registered in `src/assets.rs`).
- Main pane: still placeholder-only ("Coming soon" copy per destination,
  `src/app/components.rs`). **No real task rows, subtasks, date picker, or
  toast/notification component exist yet** — the user showed reference
  screenshots for these (Things-3-style task row with note + subtask, a
  "When" date popover with Today/This Evening/calendar grid/Someday/Add
  Reminder, a toast banner) but explicitly said only the sidebar and mode
  switch should ship in this pass; those other components are future work,
  not implemented.
- `theme.rs` was retokened this session to actually match
  `docs/DESIGN_DIRECTION.md` (it had drifted — still had Waku's old
  coral-accent palette after the strip). Light theme values were **not**
  touched; the doc only specifies dark mode.

## Decisions made this session (with why)

1. **Detach from Waku's git history entirely.** User's words: "this is meant
   to be named flow in a different git system, completely unrelated to
   waku." Not just a rename — a fresh orphan branch, no shared commits, no
   `origin`. GPL-3.0 license and the README's upstream-attribution sentence
   to Waku are kept regardless (legal requirement, not branding).
2. **Turso over Convex for persistence**, despite the PRD originally naming
   Convex as the sync-phase plan. User wanted "really fast, concurrent,
   local" with sync later and explicitly did not want to write backend
   code. Convex is server-authoritative-over-websocket, not embedded/local,
   and still requires writing TypeScript functions regardless of client
   language — doesn't match either stated requirement. Turso (Rust rewrite
   of SQLite, embedded, pure-Rust/no C toolchain, Turso Sync for the later
   phase) does. See `docs/turso.md` for the full research and
   `docs/PRODUCT_REQUIREMENTS.md` section 9, which still says "self-hosted
   Convex" and **needs updating** to reflect this decision.
3. **No colored left border on the selected sidebar row.** Added once,
   caught by the user as "vibe-coded looking sloppy UI" — it's literally
   `craft-floor.md`'s (the `impeccable` skill) banned pattern list. Reverted
   to a plain filled pill. Lesson recorded in memory
   (`flow-ui-craft-discipline`): load `impeccable`'s craft-floor checklist
   before UI work, not after a correction.
4. **Sidebar IA: Tasks/Calendar mode switch replaces the flat destination
   list**, per explicit user instruction after being shown a reference
   screenshot of a "Home | Code" segmented pill. See "Current UI state"
   above for the resulting structure. This is a real product decision that
   changed the PRD's original IA — flagged above as a doc-drift gap.
5. **Database bridging pattern**: a dedicated `current_thread` tokio runtime
   on its own OS thread (`src/db.rs`), matching the existing precedent in
   `src/analytics.rs`. Chosen because Turso requires tokio (not
   runtime-agnostic) while GPUI's `cx.background_executor()` is
   smol-backed, and Turso's `Connection` `Send`/`Sync` bounds are
   undocumented upstream — keeping the connection on one dedicated thread
   for its whole lifetime sidesteps that question entirely rather than
   assuming an answer.

## What's NOT done yet (next steps, roughly in order)

1. **Update `docs/PRODUCT_REQUIREMENTS.md`** section 9 (Convex → Turso) and
   section 5 (IA diagram → Tasks/Calendar mode switch), so the PRD stops
   contradicting the actual code and this session's decisions.
2. **Milestone 1** (per `docs/PRODUCT_REQUIREMENTS.md` section 12): the real
   task data model, Turso schema/migrations (nothing exists yet beyond
   `db.rs`'s placeholder `schema_version` table), Inbox + task row/detail +
   one-level subtasks, the five placement rules, completion animation, and
   the deterministic local NLP date/time parser. This is the bulk of
   remaining work and hasn't started.
3. **The task-row/date-picker/toast components** the user showed reference
   screenshots for — explicitly deferred out of this session's UI pass, but
   real work items for Milestone 1, once there's task data to render.
4. **New GitHub repo + push** — the user said they'll do this themselves.
   Be ready to help if asked (e.g. confirming what should/shouldn't be
   committed, checking `.gitignore`), but don't create or push to a remote
   unprompted.

## Where to find things

See `AGENTS.md`'s "Product, design, and planning docs" section — it's the
same list, kept current there so it doesn't drift out of sync with this
document. In short: `PRODUCT.md` (north star), `docs/PRODUCT_REQUIREMENTS.md`
(PRD), `docs/DESIGN_DIRECTION.md` (visual system — keep `theme.rs` matching
it), `docs/turso.md` (DB reference), `CONTEXT.md` (glossary),
`wayfinder/flow-map.md` + `wayfinder/tickets/*.md` (planning history).

This session's memory is also recorded under Claude's project memory
(`flow-project-overview`, `flow-database-choice`, `flow-doc-map`,
`flow-ui-craft-discipline`) for any Claude session working in this
directory — but that memory is personal to this Claude account and won't
travel with the repo to a new machine or a different agent, which is exactly
why this document exists as the portable, repo-committed version.
