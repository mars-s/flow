# Flow — handoff

Status as of 2026-08-19. Written for another agent (or a future session)
picking up this repo cold. Read [AGENTS.md](../AGENTS.md) first for the
standing development rules (dev watcher, performance, accessibility) — this
document is project state and decisions, not those rules. For the full
commit-by-commit history, use `git log`; this doc tracks current state and
what's still open, not a journal of everything that happened to get here.

## What Flow is

A calm, keyboard-first personal task manager: Inbox, Today, Upcoming,
Anytime, Someday, and a read-only Google Calendar glance. Native Rust/GPUI
desktop app (macOS + Linux). Not a coding-agent tool, not a project-management
suite. See [PRODUCT.md](../PRODUCT.md) for the full north star and
[docs/PRODUCT_REQUIREMENTS.md](PRODUCT_REQUIREMENTS.md) for the PRD.

## Repo / git state

- `main` is a **fresh, orphan branch** — root commit `98d1b69`, no shared
  history with anything else. Deliberately detached from the Waku repo it
  was forked from (github.com/egoist/waku). GPL-3.0 (`LICENSE`, unchanged)
  plus the README's upstream-attribution sentence to Waku carry the license
  requirement over; that's a legal obligation, not branding.
- `origin` is **https://github.com/mars-s/flow**, public, `main` only.
  `archive/waku-upstream` (pre-detachment history) and `milestone-0-strip`
  (the working branch used during the strip) are local-only archival
  branches — never merge either into `main`.
- Working tree is clean as of commit `9810093` — check `git status` before
  assuming that's still true.
- A `/loop` (fixed 10-minute interval, cron job `0759a9f8`) is running as
  of 2026-08-19, continuing this session's work autonomously. Auto-expires
  after 7 days.
- No `/loop` or other background job is currently running.

## What's built (Milestone 0 — done)

Per `wayfinder/flow-map.md` and its closed tickets: the Waku coding-agent
product (daemon, agent sessions/transcript/composer, provider/git/tooling
UI, the entire `waku-core` agent-provider backend — ~37,000 lines) is
deleted, not hidden. What survives: the GPUI window lifecycle/chrome,
`theme.rs`, `browser.rs` (generic WKWebView, kept for a future Calendar
OAuth flow), `input.rs` (generic text-input widget), and a slimmed
`crates/flow-core` (`i18n`/`identity`).

## Current UI state

- Sidebar (`src/app/sidebar.rs`, 252px, matches
  `docs/DESIGN_DIRECTION.md`'s navigation-rail spec): "Flow" wordmark, a
  working "+ Capture" row (click/enter/space or `⌘N`), a **Tasks/Calendar
  segmented mode switch** (a deliberate departure from the PRD's original
  flat 7-destination list, per explicit user request — Tasks mode lists
  Inbox/Today/Upcoming/Anytime/Someday, Calendar mode shows no list), and
  Settings pinned to the bottom via a flex spacer. All rows are icon +
  label, monochrome — no per-item color; the user explicitly rejected a
  colorful Things-3-style treatment.
- Main pane: **all five task views are real** (`src/app/tasks.rs`) —
  Inbox, Today, Upcoming, Anytime, Someday all read/write the actual
  database through `db::View`. Upcoming groups tasks by date with a
  weekday-style header. Completed tasks live in a collapsed, per-view
  "Completed" section rather than vanishing. Calendar and Settings still
  render `components.rs`'s "Coming soon" placeholder.
- Scheduling has three real paths: typing a date phrase into Capture, the
  detail card's Today/Anytime/Someday/"Schedule…" picker (NLP field
  focused immediately, quick-picks below it), and the same picker's bulk
  variant for multi-select.
