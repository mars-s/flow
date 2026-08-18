---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-0]
status: closed
assignee: unassigned
---

# Rebuild the Flow shell frame

## Scope

Exclusive ownership of:

- `src/app/sidebar.rs`
- `src/app/window_chrome.rs`
- `src/app/render.rs`
- `src/app/components.rs`
- `src/app/tests.rs`
- The top-level `src/app/mod.rs` (or equivalent module-tree file) module
  declarations — this is the one place other Milestone 0 tickets may ask this
  ticket, via a `TODO(milestone-0)` marker, to remove a `mod` line for a file
  they deleted.

No other ticket touches these paths. Do not edit files outside this list.

## Goal

Produce the navigation rail and main-pane frame from
`docs/PRODUCT_REQUIREMENTS.md` section 5: a fixed sidebar (Inbox, Today,
Upcoming, Anytime, Someday, Calendar, Settings, `+ New task`) and a single
main pane that swaps placeholder views on selection. Keep native window
lifecycle, chrome, theme primitives, and focus/keyboard handling from Flow;
delete the Flow-specific sidebar sections (agent sessions list, provider
switcher, skills entry) and command surfaces that belonged to `components.rs`.

## Definition of done

- Sidebar shows exactly the seven Flow destinations named above, each
  navigable by mouse and keyboard (`tab`/arrow/`enter` per this repo's
  accessibility conventions), with the Inbox badge wired to a stub count.
- Main pane renders a placeholder view per destination (title + "coming
  soon" is acceptable for Milestone 0) with the 120–160 ms cross-fade/slide
  the PRD specifies, respecting `cx.reduce_motion()`.
- `window_chrome.rs` still shows correct native chrome with no leftover
  Flow branding (window title, about text) — coordinate with
  `branding-and-docs-cleanup` if strings live outside this file.
- `components.rs` retains only shared primitives Flow's placeholder views
  actually use; delete unused Flow-specific components rather than leaving
  dead code.
- `tests.rs` is rewritten to cover navigation and placeholder rendering, not
  the deleted session/transcript behavior.
- App launches in under two seconds into this shell with no external backend
  dependency (Milestone 0 exit criterion).

## Dependencies

Best done after (or in tight coordination with) `strip-session-transcript-
composer` and `strip-provider-and-tooling-ui`, since those tickets may leave
`TODO(milestone-0)` markers in files this ticket owns. Take the last pass
before `verify-milestone-0-exit`.
