---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-0]
status: closed
assignee: unassigned
---

# Strip provider, git, and tooling UI

## Scope

Exclusive ownership of:

- `src/app/branches.rs`
- `src/app/commit_dialog.rs`
- `src/app/file_search.rs`
- `src/app/image_preview.rs`
- `src/app/command_palette.rs`
- `src/app/settings.rs`
- `src/app/skills_page.rs`
- `src/app/usage_meter.rs`
- `src/app/usage_page.rs`

No other ticket touches these paths. Do not edit files outside this list.

## Goal

Delete Git branch/commit UI, repo file search, image preview (agent
tool-result viewer), the coding-agent command palette, coding-provider
settings, the skills page, and usage/cost meters. None of these are Flow
product surfaces per `docs/PRODUCT_REQUIREMENTS.md` non-goals (no Git
integration, no provider config, no computer-use).

## Definition of done

- `branches.rs`, `commit_dialog.rs`, `file_search.rs`, `image_preview.rs`,
  `skills_page.rs`, `usage_meter.rs`, `usage_page.rs` are deleted.
- `command_palette.rs` is either deleted or reduced to a stub that Flow can
  later repopulate with task-specific commands (`⌘N` new task, jump to
  view) — prefer delete-and-let-`rebuild-shell-frame`-recreate-it unless
  removing it breaks `⌘K` wiring outside this ticket's ownership, in which
  case leave a `// TODO(milestone-0): rebuild command palette for Flow` stub.
- `settings.rs` is stripped down to nothing (Flow has no user-facing settings
  in Milestone 0) or reduced to a placeholder view registered by
  `rebuild-shell-frame`; do not invent new settings content here.
- `rg -i 'provider|branch|commit|skill|usage.?meter' src/app` returns nothing
  from the files this ticket owns.

## Dependencies

None. Can run in parallel with every other Milestone 0 ticket.
