---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-3]
status: open
assignee: unassigned
---

# Add the EventKit Calendar tab

## Context

See [Choose Flow's first calendar source](choose-calendar-source.md)'s
2026-08-19 supersede note and `docs/PRODUCT_REQUIREMENTS.md` §6.5/Milestone 3
for the full decision: Flow reads the local macOS Calendar app via EventKit,
not Google OAuth. `b76195e` landed the permission flow (Settings' "Connect
Calendar"), `src/eventkit.rs`/`src/platform.rs`'s EventKit bridge, and gated
the Today glance on it. `e8e910c` and `c9e6d74` landed this ticket's full
Day/Week/Month/Year scope (`src/app/calendar.rs`). **Kept open for the two
real follow-ups below — not a placeholder ticket anymore, a tracker for
genuine remaining decisions.**

## Goal — done (`e8e910c`, `c9e6d74`, `5be0aef`)

A real `Destination::Calendar` view, modeled on the user's Apple Calendar
reference screenshot: a per-account sidebar with color-coded visibility
toggles, a Day/Week/Month/Year switch, Today/‹/› navigation, and a body per
mode. Month is a traditional expanded-to-full-weeks grid with a "+N more"
overflow; Year is twelve clickable month grids with an event-dot indicator
(no per-event detail). Day and Week diverged on 2026-08-20, by explicit user
request: Day kept the original agenda-per-day list (the user liked its
Kanban-board look and asked to keep it around for later reuse), while Week
moved to a real hour grid — see the entry directly below.

## Explicit non-goals (per PRD §6.5)

- No event creation/editing — read-only, always.
- No tap-through to a provider event URL — EventKit's local events have no
  equivalent (that was a Google-Calendar-specific affordance in the
  superseded design).
- No local event cache/sync window — every view queries EventKit live for
  whatever range it's showing, same as `calendar_events_between` already
  does for Today.

## Still open

- **Real hour-of-day positioning for Day.** Week got its hour grid in
  `5be0aef` (2026-08-20) — see `render_calendar_week_grid` in
  `src/app/calendar.rs`. Day still uses the original agenda-per-day list
  (`render_calendar_body`/`render_calendar_day_column`), deliberately, per
  the same commit: the user explicitly wants that Kanban-style layout kept
  around rather than replaced. If Day is ever asked to move to the grid
  too, `render_calendar_grid_day_column` already does the hard part (lane
  sweep for overlaps, absolute time positioning) — reuse it for a
  single-column Day body instead of writing a second layout algorithm.
- **The Week grid's overlap layout is a simplification, disclosed in
  `5be0aef`'s own commit message**: a greedy lane sweep gives every
  overlapping event in a day a uniform lane width (first lane whose
  previous occupant already ended, else a new lane), not Apple Calendar's
  true interval-packing algorithm. Fine for the common case; worth
  revisiting only if real calendars with heavy overlap make it look wrong
  in practice.
- **Whether per-calendar visibility toggles should persist across
  launches.** Decided for now (`Flow::calendar_hidden_ids`'s own field
  doc): no — every launch starts with every calendar visible, since there's
  no settings-persistence file anywhere in this codebase yet and building
  one just for this toggle would be new infrastructure, not a natural
  extension of existing code. Worth revisiting once real usage says
  otherwise, or once some other feature needs the same persistence file
  first (at which point this should reuse it, not gain a second one).
- **Whether Today's own glance should respect the same visibility
  toggles**, or keep showing every calendar regardless (the glance and the
  tab could reasonably diverge: a glance is meant to be
  comprehensive-at-a-glance, the tab is where someone actually curates what
  they look at). Not yet decided either way; currently the glance ignores
  `calendar_hidden_ids` entirely.
