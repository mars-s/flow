---
kind: issue
labels: [wayfinder:map]
status: closed
---

# Flow foundation map

## Destination

Reach a build-ready, legally and operationally clear plan for a beautiful
native Flow desktop app, then hand off a small first implementation slice with
no Waku agent-product behavior remaining.

## Notes

Domain: single-user personal productivity. Consult `/wayfinder`, `/grilling`,
and `/domain-modeling` while the map is active. Use
`docs/PRODUCT_REQUIREMENTS.md` and `docs/DESIGN_DIRECTION.md` as the current
product and visual baselines. The app may reuse Waku's GPUI shell only if its
GPL-3.0 obligations are accepted.

## Decisions so far

<!-- Closed decision tickets are indexed here as they resolve. -->

- [Choose Flow's distribution boundary](tickets/choose-distribution-boundary.md)
  - Flow is GPL-3.0 open source and may reuse Waku's GPUI shell in place.
- [Choose Flow's first persistence boundary](tickets/choose-persistence-boundary.md)
  - Ship a local, NLP-first task experience, then replace persistence with
    self-hosted Convex after the interaction model is proven.
- [Choose Flow's first calendar source](tickets/choose-calendar-source.md)
  - Add read-only Google Calendar only after the local task and NLP experience
    is proper; calendar is contextual and never writable. **Superseded
    2026-08-19**: reads local macOS Calendar via EventKit instead — see the
    ticket's own supersede note and
    [Add the EventKit Calendar tab](tickets/eventkit-calendar-tab.md) (open)
    for the in-progress work.

## Milestone 0 — done

`docs/PRODUCT_REQUIREMENTS.md` section 12 defines Milestone 0 as stripping
Flow's coding-agent product down to a running Flow shell. All tickets below
are closed on branch `milestone-0-strip`, uncommitted pending review:

- [Strip daemon and agent runtime](tickets/strip-daemon-and-runtime.md)
- [Strip session, transcript, and composer UI](tickets/strip-session-transcript-composer.md)
- [Strip provider and tooling UI](tickets/strip-provider-and-tooling-ui.md)
- [Rebuild the Flow shell frame](tickets/rebuild-shell-frame.md)
- [Audit shared UI and markdown primitives](tickets/audit-shared-primitives.md)
- [Clean up the Cargo workspace](tickets/cargo-workspace-cleanup.md)
- [Rebrand product-facing docs, locales, and assets](tickets/branding-and-docs-cleanup.md)
- [Strip the agent/daemon backend from waku-core](tickets/strip-waku-core-backend.md) —
  discovered mid-Milestone-0: the original breakdown missed `crates/waku-core`
  (later renamed to `crates/flow-core` during the Waku→Flow branding sweep)
  (22,600+ lines of per-provider agent session handlers, daemon RPC, Git
  automation) and several un-owned top-level `src/*.rs` files. Kept
  `theme.rs`, `i18n`/`identity` (folded into a slimmed `flow-core`), and
  `browser.rs` (generic WKWebView, useful for the future Calendar OAuth flow);
  deleted the rest.
- [Verify Milestone 0 exit criteria](tickets/verify-milestone-0-exit.md) —
  `cargo check --workspace` and `cargo test --workspace` both clean (148
  tests passing); `rg -i 'agent|session|daemon|terminal|git' src` has no
  remaining user-reachable strings, only internal doc-comments/identifiers.
  App cold-launch timing not yet measured (visual verification skipped per
  this repo's "no visual test unless requested" convention — ask if you want
  it checked in the running debug app).

Sidebar is currently Inbox/Today/Upcoming/Anytime/Someday/Calendar/Settings
with placeholder "coming soon" panes and a working cross-fade; `+ New task`
and the command palette are inert stubs. Milestone 1 (local task vertical
slice) is next per the PRD's delivery sequence.

## Not yet specified


None required before the local task implementation slice.

## Out of scope

- Collaboration, shared lists, attachments, calendar editing, and recurring
  tasks are outside this foundation effort.
- First-run account setup, the local-to-Convex migration mechanics, and a
  future companion client belong to later efforts after the local task slice is
  validated.
