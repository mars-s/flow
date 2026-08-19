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
- Working tree is clean as of commit `1b72d08` — check `git status` before
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

**In progress (`b76195e`), redirected by explicit user decision the night
of 2026-08-19: calendar integration is macOS EventKit, not Google OAuth.**
User showed a reference screenshot of Apple Calendar's own
Day/Week/Month/Year layout and asked for that shape on the Calendar tab,
plus the Today glance hidden until a calendar is connected via Settings.
PRD §6.5/Milestone 3 rewritten to match (see that commit's own message for
the full rationale — same "avoid backend code" reasoning that picked Turso
over Convex). Landed so far:
- `src/eventkit.rs` (new, macOS-only): the objc2-event-kit bridge —
  `authorization_status`/`request_access`/`events_between`/`list_calendars`,
  converting every EKEvent/EKCalendar into plain Rust structs.
- `src/platform.rs`: the macOS/non-macOS split wrapping it
  (`calendar_authorization_status`, `calendar_request_access`,
  `calendar_events_between`, `calendar_list`, `open_calendar_privacy_pane`),
  matching this file's own existing convention.
- `src/app/settings.rs`: Settings' first real content — a calendar section
  with the read-only disclosure, a "Connect Calendar" button, and the
  denied-state System-Settings deep link. Also fixed a real routing gap
  found in the process: `render_settings` existed but `render_main_pane`
  never called it, so Settings always fell through to the generic
  placeholder pane.
- Today's `calendar_glance` now reads real `Flow::today_calendar_events`
  (fetched once per Today-view mount, not from the render path) and the
  whole card is hidden unless `calendar_auth == Granted`, per §6.5's
  "hidden entirely rather than showing an empty or disconnected state."
- Cargo.toml/`resources/Info.plist`: the exact objc2-event-kit feature set
  and `NSCalendarsFullAccessUsageDescription`/`NSCalendarsUsageDescription`
  this needs.

**Investigated, not caused by this work**: two SIGABRT crashes
(`panic_bounds_check` in `task_list`'s virtualized-list closure,
`tasks[ix]` out of bounds) appeared on this session's first post-change
relaunch. Bisected against the pre-calendar baseline (`352af5c`) through
the identical launch path — it didn't crash there either — then confirmed
clean on five more relaunches of the actual current code. The crash's own
stack is GPUI's list-item prepaint, nothing calendar-related, and Inbox
(not Today) is the default landing destination — points at the exact
undiffed-resplice race `task_list_states`'s own field doc already
discloses as a known limitation, just newly observed as an actual crash
rather than only a scroll-position quirk. **Worth root-causing properly**
if it recurs — see that field's doc for the minimal-diff-splice follow-up
already named there as the fix shape.

