---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-4]
status: open
assignee: unassigned
---

# Migrate Flow's UI from GPUI to Tauri

## Context

2026-08-20, explicit user decision, arrived at over several turns in one
session — recorded here in full since it reverses this project's own
"native Rust/GPUI desktop shell" framing (`PRODUCT.md`, `AGENTS.md`,
`docs/HANDOFF.md`'s "What Flow is").

**How this came about:** the user felt Flow "feels very prototype-y...
not really animated and satisfying" and asked whether GPUI was the wrong
platform. A real side-by-side was built to test that honestly —
`/Users/avi/Developer/vibe/flow-tauri-prototype` (Tauri v2 + React 19 +
TypeScript + Vite + Framer Motion), a small comparable task-list screen
with real spring/shared-layout motion, separate repo, nothing shared with
Flow. The user's reaction ("looks amazing and feels amazing") was
immediate and strong. The counter-argument given at the time — that the
gap was Flow's own thin motion vocabulary, not a GPUI ceiling, and that a
few real GPUI primitives (`Svg::with_transformation`, a hand-rolled
overshoot easing curve inside `with_animation` animator closures) could
close most of it without a rewrite — was heard, and two rounds of that
work did ship (`4bf2137`, `69a5272`: a real spring-pop checkbox
completion, real press feedback via `.active()`). It didn't change the
decision. The user restated the intent to migrate multiple times across
the conversation, including after the counter-argument and after the
concrete GPUI improvements shipped — this reads as a settled decision,
not a first reaction, and this ticket proceeds on that basis rather than
re-litigating it further.

**Explicitly asked and answered in the same conversation, don't
re-derive:**
- Multi-session, not one sweep — the user said so directly.
- In-place vs. parallel-build was discussed (embedding via `wry`/
  `src/browser.rs`'s dormant WKWebView wrapper vs. a real standalone
  Tauri app built in parallel and cut over at the end). The user's final
  answer was **"i wanna redesign in tauri"** — read as: build the real
  thing in the Tauri prototype as a proper parallel app, not as an
  in-place per-screen embed inside the existing GPUI window. If a future
  session finds reason to revisit that (e.g. wanting a working app
  sooner, mid-migration, rather than only at cutover), that's a real
  option still on the table — `src/browser.rs`'s WKWebView wrapper is
  still there, still unused, still viable for it.
- "Redesign," not "port" — the user's own word. This is licensed to be a
  real visual-design pass on top of the migration, not a mechanical
  1:1 recreation of GPUI's current screens. `docs/DESIGN_DIRECTION.md`'s
  actual tokens/spacing/component anatomy are a starting reference, not a
  spec to reproduce exactly.

## What exists so far

`/Users/avi/Developer/vibe/flow-tauri-prototype` — Tauri v2, React 19,
TypeScript, Vite, Framer Motion, now with its own git history
(`7d69076`, `4899219`). Past the original single-screen comparison demo:
a real app shell now exists —

- `Sidebar` (wordmark, Capture row, Tasks/Calendar mode switch with a
  `layoutId`-driven sliding highlight, Inbox/Today/Upcoming/Anytime/
  Someday nav with the same shared-layout active-row technique, Settings
  pinned to the bottom, an Inbox badge) — mirrors `src/app/sidebar.rs`'s
  actual structure, not a generic nav.
- `TaskRow` — the row↔card `layoutId` morph, checkbox press/complete
  spring feedback, subtasks, pills. The actual answer to "why did the
  prototype feel better than GPUI": a real shared-layout transform GPUI
  has no equivalent primitive for.
- `TaskList` (Inbox/Today/Anytime/Someday) and `UpcomingList` (real
  date-grouping across every bucket, matching PRD §6.3's actual
  semantics rather than filtering one bucket).
- `Settings`, a first scaffold — calendar connection row, the read-only
  disclosure copy lifted from PRD §6.5's own language.
- `theme.css` — tokens ported from `src/theme.rs`'s `dark()` as a
  starting point for the redesign, not a spec to match exactly.

Still **entirely mock data** (`src/lib/mockData.ts`) — no Tauri command
reaches real Rust logic yet, and Calendar mode has no view at all yet
(shows a bare placeholder). Both are open work, not decided against.

## Open engineering questions for whoever picks this up next

- **How does the Tauri frontend reach Flow's real data and logic?**
  **Underway, not finished.** `db.rs` turned out to have zero `gpui`
  dependency at all (verified by grep before moving anything, not
  assumed), so it moved cleanly (`9a5fb5f`) into a new workspace crate,
  `crates/flow-data` — the GPUI binary's `src/lib.rs` now just
  re-exports it (`use flow_data::db;`), so every existing
  `crate::db::…` call site in the app kept working unchanged, and all
  186 tests plus a real watcher rebuild confirmed nothing broke.
  `eventkit.rs`, `platform.rs`'s calendar-specific functions, and
  `parse.rs` are equally `gpui`-free (same grep check run against all
  three) and equally movable the same way — not moved yet.

  **Update, same day: the wiring itself is done for the task side.**
  `flow-tauri-prototype/src-tauri/src/lib.rs` depends on `flow-data`
  directly (a cross-repo path dependency, deliberate — see that file's
  own doc comment) and exposes `list_view`/`list_completed`/
  `create_task`/`set_completed`/`set_note`/`delete_task` as real Tauri
  commands, each running the underlying blocking `Db` call via
  `tokio::task::spawn_blocking`. The frontend (`src/lib/api.ts`,
  `App.tsx`) fetches real data on load and after every mutation — no
  mock state left anywhere in the app. Verified past "it compiles": the
  dev database file was confirmed created on disk after a real run.
  Points at its own dev database file, not Flow's real `flow.db` — see
  the "coexistence" question below, still open. Calendar
  (`eventkit.rs`) and NLP capture (`parse.rs`) are not wired — Capture
  in the prototype only creates a bare-title task, no date/time
  parsing yet. Subtasks (`list_subtasks`) also not wired — `Task` has
  no embedded subtasks field, so the card's subtasks section was
  removed rather than faked with stale data.
- **What happens to the GPUI app during the migration?** Kept running
  and released as-is until the Tauri app reaches parity, per "multi-
  session, not one sweep" — no plan yet for how parity is judged screen
  by screen, or whether there's an intentional overlap period where both
  exist.
- **Where does this ticket's own scope end and
  `sidebar-and-calendar-write-back.md`'s begin?** That ticket's sidebar
  redesign (collapsible rail, embedded Inbox/Upcoming sections) and
  calendar drag-to-schedule request were captured on GPUI assumptions
  earlier the same session — worth a fresh look through a "we're
  redesigning in Tauri now" lens rather than assumed to carry over
  unchanged. The calendar-write-back principle question (§6.5's
  read-only stance) still needs its own explicit confirmation regardless
  of platform.
- **Release/distribution.** Flow's current shipping story is a signed,
  notarized `.app` with a Sparkle updater (`scripts/bundle.sh`,
  `scripts/release.ts`, `docs/HANDOFF.md`'s CI/release-tooling fixes).
  Tauri has its own bundler and updater plugin; not yet decided whether
  to adopt Tauri's or keep Sparkle wired to a Tauri-built binary.

## Post-archive progress (this ticket's own gap-closing work is done; kept
open as the running log for continued Tauri-only development)

- **Second real AI block: Checklist expansion** (`b0fe349` in the
  prototype): same `useAiConfig`/`useAiFeatureState` pattern Today
  briefing established. Manual mode shows suggestions as a dismissible
  preview requiring an explicit "Add all"; Auto mode writes them
  immediately with no preview the first time a task's card is expanded
  with no subtasks yet — a real, deliberate behavior difference from
  Today briefing (this is the one block where "auto" actually creates
  data, per the user's own explicit spec for what "fully auto" means).
- **Data-integrity verification pass** (behavior actually checked, not
  just "it builds"): quit the app, queried the dev database directly
  for orphaned subtasks (parent deleted/missing) and completed parents
  with open children — zero of either across 33 real tasks (19
  completed, 27 soft-deleted) from actual testing, confirming the
  complete-with-open-subtasks confirm and delete-cascade both hold up
  against real accumulated data, not just the unit tests.

## GPUI archived (2026-08-20)

Explicit user decision: "Cool if every GPUI thing is done then archive the
GPUI code we don't really need that anymore and then continue building on
this new code" — the direct confirmation this ticket had been waiting for
since the "Explicitly deferred" entry below was written. By this point the
gap that deferral was based on had closed: task CRUD, Calendar (all four
views, real navigation, per-calendar show/hide), a Things-3-style checklist,
bulk actions, undo, the Completed section, and PRD-mandated behaviors like
the complete-parent-with-open-subtasks confirm are all live in the Tauri app.

Scope and mechanics were both confirmed directly rather than assumed (this
repo turned out to be a monorepo — `apps/web`, `packages/flow-client`, `db/`,
`website/` alongside the GPUI app — worth checking rather than guessing
which parts "GPUI code" meant): just `src/`, `resources/`, and the root
Cargo package, not the rest of the repo. `crates/flow-data`/`crates/flow-
core` are untouched — `flow-tauri-prototype`'s own `Cargo.toml` has a path
dependency straight into `../../flow/crates/flow-data`, and moving it would
break that build (verified with a real `cargo check` from the Tauri repo
after the change, not assumed safe). Mechanically: tagged the pre-archive
commit (`gpui-app-archived-2026-08-20`, pushed to origin — a permanent full
recovery point, `git checkout` restores the exact GPUI app state), then
removed `src/`/`resources/` from the working tree rather than git-mv-ing
into an in-repo archive folder. Root `Cargo.toml` rewritten from a combined
workspace+root-package manifest into a pure virtual workspace; `cargo check
--workspace` and `cargo test --workspace` both verified clean (46/46) with
the GPUI package gone. `AGENTS.md`/`CLAUDE.md`, `README.md`, `PRODUCT.md`
rewritten to point at the archive and at `flow-tauri-prototype` as where
development actually happens now.

This ticket stays in `flow-tauri-prototype`'s own commit history references
below, but its home going forward is arguably that repo, not this one — this
repo no longer builds or ships the app it describes.

## Progress log (most recent first)

- **Enforced PRD §6.2/§11: can't complete a parent with open subtasks**
  (`9c10168` in the prototype): real gap found by re-checking
  `tasks.rs`'s own `request_complete`/`confirm_complete_with_subtasks`
  against Tauri — nothing here checked for open subtasks before
  completing a task at all, silently letting a parent get marked
  complete while its subtasks stayed open, a documented acceptance
  criterion (§11) that had simply gone unenforced. `requestComplete`
  now checks `subtaskCounts[id].open` (already loaded, no extra
  fetch — an advantage over the GPUI app's own compact row, which has
  to background-fetch subtasks per click since it never loads them at
  all) and shows the same "Complete parent and all subtasks?" / Cancel
  / "Complete all" banner and exact copy GPUI's own confirm has.
  Confirms every open subtask, then completes the parent through the
  normal animated+undo path; zero open subtasks completes immediately,
  unchanged. Full sweep of `flow-data`'s own public API
  (`db.rs`/`calendar.rs`) against the Tauri command list also done —
  every method is already wired, no other backend gaps.

- **First real AI feature: Today briefing, model picker, per-feature
  toggle** (`67d7d23` in the prototype): a genuinely new feature, not
  a GPUI parity item — scoped by direct interview rather than guessed
  (which feature first: Today briefing; trigger model: a per-feature
  three-state Off/Manual/Auto control, the user's own spec, not a
  single global switch; backend: whichever of Claude/custom OpenAI-
  compatible is configured; ship the model picker before any block
  feature, since every block needs it). New `src-tauri/src/ai.rs`
  (`ai_list_models`/`ai_chat_completion` via reqwest, done in Rust
  rather than a frontend `fetch()` — most third-party OpenAI-
  compatible providers don't set permissive CORS for an arbitrary app
  origin, and Tauri's webview still enforces CORS even with
  `security.csp: null`). Settings' AI section rebuilt on a shared
  `lib/aiConfig.ts`, gained a real "Test connection" → model dropdown,
  and a Features list grouped by app area ("shiny blocks," direct user
  framing) — Today briefing (live, in the Calendar tab specifically
  per direct placement) plus the rest of the earlier brainstorm kept
  visible as an explicit backlog rather than dropped, each already
  wired to its own three-state toggle even though only one is live
  yet. Claude OAuth is still the same honest "not wired up yet"
  placeholder from before — no registered OAuth client exists for
  this app, so only the custom OpenAI-compatible path is actually
  callable right now.

- **Fixed notes not persisting (visibly) after clicking out, and a
  related stale-badge bug found while fixing it** (`50735d5`,
  `6414cdd` in the prototype): direct user report, verified at the
  data layer before trusting the diagnosis — quit the app, queried the
  dev SQLite file directly with `sqlite3`, confirmed notes were
  actually being written correctly the whole time. The real bug:
  `changeNote` never called `refresh()` afterward, so the note view
  (reading `task.note` straight off `viewTasks` state) kept showing
  the stale pre-edit value until an unrelated refresh happened to
  fire — indistinguishable from data loss from the user's side of the
  screen. Swept every other mutation in App.tsx for the same missing-
  refresh shape and found one more: `addSubtask`/`toggleSubtask`/
  `deleteSubtask` only ever refreshed the expanded card's own subtask
  list, never the new subtask-count badge's own state, so it would
  show a stale "N/M" until an unrelated refresh fired. Fixed both by
  actually calling the write, then independently confirming the
  read-back reflects it — not by reading the code and assuming a
  `.then(refresh)` was there.

- **Row indicator icons for notes and subtasks** (`a72fea2` in `flow`,
  `b98d002` in the prototype): direct user request, matching Things
  3's own row indicators — a collapsed row now shows a note icon when
  the task has one, and a checklist icon + "N/M" open/total count when
  it has subtasks. A real new feature, not a parity gap: grepped both
  `tasks.rs` and `db.rs` first and confirmed neither the GPUI app nor
  flow-data's own `Task` struct carries a subtask count anywhere.
  Added `flow-data::db::SubtaskCount`/`Db::subtask_counts()` — one
  GROUP BY query for every parent's counts, not a widened `Task`
  struct (would touch every query that returns one, GPUI's own
  included, for a UI-only affordance its rows don't show) or a
  per-row fetch. Purely additive to the shared crate; verified GPUI
  still compiles clean and flow-data's own suite still passes
  (46/46) after the change.

- **Checklist divider direction fixed** (`515bec1` in the prototype):
  direct user correction with a real Things 3 screenshot — "the lines
  are horizontal and are borders of the subtasks checklists." The
  entry below misread "lines between the subtasks" as a vertical
  connector threading through the checkboxes; replaced with a thin
  `border-bottom` under each row instead (none under the last one),
  matching the screenshot.

- **Checklist v2, schedule picker fixed, NLP-aware rename, unfocus
  bug fixed** (`de0870e` in the prototype): direct user feedback round
  on last turn's own checklist redesign — checkbox icons switched to
  Square/SquareCheckBig (a real checkbox glyph, not a circle), more
  row spacing, a single continuous connector line behind every
  checkbox (Things 3's own look), and the "Checklist" pill now only
  exists while the list is empty — growing it past the first item is
  Enter-only chaining (commit a row, a fresh draft opens right after
  it), Backspace on an empty row deletes it. Also fixed a real bug,
  not a hypothetical, in the schedule picker the user reported as
  "horrible": its quick-pick/clear buttons sat next to an autoFocus'd
  input whose onBlur closed the whole picker, so clicking any button
  blurred the input and unmounted the picker before the click ever
  fired — most of its buttons silently did nothing. Fixed with
  onMouseDown preventDefault, plus a real anchor position and NLP
  highlight/preview on its own free-text field. Renaming a task now
  gets the same live NLP highlight/preview Capture has (new
  lib/nlpPreview.ts, shared with CaptureField) and reschedules on
  commit if a date/time was recognized. Fixed main-pane's own blind
  `stopPropagation()`, which had been swallowing every click in the
  main content area before it could reach the root's own "click
  elsewhere collapses the expanded task" handler.

- **Completed section landed** (`64c6e8d` in the prototype): real gap
  found by re-checking `tasks.rs`'s own `completed_section` —
  `list_completed` was wired into `api.ts` back when Capture/CRUD
  first landed but never actually called; completed tasks simply
  vanished with no way to see them again. A "Completed (N)" disclosure
  row now sits at the bottom of every task view (all five, Upcoming
  included), capped-height scroll when open, "Clear" bulk-deletes with
  no undo (matching `clear_completed`'s own asymmetry), unchecking a
  completed row writes immediately with no animation/toast (matching
  `toggle_completed`'s early-return). Also fixed completing a task
  itself while wiring this: PRD §6.1's undo window covers completion,
  not just delete — `complete` now shows an undo toast, which had been
  silently missing since completion first landed. Deliberate scope
  reduction noted in the commit: a completed row is a simple line, not
  the full expand-into-detail-card GPUI's own version has.

- **Task/subtask rename, Things 3-style checklist, friendly dates,
  icon border fix** (`77628b2`, `db828f5` in the prototype): direct
  user reports on all four. (1) Neither a task nor a subtask could be
  renamed at all — new `set_title` command (the `flow_data::db`
  method already existed, just unwired), click-to-edit on both, one
  shared `onRename(id, title)` callback since the command doesn't
  distinguish task from subtask. (2) Checklist redesigned to match
  Things 3: dropped the "SUBTASKS (x/y)" header and left-border rail
  (Things 3 shows neither), tighter row spacing. (3) Every raw
  "2026-08-22" string in the UI now renders through new
  `dayLabel`/`formatSchedule` helpers mirroring the GPUI app's own
  `day_label`/`format_schedule` exactly ("Today"/"Tomorrow"/weekday/
  short date). (4) The River Cut icon had its own rounded-square shape
  and black margin baked into the artwork on top of macOS's own system
  mask, visible as a nested double-square — fixed by detecting and
  cropping past the artwork's own baked-in border before regenerating
  the icon set, verified by sampling corner pixels, not eyeballing.

- **App icon + theme switcher, built from user-picked AI concepts**
  (`a640783`, `e5507db` in the prototype): a throwaway Artifact gallery
  of 10 fal.ai (`openai/gpt-image-2`) icon concepts was generated and
  shown for selection (not committed — pure scratch space); the user
  picked "River Cut." Regenerated the full macOS/Windows icon set from
  it via `tauri icon` (iOS/Android output deleted — not a bundle
  target here). Also built Flow's first real theme-switching
  mechanism (`theme.css` previously had exactly one hardcoded `:root`
  palette): a `[data-theme="river-cut"]` override block, applied via
  a `data-theme` attribute on `<html>` and persisted like every other
  UI preference. The theme's colors aren't hand-picked — sampled
  directly off the generated icon PNG (filtered for saturated,
  non-background pixels, clustered by hue/lightness), and the whole
  neutral scale reshifts to a warmer moss-gray, not just the accent.
  New `ThemeSwitcher` in Settings.

  Found and fixed a real incident along the way: `scripts/dev-app.ts`'s
  `stopApp` used a bare `pkill -x` on the Cargo binary name, which the
  release build shares with the debug build — it silently killed the
  just-installed `/Applications/Flow.app` on the very next dev
  rebuild. Fixed by matching the debug bundle's full executable path
  via `pkill -f` instead, verified with `pgrep -f` and by running both
  a debug and release instance side by side through a rebuild cycle.

- **First real Flow.app shipped to /Applications** (`d653747` in the
  prototype): explicit user request, not a ticket-driven step —
  `tauri.release.conf.json` overrides productName/identifier/window
  title to "Flow"/`com.avi.flow` via `tauri build`'s own `--config`
  merge (dev build stays "Flow Debug"/`com.avi.flow-tauri-prototype`,
  unchanged), built and installed to `/Applications/Flow.app`,
  quarantine cleared, launches and is Spotlight-findable. Explicit
  user decision on data: the release build keeps its own separate
  database rather than pointing at GPUI Flow's real `flow.db` —
  running two independent processes against the same SQLite file was
  the risk not worth taking. Nothing from GPUI carries over
  automatically; `com.avi.flow/flow.db` starts clean. This is the
  actual daily-use app now, distinct from the "Flow Debug" live-
  reload one this whole ticket's dev loop targets — rebuilding it
  after further changes is a manual `tauri build --config
  src-tauri/tauri.release.conf.json` + reinstall, not automatic.

- **Fixed the Connect Calendar prompt never appearing** (`7c27b38`):
  direct user report. Root cause, two parts: (1) the bundle had
  neither `NSCalendarsUsageDescription` nor
  `NSCalendarsFullAccessUsageDescription` in Info.plist — macOS won't
  show the EventKit permission dialog at all without one, it just
  silently denies. Added `src-tauri/Info.plist` (tauri-build merges it
  automatically), same copy the GPUI app's own Info.plist uses. (2)
  the dev session had been running bare `bun run tauri dev`, which
  launches the raw unbundled binary with no `.app` wrapper for macOS
  to read an Info.plist from at all, even with the keys in place —
  switched to `scripts/dev-app.ts`'s own bundled watcher.

- **Links highlight and open** (`7f72d3e`): direct user report
  ("links should highlight"), not a GPUI gap — GPUI has no link
  detection at all. New `lib/linkify.tsx` renders http(s) URLs in task
  titles/notes as clickable links opening in the real browser via
  `tauri-plugin-opener`. The note field became a click-to-edit view/
  edit toggle to make this possible — a native `<textarea>` can't
  render part of its content as a link.

- **Removed the dead Move/Flag pills** (`fb4eb74`): neither
  corresponds to a real Flow feature — grepped `tasks.rs`/`db.rs` for
  any flag/priority/move concept and found none; the GPUI detail card
  only ever had Schedule/Subtask/Delete. Leftover from the original
  small Tauri demo, no onClick, no backend support. Removed rather
  than wired up, since there's nothing on the GPUI side to port.

- **Bare-space-to-open-Capture** (`5b4364e`): mirrors the GPUI app's
  own `handle_space_capture_action` — scoped to task views only, and
  skipped when any input/textarea/contenteditable already has focus
  (which also covers the schedule picker's own auto-focused input for
  free). Found via a sweep of `render.rs`'s wired `on_action` handlers
  against what's Waku-inherited dead code (`OpenSettings`/
  `ToggleSidebar` menu items have no handler at all in Flow, not a
  gap) versus real: only `NewTask` (already ported) and
  `SpaceCapture` turned out to be real, live Flow behavior.

- **Fixed Today silently excluding overdue tasks; added the calendar-
  glance card** (`fef5b8e`): a real bug found by re-checking
  `sidebar.rs`'s own Today description ("Overdue and today's active
  tasks") against the actual backend query — `list_view(View::Today)`
  already selects `scheduled_date <= today` and is correct on the Rust
  side; the frontend was re-deriving view membership client-side from
  one merged list using `=== today`, so overdue tasks silently landed
  in Upcoming instead. Fixed by keeping five separate per-view lists
  straight from `list_view` instead of merging and re-filtering.
  Also added the PRD §6.3 calendar-glance card ("A compact
  calendar-glance card precedes the tasks" in Today), hidden until
  EventKit is granted per §6.5 — mirrors the GPUI app's own
  `components::calendar_glance` exactly, including its sort order.

- **Fixed the Denied-calendar dead end** (`186b050`): real gap found
  by re-checking against GPUI's `settings.rs` — PRD §6.5 requires a
  way to reach System Settings → Privacy & Security → Calendars, since
  Flow can't re-request a denied permission itself. The Denied state
  previously just showed a disabled Connect button with no way
  forward. Added `lib/system.ts::openCalendarPrivacyPane()` (same
  `x-apple.systempreferences` deep link the GPUI app uses) via the
  already-installed `tauri-plugin-opener`, wired into both Settings
  and Calendar's own not-connected state.

- **Calendar sidebar: per-calendar show/hide** (`bece23f`): a real gap
  found by re-checking the migration ticket against GPUI's actual
  `calendar.rs`, not from a user report — the GPUI app has a whole
  sidebar (grouped by account, a toggle row per calendar) that the
  prototype never had; `calendarList()` was wired into `api.ts` back
  when Calendar first landed but never actually called from the UI.
  Same on/off treatment: filled dot = shown, hollow = hidden (a shape
  change, not just dimmer color). Hidden calendars' events are
  filtered out of all four views via one `visibleEvents` derivation.

- **A real time-grid Week view** (`9c283d1`): mirrors the GPUI app's
  own `render_calendar_week_grid` — fixed hour gutter, one column per
  day, an all-day strip above the grid, timed events absolutely
  positioned by time-of-day/duration, same deliberate greedy-lane-
  sweep simplification for overlaps (not true interval-packing). Grid
  opens scrolled to 7 AM instead of midnight, same seed-once behavior
  as the GPUI app's `calendar_week_scrolled_once`. Event text color
  picks white/black off the event color's own HSL lightness. Day mode
  deliberately keeps the simpler agenda-per-day layout instead — same
  choice the GPUI app itself made when Week moved to a grid. Calendar
  now has full parity: all four modes, real navigation, and the same
  Day-stays-Kanban/Week-is-a-grid split as the GPUI app.

- **Calendar Year view** (`f4b0956`): mirrors the GPUI app's own
  `render_calendar_year_grid` — a 4-column grid of 12 mini months,
  each day cell just a number with a dot marking "has an event" (not
  the events themselves, unreadable at this size), today highlighted.
  Clicking a month jumps to Month mode for it. All four of the GPUI
  app's calendar modes now exist in the prototype; Day/Week still use
  the agenda-per-day Kanban layout rather than a true time grid, which
  remains the one real visual gap left in Calendar.

- **Calendar Day/Month views + navigation** (`e6099c5`): a Day/Week/
  Month mode toggle and Prev/Next/Today navigation, matching the GPUI
  app's own header/toggle row. Month renders a real grid (Monday-
  start, spilling into neighboring months to fill whole weeks, same
  shape as the GPUI app's own month-grid range math) with up to 3
  events per cell and a "+N more" overflow; clicking a day jumps to
  Day view for it. Event range now follows the active mode/cursor
  instead of always being "this week from now". Year view and the
  true time-grid Week (Day/Week both still use the agenda-per-day
  Kanban layout) remain open.

- **Bulk actions landed** (`f0693ce`): Cmd+click a compact row to
  toggle it into a multi-select set instead of opening it (same
  `toggle_selected` interaction as the GPUI app), a floating bulk-
  action bar with Today/Anytime/Someday/Delete once anything's
  selected, bulk delete gets the same combined Undo-toast affordance a
  single row's delete has. No new Tauri commands — bulk actions just
  run the existing single-task `schedule_task`/`delete_task` commands
  over the selected set from the frontend.

- **Scheduling picker wired to the "Schedule…" pill** (`30de752`):
  new `SchedulePicker` popover with Today/Anytime/Someday/Clear
  quick-picks plus a free-text field that reuses the real NLP parser
  via new `schedule_task`/`schedule_task_from_text` Tauri commands.
  Found and fixed a real bug while building it: Today/Upcoming were
  comparing `task.scheduled_date` against the literal string `"today"`
  — a leftover from early mock data — so neither view ever matched
  anything once real backend data (actual ISO date strings) was wired
  in; both views had been silently empty since that point. Fixed with
  a new `lib/date.ts::todayIso()` that builds the date from local
  `Date` components rather than `toISOString()` (UTC, drifts a day off
  depending on the user's offset).

- **Leading date/time phrases now recognized, not just trailing**
  (`375c15c` in `flow`), from a direct user report that "in 8 days
  take out trash" didn't parse. PRD §6.4 only specified trailing-
  phrase recognition; this is a genuine extension, not a PRD override
  — trailing patterns still try first with unchanged behavior, leading
  patterns are a fallback. Leading patterns require mandatory
  whitespace after the match (not just a word boundary) so "Friday's
  report is due" doesn't false-positive on "Friday" (a word boundary
  sits right before the apostrophe too) — covered by a dedicated
  regression test. 6 new tests (45 in `flow-data`, 192 workspace-wide).

- **`parse.rs` extracted to `flow-data`, real NLP capture wired**
  (`1d13944` in `flow`, `f6565d4` in the prototype). Same verified-move
  pattern as `db.rs`: confirmed `gpui`-free by grep before touching
  anything, moved with `git mv`, `src/lib.rs` re-exports it so nothing
  else in the GPUI app changed, all 186 tests still pass post-move. The
  prototype's Capture field now calls a real `capture_task` command
  that parses date/time and schedules the task, matching the GPUI
  app's `submit_capture` behavior exactly (including its deliberate
  PRD-14 override and the time-without-date-defaults-to-today rule) —
  not the bare-title `create_task` it used before.

- **Subtasks wired** (`e345107` in the prototype): `list_subtasks`/
  `create_subtask` commands, a real Subtasks (done/total) section in
  the expanded card with per-subtask completion toggles and an inline
  add row. Fetched only for the currently-expanded task, same
  reasoning the GPUI app's own `subtask_context` uses.

- **Delete wired into the UI** (`3ebf40f`), then **real Undo landed
  right after** (`00426af`) and delete's own pill went back to a
  single click now that the actual safety net exists — the earlier
  click-to-confirm was an explicit stopgap for this exact gap, not a
  design worth keeping once Undo was real. `restore_task` command, a
  bottom-of-screen `UndoToast`, 10s window matching PRD §6.1.

- **Cmd+N and live NLP feedback fixed** (`de90ea7`), from a direct user
  report while away from the keyboard. Cmd+N was never wired to
  anything. "NLP numbers didn't work, nothing highlights" turned out to
  be a missing-feedback bug, not a parser bug — verified with 6 new
  Rust tests (including a UTF-16-vs-byte-offset case) that
  `flow_data::parse::parse` was already correct. Capture now has a real
  live preview: highlights the recognized phrase in place and shows a
  formatted date/time line, via a new `preview_capture` command run
  debounced on every keystroke.
- **Flow Debug.app** (`062bd0b`): a real, Spotlight-discoverable,
  auto-rebuilding `.app` bundle (`scripts/dev-app.ts`, the Tauri
  analogue of Flow's own `scripts/dev.ts`), answering a direct request
  for "an app like Flow Dev called Flow Debug." Two real bugs found and
  fixed before trusting it — not shipped blind: `pkill` was matching
  the bundle's *display* name, which is never the actual running
  process name for a Tauri bundle, so it silently killed nothing and
  hung the watcher; and a genuine infinite rebuild loop from watching
  "src" and "src-tauri/src" as separate recursive watches, letting
  cargo's own build-output churn under "src-tauri/target/..." leak into
  the "src" watcher (confirmed by instrumenting the raw fs.watch events,
  not guessed) — fixed with one recursive watch over the repo root and
  hand-checked path boundaries instead of trusting the scoping of
  several independent watches not to collide.

- **Calendar/EventKit wiring landed** (`60de4db` in `flow`, `f9c3944` +
  `ac87a2b` in the prototype) — the other big remaining piece from the
  "Not started" list below, now mostly done. `eventkit.rs` and the
  `calendar_*` types/functions moved out of `platform.rs` into
  `flow-data::calendar`, same verified pattern as `db`/`parse`. Real
  Tauri commands (`calendar_auth_status`/`calendar_connect`/
  `calendar_events`/`calendar_list`) call straight into it — the actual
  EventKit permission system and the user's real macOS Calendar data,
  not a stub. One real bug found before shipping: the permission-
  request future bridges an Objective-C completion block and can never
  be `Send`, which Tauri's async commands require — fixed by driving it
  to completion inside `spawn_blocking`'s own dedicated thread, the same
  "give the non-Send thing its own thread" shape `flow_data::db` already
  uses. The frontend's `Calendar` view gates on real auth status, shows
  a Connect button when not granted, and renders a 7-day agenda-per-day
  view of real events once granted — deliberately the Kanban-style
  layout the user liked and kept for Day mode in the GPUI app (`5be0aef`
  there), not the full Day/Week/Month/Year grid system that app has;
  Month/Year views and the true Week time-grid aren't ported yet.

- **Keyboard accessibility pass** (`87346cc`): task rows (tabIndex,
  Enter/Space matching the GPUI app's exact convention — no separate tab
  stop for the checkbox), card headers, and card pills (converted from
  styled `<div>`s to real `<button>`s) are all keyboard-reachable now,
  with a consistent `:focus-visible` ring. Sidebar's own controls were
  already real `<button>`s, just needed the same visible ring. Found and
  fixed a real bug along the way: the subtask-add input's Escape handler
  didn't stop propagation, so it also collapsed the whole card via the
  app root's own Escape handler.

**Partial answer to "a proper agent debug/inspection feature"**: not a
dedicated feature yet, but `scripts/dev-app.ts`'s own build/runtime log
(piped to a file every session) already gives a real, usable way to
verify a change actually built and launched without needing visual
access — used throughout this session's own verification. A true
in-app inspector (mirroring the GPUI app's own Cmd+Option+I panel /
`debug_snapshot`) is still open.

## Current state (rewritten 2026-08-20, superseding the stale version of
this section below the fold in earlier history)

The Tauri app's task-manager loop and Calendar are at genuine functional
parity with GPUI's own: task CRUD (Capture with real NLP parsing,
list/complete/note/delete, rename with the same live NLP highlight/
reschedule-on-commit Capture has), a Things-3-style checklist (rename,
Enter-to-chain-add, backspace-to-delete), scheduling from the UI picker,
bulk actions, a Completed section per view with undo-on-complete, and
Calendar (connect flow, per-calendar show/hide, all four Day/Week/Month/
Year views with real navigation, Week as a true time-grid). Plus real
additions GPUI doesn't have: link highlighting, a theme switcher, AI
settings scaffolding (inert, off by default).

Reached by repeatedly sweeping GPUI source files
(`render.rs`/`sidebar.rs`/`tasks.rs`/`calendar.rs`/`settings.rs`) against
what's actually wired in the Tauri app rather than trusting this
ticket's own summary — every pass so far has turned up at least one
real, previously-silent gap (the calendar sidebar, the Denied-calendar
dead end, Today excluding overdue tasks, the calendar-glance card,
bare-space-to-capture, the Completed section, undo-on-complete). Worth
repeating the same way over any file not yet swept before calling a
section fully done — a spot-check of `input.rs`'s `ComposerInput`
keybindings against Tauri turned up nothing (they're generic text-field
editing GPUI had to hand-build that native `<input>`/`<textarea>`
elements already give Tauri for free — not a gap).

## Not started

Arrow-key navigation between rows specifically (Tab order is the only
way to move focus between tasks right now) — not a real parity gap,
GPUI's own tasks.rs explicitly says arrow-key row navigation was
"deliberately not attempted" there either. Release/distribution tooling
is blocked on credentials this agent doesn't have (signing identity,
notarization) — not a code gap; revisit once a real ship decision is
imminent.

**Explicitly deferred, asked for directly and not done (2026-08-20):**
the user asked to delete the GPUI code now that the Tauri app is wired
to real data — "this is the main one now." Not done, and flagged rather
than silently actioned: at the time this was asked, Tauri had task
create/complete/note/delete only, a real functional gap against GPUI.
That gap has substantially closed since (see "Current state" above),
but the decision to actually delete GPUI's source and cut over is still
the user's to make explicitly, not something to infer from "the gap
list got shorter." Re-raise it as a direct question once the user
signals they're ready to make Tauri the sole shipping app, not before.
