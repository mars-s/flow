import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import { api } from "../lib/api";
import "./CaptureField.css";

type Props = {
  open: boolean;
  onSubmit: (title: string) => void;
  onClose: () => void;
};

export function CaptureField({ open, onSubmit, onClose }: Props) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [value, setValue] = useState("");
  const [highlight, setHighlight] = useState<{ start: number; end: number } | null>(null);
  const [preview, setPreview] = useState<{ date: string | null; time: string | null } | null>(null);

  useEffect(() => {
    if (open) inputRef.current?.focus();
    else {
      setValue("");
      setHighlight(null);
      setPreview(null);
    }
  }, [open]);

  // Debounced live parse — flow_data::parse::parse is a pure regex match
  // against a short string (no I/O), but still not worth round-tripping on
  // every single keystroke of a fast typist.
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

  if (!open) return null;

  // UTF-16 code-unit slicing, matching the Rust side's own offsets
  // (JS strings are UTF-16 under the hood, so plain slice() already speaks
  // the same units preview_capture converts to — no further conversion
  // needed here).
  const before = highlight ? value.slice(0, highlight.start) : value;
  const matched = highlight ? value.slice(highlight.start, highlight.end) : "";
  const after = highlight ? value.slice(highlight.end) : "";

  return (
    <motion.form
      className="capture-field"
      initial={{ opacity: 0, y: -6 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -6 }}
      transition={{ type: "spring", stiffness: 500, damping: 36 }}
      onSubmit={(event) => {
        event.preventDefault();
        const title = value.trim();
        if (title) onSubmit(title);
      }}
    >
      <div className="capture-input-wrap">
        <div className="capture-highlight-layer" aria-hidden="true">
          {before}
          {matched && <mark>{matched}</mark>}
          {after}
          {/* A trailing space keeps the layer's box from collapsing
              shorter than the input on an empty/short value, which would
              otherwise let the two boxes' heights disagree by a hair. */}
          {"​"}
        </div>
        <input
          ref={inputRef}
          className="capture-input"
          placeholder="Take out laundry 8am tomorrow…"
          value={value}
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape") onClose();
          }}
          onBlur={onClose}
        />
      </div>
      {preview && (
        <div className="capture-preview">
          {preview.date}
          {preview.date && preview.time ? " · " : ""}
          {preview.time}
        </div>
      )}
    </motion.form>
  );
}
