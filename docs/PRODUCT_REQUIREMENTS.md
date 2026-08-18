# Flow — Product Requirements Document

**Status:** foundation specification  
**Audience:** product, design, and implementation agents  
**Product:** Flow, a private desktop task manager with a calendar glance  
**Reference build:** Flow at `e8b843a9b72dea99ec04900496cd4a29c53c3298`

## 1. Product summary

Flow is a calm, keyboard-first personal task manager inspired by Things 3’s
mental model, not a clone of its visual or interaction design. It lets one
person capture tasks quickly, decide when they matter, break work into
subtasks, and see enough of their calendar to make sensible choices.

The core promise is: **write the task naturally; Flow turns time language into
an explicit, reviewable plan.**

Example: entering `take out laundry 8 am tomorrow` produces a task titled
“Take out laundry” scheduled for tomorrow at 08:00 in the user’s timezone.
Entering `bring Mya cake in 3 days` produces “Bring Mya cake” scheduled for the
local date three days from now. The parsed phrase remains visible until save,
then the date is represented by task metadata, not buried in the title.

Flow is not a calendar app, project-management suite, habit tracker, or team
workspace. Calendar events are read-only context. Tasks are the source of
truth for tasks.

## 2. Goals and non-goals

### Goals

- Make capture nearly frictionless from anywhere in the app.
- Give every open task one clear home: Inbox, Today, Upcoming, Anytime, or
  Someday.
- Support nested work without turning a task list into a project-management
  system.
- Make dates and times trustworthy, visible, timezone-correct, and reversible.
- Surface a compact, privacy-respecting calendar glance alongside tasks.
- Feel native and deliberate: dark desktop shell, low visual noise, responsive
  feedback, reduced-motion support, and complete keyboard operation.
- Start with a backend whose functions and schema are easy for AI agents to
  change, and which can move onto the user’s k3s server without a rewrite.

### Non-goals for v1

- Multi-user shared lists, assignments, comments, or real-time collaboration.
- Creating, editing, or RSVP-ing calendar events.
- Recurring tasks, reminders, attachments, tags, search, areas, projects, and
  calendar write-back.
- A web client, mobile client, offline-first multi-device conflict resolution,
  or importing from Things/Reminders.
- AI completion of tasks. Natural-language parsing is deterministic product
  behavior, not an LLM feature.

## 3. Product principles

1. **Capture first; classify later.** New tasks land in Inbox unless the input
   says otherwise.
2. **A task is small.** A task may have a short note and one level of
   subtasks. Bigger planning belongs in a later project feature.
3. **The system must not guess silently.** Ambiguous time language gets a
   preview or a clarification, never an invisible interpretation.
4. **Calendar is context, not command.** Events inform the task view and
   never create or reschedule tasks automatically.
5. **Animation explains state, never delays work.** Every operation is usable
   during animation and respects reduced-motion preferences.
6. **Boring infrastructure beats clever infrastructure.** One product
   backend, one database, explicit migrations, encrypted provider tokens.

## 4. Users and jobs

The initial user is the owner of the self-hosted installation: a desktop-heavy
individual who has a calendar and wants a task manager that is faster than a
full project tool.

| Job | Desired outcome |
| --- | --- |
| Capture a thought | It is safely in Inbox in under two seconds. |
| Plan today | I see today’s work with today’s calendar obligations. |
| Plan the next week | I see dated tasks in order without a dense calendar grid. |
| Defer work | I can move a task to Anytime or Someday without losing it. |
| Execute a multi-step task | I can complete and reveal subtasks in place. |
| Understand a parsed date | I can see exactly what Flow understood and undo it. |

## 5. Information architecture

The desktop has a fixed navigation rail and a single main pane. The navigation
matches the reference screens’ density and hierarchy, while Flow uses its own
icons, typography, spacing, and colors.