- `theme.rs` matches `docs/DESIGN_DIRECTION.md` (dark mode only; light
  theme values were never part of that doc and haven't been touched).

## Decisions made (with why)

1. **Detach from Waku's git history entirely.** User's words: "this is
   meant to be named flow in a different git system, completely unrelated
   to waku." A fresh orphan branch, no shared commits, no `origin` carried
   over.
2. **Turso over Convex for persistence**, despite the PRD originally naming
   Convex. User wanted "really fast, concurrent, local" with sync later and
   explicitly did not want to write backend code — Convex is
   server-authoritative-over-websocket and still requires TypeScript
   functions regardless of client language, matching neither requirement.
   See `docs/turso.md` for the full research.
3. **No colored left border on the selected sidebar row.** Added once,
   caught by the user as "vibe-coded looking sloppy UI" — literally
   `impeccable`'s craft-floor banned pattern. Reverted to a plain filled
   pill. Lesson recorded in memory (`flow-ui-craft-discipline`): load
   `impeccable`'s craft-floor checklist before UI work, not after a
   correction.
4. **Sidebar IA: Tasks/Calendar mode switch replaces the flat destination
   list**, per explicit user instruction (shown a reference screenshot of a
   "Home | Code" segmented pill). A real product decision that changed the
   PRD's original IA; the PRD text now reflects it.
5. **Database bridging pattern**: a dedicated `current_thread` tokio
   runtime on its own OS thread (`src/db.rs`), matching the existing
   precedent in `src/analytics.rs`. Turso requires tokio (not
   runtime-agnostic) while GPUI's `cx.background_executor()` is
   smol-backed, and Turso's `Connection` `Send`/`Sync` bounds are
   undocumented upstream — one dedicated thread for the connection's whole
   lifetime sidesteps that question entirely.
6. **Capture activates a task the instant it parses a date** (overrides
   PRD §14's original text, which said a parsed date should not
   auto-activate). Explicit user correction; the PRD has been updated to
   match — the code was never the thing that needed to change twice.
7. **Completion and deletion share one Undo toast + timer** (`UndoKind`
   enum distinguishes which DB-reversal action Undo performs), instead of
   two toast systems — PRD §6.1 only names a 10-second window for deletion,
   reused for completion since nothing else states a different one.

## Milestone 1 (local task vertical slice) — feature status

**Shipped and wired to the real database:** task CRUD, all five views,
Capture (with live NLP-parse highlighting), Inbox's inline Process action
(Today/Anytime/Someday/Schedule…), the task detail card (note, schedule
pill, delete), Cmd+click multi-select with a bulk-action bar, one-level
subtasks (add/complete/the "complete parent and all subtasks?" confirm),
completion collapse animation + Undo toast, deletion Undo toast, a "Clear"
button on the expanded Completed section, a hidden dev inspector
(Cmd-Option-I, debug builds only — GPUI's own `Inspector`/
`DivInspectorState`, no menu item, same convention as Zed's), and
Capture's failed-save handling (`d33f22e`, PRD §6.1: restores the typed
title and shows an inline error + Retry instead of silently discarding it
on a write failure), and an animated Tasks/Calendar mode pill.

**Fixed (`9810093`, user-reported): the completion checkbox's flicker.**
Two real, separate causes, both now fixed — see that commit's message for
the full mechanics: (1) `completing_ids` was cleared as soon as the local
180ms collapse animation finished, not when the DB write actually landed,
so the row briefly re-rendered as a normal unchecked row mid-flight; (2)
`render_task_view` showed a full loading skeleton on every cache
invalidation, which every task mutation triggers, so any tick/delete/
schedule blanked the whole list for a frame. Flow now keeps
`last_tasks`/`last_completed` per view and draws that instead of a
skeleton whenever a fresh value isn't ready yet — the skeleton is reserved
for a view's genuine first load. **Worth a direct re-check** the next time
someone's watching the app: this was fixed once already (motion pass round
1) and still had a real defect, so don't assume it's fully clean until
someone's actually ticked a task and watched it.

