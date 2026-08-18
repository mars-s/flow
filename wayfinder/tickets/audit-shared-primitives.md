---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-0]
status: closed
assignee: unassigned
---

# Audit shared UI and markdown primitives

## Scope

Exclusive ownership of:

- `src/ui/` (`menu.rs`, `mod.rs`, `motion.rs`, `scrollbar.rs`, `text_field.rs`,
  `tooltip.rs`)
- `src/md/` (`highlight.rs`, `mend.rs`, `mod.rs`, `parser.rs`, `render.rs`,
  `selection.rs`, `veil.rs`)

No other ticket touches these paths. Do not edit files outside this list.

## Goal

This ticket is a decision, not a deletion pass. Determine, per directory,
whether Flow keeps it as-is, trims it, or removes it — then act on that
decision.

`src/ui/` is generic GPUI chrome (menus, motion helpers, scrollbar, text
field, tooltip) that Flow's task rows, detail editor, and pickers will need.
Default expectation: keep it close to as-is.

`src/md/` is the transcript markdown renderer (highlighting, veils for
streaming, selection). `docs/PRODUCT_REQUIREMENTS.md` section 6.1 gives a task
only an "optional plain-text note," not markdown, in v1. Default expectation:
delete it, since keeping unused code around is exactly what Milestone 0's
"delete rather than hide" instruction warns against — but check first whether
`rebuild-shell-frame`'s task-detail placeholder or a near-term Milestone 1 need
already depends on any piece of it (e.g. `veil.rs`'s streaming-reveal
primitive, if `src/ui/motion.rs` composes with it) before removing.

## Definition of done

- A short note in the PR description states the keep/trim/delete decision for
  each of `src/ui/` and `src/md/`, with the one-line reason.
- `src/md/` is deleted unless the note above found a real Milestone 0 or
  Milestone 1 dependency, in which case it's kept and the note says why.
- `src/ui/` keeps only primitives with an identified Flow caller after this
  round of Milestone 0 tickets lands; anything Flow-specific (e.g. a menu
  entry only relevant to coding-provider selection) is trimmed out.
- No build breakage introduced in files this ticket doesn't own — if trimming
  `src/ui/` would break a caller in another ticket's file, leave the primitive
  in place and note the caller instead of deleting.

## Dependencies

None to start, but its `src/md/` decision should land before
`verify-milestone-0-exit` since an unused, undeleted module would fail that
ticket's dead-code check.
