# Blank main-pane regression

## Symptom

Flow launched normally and the sidebar rendered, but Inbox, Calendar, and
every other main pane were blank. The development watcher was also stopped
after the previous app instance exited, so source changes were not rebuilding.

## Root cause

`Flow::render_drag_bar` returned a full-width, non-shrinking flex child. It
was appended to the horizontal app shell alongside the sidebar and main pane.
The drag bar therefore consumed the available width and reduced the main pane
to zero width.

## Fix

The drag bar is an overlay, so it must be positioned as one:

```rust
div()
    .id("drag-bar")
    .absolute()
    .top_0()
    .left_0()
    .right_0()
```

This keeps the titlebar draggable without participating in the shell's flex
layout. The existing main-pane fade remains unchanged.

## Prevention and verification

- Any visual overlay added to `flow-shell` must be `absolute()` (or otherwise
  removed from the horizontal flex layout).
- When the app looks stale, first check `pgrep -f 'bun ./scripts/dev.ts'`.
  The watcher intentionally stops when Flow Dev exits; restart it with
  `bun ./scripts/dev.ts` when it is not running.
- After touching shell or window-chrome layout, verify both a task view and a
  non-task view. Inbox should show tasks (or its empty state), and Calendar
  should show its own tab content (or Settings' "no calendar connected"
  state — whichever destination you check, it must not be blank).
