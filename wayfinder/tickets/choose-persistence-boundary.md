---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:grilling]
status: closed
assignee: codex
---

# Choose Flow's first persistence boundary

## Question

Should the first usable Flow build keep task data local and introduce
self-hosted Convex only after the task experience is proven, or should it begin
with self-hosted Convex from the first vertical slice?

The answer fixes the initial setup burden, data ownership model, and whether
calendar OAuth is part of the first build boundary.

## Resolution

The first usable Flow build stores task data locally. It prioritizes capture,
one-level subtasks, task placement, and deterministic natural-language date
parsing without account setup or a running backend. Self-hosted Convex follows
once this task experience is proven; it replaces local persistence behind a
typed boundary. Google Calendar remains a later integration.
