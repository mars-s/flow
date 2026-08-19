# Flow context

## Glossary

### User

The person who owns a private Flow installation and its tasks and calendar
connections.

### Task

An actionable item owned by one User. A Task can have a title, a note, a
placement, an optional schedule, and direct Subtasks.

### Subtask

A Task whose parent is another Task. Flow permits one parent-child level only.

### Placement

The intentional state that determines where an incomplete Task belongs:
Inbox, active work, or Someday. Today, Upcoming, and Anytime are Computed
Views of active work, not placements themselves.

### Schedule

The optional local calendar date and local wall-clock time attached to a Task.
It is evaluated in the User's timezone and does not change its wall-clock time
when the User travels.

### Computed view

A task list derived from Placement and Schedule. Today shows active work due
today or earlier. Upcoming shows later active scheduled work. Anytime shows
active work with no Schedule.

### Calendar connection

**Revised 2026-08-19** (was originally a Google OAuth grant cached
server-side — see `docs/PRODUCT_REQUIREMENTS.md` §6.5's own revision note
and `wayfinder/tickets/choose-calendar-source.md` for the full decision):
the read-only macOS EventKit permission that lets Flow read a User's local
Calendar app. A system permission grant, not a per-provider authorization —
whatever calendar accounts (iCloud, Google, Exchange, ...) are already
configured in Apple Calendar become visible through it at once. Nothing is
cached locally; every read queries EventKit live.

### Calendar event

Read-only contextual information from a Calendar connection. It is never a
Task and Flow does not modify it.
