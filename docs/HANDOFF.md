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
- Working tree is clean as of commit `f4a264f` — check `git status` before
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

**Exit bar met (PRD §12: "all Task core and Parsing acceptance criteria
pass locally"), verified 2026-08-19, not just claimed:**
- Task core (§11): every verb in "create, edit, complete, reopen, move,
  schedule, and undo-delete" now has a real, working path — `edit` was
  the one genuine gap (no way to rename a task existed at all), found by
  this exact audit and fixed in `d1c3621`/`346c2b6`. The rest were
  already there: create (Capture), complete/reopen (checkbox/Space),
  move (`process_task`/`bulk_process`/the schedule picker's quick-picks),
  schedule (NLP parsing + the picker), undo-delete (delete + the Undo
  toast).
- Parsing: both §6.4 acceptance examples ("take out laundry 8 am
  tomorrow", "bring Mya cake in 3 days") have dedicated tests in
  `parse.rs`; an unrecognized phrase leaves the title untouched rather
  than blocking save; Capture is a live-editable field, so every parse
  result can be overridden before saving; stored dates are plain
  `NaiveDate` calendar days with no DST-sensitive time math, so they
  can't drift across a transition.

This doesn't mean Milestone 1 is "done" in every sense — Calendar/
Settings are still placeholders, the keyboard-accessibility pass has
disclosed gaps (Completed/Upcoming rows, arrow-key nav), and nothing in
this session has been visually verified. It means the specific bar the
PRD itself set for exiting this milestone is met, which is a narrower
and more useful claim than "feels finished."

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
on a write failure), a Tasks/Calendar mode pill with a real sliding thumb
(`00377d3` — the first attempt was a per-segment opacity fade that read as
no animation at all per user feedback), and a debug event log + "Flow
state" inspector panel (`d8d1206` — `src/debug_log.rs`,
`crate::debug_log!(...)`, `Flow::debug_snapshot()`; see the
`.claude/skills/flow-debug` skill, gitignored like the other project-local
skills, for how to use both), and virtualized flat-view rendering
(`59825d7`, user-requested — GPUI's `list()`/`ListState`, proven to bound
render cost regardless of item count in `ui/virtualized_list.rs`'s tests,
now backing Inbox/Today/Anytime/Someday for real). Upcoming stays
unvirtualized (date-grouped sections don't fit `list()`'s flat model
without materially more work). **Known limitation**, disclosed in
`task_list_states`'s own field doc: a data change resplices the whole
range rather than diffing old vs new, so any mutation resets scroll
position to the top, not just ones that actually change what's above the
viewport — a minimal-diff splice is the natural follow-up if that's felt
in practice. A real crash was found and fixed here via a macOS crash
report (not visually) — see that commit's message for the exact
mechanism (`sidebar.rs`'s `inbox_count` racing `render_task_view`'s
list-state creation).

**Added (`30e19e3`): the fixture-backed calendar-glance card in Today**,
closing a real gap in Milestone 1's own exit scope (PRD §12: "a small
fixture-backed calendar-glance component only to prove layout" — real
Google Calendar data is Milestone 3). `components::calendar_glance` — a
compact card, today's date, three fixture events (time/title/a per-event
color dot, literal `Hsla` not a theme token, per `DESIGN_DIRECTION.md`'s
"calendar colors never Flow status colors"), gated to `View::Today` only
per §6.3's literal text. Asked the user before building rather than
guessing which shape it should take: `DESIGN_DIRECTION.md` describes a
persistent 3rd-column "calendar rail" that predates and was never
reconciled with this session's Tasks/Calendar mode-switch decision — user
picked the simpler inline-card-in-Today reading over the rail. Skips the
connected/loading/error states the rail's own required-states table
lists, since there's no real "connected" concept yet to be in any of
them — Milestone 3's job.

**Added (`d1c3621`): task title editing** — PRD §11's "edit" verb, found
completely missing by re-checking the acceptance criteria against the
shipped app (same audit technique that found the delete-Undo-toast and
Capture-failure gaps earlier). Click the detail card's title (or Tab to
it, Enter/Space) to edit it inline; Enter saves, an empty submit is
rejected rather than silently discarded. `Db::set_title` mirrors
`set_note`'s shape; `Flow::editing_title`/`title_input` mirror
`subtask_input`'s single-line/Enter-submits shape, not `note_input`'s
blur-saves one. **Real, disclosed behavior change**: the title's own
click now starts editing instead of collapsing the card, so a
collapse-by-click path was added to the header row's own background
instead — but since the title used to occupy nearly the full row width,
that remaining background area is now much thinner than the old target.
Escape and clicking a different row still fully work. Worth a direct
look once there's a chance to see it rendered — this is the kind of
narrowing that's easy to get wrong invisibly.