```text
┌──────────────────────┬─────────────────────────────────────────────┐
│ Flow                 │  [view icon]  View title              ⌘K   │
│ + New task           │                                             │
│                      │  Calendar glance (where relevant)          │
│ Inbox            3   │  ─────────────────────────────────────────  │
│ Today                │  Task rows / date sections                  │
│ Upcoming             │                                             │
│ Anytime              │                                             │
│ Someday              │                                             │
│                      │                                             │
│ Calendar             │                                             │
│ Settings             │                                             │
└──────────────────────┴─────────────────────────────────────────────┘
```

The navigation badge counts only uncompleted Inbox tasks. Completed items are
hidden from primary views by default but remain available in a later Logbook
view; v1 retains completion history in storage for that future view.

### Canonical placement rules

These are mutually exclusive for an incomplete task:

| Place | Persisted rule | What appears there |
| --- | --- | --- |
| Inbox | `bucket = inbox` | Newly captured or explicitly sent tasks, regardless of date. |
| Today | `bucket = active` and `scheduled_date <= local_today` | Active tasks scheduled today or overdue. |
| Upcoming | `bucket = active` and `scheduled_date > local_today` | Active dated tasks grouped by local day. |
| Anytime | `bucket = active` and no schedule | Available work without a chosen date. |
| Someday | `bucket = someday` | Intentionally deferred work, with or without a future date hidden until activated. |

Inbox is a deliberate state rather than an accidental absence of data. Moving
an Inbox item to Today/Upcoming/Anytime changes its bucket to `active`.
Scheduling an Inbox task does **not** silently remove it from Inbox; the user
must explicitly “Move to active” or use the parsed-date prompt’s “Schedule and
activate” action. This avoids losing unprocessed capture.

## 6. Functional requirements

### 6.1 Capture and editing

- `⌘N` opens an always-available task composer focused in the current view.
- Pressing Enter saves a valid task. `Shift+Enter` inserts a line break in the
  note field. Escape dismisses an empty composer or abandons unsaved changes
  after confirmation.
- A task has a required nonempty title, optional plain-text note, placement,
  optional schedule date, optional time, and optional parent task.
- Inline editing is available from the task row. Expanding a row opens the
  detail editor without navigation away from the list.
- Changes save optimistically, show a non-blocking error with Retry on failure,
  and never discard typed content.
- A task can be moved among the five placements, rescheduled, completed,
  reopened, or deleted. Deletion shows an undo toast for 10 seconds; storage
  uses a soft-delete timestamp until a future permanent-delete policy exists.

### 6.2 Tasks and subtasks

- A task may have zero or more direct subtasks; a subtask cannot have children
  in v1. This one-level ceiling is intentional and must be enforced in both UI
  and backend validation.
- Subtasks are ordered manually and shown indented beneath an expanded parent.
- A parent’s progress is `completed child count / total child count`; it is a
  visual ring, not a second workflow state.
- Completing a parent with incomplete children asks: “Complete parent and all
  subtasks” or “Cancel.” It never leaves a completed parent with open children.
- Completing the final subtask does not auto-complete its parent; the parent
  may represent final review. The row presents “All subtasks done” instead.
- A child inherits no schedule automatically. The user can assign one when it
  is a real commitment.

### 6.3 Views

**Inbox** lists open Inbox tasks in capture order, then optional dates as small
metadata. It has an inline “Process” action that offers Today, Anytime, Someday,
and schedule.

**Today** shows overdue tasks first, then today’s active tasks. A compact
calendar-glance card precedes the tasks when a calendar is connected. The card
shows all-day items and timed events for the local day, sorted by start time.

**Upcoming** groups active tasks by local date from tomorrow onward. Each date
section includes that day’s calendar events. Empty days with events still show;
empty days with neither tasks nor events do not.

**Anytime** lists active undated tasks, ordered by manual position then creation
time. It is the default place for non-urgent active work.

**Someday** lists deferred tasks. It is intentionally visually quieter and does
not show its scheduled dates until a task is activated, preventing Someday from
becoming a disguised upcoming queue.

### 6.4 Natural-language date and time parsing

Parsing runs locally on input pause (150 ms) and on save. It must work without
a network connection and must not call an LLM or transmit task titles.

