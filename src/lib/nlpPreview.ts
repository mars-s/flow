import { useEffect, useState } from "react";
import { api } from "./api";

export type NlpPreview = { date: string | null; time: string | null };
export type NlpHighlight = { start: number; end: number };

// Debounced live parse against flow_data::parse::parse (via preview_
// capture) — shared by Capture's own field and by renaming a task, which
// should "look out for the nlp input" the same way (direct user request).
// Not worth round-tripping on every keystroke of a fast typist even
// though the parse itself is cheap (a regex match, no I/O).
export function useNlpPreview(value: string): { highlight: NlpHighlight | null; preview: NlpPreview | null } {
  const [highlight, setHighlight] = useState<NlpHighlight | null>(null);
  const [preview, setPreview] = useState<NlpPreview | null>(null);

  useEffect(() => {
    if (!value.trim()) {
      setHighlight(null);
      setPreview(null);
      return;
    }
    const timeout = setTimeout(() => {
      api
        .previewCapture(value)
        .then((result) => {
          setPreview(result.date || result.time ? { date: result.date, time: result.time } : null);
          setHighlight(
            result.highlight_start !== null && result.highlight_end !== null
              ? { start: result.highlight_start, end: result.highlight_end }
              : null,
          );
        })
        .catch(() => {
          setPreview(null);
          setHighlight(null);
        });
    }, 120);
    return () => clearTimeout(timeout);
  }, [value]);

  return { highlight, preview };
}

// UTF-16 code-unit slicing, matching the Rust side's own offsets (JS
// strings are UTF-16 under the hood, so plain slice() already speaks the
// same units preview_capture converts to).
export function splitHighlight(value: string, highlight: NlpHighlight | null) {
  const before = highlight ? value.slice(0, highlight.start) : value;
  const matched = highlight ? value.slice(highlight.start, highlight.end) : "";
  const after = highlight ? value.slice(highlight.end) : "";
  return { before, matched, after };
}

// The title with its recognized date/time phrase removed — same
// trim-and-drop shape flow_data::parse::parse itself does, so renaming
// "call mom tomorrow" behaves like capturing it: the phrase disappears
// from the title and drives the reschedule instead.
export function stripHighlight(value: string, highlight: NlpHighlight | null): string {
  if (!highlight) return value.trim();
  const { before, after } = splitHighlight(value, highlight);
  return `${before}${after}`.replace(/\s+/g, " ").trim();
}
