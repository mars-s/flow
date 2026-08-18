---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-0]
status: closed
assignee: unassigned
---

# Strip the agent/daemon backend from flow-core; recover generic infra

## Background

The original Milestone 0 breakdown scoped only `src/app/*` and a handful of
top-level files. It missed that `crates/flow-core` (22,600+ lines) and several
un-owned top-level `src/*.rs` files are almost entirely Flow's coding-agent
daemon backend: per-provider session handlers, Git worktree/checkout/commit
automation, computer-use, skills discovery, usage/cost tracking, and the
daemon RPC server itself. None of that is Flow product surface.

`strip-daemon-and-runtime` already deleted `crates/flow-protocol` and
`crates/flow-client` wholesale, which broke `flow-core`'s `theme.rs`,
`i18n.rs`, `identity.rs`, and `protocol.rs` — each of those is just a one-line
`pub use flow_protocol::...` re-export, and the real generic type definitions
(theme tokens, i18n scaffolding, identity) live inside the now-deleted
`flow-protocol` crate. The user has confirmed: **keep the reusable
infrastructure (theme, and anything else genuinely generic), delete the
agent/AI-specific pieces.**

## Scope

Exclusive ownership of:

- `crates/flow-core/` in full
- `src/computer_use.rs`
- `src/daemon.rs`
- `src/terminal.rs`
- `src/js_repl.rs`
- `src/browser.rs` (verify first — if this is the generic embedded-webview
  surface rather than an agent tool result viewer, keep it; if it exists only
  to render agent tool browser output, delete it. Check actual callers before
  deciding.)
- `src/theme.rs` (needs its `flow_protocol` import fixed, not necessarily
  rewritten)
- Top-level `Cargo.toml` `[dependencies]` (further pruning after this
  deletion pass — `cargo-workspace-cleanup` already ran once; you may edit
  this file further since that ticket is closed and no other agent should be
  touching it concurrently)

Do NOT touch `src/review_diff.rs` — that's owned by the in-flight
`rebuild-shell-frame` ticket. Do NOT touch `src/app.rs`, `src/lib.rs`,
`src/app/sidebar.rs`, `src/app/window_chrome.rs`, `src/app/render.rs`,
`src/app/components.rs`, `src/app/tests.rs` — same reason. If you find a
caller in one of those files, leave a `// TODO(milestone-0): ...` note and
report it rather than editing.

## Goal

1. **Recover the generic pieces `flow-protocol` used to hold.** Before
   deleting `crates/flow-protocol` is confirmed gone from the working tree,
   read its original source via `git show main:crates/flow-protocol/src/...`
   (it still exists in the `main` branch's history — nothing was lost, only
   deleted from this branch's working tree). Find the actual definitions
   behind `flow_protocol::theme::*`, `flow_protocol::i18n::*`, and
   `flow_protocol::identity::*`. Inline those definitions directly into
   `crates/flow-core/src/theme.rs`, `i18n.rs`, `identity.rs` (or into
   `src/theme.rs` directly if that's a cleaner home — your call, but keep it
   in one obvious place, not scattered). The goal is that `src/theme.rs` and
   any other genuinely generic caller compiles again without needing
   `flow-protocol` to exist.
2. **Delete everything else in `crates/flow-core` that is agent/daemon-
   specific.** That's the entire crate minus the theme/i18n/identity pieces
   you just recovered: `amp_session.rs`, `attachments.rs`, `blob_store.rs`,
   `checkpoint.rs`, `claude_session.rs`, `command_env.rs`,
   `composer_complete.rs`, `computer_use.rs`, `cursor_session.rs`,
   `daemon.rs`, `deepseek_pool.rs`, `deepseek_session.rs`, `git_branch.rs`,
   `git_commit.rs`, `grok_session.rs`, `model.rs`, `model_catalog.rs`,
   `opencode_pool.rs`, `opencode_session.rs`, `persistence.rs`,
   `projectless.rs`, `protocol.rs` (once nothing needs its re-exports),
   `server.rs`, `settings.rs`, `skills.rs`, `terminal.rs`, `usage.rs`,
   `usage_history.rs`, `workspace.rs`, `worktree.rs`. If, while reading one of
   these, you find something that's actually generic and not agent-specific
   (double-check — several of these have misleadingly generic names but their
   doc comments all say "Daemon-owned"), pull it out the same way you pulled
   out theme/i18n/identity and say so in your report. Default to delete.
3. **Decide whether `crates/flow-core` survives at all** once step 2 is done.
   If everything worth keeping fits in a couple of small files, it's
   reasonable to fold them directly into the top-level `flow` crate (e.g.
   `src/theme.rs`) and delete `crates/flow-core` entirely, removing it from
   the workspace. If enough survives to justify a separate crate, keep it
   slim. State your reasoning in the report either way.
4. **Delete the daemon/agent-specific top-level files**: `src/computer_use.rs`
   (headless computer-use state), `src/daemon.rs` (provider/driver-event wire
   translation), `src/terminal.rs` (daemon-owned PTYs — Flow has no terminal
   per the PRD's non-goals), `src/js_repl.rs` (daemon JS runtime debug REPL).
5. **`src/browser.rs`**: check its actual usage before deciding. Flow may
   eventually want an embedded browser for the Google Calendar OAuth flow
   (Milestone 3) — if this file is a generic WKWebView wrapper, it may be
   worth keeping even though nothing calls it yet in Milestone 0; if it's
   specifically wired to agent tool output, delete it. State your reasoning.
6. **Prune `Cargo.toml` further**: once `src/terminal.rs` is gone,
   `alacritty_terminal` is genuinely dead — remove it. Check for any other
   dependency that only existed for the files you just deleted.

## Definition of done

- `src/theme.rs` (and any other file with a real generic dependency on the
  old `flow_protocol` theme/i18n/identity types) compiles without
  `flow-protocol` existing.
- Every agent/daemon-specific file listed above is deleted.
- `crates/flow-core`'s fate (kept-slim vs. folded-and-removed) is decided and
  executed, with reasoning in the report.
- `cargo check` run against everything you own shows no errors originating
  from your files (errors in `rebuild-shell-frame`'s files are not yours to
  fix — note them if relevant, don't fix them).
- Report back: what you recovered and where it now lives, exactly what you
  deleted, the `crates/flow-core` fate decision, the `browser.rs` decision,
  and any TODOs left in files you don't own.

## Dependencies

Runs after `strip-daemon-and-runtime` (already landed) and independent of the
in-flight `rebuild-shell-frame` / already-landed `cargo-workspace-cleanup`
except where explicitly noted above. `verify-milestone-0-exit` depends on
this ticket landing too.
