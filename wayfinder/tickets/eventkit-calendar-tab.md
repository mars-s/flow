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
the Today glance on it. `e8e910c` landed this ticket's own Day/Week slice
(`src/app/calendar.rs`) — an agenda-per-day layout, disclosed as a
simplification against Apple Calendar's real hour-grid positioning in that
commit's own message. **Still open below: Month, Year, and the two
unresolved questions this ticket already named.**

## Goal

A real `Destination::Calendar` view — currently still the generic
`placeholder_pane`. User-provided reference: Apple Calendar's own layout —

- A left sidebar listing every calendar the user has, grouped by account
  (`platform::calendar_list`'s `source_title`), each with its own color dot
  and an on/off visibility toggle (`platform::CalendarInfo` already has
  everything this needs — `id`, `title`, `source_title`, `color` — nothing
  new to fetch, just something to render and a per-calendar shown/hidden set
  to add to `Flow`).
- A top switch between Day / Week / Month / Year (Day/Week done; Month/Year
  need their own switch entries added once built).
- Day and Week: **done** (`e8e910c`) as an agenda-per-day layout rather than
  a true pixel-positioned hour grid — see that commit's message for why.
  Upgrading to real hour-of-day positioning with overlap resolution (an
  absolute-layout algorithm for concurrent events) is still open if the
  agenda layout is felt to be insufficient in practice; nothing about the
  agenda approach blocks it later, it's a rendering change only.
- Month: a traditional 5–6 week grid, one cell per day, showing a few
  events per day plus an overflow count.
- Year: twelve small month grids (a bird's-eye view, no per-event detail).

## Explicit non-goals (per PRD §6.5)

- No event creation/editing — read-only, always.
- No tap-through to a provider event URL — EventKit's local events have no
  equivalent (that was a Google-Calendar-specific affordance in the
  superseded design).
- No local event cache/sync window — every view queries EventKit live for
  whatever range it's showing, same as `calendar_events_between` already
  does for Today.

## Open questions worth resolving before or during the build, not guessing through

- Exact date range EventKit is queried for per tab (Day: that day; Week: the
  ISO week the currently-shown day falls in — check whether Flow's other
  week-shaped UI, if any, already has a Monday/Sunday-start convention to
  match; Month: the visible month, expanded to the full weeks shown at its
  edges like Apple Calendar's own grid does; Year: the twelve visible
  months) — pick the convention, don't leave it implicit per view.
- Whether per-calendar visibility toggles persist across launches. Decided
  for now (`e8e910c`, `Flow::calendar_hidden_ids`'s own field doc): no —
  every launch starts with every calendar visible, since there's no
  settings-persistence file anywhere in this codebase yet and building one
  just for this would be new infrastructure, not a natural extension. Worth
  revisiting once real usage says otherwise, or once some other feature
  needs the same persistence file first.
- Whether Today's own glance should also start respecting the same
  per-calendar visibility toggles once they exist, or stay showing every
  calendar regardless (the glance and the tab could reasonably diverge: a
  glance is meant to be comprehensive-at-a-glance, the tab is where someone
  actually curates what they look at).

## Suggested slicing

Day/Week landed first (`e8e910c`) — the two most task-adjacent views, "what
do I have today/this week" being the actual job the calendar glance already
serves, just expanded. Sidebar visibility toggles also landed as part of
that same commit (they were cheap once `platform::calendar_list` existed).
Still open, in order: Month, then Year. Each is independently shippable and
testable against a real Apple Calendar account (no test-calendar fixture
exists for EventKit — see Milestone 3's own exit-criteria note on this).