Supported v1 forms:

| Input suffix | Meaning | Stored result |
| --- | --- | --- |
| `today`, `tomorrow` | Current local calendar day or next local day | `scheduled_date` |
| `in N days` | N local calendar days after today, N = 1–365 | `scheduled_date` |
| `Monday`, `next Monday` | Next matching weekday; “next” always means the following week if today matches | `scheduled_date` |
| `Aug 23`, `23 Aug`, `2026-08-23` | Explicit local calendar date | `scheduled_date` |
| `8am`, `8 am`, `08:30`, `8:30 pm` | Local wall-clock time | `scheduled_time` |
| combinations | Date plus time in either order | both fields |

Parsing behavior:

- Only a recognized temporal phrase at the end of the title is removed. The
  source phrase is retained in `parse_source` until the task is saved.
- The composer renders a clickable preview: `Tomorrow · 8:00 AM`. Clicking it
  opens a date/time picker; Backspace restores the original text.
- “At 8” without am/pm, “next week”, dates without a year that have already
  passed, and impossible dates are ambiguous. Keep the title unchanged and
  show a concise clarification control rather than making a date up.
- The parser evaluates relative phrases in the user’s configured IANA timezone,
  captures that timezone with the task, and stores dates as `YYYY-MM-DD` plus
  optional `HH:mm` local time. It does not convert a task due at 08:00 into a
  different wall time when the user travels.
- Example acceptance cases: on 2026-08-18 in `Australia/Melbourne`, `take out
  laundry 8 am tomorrow` becomes title `take out laundry`, date `2026-08-19`,
  time `08:00`; `bring Mya cake in 3 days` becomes title `bring Mya cake`, date
  `2026-08-21`.

### 6.5 Calendar connection and glance

- v1 supports one Google Calendar connection using read-only OAuth scope.
- The connection screen clearly names the read-only access and permits
  disconnecting, which revokes locally stored credentials and deletes cached
  events.
- Sync only a rolling window: 30 days before today to 90 days after today;
  refresh after successful connection, on explicit refresh, on app focus after
  15 minutes, and at most once every five minutes in the background.
- Cache event ID, calendar ID, title, start/end, all-day status, and display
  color. Do not fetch attendees, descriptions, conferencing links, or location
  in v1.
- Calendar errors never block task CRUD. A failure replaces the card with a
  quiet “Calendar unavailable · Retry” state.
- The app must show the source calendar color and distinguish all-day and timed
  events. Tapping an event opens the provider’s event URL when available.

## 7. Interaction and motion requirements

The detailed visual system, component anatomy, token palette, and keyboard
model live in [DESIGN_DIRECTION.md](DESIGN_DIRECTION.md). This section records
the product requirements that implementation must preserve.

- The shell is a dark, desktop-first surface: 260–280 px sidebar, generous
  content margin, 16 px task-row rhythm, soft separators, and one clear accent
  color per destination. Do not copy Things 3 assets, layout measurements, or
  iconography.
- Completing a task checks the control immediately, fades and collapses the row
  over 180–220 ms, then exposes Undo. Reopening reverses it.
- Opening a task detail expands in place over 160 ms; subtask indentation and
  progress ring update without layout jumps.
- Navigation cross-fades or slides only the main pane (120–160 ms); the sidebar
  stays still so orientation is retained.
- Respect OS reduced motion: replace movement with an opacity transition of at
  most 100 ms. No essential information may exist only in animation.
- All controls have visible focus states, hit targets at least 28 px desktop,
  semantic labels, and color-independent selected/error/status indicators.

## 8. Data model

The backend owns authorization and validation. Clients may optimistically
render changes but cannot assume writes succeeded.

