# Flow visual direction

## Design read

A native personal productivity desktop app for a keyboard-heavy solo user, with
a quiet editorial workspace and the calm, precise mechanics of Flow and Codex.
It should feel like an instrument for deciding the next action, not a project
dashboard.

**Dials:** variance 4, motion 4, density 7.

## The idea: focus light

Flow is a charcoal work surface interrupted by a single cool focus light. Blue
means an intentional action or current focus. Calendar colors remain calendar
colors, never Flow status colors. This gives the app an identifiable signature
without decorating every task.

The visual hierarchy is deliberately asymmetric:

```text
native titlebar
┌───────────────┬───────────────────────────────────────┬────────────┐
│ Flow          │ Today                                 │ Tue, 18 Aug│
│               │ Good afternoon. Three things matter.  │ 08:00      │
│ + Capture     │                                       │ Laundry    │
│               │ [ Capture a task...              ⌘N ] │            │
│ Inbox       3 │                                       │ 10:00      │
│ Today         │ Now                                   │ Research   │
│ Upcoming      │ ○ Send proposal                       │            │
│ Anytime       │ ○ Take out laundry                    │ 15:30      │
│ Someday       │                                       │ Design sync│
│               │ Later today                           │            │
│ Calendar      │ ○ Bring Mya cake                      │ + Connect  │
│ Settings      │                                       │ calendar   │
└───────────────┴───────────────────────────────────────┴────────────┘
```

The calendar rail is the signature. It is quiet, aligned to task sections, and
can collapse with `Cmd+Shift+C`. It is a glance, never a second application.

## Tokens

Use semantic GPUI theme tokens, not raw colors scattered across components.

| Token | Value | Role |
| --- | --- | --- |
| `canvas` | `#171A1E` | Main workspace and window background |
| `sidebar` | `#1C2025` | Navigation rail |
| `raised` | `#242A31` | Composer and active transient surface |
| `hover` | `#2A3139` | Hovered rows and menus |
| `line` | `#343B44` | Sparse structural dividers |
| `text` | `#EEF1F5` | Primary copy |
| `muted` | `#969FAA` | Metadata and unselected navigation |
| `focus` | `#69A9FF` | Selection, keyboard focus, primary action |
| `focus-soft` | `#234C78` | Selected navigation background |
| `danger` | `#EC6B73` | Destructive action only |

The default is dark mode. A future light theme must swap the entire token set,
not invert individual components.

## Type and spacing

- Use the platform system UI face for all interface text. It keeps Flow native
  on macOS and Linux without adding a font dependency.
- Use the existing monospaced UI face only for shortcut hints, local times, and
  machine-like metadata.
- Titles: 26 px, semibold, tight tracking. Section labels: 12 px, semibold,
  muted. Task titles: 15 px, medium. Supporting copy: 13 px.
- Base spacing is 4 px. Use 8, 12, 16, 24, and 32 px steps. Do not use large
  rounded cards to create hierarchy.
- Corner rule: surfaces use 10 px radius, small controls use 7 px, circular
  completion controls remain circular.

## Core components

### Navigation rail

Width: 252 px, fixed. The wordmark is text, not a decorative logo. A compact
Capture button sits below the titlebar. Destinations are icon plus label, with
Inbox's count in a small rounded pill aligned right. The active row is filled
with `focus-soft` and primary text; no separate border or focus bar. Hover
only changes the row surface.

Navigation is grouped by meaning, with one quiet divider before Calendar and
Settings. Icons are monochrome and uncolored per item; the Inbox count pill is
the one functional exception, not decoration.

### Main task canvas

Maximum readable width: 780 px. Content is left aligned with a wide breathing
margin rather than centered in the full window. The page title is paired with a
short factual context line, not marketing copy.

The Capture field is the primary affordance. It is a raised surface with a
visible text cursor and a concise command hint. When the parser recognizes a
time phrase, it converts it into an inline blue date chip that can be changed
or removed before save.

Tasks live in sections such as Now, Later today, and Tomorrow. Sections use
space and a single baseline rule, not enclosing cards. A task row is 40 px
high, and carries only completion, title, relevant schedule metadata, and
subtask progress. Metadata fades until hover or keyboard focus.

### Completion control

The completion control is a 17 px outlined circle. Hover fills it with a thin
blue ring. Completion uses a blue check and a 180 ms opacity/collapse
transition. The row stays available via Undo. Reduced motion replaces movement
with a 100 ms opacity change.

### Task detail

Opening a task expands it in place below the row. The detail uses a 10 px
raised surface, not a modal. It exposes note, subtasks, schedule, move, and
delete in that order. Subtasks use one indentation level and a slender left
guide that ends at the last child.

### Calendar rail

Width: 236 px. The rail has a readable date heading and a vertical time
sequence. Events use the provider color as a 2 px rule only; the event title
remains neutral. All-day events appear above the time sequence. Empty time is
left empty. This is a context pane, not an appointment editor.

## Motion

- Main-view change: 140 ms crossfade with 6 px horizontal offset.
- Capture field: focus border and soft blue internal highlight over 120 ms.
- Task completion: 180-220 ms opacity and vertical collapse.
- Task detail: 160 ms opacity and scale from 0.985 to 1.0.
- Sidebar selection: 120 ms background-color transition.
- No perpetual motion, parallax, magnetic effects, or animated gradients.
- `prefers-reduced-motion` removes transforms and limits transitions to 100 ms.

## Required UI states

| Surface | Empty | Loading | Error |
| --- | --- | --- | --- |
| Inbox | "Nothing to process. Capture the next thing." and Capture action | Three task-row skeletons | Inline retry without losing draft |
| Today | "Your day is clear." and Capture action | Task rows preserve their layout | Non-blocking task sync banner |
| Calendar rail | "Connect a calendar for a quick glance." | Event-shaped skeletons | "Calendar unavailable" with Retry |
| Task parser | No chip | Inline parsing indicator only if over 150 ms | Preserve exact text and offer date picker |

## Keyboard and focus

- `Cmd+N`: capture task.
- `Cmd+K`: command palette.
- `Cmd+1` through `Cmd+5`: Inbox, Today, Upcoming, Anytime, Someday.
- `Cmd+Shift+C`: toggle calendar rail.
- `Enter`: complete focused task.
- `Space`: expand or collapse focused task detail.
- `Esc`: return focus to the task list or dismiss an empty capture field.

Keyboard focus must use the same blue focus light as selection, with a visible
2 px outline that does not rely on color alone.

## Implementation boundaries

- Reuse Flow only for GPUI window lifecycle, window chrome, focus handling,
  command infrastructure, and theme primitives.
- Delete, do not visually hide, Flow agent, session, daemon, terminal, Git,
  webview, and usage-product surfaces during the shell milestone.
- Do not add a web component library, a CSS framework, or custom icons. Use the
  existing native icon infrastructure and platform rendering.
- Do not build a visual chart, a calendar grid, or a dense dashboard for v1.

## Visual acceptance checks

- The main pane reads as a task list within one second at 1440 px wide.
- Today communicates tasks before calendar events at every window size.
- Only focus blue represents Flow state; event colors represent calendar data.
- A keyboard-only user can see focus, capture, complete, expand, and move a
  task without pointer-only controls.
- The UI still reads clearly with all animations disabled.
