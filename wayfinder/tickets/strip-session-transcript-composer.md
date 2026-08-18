---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-0]
status: closed
assignee: unassigned
---

# Strip session, transcript, and composer UI

## Scope

Exclusive ownership of:

- `src/app/sessions.rs`
- `src/app/transcript.rs`
- `src/app/transcript_view.rs`
- `src/app/composer.rs`
- `src/app/autocomplete.rs`
- `src/app/drafts.rs`
- `src/app/activity_diff.rs`
- `src/app/right_panel.rs`

No other ticket touches these paths. Do not edit files outside this list.

## Goal

Delete every coding-agent chat/session concept: sessions list, transcript
rendering, message composer, autocomplete for slash-commands/@-mentions, saved
drafts, tool-activity diffs, and the right-hand agent activity panel. None of
this exists in Flow's product surface (Inbox/Today/Upcoming/Anytime/Someday +
task detail).

## Definition of done

- All eight files are deleted.
- Any `mod` declarations and re-exports for them are removed from
  `src/app/mod.rs` (or wherever the module tree is declared) — check first
  whether that file falls under `rebuild-shell-frame`'s ownership per that
  ticket; if so, leave a `// TODO(milestone-0): remove sessions/transcript/
  composer module decls` marker instead of editing it directly.
- `rg -i 'session|transcript|composer|autocomplete|draft' src/app` returns
  only false positives (e.g. unrelated uses of "draft" as an English word) or
  nothing from the files this ticket owns.
- Note in the PR description any type or trait these files exposed that a
  file owned by another Milestone 0 ticket still imports, so that ticket can
  clean up its side.

## Dependencies

None. Can run in parallel with every other Milestone 0 ticket.
