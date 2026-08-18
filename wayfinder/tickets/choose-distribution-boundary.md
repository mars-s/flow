---
kind: issue
parent: ../flow-map.md
labels: [wayfinder:grilling]
status: closed
assignee: codex
---

# Choose Flow's distribution boundary

## Question

Will Flow retain and modify Waku's GPL-3.0 GPUI shell for the first release, or
must it be a clean-room GPUI application to permit a non-GPL distribution?

The answer determines whether the shell can be stripped in place or must be
rebuilt before product work begins.

## Resolution

Flow will remain GPL-3.0 and open source. It may retain and modify Waku's GPUI
shell, provided the distributed source, license text, and upstream notices are
preserved. The next implementation phase can therefore strip Waku in place
rather than rebuild a clean-room shell.
