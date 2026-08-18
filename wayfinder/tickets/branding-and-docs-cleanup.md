---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:milestone-0]
status: closed
assignee: unassigned
---

# Rebrand product-facing docs, locales, and assets

## Scope

Exclusive ownership of:

- `locales/`
- `resources/`
- `assets/`
- `website/`
- `README.md`, `CHANGELOG.md`, `RELEASING.md`, `AGENTS.md`
- App/window title strings and icon references outside `src/app/
  window_chrome.rs` (coordinate with `rebuild-shell-frame` for that file
  itself)

No other ticket touches these paths. Do not edit files outside this list.

## Goal

Nothing user-visible should read "Flow," describe coding-agent workflows, or
reference terminal/Git/provider features once Milestone 0 ships. This is
branding and documentation, not code deletion — the GPL-3.0 license text and
required upstream notices must be preserved per `docs/PRODUCT_REQUIREMENTS.md`
section 9 and the acceptance criteria in section 11.

## Definition of done

- `README.md` describes Flow (personal task manager) rather than Flow (coding
  agent control plane); it keeps accurate build/run instructions for macOS
  and Linux.
- `CHANGELOG.md` gets a new top entry marking the Flow fork point rather than
  being rewritten wholesale.
- `locales/` strings that reference sessions, agents, providers, git, or
  terminal are removed or replaced with Flow equivalents (task, inbox,
  schedule, etc.) — do not translate ahead of need; only touch what Milestone
  0's UI actually surfaces.
- `resources/` and `assets/` icons/images that were Flow-specific (coding
  provider logos, terminal icons) are removed; app icon is replaced or
  clearly marked placeholder if a final Flow icon doesn't exist yet.
- `website/` either becomes a minimal Flow placeholder or is removed if it has
  no Milestone 0 purpose — don't invest in marketing copy this early.
- `LICENSE` and any `NOTICE`/upstream-attribution files are untouched and
  still present and correct.
- `rg -i 'flow|coding agent|daemon' README.md CHANGELOG.md locales resources
  website` returns nothing user-facing (license/attribution mentions of
  "Flow" as the upstream project name are fine and expected).

## Dependencies

None. Can run in parallel with every other Milestone 0 ticket.
