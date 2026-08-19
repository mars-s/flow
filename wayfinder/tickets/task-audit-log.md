---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-1]
status: open
assignee: unassigned
---

# Implement the task_audit table

## Context

Found via a data-integrity audit sweep, not a user report:
`docs/PRODUCT_REQUIREMENTS.md` §8's data model lists a `task_audit` table
("v1 storage, no UI") —

```text
task_audit (v1 storage, no UI)
  id, task_id, actor_user_id, action, before_json?, after_json?, created_at
```

— with no milestone-deferral note next to it, unlike `calendar_connections`/
`calendar_events` (annotated as Milestone 2 scope) or the client-mutation-ID
bullet in §10 (annotated as Milestone 2 scope, both during this same audit
sweep). Nothing in `src/db.rs` creates this table or writes to it — no
`CREATE TABLE`, no insert on any mutation path.

## Why this wasn't just built during the audit

Wiring an audit trail properly means a write hook on every mutation path
(`create_task`, `set_completed`, `schedule`, `set_note`, `set_title`,
`delete_task`/`restore_task`'s new cascades, `create_subtask`, bulk
variants) with a real decision about what `before_json`/`after_json`
actually capture — that's a real feature with its own design surface
(schema shape already fixed by §8, but capture granularity, whether bulk
operations get one audit row or one per task, and how the "v1 storage, no
UI" phrase constrains scope are all open), not a bug with an obvious
one-line fix. Building it hastily, unreviewed, overnight, risks either an
incomplete hook set (silently missing mutations, worse than not having the
table at all — a false sense of a complete trail) or schema churn nobody
asked for yet. Flagging it here rather than either building it blind or
leaving it undocumented.

## Open questions for whoever picks this up

- Does "v1 storage, no UI" mean every mutation from Milestone 1 onward
  needs an audit row retroactively meaningful, or is starting the trail
  from whenever this ticket lands acceptable (no backfill for rows written
  before it existed)?
- One row per task per bulk operation, or one row for the whole batch? The
  schema's `task_id` column reads as one-row-per-task, but that means a
  bulk-process of 20 tasks writes 20 audit rows for what was one user
  action — worth confirming that's the intended granularity before wiring
  20 call sites to do exactly that.
- `actor_user_id`: Flow is currently single-user with no auth model at all
  (`docs/turso.md` — this predates any login flow). Does this column get a
  placeholder constant, or wait until Milestone 2 introduces real user
  identity?