**Known, deliberate scope cuts** (not bugs):
- The compact task row shows no subtask progress — only fetched once a
  task is expanded, to avoid an N+1 fetch on every visible row
  (`CLAUDE.md`'s render-path I/O rule).
- PRD §6.3's "empty days with events still show" in Upcoming isn't
  implemented — there's no calendar-events data yet to populate an empty
  day with (Google Calendar glance is a later milestone).

**Not done yet, in the order they're planned:**

- [x] **Motion pass — done, both rounds.** Round 1 (`0b4e4e9`) fixed the
      one reported glitch (completion collapse: opacity and height shrank
      on the same pace, so `overflow_hidden` clipped a still-visible
      checkbox/title mid-shrink — fade now races ahead of the collapse)
      and the two starkest hard-cuts (detail card mount, Completed reveal),
      plus consolidated two duplicated magic durations into
      `ui::motion::REVEAL`/`TRANSITION`. Round 2 (`609b8d5`) added the same
      reveal-on-mount fade to the three remaining `.when(...)`-gated
      surfaces that mounted instantly: the schedule picker's field↔pills
      swap, `bulk_action_bar`, and the complete-with-subtasks confirm
      banner. Every appear/disappear in the app now shares one motion
      vocabulary. Deliberately left alone: the sidebar destination switch's
      instant selection feedback, which matches native macOS convention
      (Mail, Finder) rather than being a gap. Not yet visually verified —
      this session has no screen-recording access; worth an actual look.
- [ ] **`src/app/tasks.rs` has zero keyboard-accessible controls.** Found
      by audit (not requested), 2026-08-19: 0 matches for `track_focus|
      tab_index|on_key_down` against 17 `.on_click(` sites (task rows, the
      completion checkbox, title, schedule pill, process/quick-pick pills,
      delete, subtask checkboxes, the add-subtask row, the Undo toast
      button, "Clear completed", the complete-with-subtasks confirm) —
      every one mouse-only. Contradicts `CLAUDE.md`'s "every mouse-
      reachable control must be keyboard-reachable," the PRD's "complete
      keyboard operation" goal, and §11's acceptance criterion naming a
      task's full lifecycle "without leaving the keyboard."
      `src/app/sidebar.rs` already has the working pattern to copy
      (`render_mode_switch`: `track_focus(&handle)`, `tab_index(N)`,
      `focus_visible(...)`, `on_key_down` matching `"enter" | "space"`).
      **Deliberately not attempted blind**: task rows are a *dynamic* list
      (unlike the sidebar's fixed seven), so this needs a real decision
      about where per-row `FocusHandle`s live (likely a
      `HashMap<String, FocusHandle>` on `Flow`) and probably arrow-key
      navigation between rows — a structural choice that deserves visual
      verification of tab order and focus rings, which this session's
      terminal cannot currently do (no screen-recording permission).

## Where to find things

See `AGENTS.md`'s "Product, design, and planning docs" section — it's the
same list, kept current there so it doesn't drift out of sync with this
document. In short: `PRODUCT.md` (north star), `docs/PRODUCT_REQUIREMENTS.md`
(PRD), `docs/DESIGN_DIRECTION.md` (visual system — keep `theme.rs` matching
it), `docs/turso.md` (DB reference), `CONTEXT.md` (glossary),
`wayfinder/flow-map.md` + `wayfinder/tickets/*.md` (planning history),
`docs/main-pane-blank-regression.md` (a real incident writeup — worth
reading before touching `flow-shell`'s layout or adding any new overlay to
it).

This session's memory is also recorded under Claude's project memory
(`flow-project-overview`, `flow-database-choice`, `flow-doc-map`,
`flow-ui-craft-discipline`, `flow-gpui-skills`) for any Claude session
working in this directory — but that memory is personal to this Claude
account and won't travel with the repo to a new machine or a different
agent, which is exactly why this document exists as the portable,
repo-committed version.