**Added (`e8e910c`): the Calendar tab's Day/Week views** —
`Destination::Calendar` is real now (`src/app/calendar.rs`), not the
generic placeholder: a per-account calendar sidebar with color-coded
visibility toggles, Day/Week switch, Today/‹/› navigation, and a quiet
"No calendar connected → Open Settings" state matching Today's own
hidden-until-connected principle. Keyboard-accessible from the start, not
retrofitted. **Disclosed simplification**: an agenda-per-day layout (each
day's events listed top-to-bottom), not Apple Calendar's pixel-accurate
hour grid with time-of-day positioning and overlap resolution — see that
commit's message for the full reasoning. A multi-day event files under its
start date's column only.

**Added (`c9e6d74`): the Calendar tab's Month and Year views**, completing
the Day/Week/Month/Year scope `wayfinder/tickets/eventkit-calendar-tab.md`
set out. Month is a traditional expanded-to-full-weeks grid (3 events per
cell then "+N more"); Year is twelve small month grids with an event dot
per day and click-to-drill-down into Month mode. `Flow::navigate_calendar`
now takes a `-1/0/1` step and decides the actual jump size (day/week/
month/year) per mode internally, rather than each caller computing it.

**Fixed (`5967647`, found via self-review, not a user report): the Calendar
tab's fetch could apply a superseded result.** `refresh_calendar_tab` had
no generation guard — rapid navigation/mode-switching could let a slower
earlier fetch (e.g. a full-year query) land after a faster later one and
silently overwrite `calendar_range_events` with stale data. New
`calendar_fetch_generation`, same fix shape as `query.rs`'s own generation
counter for `tasks`/`subtasks`, just not routed through `QueryCache` since
a calendar fetch's key (mode + cursor) isn't `View`-shaped.

**Fixed (`51105df`, found via a PRD §7 hit-target audit against last
cycle's own new code): four Calendar tab controls were under the app's own
28px minimum** — the sidebar visibility toggle row, ‹/› nav, "Today", and
the Day/Week/Month/Year mode buttons were 20–24px against a 28px floor the
rest of the app already respects (`sidebar.rs`'s `ROW_HEIGHT`). Bumped all
four, extending the hit region rather than the glyph.

**Fixed (`041d301`, found via a self-review sweep): Settings' "Connect
Calendar" and "Open System Settings" buttons were mouse-only** — no
`track_focus`/`tab_index`/`on_key_down` at all, and no explicit height
(close to but not reliably at the 28px minimum). Both fixed the same way
as every other button this session: single-stable `FocusHandle`s
(`settings_connect_calendar_focus`, `settings_open_privacy_focus`),
`focus_visible`, Enter/Space handling, `h(px(28.0))`.

**Fixed (`3fbcf7d`, found via a self-review sweep): the calendar sidebar's
visibility toggle encoded "hidden" in opacity alone.** No icon, no shape
change, no text distinction — CLAUDE.md: "never encode meaning in color,
hover, or motion alone." Fixed by making the dot itself filled (visible)
vs. hollow (hidden), with the calendar's own color kept as the border
either way, plus `text_ghost` instead of blanket opacity on the title.

**Not done yet**: whether calendar visibility toggles should persist
across launches (currently don't — no settings-persistence infrastructure
exists anywhere in this codebase yet, a real gap worth its own ticket if
felt in practice rather than bolted on here). The true pixel-accurate
hour-grid positioning for Day/Week (vs. the current agenda-per-day layout)
is also still open, disclosed as a simplification in `e8e910c`'s message.
See `wayfinder/tickets/eventkit-calendar-tab.md` for both.

**Fixed (`64b0f36`, found via a PRD §11 acceptance-criteria audit): the
compact row's own checkbox (and Space) could complete a parent with
open subtasks with zero confirmation.** §6.2's "Complete parent and
all subtasks? / Cancel" confirm only ever ran through the detail
card's checkbox, which has its subtasks already loaded to decide
`has_open_subtasks`. The compact row — the primary completion path for
every task — called `toggle_completed` directly with no gate. New
`request_complete_from_row`: a background `list_subtasks` fetch
on-click (a one-shot action, not a render path, so this doesn't
reopen the render-path-I/O question — see that method's own doc), then
either completes immediately (no open subtasks, the common case) or
expands the card into the same confirm banner the detail card already
has. Reopening is untouched.

**Fixed (`80485d5`, same doc-staleness sweep): PRODUCT.md still named
Convex and Google Calendar** in its Stack/Capabilities sections (both
superseded — Turso, EventKit), plus a "strips Flow's GPUI shell" typo that
should read "Waku's" (confirmed against
`wayfinder/tickets/choose-distribution-boundary.md`'s own resolution text).

**Flagged, not fixed (`359411c`, same doc-staleness sweep):
`docs/DESIGN_DIRECTION.md`'s hero mockup and "Calendar rail" section
describe the pre-mode-switch 3-column IA** (already superseded per this
doc's own "Decisions made" §4, and now further out of date given tonight's
full Day/Week/Month/Year Calendar tab). Added a banner rather than
rewriting the mockup/component spec myself — that's real design-visual
work this session deliberately left for a dedicated pass (possibly
`impeccable document`), not a mechanical text fix. Spot-checked before
claiming the rest of the doc still matches: theme.rs's dark-mode token
hex values, `SIDEBAR_WIDTH` (252px), task row height (40px), corner radius
(10px) all genuinely check out.

**Fixed (`7af22a4`, found via a doc-staleness sweep): `docs/performance.md`
and two spots in `AGENTS.md` still described the Waku coding-agent
product.** `docs/performance.md` is entirely about streaming-transcript
performance (provider chunks, a reasoning veil, pane caching under a
stream) — mechanisms Milestone 0's strip deleted; confirmed via grep
(zero references anywhere in `src/`) and clippy (the pulse-clock functions
it governs are all dead code). Added a staleness banner rather than
deleting it — the counter-based measurement playbook in its own
"Measuring" section is still real, reusable technique. `AGENTS.md`'s own
Performance section rewritten to match, plus two more stale spots caught
in the same pass: the top-line description still said "Google Calendar
glance," and the `turso.md` pointer still said "once the local task store
is being built."

**Fixed (`ad95f6f`, found via a PRD §10 idempotency audit — a correction
of a dismissal made earlier the same night): Capture's create+schedule
wasn't atomic, a real local duplicate-task path.** `submit_capture` did
`create_task` then `schedule` as two separate writes; if the schedule call
failed after create landed, the whole capture was reported as failed while
a real task sat unscheduled in Inbox, invisible to the user — and clicking
Retry created a second, duplicate task on top of the first. New
`Db::create_task_scheduled` wraps both in a real `BEGIN`/`COMMIT`/
`ROLLBACK` transaction, skipped entirely for the common unscheduled-capture
case. New tests: `create_task_scheduled_is_atomic`,
`create_task_scheduled_activates_the_task_when_it_succeeds`. (The PRD's own
client-mutation-ID note from earlier tonight has a correction appended —
that dismissal was right about network retries but missed this local gap.)

**Fixed (`4994c56`, found via a PRD §6.1 audit): bulk delete had no Undo
toast.** Single-row delete always showed the 10-second Undo toast; the
multi-select action bar's "Delete" permanently removed every selected task
with zero recovery — arguably the higher-risk path, since it's one shared
button after selecting several rows. `UndoToast.task_id: String` became
`task_ids: Vec<String>` (a new `summary: bool` decides quoted-title vs.
plain-count display); `undo_last_action`'s Delete branch now restores every
id in a loop; `bulk_delete` only shows the toast for ids that actually
succeeded.

**Hardened (`1d4fa7b`, found via self-review of the cascade fix below): the
subtask delete/restore cascade wasn't itself transactional.** Same
non-atomicity shape as `create_task_scheduled`'s own fix — two sequential
UPDATEs (parent, then subtask cascade) with nothing wrapping them, so a
failure between them could still produce the exact orphaning bug the
cascade exists to prevent, just less likely to trigger. Both now use a
real `BEGIN`/`COMMIT`/`ROLLBACK`, matching `create_task_scheduled`'s
pattern.

**Fixed (`57c2b1d`, found via a data-integrity audit): deleting a parent
task orphaned its subtasks instead of deleting them.** `delete_task` only
ever touched the one row by id; a subtask never appears in any top-level
view (`list_view` always filters `parent_id IS NULL`), so the subtasks
vanished from the UI along with their parent but stayed
`deleted_at IS NULL` in the database forever — permanently unreachable,
never cleaned up. `delete_task` now cascades to every non-deleted row with
that `parent_id`; `restore_task` (Undo) symmetrically un-cascades, so
Undo-ing a parent's delete brings the whole family back. New tests:
`deleting_a_parent_cascades_to_its_subtasks`,
`undoing_a_parent_delete_restores_its_subtasks_too`.

**Fixed (`58fc090`, found via a PRD §8/§10 constraints audit): a bare
parsed time ("8am") wrote invalid data instead of the supported form PRD
§6.4's own table describes.** `parse.rs`'s `TIME_ONLY` pattern produces
`date: None, time: Some(_)` by design (a bare time is a listed, supported
form), but nothing defaulted a date for it, so it silently wrote
`scheduled_date: NULL, scheduled_time: '15:00'` — invalid per §8, and
functionally orphaned (a NULL date lands the task in Anytime, which never
renders a time at all). Fixed at both layers: `Db::schedule` now rejects
the combination outright (the actual enforcement point, any caller), and
both call sites that read a parser result (`submit_capture`, the
schedule-input handler) now default a bare time to today's date before it
reaches `schedule` — the real fix, not just a rejection, since "8am" alone
typed into Capture must keep working per the PRD. New test:
`scheduling_a_time_with_no_date_is_rejected`.

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

**Fixed (`1b72d08`, same craft-floor self-review, continued further): a
day column's empty state rendered no text at all** — `render_calendar_body`'s
placeholder was `.child("")`, a genuinely empty string, instead of the
"No events today" treatment the Today glance already established for the
same situation. Fixed to "No events". Also swept `settings.rs` against
the same checklist — clean.

**Fixed (`af12d82`, found via a self-review against the `impeccable`
craft-floor checklist): two banned patterns in the Calendar tab.**
`render_calendar_event_card` used a 2px colored left border — explicitly
banned ("A colored border-left or border-right above 1px on cards, list
items, callouts, or alerts") — replaced with the same filled-dot indicator
already used elsewhere in the tab. `calendar_nav_button` rendered "‹"/"›"
as literal text glyphs — banned ("Unicode glyphs or emoji standing in for
an icon system") — replaced with `icons/arrow-left.svg`/`arrow-right.svg`,
both already-embedded assets. Built without loading the checklist first,
same lesson this project already learned once before
(`flow-ui-craft-discipline` memory) — this pass ran it against my own work
rather than assuming it was clean.

**Real cleanup candidate, flagged rather than acted on unilaterally**
(found while auditing for dead code after tonight's EventKit work):
`src/browser.rs` (1640 lines, a generic WKWebView wrapper) has zero
callers anywhere in the codebase (`grep -rn "browser::" src/` — nothing;
`mod browser;` in `lib.rs` is the only reference). It was deliberately
kept during Milestone 0's strip specifically "for a future Calendar OAuth
flow" (`wayfinder/tickets/strip-waku-core-backend.md`'s own stated
reasoning, quoted almost verbatim in this doc's own Milestone 0 section
above) — that specific justification is now void, since Milestone 3 uses
EventKit, not OAuth, so there's no browser-based auth flow coming. Not
deleted here: this session already made the symmetric call to *keep*
`calendar_connections`/`calendar_events`'s DB schema shape "for a future
non-macOS or OAuth-based provider" (§8's annotation, `343480c`)  — the
same hypothetical future provider is exactly what `browser.rs` would
serve, just at 1640 lines of cost instead of a few schema lines. Deleting
1640 lines of previously-intentional code on a solo 3am judgment call,
when the two decisions pull toward opposite defaults depending on how
speculative "a future OAuth provider" is judged to be, is a bigger and
less obviously-correct action than this session's many "add a fix + a
test" changes — flagged for a deliberate call instead.

**Known, genuinely undisclosed gap, flagged rather than built blind**
(found via the same data-integrity audit sweep that found tonight's
subtask-cascade bug): §8's `task_audit` table ("v1 storage, no UI") has no
implementation anywhere in `src/db.rs` — no `CREATE TABLE`, no insert on
any mutation path. Unlike `calendar_connections`/the mutation-ID bullet
(both correctly Milestone-2-deferred and now annotated as such in the
PRD), nothing marks this one as deferred, so it reads as a v1 gap, not a
later-milestone item. Not built tonight — it's a real feature (a write
hook on every mutation path, with real open questions about capture
granularity for bulk operations) rather than an obvious bug fix, and
building it hastily overnight risked an incomplete hook set worse than no
table at all. See `wayfinder/tickets/task-audit-log.md` for the open
questions.

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
      **Every task row in every view is now keyboard-reachable** — the
      original pass's two disclosed gaps are both closed: Upcoming
      (**`df0c882`**, `render_upcoming_section` fetching a real per-row
      `FocusHandle` via `entity.update`/`Flow::row_focus`, the same pruned
      map the virtualized flat views already share) and the Completed
      section (**`76ed773`**, `completed_section` doing the same via a
      *separate* `completed_row_focuses` map — a completed task's id never
      appears in the active view's list, so sharing `row_focuses` would
      have its own pruning delete the handle on the next unrelated
      refetch, same reasoning as `subtask_focuses` being its own map).
      Both were cheap: neither view is virtualized, so every row already
      rendered eagerly each frame regardless — a `HashMap` lookup per row
      is not new per-frame I/O.
      **Genuinely out of scope, not silently dropped:**
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