**Fixed (`f7e45bb`, found via a §10 privacy audit, not a user report):
task titles were leaking into the debug log and inspector panel.** PRD
§10: "Treat task titles and calendar event titles as private data in
UI, logs, and telemetry." A handful of `debug_log!` calls and
`debug_snapshot()`'s Undo-toast line — all added earlier this
session — embedded raw task titles into the persistent plain-text
debug log file and the Cmd-Option-I inspector panel, exactly the
"logs and telemetry" case the requirement names (and this session's
own `flow-debug` skill tells agents to go read that log file, so it
was actively being surfaced, not just theoretically at risk). Fixed
four call sites, keeping the task id (already enough to find the
event) and dropping the title: `tasks.rs::delete_task`'s log line,
both of `app.rs`'s capture success/failure log lines, and
`debug_snapshot()`'s Undo-toast line (now shows `toast.task_id`
instead of `toast.title`). Every other `debug_log!` site was checked
and already only logs ids/error messages. `cargo check`/`cargo test`
clean (180 passing); rebuilt via the dev watcher, no new crash report.

**Fixed (`f4a264f`, found via a PRD §8 data-model constraints audit):
a completed parent could still accept a new subtask.** §8 says "A
deleted or completed parent cannot accept a new open subtask" —
`create_subtask`'s parent lookup only ever checked `deleted_at IS
NULL`, so the completed half was silently unenforced (a completed
task's detail card and its "+ Add subtask" row are both still
reachable in the UI). Fixed by adding `completed_at IS NULL` to the
same lookup query. New test:
`a_completed_parent_cannot_accept_a_new_subtask`.

**Fixed (`346c2b6`, found by self-reviewing the commit above): title
edits could be silently lost.** The field only saved on Enter — exactly
the same exposure `note_input` had before `set_expanded_task`'s
proactive-flush fix earlier this session: GPUI does not blur a focused
element just because a click lands on something that isn't itself
`track_focus`'d (the detail card's own completion checkbox, for one), so
a typed-but-unsubmitted rename could vanish with no save and no warning.
Fixed the same way: a real `on_title_blur` handler plus a proactive
`flush_title` call from `set_expanded_task`. Also caught a second bug
introduced while wiring the first fix: `toggle_expanded` cleared
`editing_title` *before* calling `set_expanded_task`, so `flush_title`
always saw a false "nothing was being edited" and no-opped — the exact
bug being fixed, still present via that one path. Both are fixed now;
Escape's own path deliberately still skips flushing (discard, not save,
matching Capture's Escape convention).

**Fixed (`85cbc31`, user-reported via screenshot): rows collapsed to a
narrow shrink-wrapped column** instead of filling the pane. Root cause
(confirmed by reading `gpui::list()`'s own source, not guessed): each
visible row is laid out via `element.layout_as_root(...)` — the root of
its own fresh layout tree, not a flex child of the stretch-by-default
container the previous plain `.children()` list gave it implicitly. Both
shapes `render_task_row` returns (the compact row, the expanded detail
card) needed an explicit `.w_full()` they'd never needed before. The same
screenshot also showed a task titled "deleting doesnt work" — plausibly
the same root cause (the shrink-wrapped detail card likely misaligned its
delete button's hit target from what was visible), but **not
independently confirmed** — worth the user re-testing delete specifically
now that this is fixed, rather than assuming it's resolved.

**Fixed (`2a37a88`, found by reading further, not a new report): a
second, subtler width bug in the same area.** `gpui::list()`'s own
`.px()` is a no-op for item positioning — `prepaint_items` always places
each item at `bounds.origin.x + 0`, ignoring horizontal padding entirely
(only vertical padding, via `item_origin.y += padding.top`, actually
does anything). The immediately preceding fix's `.px(px(24.0))` directly
on the `list()` element compiled clean and looked reasonable but was
silently inert — rows would have hugged the pane's true left edge with
no inset. Fixed by wrapping `list()` in a plain padded div instead, so
normal box-model layout (not `list()`'s special item-positioning code)
does the insetting; the overlay scrollbar moved to be a sibling of that
wrapper, not a child, so it still hugs the pane's true edge rather than
getting pulled in by the wrapper's own padding. **Worth watching this
specific spot** (`gpui::list()` + padding) if it ever comes up again
elsewhere in this codebase — it's an easy trap that compiles silently.

**Fixed (`dea6184`, found while adding the log above): a stuck-UI bug.**
`write_completed`/`delete_task` silently swallowed a failed DB write with
no log and no recovery. Writing the log line surfaced a real consequence
in the completing=true path specifically: without a successful write,
`completing_ids`' pruning (added a few fixes ago, see below) never gets
the fresh `Ready` read it needs to clear the id — so a failed
completing-write left a row permanently stuck showing as collapsed/
checked, with no way out short of restarting the app. Both paths now log
the failure; the completing one also explicitly clears `completing_ids`
instead of waiting for a confirmation that would never arrive.

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
for a view's genuine first load. `043d38a` applied the same fix to
subtasks (`last_subtasks`) — toggling one had the identical flicker on its
own smaller scale (the "Subtasks (N/M)" count and indented list, not the
whole view). `75e5a88` found and fixed a third instance: the sidebar's
Inbox badge fell back to a hardcoded `0`, so completing/deleting/
processing an Inbox task flashed the badge number to 0 and back too — now
reuses the same `last_tasks` fallback. **Worth a direct re-check** the
next time someone's watching the app: the row-level piece of this was
"fixed" once already (motion pass round 1) and still had a real defect, so
don't assume any of it is fully clean until someone's actually ticked a
task and watched it.

That last warning turned out to be justified: `a95a305` (user-reported,
same session) found that `9810093`'s own two fixes interacted badly —
clearing `completing_ids` the moment the write succeeded raced against
`last_tasks`' stale fallback (which still held the pre-write snapshot for
one more render), so the row flashed back to full height, visible as
everything below it jumping up. `completing_ids` is no longer cleared
speculatively in `write_completed`'s success path; it's pruned in
`render_task_view` itself, only once a genuinely fresh `Query::Ready`
confirms the task is actually gone. Reopening (checkbox or Undo) still
clears it immediately and synchronously — a deliberate reversal doesn't
need a fetch to confirm it, and without that carve-out an Undo landing
mid-flight would permanently stick the row in its collapsing state. Given
this exact area has now had a real defect survive two "fixed" rounds in a
row, treat any future report on it as probably real rather than assuming
it's a duplicate — this is the kind of interaction that's easy to
re-introduce with the next unrelated change nearby.

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
- [x] **`src/app/tasks.rs` keyboard accessibility — every individually-
      tracked control done, across eight commits** (`01156dc` through
      `7e9987e`). Original audit (2026-08-19): 0 matches for
      `track_focus|tab_index|on_key_down` against 17 `.on_click(` sites,
      contradicting `CLAUDE.md`'s "every mouse-reachable control must be
      keyboard-reachable," the PRD's "complete keyboard operation" goal,
      and §11's acceptance criterion naming a task's full lifecycle
      "without leaving the keyboard." All fixed now: the compact row
      (Enter opens, Space completes — a deliberate choice over a second
      tab stop for the checkbox, extending the app's existing "bare Space
      acts on the task" convention rather than doubling tab stops across
      a long list), the Undo toast button, "Clear completed", the detail
      card's delete button, schedule pill, the picker's
      Today/Anytime/Someday/Clear quick-picks, the "Complete parent and
      all subtasks?" confirm's two buttons, subtask checkboxes (same
      Space-toggles convention, no Enter since a subtask has no card of
      its own under the one-level ceiling), and the "+ Add subtask" row.
      Two focus-handle shapes used depending on cardinality: single
      stable fields for anything bounded to "one at a time" (delete
      button, schedule pill, confirm banner — only one card expanded
      ever), pruned `HashMap<String, FocusHandle>`s for genuinely dynamic
      per-task sets (`row_focuses` for top-level rows, a *separate*
      `subtask_focuses` map for subtask rows — deliberately not shared,
      since `row_focuses`' pruning runs against the flat top-level list
      and would delete every subtask handle on the next unrelated
      refetch; documented in both fields' doc comments so it isn't
      "simplified" back together later).
      **Genuinely out of scope, not silently dropped:**
      - Completed-section rows and Upcoming's rows still pass `None` for
        `focus` (`render_task_row`'s param doc) — mouse-only.
      - Arrow-key navigation between rows — Tab order is currently the
        only way to move focus between tasks; no listbox-style arrow
        handling.
      **Fixed separately** (`c5c91f9`): every `ComposerInput` (Capture,
      notes, the schedule field, subtask-add) now has `.tab_index(0)`
      alongside its existing `track_focus`, one line in the shared
      component covering all four fields at once. They were already
      correctly auto-focused via explicit `window.focus(...)` calls when
      their section opens — the actual gap was no way *back* into the
      field via Tab once focus left it. No `.focus_visible()` added: a
      text field's caret/selection already carries focused state visually.
      Not yet visually verified — this session has no screen-recording
      access; worth an actual Tab/Enter/Space walkthrough of a real task
      list before trusting the feel of it.

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
