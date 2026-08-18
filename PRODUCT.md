# Product

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

## Stack

Existing native Rust and GPUI desktop application for macOS and Linux. The
first implementation keeps task data local; self-hosted Convex is a later
sync/deployment phase.

## Users

The primary user is the owner of a private Flow installation who spends much
of the day at a desktop and needs to capture, decide, and complete personal
work without using a project-management suite.

## Product Purpose

Flow makes personal tasks easy to capture and turn into clear next actions.
It succeeds when the user can see what matters now, schedule tasks naturally,
and keep the calendar as supporting context rather than another planning job.

## Positioning

Flow is an open-source native task manager with a Things-like placement model
and deterministic local time-language parsing. It is task-first, keyboard-led,
and calendar-aware without becoming a calendar editor or team workspace.

## Operating Context

The user captures tasks while working at a desktop, reviews Today and
Upcoming around a personal calendar, and later self-hosts their data on k3s.
Natural phrases such as "take out laundry 8 am tomorrow" must become visible,
editable schedules without sending task titles to an LLM.

## Capabilities and Constraints

- Inbox, Today, Upcoming, Anytime, and Someday are the core task surfaces.
- Tasks support a note and one level of subtasks.
- Initial persistence is local. Self-hosted Convex and read-only Google
  Calendar arrive after the task and NLP experience is proper.
- Flow remains GPL-3.0 open source and strips Flow's GPUI shell in place while
  retaining required upstream and third-party notices.
- Calendar events are read-only context. Flow never writes calendar events.

## Brand Commitments

The name is Flow. The app is a dark native workspace inspired by Flow's
desktop precision and Codex's focused command surface. It must feel calm,
high-craft, and task-first rather than chat-first or dashboard-like.

## Evidence on Hand

The user supplied three reference screenshots in the original conversation.
`docs/PRODUCT_REQUIREMENTS.md` and `docs/DESIGN_DIRECTION.md` capture the
approved product and visual direction. No customer metrics, testimonials, or
commercial claims are available and none may be invented.

## Product Principles

1. Capture first, classify deliberately.
2. Time language must be explicit, correctable, and private.
3. Calendar is context, not a competing source of work.
4. One decisive next action is better than an expansive productivity system.
5. Native speed and keyboard access are baseline behavior.

## Accessibility & Inclusion

Flow supports complete keyboard operation, visible focus, semantic status
signals, and reduced motion. Color alone must not convey selection, progress,
or errors.
