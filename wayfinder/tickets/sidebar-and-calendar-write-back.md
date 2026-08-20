---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-3]
status: open
assignee: unassigned
---

# Sidebar redesign + calendar scroll fixes + drag-to-schedule write-back

## Context

Captured verbatim from the user's own request (2026-08-20), mid-session, with
an explicit ask to log it as a ticket rather than build it immediately —
partly because of size, partly because a full-GPUI-vs-Tauri platform
conversation was still active at the same time (see `docs/HANDOFF.md`'s
`4bf2137` entry for that context). **Nothing in this ticket is built yet.**

## The ask, split into independently-shippable pieces

1. **Scrollable Month and Year calendar grids.** Week already scrolls
   (`5be0aef`, `b212183`). Month (`render_calendar_month_grid`) and Year
   (`render_calendar_year_grid`) both currently use `.flex_1()` fit-to-space
   layouts with no scroll container at all — on a short window, weeks/month
   cells compress rather than overflow. Smallest, safest, most clearly
   scoped piece of this ticket: give both the same `.id().overflow_y_scroll()`
   treatment Week already has. No product-decision risk, no data-model
   change.

2. **A collapsible calendar sidebar.** The Calendar tab's own per-account
   sidebar (`render_calendar_sidebar`, `src/app/calendar.rs`) should collapse
   as a whole (hide the whole rail, reclaiming width for the grid), and each
   calendar-account group within it should collapse independently too (so a
   long list of subscribed calendars doesn't dominate the rail). Needs: a
   collapsed/expanded bool per account group (a new `HashMap<String, bool>`
   or similar on `Flow`, same pruned-map shape `calendar_row_focuses`
   already uses) plus one for the whole rail. Purely a rendering/state
   change, no calendar-write implications.

3. **An "Inbox" section embedded directly in the main sidebar**, right under
   the Tasks/Calendar mode switch — expandable/collapsible, listing Inbox's
   own tasks inline rather than requiring a full navigation to the Inbox
   destination. This is a real information-architecture change to
   `src/app/sidebar.rs`, not a small tweak: it means task rows (or a
   condensed version of them) render inside the sidebar's own 252px column,
   with their own click/complete/expand affordances, alongside the existing
   nav-row list. Needs design thought before implementation — how condensed
   a row can get at that width, whether it reuses `render_task_row` or needs
   its own compact variant, whether it opens the detail card inline or still
   navigates to the full Inbox view.

4. **An "Upcoming" section in the sidebar, collapsed by default, grouped by
   date** — the same date-grouping `render_upcoming_section` already does
   for the Upcoming destination, but as a second collapsible block under the
   Inbox one from #3. Same open design questions as #3, plus: does this
   duplicate `render_upcoming_section`'s grouping logic, or factor it out
   for both call sites to share?

5. **Drag-and-drop a task onto the calendar to schedule it as a real
   calendar event**, living under **a new, dedicated Flow-managed calendar**
   (not one of the user's existing subscribed calendars), with the specific
   calendar to write into configurable in Settings (name, and presumably
   which of the user's writable local calendars/calendar sources backs it).

   **This is a product-principle reversal, not an extension, and needs an
   explicit yes before any of it is built:** `docs/PRODUCT_REQUIREMENTS.md`
   §6.5 states plainly — "No event creation/editing — read-only, always" —
   and the shipped `CHANGELOG.md` entry for this exact feature tells users
   "Flow never creates, edits, or deletes anything in your calendar." Both
   were deliberate, explicit product decisions made earlier this session
   (`docs/HANDOFF.md`'s EventKit entries), not oversights. Writing a real
   `EKEvent` via `EKEventStore.save(_:span:)` is a straightforward EventKit
   API addition on its own (`src/eventkit.rs` already has the read half
   built and can grow a write half the same way), but it changes what Flow
   *is allowed to touch* on the user's real calendar data — worth a
   deliberate, standalone confirmation from the user before scoping the
   actual implementation, not something to infer from "log this as a
   ticket for later."

   Once confirmed, the real design questions are: how "new dedicated
   calendar" is created (EventKit's `EKCalendar` creation requires a source
   — typically the local/On My Mac source, or iCloud if the user wants it
   synced — and Settings would need to expose that choice per §6.5's own
   Settings-lives-the-connection-lives-here convention); what a
   drag-and-drop gesture even looks like in GPUI (no existing drag-and-drop
   between two different tab destinations exists anywhere in this codebase
   today — `browser.rs`'s `on_drag`/`DragMoveEvent` plumbing is the closest
   prior art, but that's for a WKWebView content drag, a different shape of
   problem); and what happens to the *task* once its dragged copy becomes a
   calendar event — does the task get a scheduled date/time to match (likely
   yes, reusing the existing `Db::schedule` path), does it stay a task at
   all, and what does Flow do if the calendar event is later moved or
   deleted from outside Flow (EventKit gives no write-back notification
   Flow currently listens for).

## Suggested order

1 and 2 are safe, scoped, no-decision-needed pieces — do these whenever
picked up, independently of everything else.

3 and 4 need a real design pass (a `shape`/`new-work` pass per the
`impeccable` skill, not just code) before implementation — the sidebar is
already a defined, documented component in `docs/DESIGN_DIRECTION.md`, and
embedding two more collapsible task-list sections into a 252px rail is a
material change to it.

5 needs the explicit read-only-reversal confirmation above *before* any
design or implementation work starts, since it changes what the PRD and the
shipped changelog promise the user about their own calendar data.
