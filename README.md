# Flow

Flow is a calm, keyboard-first personal task manager that keeps your tasks
on your machine.

Flow gives every open task one clear home — Inbox, Today, Upcoming, Anytime,
or Someday — and turns natural time language ("take out laundry 8 am
tomorrow") into an explicit, editable schedule. A read-only glance at your
Mac's own Calendar app sits alongside your tasks for context; Flow never
creates or edits calendar events.

## Highlights

- Capture a task in under two seconds and place it deliberately later.
- Natural-language dates and times, parsed locally and always reviewable
  before you save.
- Nested subtasks for small multi-step work.
- A compact, privacy-respecting calendar glance next to your tasks.
- Store app state locally, with no account required to get started.

## This repository today

Flow's original implementation was a native Rust/GPUI desktop app, built
here. That app has since been **archived** (2026-08-20) in favor of a Tauri
(React/TypeScript) rewrite that reached functional parity with it — see
`AGENTS.md`/`CLAUDE.md` for the full picture. The archived GPUI source is
preserved at the git tag `gpui-app-archived-2026-08-20`; nothing is deleted,
only moved out of the working tree.

**Active app development now happens in the separate `flow-tauri-prototype`
repository.** This repository still hosts `crates/flow-data` and
`crates/flow-core`, the shared local task store the Tauri app depends on
directly, plus the durable product/design docs and planning history.

## License

Flow is a fork of [Waku](https://github.com/egoist/waku) and is licensed
under the [GNU General Public License v3.0 only](LICENSE).
