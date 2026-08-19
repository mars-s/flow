---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:grilling]
status: closed
assignee: codex
---

# Choose Flow's first calendar source

## Question

Should Flow's first calendar glance connect to Google Calendar with read-only
OAuth, or read a local ICS feed first to keep early setup private and simple?

This decision must preserve Flow's rule that calendar data is read-only context
and never writes calendar events.

## Resolution

Flow will integrate Google Calendar using read-only OAuth after the local task
experience is proper: capture, task placement, one-level subtasks, and
deterministic natural-language scheduling must come first. Calendar remains a
quick-glance context pane and never writes events.

## Superseded (2026-08-19)

Explicit user decision, given once the local task experience above was in
fact proper (Milestone 1's exit bar was verifiably met the same day — see
`docs/HANDOFF.md`): read the local macOS Calendar app via **EventKit**
instead of Google OAuth. Same reasoning as [Choose Flow's first persistence
boundary](choose-persistence-boundary.md)'s Turso-over-Convex call — Flow is
local-first and explicitly avoids backend code, and EventKit needs no
server, no encrypted-token storage, no per-provider OAuth screen, while
transparently aggregating whatever accounts (iCloud, Google, Exchange, ...)
are already configured in Apple Calendar. The "read-only, never writes"
rule this ticket set carries over unchanged — EventKit access is requested
via `requestFullAccessToEventsWithCompletion`, never a write scope.

See [Add the EventKit calendar tab](eventkit-calendar-tab.md) for the open
follow-on work. `docs/PRODUCT_REQUIREMENTS.md` §6.5 and Milestone 3 are the
current source of truth; this ticket stays closed as a record of the
original call and why it changed, not as still-active guidance.
