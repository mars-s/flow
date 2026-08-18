# Flow

Flow is a calm, keyboard-first personal task manager. It is built in Rust
with [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) and
keeps your tasks on your machine.

Flow gives every open task one clear home — Inbox, Today, Upcoming, Anytime,
or Someday — and turns natural time language ("take out laundry 8 am
tomorrow") into an explicit, editable schedule. A read-only Google Calendar
glance sits alongside your tasks for context; Flow never creates or edits
calendar events.

## Highlights

- Capture a task in under two seconds and place it deliberately later.
- Natural-language dates and times, parsed locally and always reviewable
  before you save.
- Nested subtasks for small multi-step work.
- A compact, privacy-respecting calendar glance next to your tasks.
- Store app state locally, with no account required to get started.

## Development

Development is supported on macOS and Linux and requires
[Rust 1.96 or newer](https://www.rust-lang.org/tools/install) and
[Bun](https://bun.sh/). Linux supports both Wayland and X11; install the native
build prerequisites listed in [CONTRIBUTING.md](CONTRIBUTING.md) first.

```sh
bun install
bun run dev
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow and checks.
Release maintainers should also read [RELEASING.md](RELEASING.md).

## License

Flow is a fork of [Waku](https://github.com/egoist/waku) and is licensed
under the [GNU General Public License v3.0 only](LICENSE).
