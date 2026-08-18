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
  repo it was forked from (github.com/egoist/waku).
- `origin` is **https://github.com/mars-s/flow**, a public repo created
  2026-08-18 (`gh repo create flow --public --source=. --remote=origin
  --push`). Only `main` was pushed; `archive/waku-upstream` and
  `milestone-0-strip` are intentionally local-only. GPL-3.0 (`LICENSE`,
  unchanged) plus the README's upstream-attribution sentence to Waku
  satisfy the license carry-over the user asked for when creating this repo.
- Two other local branches exist purely as an archival safety net, not for
  active work: `archive/waku-upstream` (the pre-detachment Waku-based
  history) and `milestone-0-strip` (the working branch used during the
  strip, before detachment). Neither should be merged into `main`.
- Current commits on `main` (oldest first):
  1. `98d1b69` — initial detached snapshot (Milestone 0 strip + Waku→Flow
     rename, squashed into one commit).
  2. `789344a` — untracked local Claude Code plugin/index scaffolding that
     had accidentally been swept into the first commit.
  3. `1d61bca` — retokened `theme.rs` to match `docs/DESIGN_DIRECTION.md`,
     rebuilt the sidebar (Tasks/Calendar mode switch + task list + pinned
     Settings), added several missing icons.
  4. `39dcab4` — wired up Turso (`src/db.rs`): dedicated tokio thread,
     placeholder `schema_version` table, ping smoke test passing.
  5. `6109057` — noted the new GitHub remote in this doc.
  6. `2c098f8` — real task data model/schema/CRUD in `db.rs`.
  7. `9908636` — handoff doc: Milestone 1 checklist added.
  8. `c215709` — Inbox view + sidebar badge wired to the real database (see
     "Milestone 1 progress" below).
- Working tree is clean as of commit `c215709` unless the "Milestone 1
  progress" section below says otherwise — check there for what's currently
  in flight before assuming everything is committed.

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

`cargo check --workspace` is clean throughout. Test count: 148 as of the
Milestone 0 commit, 151 as of `2c098f8` (3 added for `db.rs`'s task CRUD).
Run `cargo test --package flow --lib` to check the current count/status.

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
- Main pane: **Inbox is real now** (`src/app/tasks.rs`, added in Milestone
  1 — see that section below), reading/writing the actual database. Every
  other destination (Today/Upcoming/Anytime/Someday/Calendar/Settings)
  still renders `components.rs`'s "Coming soon" placeholder. **No subtasks,
  date picker, or toast/notification component exist yet** — the user
  showed reference screenshots for these (Things-3-style task row with note
  + subtask, a "When" date popover with Today/This Evening/calendar
  grid/Someday/Add Reminder, a toast banner) but explicitly deferred them
  out of the sidebar-focused UI pass; still future work.
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

## Milestone 1 progress (local task vertical slice)

Started 2026-08-18. This section is the live checklist — updated after each
verified, committed increment, not just at the end, per the user's explicit
request to keep this document current as work happens rather than write it
once at a session's close. Check the git log above for exactly which commit
landed which line.

**Done:**

- [x] Task data model + `Bucket` enum, matching
      `docs/PRODUCT_REQUIREMENTS.md` §8 minus the `users` table (see commit
      `2c098f8`'s message for why that's an intentional simplification, not
      an oversight).
- [x] Real migration runner in `db.rs` (`MIGRATIONS` const, versioned via
      `schema_version`), replacing the earlier placeholder.
- [x] `Db::create_task` / `Db::list_bucket` / `Db::set_completed`, each with
      a passing test against a real temp-file database (not mocked).

**Done (continued):**

- [x] `Db` wired into `Flow` — opened once in `Flow::new` (`src/app.rs`), a
      one-time sub-millisecond local-file open, not a render-path cost (see
      the comment there for why that's an intentional exception, not an
      oversight). `None` on failure, degrades gracefully rather than
      panicking.
- [x] Reactive task-list reads via the existing `QueryCache` in
      `src/query.rs` (`Flow::read_bucket` in `src/app/tasks.rs`) — this repo
      already had the exact right primitive for this (read from `render`,
      background-fetch on a miss, `cx.notify()` on arrival), no new
      abstraction needed.
- [x] Real Inbox view (`src/app/tasks.rs::render_inbox`) replaces the
      placeholder pane: task rows (title + a 17px completion circle per
      `docs/DESIGN_DIRECTION.md`), a loading skeleton, an empty state
      ("Nothing to process. Capture the next thing." per the direction
      doc's required-states table), and a database-unavailable fallback.
- [x] Completion toggle (`Flow::toggle_completed`) — writes via
      `Db::set_completed`, invalidates the Inbox cache entry, refetches.
      **Partial**: the row does fade via `with_animation` on initial
      render, but there's no dedicated completion collapse/Undo yet — the
      row just disappears on the next fetch since `list_bucket` filters out
      completed tasks. `docs/DESIGN_DIRECTION.md`'s "10-second Undo toast"
      is not implemented.
- [x] Sidebar Inbox badge reads the same cache instead of a hardcoded `0`
      (`Flow::inbox_count` in `sidebar.rs`).
- 145 tests passing as of commit `c215709`; verified in the running debug
  app, not just `cargo check` (dev watcher rebuilt clean, process alive,
  matches this repo's `AGENTS.md` validation rule).

**Not done yet, in the order they're planned:**

- [ ] **Capture is still inert.** The "+ Capture" button has no `on_click`
      at all — it needs a real composer (text input, at minimum a title;
      `input.rs`'s `ComposerInput` already exists generically and is a
      candidate to reuse) wired to `Db::create_task`. This is the most
      visible remaining gap: there's currently no way to add a task from
      the UI, only to complete/reopen ones that already exist (e.g. via a
      direct `Db::create_task` call from a test or a temporary debug path).
- [ ] A proper completion-collapse animation + Undo toast, per
      `docs/DESIGN_DIRECTION.md`'s "180-220ms opacity and vertical
      collapse... available via Undo" spec — currently the row just
      vanishes on refetch, no collapse motion, no undo.
- [ ] Today/Upcoming/Anytime/Someday views — bucket-filtering logic exists
      in `db.rs` (`Bucket::Active` + date comparison, per PRD §5's table)
      but no view renders them yet.
- [ ] One-level subtasks, task detail expansion, the task-row/date-picker/
      toast components the user showed reference screenshots for earlier in
      this session (explicitly deferred out of the sidebar-focused UI pass).
- [ ] Deterministic local NLP date/time parser (PRD §6.4) — not started;
      currently the biggest single unstarted piece of Milestone 1.
- [ ] Update `docs/PRODUCT_REQUIREMENTS.md` §9 (Convex → Turso) and §5 (IA
      diagram → Tasks/Calendar mode switch) so the PRD stops contradicting
      the actual code and this session's decisions. Low-risk, low-effort,
      just hasn't been done yet — do this whenever convenient, doesn't need
      to block on the rest of the checklist.

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