```text
users
  id, email?, timezone, created_at, updated_at

tasks
  id, user_id, parent_id?, title, note?, bucket,
  scheduled_date?, scheduled_time?, scheduled_timezone?,
  position, completed_at?, deleted_at?, created_at, updated_at

calendar_connections
  id, user_id, provider, encrypted_refresh_token, scopes,
  calendar_account_email?, connected_at, last_sync_at?, last_error?

calendar_events
  id, connection_id, provider_event_id, calendar_id, title,
  starts_at?, ends_at?, all_day, local_date, color?, event_url?,
  updated_at, expires_at

task_audit (v1 storage, no UI)
  id, task_id, actor_user_id, action, before_json?, after_json?, created_at
```

Constraints:

- Every mutable row is scoped to its owner user.
- `bucket` is `inbox | active | someday` only; Today, Upcoming, and Anytime
  are computed views.
- `parent_id` must reference another owned task, cannot reference itself, and
  must not itself have a `parent_id`.
- A deleted or completed parent cannot accept a new open subtask.
- `scheduled_time` requires `scheduled_date`; scheduled time is a local wall
  time in `HH:mm` format.
- Client-provided `position` is normalized server-side within its sibling list.

## 9. Technical direction

### Shell

Use the cloned Flow GPUI desktop application as the visual and platform
foundation, retaining only the native application lifecycle, window chrome,
theme primitives, input/focus infrastructure, and sidebar/content layout.
Remove agent sessions, daemon RPC, transcript/composer semantics, terminal,
Git, computer-use, provider integrations, and Flow-specific settings.

This is not a clean-room implementation: Flow is GPL-3.0-only. Any distributed
Flow binary that retains or modifies its code must be distributed under GPL-3.0
with its corresponding source and required notices. Before a closed-source or
permissively licensed product is contemplated, replace the shell with a
clean-room GPUI shell rather than copying Flow code.

### Backend

Use a local task store for the first usable build. It keeps capture and
deterministic natural-language date parsing free of setup friction. Adopt
self-hosted Convex after that interaction model is proven: it supplies
TypeScript schema, queries, mutations, generated client types, reactive
subscriptions, and server functions in one place. Convex's self-hosted backend
and dashboard run as containers; start with its persisted SQLite volume, then
migrate its durable store to PostgreSQL and object storage when deploying on
k3s. Keep application data behind a small repository boundary so GPUI code
does not depend on storage details.

The first deployment is a single-user install. Add authentication before
exposing it beyond a trusted network. OAuth callback URLs, Convex deployment
keys, and calendar refresh tokens are secrets: store them in Kubernetes Secrets
or an external secret store, never in the repository or client state.

### Repository target after stripping

```text
src/
  app/                 # Flow shell, sidebar, views, task components
  main.rs              # native entrypoint retained from Flow foundation
convex/
  schema.ts
  tasks.ts
  calendar.ts
  auth.ts
docs/
  PRODUCT_REQUIREMENTS.md
deploy/
  compose/             # local Convex and Flow development
  k8s/                 # later: namespace, secrets references, ingress, PVCs
```

The desired executable path is intentionally short:

```text
GPUI view → typed Flow client → Convex function → Convex storage
                         └→ calendar sync function → cached event rows
```

There is no separate REST API, ORM, custom websocket server, message queue, or
microservice in v1.

## 10. Security, privacy, and reliability

- Treat task titles and calendar event titles as private data in UI, logs, and
  telemetry. No title or event value may be sent to error reporting by default.
- Use Google’s minimum read-only calendar scope. Encrypt refresh tokens at rest
  with a deployment-managed key and never return them to the client.
- Authenticate every backend function; authorize every row by `user_id`.
- Rate-limit calendar refresh, validate all parsed values server-side, and
  reject malformed dates and parent relationships.
- Keep a daily durable backup of backend data plus calendar cache. Restore must
  be tested before calling k3s production-ready.
- Local optimistic writes must be idempotent using a client mutation ID; retries
  cannot create duplicate tasks.

## 11. Acceptance criteria

### Task core

- A user can create, edit, complete, reopen, move, schedule, and undo-delete a
  task without leaving the keyboard.
- A new plain task is visible in Inbox immediately and remains after relaunch.
- Today shows only active tasks scheduled today or earlier; Upcoming starts
  tomorrow; Anytime shows only active unscheduled tasks.
