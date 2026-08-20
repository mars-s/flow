import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import { splitHighlight, useNlpPreview } from "../lib/nlpPreview";
import "./CaptureField.css";

type Props = {
  open: boolean;
  onSubmit: (title: string) => void;
  onClose: () => void;
};

export function CaptureField({ open, onSubmit, onClose }: Props) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [value, setValue] = useState("");
  const { highlight, preview } = useNlpPreview(value);

  useEffect(() => {
    if (open) inputRef.current?.focus();
    else setValue("");
  }, [open]);

  if (!open) return null;

  const { before, matched, after } = splitHighlight(value, highlight);

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
            if (event.key === "Escape") {
              event.stopPropagation();
              onClose();
            }
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
