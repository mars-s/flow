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
