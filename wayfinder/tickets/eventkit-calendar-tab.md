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

## Goal — done (`e8e910c`, `c9e6d74`)

A real `Destination::Calendar` view, modeled on the user's Apple Calendar
reference screenshot: a per-account sidebar with color-coded visibility
toggles, a Day/Week/Month/Year switch, Today/‹/› navigation, and a body per
mode — Day/Week as an agenda-per-day layout, Month as a traditional
expanded-to-full-weeks grid with a "+N more" overflow, Year as twelve
clickable month grids with an event-dot indicator (no per-event detail).

## Explicit non-goals (per PRD §6.5)

- No event creation/editing — read-only, always.
- No tap-through to a provider event URL — EventKit's local events have no
  equivalent (that was a Google-Calendar-specific affordance in the
  superseded design).
- No local event cache/sync window — every view queries EventKit live for
  whatever range it's showing, same as `calendar_events_between` already
  does for Today.

## Still open

- **Real hour-of-day positioning for Day/Week.** The shipped Day/Week body
  is an agenda-per-day layout (each day's events listed top-to-bottom), not
  Apple Calendar's pixel-accurate hour grid with true time-of-day vertical
  positioning and overlap resolution for concurrent events — a separate
  absolute-layout algorithm, disclosed as a simplification in `e8e910c`'s
  own commit message. Worth doing if the agenda layout is felt to be
  insufficient in practice; nothing about it blocks the upgrade later, it's
  a rendering change only, no data-model change needed.
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
