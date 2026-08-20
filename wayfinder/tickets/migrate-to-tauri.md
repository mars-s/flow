---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-4]
status: open
assignee: unassigned
---

# Migrate Flow's UI from GPUI to Tauri

## Context

2026-08-20, explicit user decision, arrived at over several turns in one
session — recorded here in full since it reverses this project's own
"native Rust/GPUI desktop shell" framing (`PRODUCT.md`, `AGENTS.md`,
`docs/HANDOFF.md`'s "What Flow is").

**How this came about:** the user felt Flow "feels very prototype-y...
not really animated and satisfying" and asked whether GPUI was the wrong
platform. A real side-by-side was built to test that honestly —
`/Users/avi/Developer/vibe/flow-tauri-prototype` (Tauri v2 + React 19 +
TypeScript + Vite + Framer Motion), a small comparable task-list screen
with real spring/shared-layout motion, separate repo, nothing shared with
Flow. The user's reaction ("looks amazing and feels amazing") was
immediate and strong. The counter-argument given at the time — that the
gap was Flow's own thin motion vocabulary, not a GPUI ceiling, and that a
few real GPUI primitives (`Svg::with_transformation`, a hand-rolled
overshoot easing curve inside `with_animation` animator closures) could
close most of it without a rewrite — was heard, and two rounds of that
work did ship (`4bf2137`, `69a5272`: a real spring-pop checkbox
completion, real press feedback via `.active()`). It didn't change the
decision. The user restated the intent to migrate multiple times across
the conversation, including after the counter-argument and after the
concrete GPUI improvements shipped — this reads as a settled decision,
not a first reaction, and this ticket proceeds on that basis rather than
re-litigating it further.

**Explicitly asked and answered in the same conversation, don't
re-derive:**
- Multi-session, not one sweep — the user said so directly.
- In-place vs. parallel-build was discussed (embedding via `wry`/
  `src/browser.rs`'s dormant WKWebView wrapper vs. a real standalone
  Tauri app built in parallel and cut over at the end). The user's final
  answer was **"i wanna redesign in tauri"** — read as: build the real
  thing in the Tauri prototype as a proper parallel app, not as an
  in-place per-screen embed inside the existing GPUI window. If a future
  session finds reason to revisit that (e.g. wanting a working app
  sooner, mid-migration, rather than only at cutover), that's a real
  option still on the table — `src/browser.rs`'s WKWebView wrapper is
  still there, still unused, still viable for it.
- "Redesign," not "port" — the user's own word. This is licensed to be a
  real visual-design pass on top of the migration, not a mechanical
  1:1 recreation of GPUI's current screens. `docs/DESIGN_DIRECTION.md`'s
  actual tokens/spacing/component anatomy are a starting reference, not a
  spec to reproduce exactly.

## What exists so far

`/Users/avi/Developer/vibe/flow-tauri-prototype` — Tauri v2, React 19,
TypeScript, Vite, Framer Motion. Currently just the original comparison
demo (six hardcoded tasks, no real data, no backend). Not yet the actual
migration target — treat it as the seed to build the real thing in, not
as finished work to preserve exactly as-is.

## Open engineering questions for whoever picks this up next

- **How does the Tauri frontend reach Flow's real data and logic?**
  `src/db.rs` (Turso/SQLite), `src/eventkit.rs` (calendar), `src/parse.rs`
  (NLP scheduling) are all real, tested Rust — the plan should reuse them
  as a Tauri backend (via `#[tauri::command]`s calling into that logic,
  most likely by extracting the non-GPUI parts of Flow's `src` into a
  shared crate both the old GPUI binary and the new Tauri backend can
  depend on) rather than rewriting task/calendar/parsing logic in
  TypeScript. Not yet scoped which modules cut cleanly (they were written
  assuming a GPUI `Context`/`cx.background_executor()` runtime in
  places) versus which need real separation work first.
- **What happens to the GPUI app during the migration?** Kept running
  and released as-is until the Tauri app reaches parity, per "multi-
  session, not one sweep" — no plan yet for how parity is judged screen
  by screen, or whether there's an intentional overlap period where both
  exist.
- **Where does this ticket's own scope end and
  `sidebar-and-calendar-write-back.md`'s begin?** That ticket's sidebar
  redesign (collapsible rail, embedded Inbox/Upcoming sections) and
  calendar drag-to-schedule request were captured on GPUI assumptions
  earlier the same session — worth a fresh look through a "we're
  redesigning in Tauri now" lens rather than assumed to carry over
  unchanged. The calendar-write-back principle question (§6.5's
  read-only stance) still needs its own explicit confirmation regardless
  of platform.
- **Release/distribution.** Flow's current shipping story is a signed,
  notarized `.app` with a Sparkle updater (`scripts/bundle.sh`,
  `scripts/release.ts`, `docs/HANDOFF.md`'s CI/release-tooling fixes).
  Tauri has its own bundler and updater plugin; not yet decided whether
  to adopt Tauri's or keep Sparkle wired to a Tauri-built binary.

## Not started

Everything. This ticket exists to carry the decision and its reasoning
across sessions, not to claim progress that hasn't happened.
