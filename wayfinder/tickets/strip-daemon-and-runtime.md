---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-0]
status: closed
assignee: unassigned
---

# Strip daemon and agent runtime

## Scope

Exclusive ownership of:

- `crates/flow-daemon/`, `crates/flow-client/`, `crates/flow-protocol/`
- `src/driver/mod.rs`
- `src/bin/flow_js_repl.rs`
- `src/app/runtime.rs`
- `src/app/background_work.rs`
- `src/app/streaming.rs`
- The `[workspace]` member list in `Cargo.toml` (remove the three daemon
  crates only; leave dependency versions for ticket
  [cargo-workspace-cleanup](cargo-workspace-cleanup.md)).

No other ticket touches these paths. Do not edit files outside this list.

## Goal

Flow has no daemon process, no agent runtime, no RPC protocol, and no
background coding-agent work queue. `docs/PRODUCT_REQUIREMENTS.md` Milestone 0
calls for deleting the daemon runtime rather than hiding it behind a flag.

## Definition of done

- `crates/flow-daemon`, `crates/flow-client`, `crates/flow-protocol` are
  deleted and removed from the workspace member list.
- `src/driver/mod.rs` and `src/bin/flow_js_repl.rs` are deleted (js-repl only
  existed to debug the daemon's JS runtime).
- `src/app/runtime.rs`, `background_work.rs`, `streaming.rs` are deleted.
- The crate still builds after removal, even if other Milestone 0 tickets
  haven't landed yet — stub out or delete the call sites this ticket owns;
  leave a `// TODO(milestone-0):` marker only where a caller lives in a file
  another ticket owns, so that ticket knows what to remove on its side.
- `cargo check` succeeds, or the remaining breakage is fully enumerated in the
  PR description as "owned by ticket X."

## Dependencies

None to start. `cargo-workspace-cleanup` depends on this ticket for its crate
list.