- A parent cannot be completed without explicitly completing its open children.
- Completing or reopening a task has an accessible, reduced-motion-safe result.

### Parsing

- The two example inputs in section 6.4 parse to their stated title/date/time
  in `Australia/Melbourne` on 2026-08-18.
- A parser failure leaves the input untouched and does not prevent save.
- A user can override every parser result before saving.
- Stored relative-date output stays correct across daylight-saving changes.

### Calendar

- A connected Google calendar’s events appear in Today and Upcoming within the
  rolling sync window, ordered correctly and without blocking task interaction.
- Revoking/disconnecting removes cached events and tokens from the active
  installation.
- A provider failure is recoverable from the UI and does not expose credentials.

### Platform

- The app starts into the task shell with no Flow agent/project/session UI,
  daemon, terminal, Git, or coding-provider configuration reachable.
- Build and test instructions work on macOS and Linux, the platforms Flow
  documents for development.
- The shipped project includes GPL-3.0 license and upstream notices while it
  retains Flow-derived code.

## 12. Delivery sequence

### Milestone 0 — Strip to a running shell

Retain the GPUI app lifecycle, window chrome, theme, commands, focus handling,
and a static sidebar/main-pane frame. Delete Flow domain modules and daemon
runtime rather than hiding them. The result launches with placeholder Flow
views and has no external backend dependency.

Exit: a native Flow window starts in under two seconds, navigation changes the
main title, and `rg -i 'agent|session|daemon|terminal|git' src` has no
user-reachable product strings.

### Milestone 1 — Local task vertical slice

Build Inbox, task row/detail, tasks and one-level subtasks, local persistence,
the five placement rules, completion animation, and the deterministic parser.
Use a small fixture-backed calendar-glance component only to prove layout.

Exit: all Task core and Parsing acceptance criteria pass locally.

### Milestone 2 — Convex sync

Introduce the Convex schema/functions and replace local task persistence with
typed reactive mutations and queries. Add idempotency and the audit rows.

Exit: two desktop instances for the same account converge after a mutation;
the app remains useful when the server is temporarily unavailable.

### Milestone 3 — Google calendar glance

Add OAuth connection, encrypted credentials, rolling event sync, event cache,
and failure/disconnect states. Keep write scopes out.

Exit: Calendar acceptance criteria pass against a test calendar.

### Milestone 4 — k3s deployment

Package Flow, self-hosted Convex backend/dashboard, persistent volume strategy,
TLS ingress, backup/restore procedure, and secret injection. Move storage to
PostgreSQL only when the SQLite volume is no longer operationally sufficient.

Exit: an isolated k3s install survives a pod restart and a restore drill.

## 13. Decisions deferred deliberately

- **Recurring tasks and reminders:** add after daily scheduling and calendar
  glance are trusted; both need a background-delivery design.
- **Areas/projects/tags:** add only when flat task lists no longer hold the
  user’s active work; do not pre-build generic containers.
- **Additional calendar providers:** add behind a provider adapter only when a
  second provider is actually required.
- **Shared lists and mobile/web clients:** require a real authentication,
  permissions, offline, and conflict-resolution design; they are not a quick
  extension of this single-user v1.

## 14. Open product decisions

1. Should Inbox tasks with a parsed date be activated automatically on save?

## Confirmed foundation decisions

- Flow is GPL-3.0 open source and will reuse and strip Flow's GPUI shell in
  place. The distributed project must retain its GPL license and upstream
  notices.
- The first usable build is local-first and NLP-first. Self-hosted Convex and
  read-only Google Calendar follow after task capture, scheduling, and
  subtasks feel right.
   This PRD recommends **no**: preserve Inbox as a review queue.

## 15. Implementation references

- [Flow source and GPL-3.0-only license](https://github.com/egoist/waku)
- [Convex self-hosting documentation](https://docs.convex.dev/self-hosting)
- [Convex self-hosted Docker deployment guide](https://github.com/get-convex/convex-backend/blob/main/self-hosted/README.md)
