---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-0]
status: closed
assignee: unassigned
---

# Verify Milestone 0 exit criteria

## Scope

Read-only verification plus whatever minimal glue fixes are needed to satisfy
the exit bar. If a real fix belongs to another ticket's file ownership, file
it back to that ticket rather than patching around it here.

## Goal

Confirm `docs/PRODUCT_REQUIREMENTS.md` Milestone 0's exit criteria hold once
every other Milestone 0 ticket has landed:

> A native Flow window starts in under two seconds, navigation changes the
> main title, and `rg -i 'agent|session|daemon|terminal|git' src` has no
> user-reachable product strings.

## Definition of done

- `rg -i 'agent|session|daemon|terminal|git' src` is run and its output is
  triaged: every remaining hit is either a false positive (comment, unrelated
  English word, GPL/license text) or is filed back to the owning ticket.
- Cold-launch timing is measured against the freshly built, signed debug app
  (per this repo's `CLAUDE.md` dev-runtime guidance) and recorded as under two
  seconds, or a follow-up ticket is filed if it isn't.
- Manual pass through all seven sidebar destinations confirms the main title
  updates and the cross-fade/slide + reduced-motion behavior from
  `rebuild-shell-frame` works.
- `cargo build` and this repo's existing test suite pass clean.
- The Milestone 0 tickets in `wayfinder/flow-map.md`'s "Decisions so far" /
  ticket index are marked closed, and `flow-map.md` is updated to record
  Milestone 0 as done, unblocking Milestone 1 (`docs/PRODUCT_REQUIREMENTS.md`
  section 12).

## Dependencies

Depends on all other `wayfinder:milestone-0` tickets landing first:
`strip-daemon-and-runtime`, `strip-session-transcript-composer`,
`strip-provider-and-tooling-ui`, `rebuild-shell-frame`,
`audit-shared-primitives`, `cargo-workspace-cleanup`,
`branding-and-docs-cleanup`.
