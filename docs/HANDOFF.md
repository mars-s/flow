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
  8. `c215709` — Inbox view + sidebar badge wired to the real database.
  9. `e28af28` — handoff doc: Inbox+badge checkpoint noted.
  10. `e407738` — "+ Capture" now creates real tasks.
  11. `6ecd6db` — handoff doc: capture-wiring checkpoint noted.
  12. `863d31d` — Today/Upcoming/Anytime/Someday wired to real
      bucket-filtered data, plus `Db::schedule`.
  13. `b5dca80` — handoff doc: five-views + `schedule()` checkpoint noted.
  14. `c765ece` — the NLP date/time parser (`src/parse.rs`), wired into
      capture; also records the user's global-hotkey quick-capture north
      star in the PRD §13.
  15. `fe8df26` — handoff doc: NLP parser + capture-wiring checkpoint noted.
  16. `05ef517` — Inbox's inline "Process" action (Today/Anytime/Someday).
  17. `b688ac4` — handoff doc: Process-action checkpoint noted.
  18. `1a026b1` — "Schedule…" added to Process, reusing `parse.rs` for
      arbitrary-date entry instead of a calendar widget.
  19. `ecfaf3d` — handoff doc: Schedule-action checkpoint noted.
  20. `c4bc8c3` — task detail card redesign (raised surface, note field,
      schedule pill, delete), the schedule-clipping display bug fix, and
      Cmd+click multi-select with a bulk-action bar — built in an isolated
      git worktree by a subagent, reviewed, and fast-forward merged (see
      "Milestone 1 progress" below).
  21. `75df865` — fixed a duplicate task title rendering in the expanded
      detail card (the compact row and the card both drew a header; the
      card now replaces the row instead of sitting below it).
  22. `9d6a156` — stopped bulk process/delete from silently swallowing
      per-task errors (now logged via `eprintln!` instead of `let _ =`).
  23. `6869c1c` — icons on the Today/Anytime/Someday/Schedule… pills,
      space-to-capture from any task view, friendly schedule formatting,
      a collapsed-by-default "Completed" section per view, and live
      highlighting of the parser-recognized date/time phrase in Capture/
      Schedule… — built in an isolated git worktree by a subagent
      (interrupted once by a usage-limit reset mid-run, resumed from the
      same worktree), independently re-verified, fast-forward merged.
  24. `c8f2922` — made the custom titlebar actually draggable
      (`window.start_window_move()` on mouse-down; the `window_control_area`
      tag alone only feeds Windows' non-client hit-testing, not macOS),
      fixed the sidebar divider not running the full window height, and
      reverted an over-correction that had centered the task list into a
      narrow column — kept as widescreen per explicit user feedback.
  25. `013c5fb` — a "Clear" option on the schedule picker to remove a
      task's schedule (previously only changeable, never removable).
  26. `0133d87` — completion collapse animation and a 10-second Undo toast.
  27. `326837b` — renamed the debug app from "Flow Debug" to "Flow Dev"
      (display name and data directory; bundle identifier unchanged),
      done mid-incident to rule out stale per-app WindowServer/
      LaunchServices state as the cause of the regression `53a744b` fixed.
  28. `53a744b` — fixed a blank-main-pane regression: `render_drag_bar`
      needed `absolute()` (it was starving the main pane to zero width as
      a plain flex sibling demanding the full row's width). See
      `docs/main-pane-blank-regression.md` for the full incident writeup —
      worth reading before touching `flow-shell`'s layout or adding any
      new overlay to it.
- Working tree is clean as of commit `53a744b` unless the "Milestone 1
  progress" section below says otherwise — check there for what's currently
  in flight before assuming everything is committed.
- The `/loop` (self-paced autonomous mode) that was running earlier in this
  session was stopped once the user started actively steering with live
  feedback (reference screenshots, specific complaints) — direct
  conversation, not autonomous looping, is the current mode. A `/loop` can
  be restarted later if the user wants to hand off to autonomous work again.

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
Milestone 0 commit, 147 in `src/app` + `src/db.rs`'s lib tests as of commit
`863d31d` (`cargo test --package flow --lib`). Run that command to check
the current count/status rather than trusting this number as it ages.

## Current UI state

- Sidebar (`src/app/sidebar.rs`, 252px, matches
  `docs/DESIGN_DIRECTION.md`'s navigation-rail spec):
  - "Flow" wordmark, a "+ Capture" row that **works** — click/enter/space or
    `⌘N` opens a real composer field; Enter creates the task via
    `Db::create_task`; Escape closes it. See "Milestone 1 progress" below.
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
- Main pane: **all five task views are real** (`src/app/tasks.rs`,
  `render_task_view`) — Inbox, Today, Upcoming, Anytime, and Someday all
  read/write the actual database through `db::View`. Task rows show a
  friendly schedule label ("Today"/"Tomorrow"/weekday/short date + 12-hour
  time — see the "Milestone 1 progress" section below for
  `format_schedule`, which superseded the raw `YYYY-MM-DD [· HH:mm]` this
  paragraph originally described). Completed tasks live in a collapsed
  "Completed" section per view rather than vanishing. Calendar and Settings
  still render `components.rs`'s "Coming soon" placeholder.
  **No subtasks or toast/notification component exist yet** — the user
  showed reference screenshots for these (Things-3-style task row with
  note + subtask, a toast banner) but explicitly deferred them out of the
  sidebar-focused UI pass; still future work. Scheduling a task now has
  three real paths: typing a date phrase into Capture, the detail card's
  Today/Anytime/Someday/"Schedule…" picker, and the same picker's bulk
  variant for multi-select — the original "no UI path to schedule" gap
  this paragraph described is closed.
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
- [x] **Capture works.** "+ Capture" (click/enter/space) or `⌘N` from
      anywhere opens the sidebar's Capture row as a real `ComposerInput`
      field (reused, not a new widget — it already emitted
      `ComposerEvent::Submit` on Enter and self-cleared). Submitting calls
      `Db::create_task` and invalidates the Inbox cache. Escape closes it —
      this repurposes two actions that were already bound but unhandled
      dead code left over from the Waku strip (`NewTask` on `secondary-n`/
      the app menu, `CancelTurn` on `escape` at the "Flow" key context) —
      see commit `e407738`'s message. **Known gap**: no confirmation before
      Escape discards unsaved text, since a bare title field has nothing to
      confirm yet; revisit once the composer grows a note field.
- [x] **All five task views render real, bucket-filtered data**, not just
      Inbox. `db::View` (Inbox/Today/Upcoming/Anytime/Someday) is the
      UI-facing address, distinct from the storage-level `Bucket`
      (Inbox/Active/Someday); Today/Upcoming/Anytime all read
      `Bucket::Active`, sliced by `scheduled_date` against
      `chrono::Local::now()`. One generic `render_task_view`/
      `render_task_row` pair serves all five — `Destination::view()`
      (`sidebar.rs`) is the single Destination→View mapping everything
      else routes through. Today/Upcoming rows show a trailing
      `scheduled_date`/`scheduled_time` label (raw, unformatted).
- [x] `Db::schedule(id, bucket, date, time)` — moves a task between
      placements with an optional date/time, PRD §5's "Move to active" /
      "Schedule and activate" actions. Two tests move a real task through
      Anytime → Today → Upcoming by date (against real `chrono` dates, not
      fixed strings) and confirm Someday stays isolated.
- [x] **Deterministic NLP date/time parser** (`src/parse.rs`) — the full
      PRD §6.4 supported-forms table: today/tomorrow, "in N days" (1-365),
      weekday/"next weekday" (with the today-matches-next-week rule),
      explicit dates (three input orders), 12h/24h times, both date-then-
      time and time-then-date combinations. Ambiguous forms (bare "at 8",
      "next week", a past month/day with no year, an impossible date) are
      left unrecognized rather than guessed, per PRD principle 3. Pure and
      deterministic — takes `today` as a parameter, never reads the clock
      itself. 16 tests, including both of the PRD's exact acceptance-case
      titles verbatim.
- [x] **Capture now runs every title through the parser.** A recognized
      suffix is stripped and stored as `scheduled_date`/`scheduled_time`
      via `Db::schedule`; the task's bucket stays `Inbox` (PRD §14: a
      parsed date does not auto-activate a task). A db.rs test locks this
      in — a scheduled Inbox task stays out of Today. This is the one real
      way, right now, that a task ends up with a schedule at all: typing
      "take out laundry 8am tomorrow" into Capture works end to end. There
      is still no way to schedule or reschedule a task **after** it's
      captured (no "Process" action, no date picker) — see the gap below.
- [x] Recorded the user's stated long-term direction for capture — a
      global-hotkey, always-on-top, natural-language quick-capture popup
      reachable from any app, likely paired with a menu bar mode — in
      `docs/PRODUCT_REQUIREMENTS.md` §13 and this project's memory. Not
      started; noted here so it isn't lost. The in-app composer
      (`open_capture`/`capture_input` in `app.rs`) is deliberately built as
      a self-contained, reusable unit specifically so this later surface
      can host the same field and submit logic.
- [x] **Inbox's inline "Process" action** (PRD §6.3: "an inline 'Process'
      action that offers Today, Anytime, Someday, and schedule"). Clicking
      an Inbox row (not its completion circle) opens a row of three quick
      buttons underneath it; picking one calls `Db::schedule` and
      invalidates both Inbox and the destination view
      (`Flow::process_task`/`toggle_processing` in `app/tasks.rs`). Only
      one row processes at a time (`Flow.processing_task_id`). This is the
      first way, besides typing a date phrase into Capture, to move a task
      out of Inbox.
- [x] **"Schedule…" (the fourth PRD-named Process option, an arbitrary
      date).** Rather than a calendar widget, it swaps the three buttons
      for a free-text field and runs the input through the same
      `parse.rs` Capture uses (`Flow::on_schedule_event`, `app.rs`) —
      reuse over building a new component. Schedules via
      `Db::schedule(id, Bucket::Active, ...)`; unlike Capture's
      parse-at-creation path (which deliberately leaves the bucket as
      Inbox, PRD §14), this is the explicit user-driven "Schedule and
      activate" action, so it does activate. An unrecognized phrase leaves
      the field open rather than guessing. Escape now closes whichever of
      scheduling/processing/capturing is open, checked in that order.
- [x] **Fixed: the schedule-clipping display bug.** A task's
      `scheduled_date`/`scheduled_time` is now shown on its row in *every*
      view, not just Today/Upcoming. This was a real bug, not a missing
      feature: `parse.rs` was already correctly stripping "tomorrow" out of
      "take out trash tomorrow" and storing the date, but nothing displayed
      it for an Inbox task — the date effectively vanished. User-confirmed:
      the title-clipping itself is correct and intended; only the display
      gate was wrong. Fixed in `render_task_row` (`app/tasks.rs`) —
      `schedule_label(&task)` now runs unconditionally instead of being
      gated on `view`.
- [x] **Task detail card**, replacing the old bare pill-row Process
      interaction. Matches `docs/DESIGN_DIRECTION.md`'s already-specified
      "Task detail" component (a `theme.raised`, 10px-radius surface, not a
      modal) rather than the user's reference app's literal look — their
      reference showed tag/checklist/flag icons that are *that app's*
      features, which Flow doesn't have, so those were deliberately not
      copied; only real, wired controls made it in. Contains: checkbox +
      title header, a note field (new `Db::set_note`, saved on blur since
      the field is multiline and Enter inserts a newline instead of
      submitting), a schedule-status pill showing the task's current
      placement (click reopens the Today/Anytime/Someday/"Schedule…"
      picker — the same one from before, relocated), and a delete action
      (new `Db::delete_task`, soft delete via `deleted_at`, which every
      `list_view` query already filters on for free). Available from every
      task view now, not just Inbox, per the design doc's general spec.
      Subtasks are skipped — no `parent_id` UI or queries exist yet.
- [x] **Cmd+click multi-select**, with a bulk-action bar (same
      Today/Anytime/Someday/Delete actions, applied to the whole selection)
      appearing at 2+ selected rows. Uses `gpui::Modifiers::secondary()` —
      a ready-made cross-platform Cmd(mac)/Ctrl(other) check already in the
      pinned GPUI fork, found by reading `ClickEvent`/`MouseDownEvent`
      usage in `src/input.rs` rather than guessing at field names.
- Built via an isolated `git worktree` subagent (per explicit user
  request), independently re-verified by the coordinating session before
  merging — not just trusted from the agent's own report. 166 tests
  passing as of commit `c4bc8c3` (2 new: `set_note`, `delete_task`);
  verified in the running debug app after merge, not just `cargo check`.
- [x] **Fixed: duplicate task title in the expanded detail card** (`75df865`)
      and **bulk process/delete no longer silently swallow per-task errors**
      (`9d6a156`).
- [x] **Icons on the schedule-picker pills** — Today/Anytime/Someday/
      Schedule… now show their sidebar-matching icon (`star`/`layers`/
      `archive`/`calendar`) instead of plain text, no per-item color (kept
      the single-accent rule).
- [x] **Space opens Capture from any task view.** Bound globally
      (`SpaceCapture`/`lib.rs`) but scoped off composer focus via a
      `"Flow && !ComposerInput"` key-context predicate, plus a defensive
      `capturing`/`scheduling`/`expanded_task_id` guard, so a literal typed
      space in Capture/Schedule…/the note field is unaffected.
- [x] **Friendly schedule formatting** (closes the gap below) —
      `format_schedule` in `app/tasks.rs` renders Today/Tomorrow/weekday
      name (next 6 days)/short date (`"Aug 23"`), with 12-hour time
      appended (`"Tomorrow 6:00 PM"`). Used by both the row's trailing
      label and the detail card's status pill.
- [x] **Collapsed-by-default "Completed" section per view**, instead of a
      completed task disappearing entirely. New `Db::list_completed(view)`
      mirrors `list_view`'s bucket/date filtering with
      `completed_at IS NOT NULL`. Docked at the bottom of the view; when
      expanded it grows upward capped at 280px before scrolling internally
      rather than pushing the open list further up or growing unbounded.
      Un-completing a row from there works via the same completion circle.
- [x] **Live highlight of the recognized date/time phrase while typing**
      Capture (and Schedule…) — `parse.rs`'s `ParsedTitle` now carries a
      `source_range`, and `ComposerEvent::Edited` repaints it live using
      the composer's existing search-match highlight mechanism
      (`ComposerInput::set_search_matches`), reused rather than building a
      second highlight system. This is the "clickable preview chip" gap
      below, resolved as an in-place highlight instead of a separate chip
      per explicit user direction ("just highlighting the words is fine").
- [x] **Custom titlebar is actually draggable**, and the traffic-light /
      "Flow" wordmark overlap is fixed. An invisible full-width strip is
      layered on top of the window (not its own flex row, to keep the
      sidebar's divider running the full height) calling
      `window.start_window_move()` on mouse-down; double-click zooms via
      `window.titlebar_double_click()`. The sidebar's top padding is tied
      to the same `DRAG_BAR_HEIGHT` constant so the two can't drift apart.
- 169 tests passing as of commit `6869c1c` (3 new: `list_completed`, two
  `parse.rs` `source_range` tests); the window-chrome fixes in `c8f2922`
  added no new tests (pure layout/interaction, no new logic to unit-test).
  Both increments verified in the running debug app, not just `cargo check`.

- [x] **A "Clear" option to remove a task's schedule** (`013c5fb`) — a
      fifth pill in the schedule picker, shown only when the task actually
      has a schedule. `Flow::clear_schedule` writes `date=None, time=None`
      via `Db::schedule` while keeping the task's current bucket, so an
      Active task drops back into Anytime rather than needing a bucket
      choice of its own.

- [x] **Completion collapse animation + 10-second Undo toast** (`0133d87`)
      — the checkbox fills immediately, the row fades and collapses over
      the existing 180 ms `ROW_TRANSITION` (reusing the row's mount
      fade-in mechanism for the reverse direction, keyed to a distinct
      animation id so it gets a fresh timeline instead of jumping to the
      fade-in's already-elapsed one), then a floating single-slot toast
      offers Undo for 10 seconds (PRD §6.1's delete-undo window, reused
      since completion's own spec doesn't name a different one). Reopening
      (including via the toast, or from the Completed section) still
      writes immediately — nothing to animate out.

**Not done yet, in the order they're planned:**

- [ ] Upcoming currently renders as one flat list ordered by date, not
      PRD §6.3's grouped-by-day sections with weekday headers. Correct
      data, simplified presentation — a deliberate scope cut to land real
      data first, not an oversight.
- [ ] One-level subtasks and task detail expansion (`parent_id` has no UI
      or queries yet) — still deferred, not part of this session's UI
      passes.
- [ ] **Notes don't actually persist/work right now** — user-reported,
      not yet diagnosed. `Db::set_note` exists and is tested at the DB
      layer, and the note field saves on blur (`Flow::on_note_blur`), but
      the user reports "a lot of things don't work" around notes in the
      live app. Explicitly deferred by the user pending a dedicated look;
      next session should reproduce and diagnose before assuming the fix.
- [ ] Update `docs/PRODUCT_REQUIREMENTS.md` §9 (Convex → Turso, still
      describes the old self-hosted-Convex sync plan throughout — Milestone
      2's description, the deployment target, the architecture diagram, the
      reference links at the bottom) and §5's IA diagram (still shows the
      flat 7-destination sidebar, not the Tasks/Calendar mode switch) so
      the PRD stops contradicting the actual code and this session's
      decisions. Low-risk, low-effort, just hasn't been done — do this
      whenever convenient, doesn't block anything else. (One stray "Flow's
      GPUI shell" → "Waku's GPUI shell" typo nearby *was* fixed while
      editing §13 for the quick-capture note above.)

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
