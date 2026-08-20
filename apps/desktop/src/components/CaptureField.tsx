import { useEffect, useMemo, useRef, useState } from "react";
import { motion } from "framer-motion";
import { Search } from "lucide-react";
import { splitHighlight, useNlpPreview } from "../lib/nlpPreview";
import { findDuplicate } from "../lib/similarity";
import { useAiConfig, useAiFeatureState } from "../lib/aiConfig";
import type { Task } from "../lib/types";
import "./CaptureField.css";

type Props = {
  open: boolean;
  onSubmit: (title: string) => void;
  onClose: () => void;
  existingTasks: Task[];
};

export function CaptureField({ open, onSubmit, onClose, existingTasks }: Props) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [value, setValue] = useState("");
  const { highlight, preview } = useNlpPreview(value);

  // Fourth block, but the one local check that isn't a model call — see
  // lib/similarity.ts. Auto mode checks live as the title is typed;
  // Manual mode only checks when the user asks, via the search-icon
  // button below.
  const { enabled } = useAiConfig();
  const [dupeMode] = useAiFeatureState("duplicate-detection");
  const [manualChecked, setManualChecked] = useState(false);
  const liveDuplicate = useMemo(
    () => (enabled && dupeMode === "auto" ? findDuplicate(value, existingTasks) : null),
    [enabled, dupeMode, value, existingTasks],
  );
  const manualDuplicate = useMemo(
    () => (manualChecked ? findDuplicate(value, existingTasks) : null),
    [manualChecked, value, existingTasks],
  );
  const duplicate = dupeMode === "auto" ? liveDuplicate : manualDuplicate;

  useEffect(() => {
    if (open) inputRef.current?.focus();
    else setValue("");
    setManualChecked(false);
  }, [open]);

  // Re-arm the manual check whenever the title changes so a stale
  // "duplicate" result from a previous title doesn't linger silently.
  useEffect(() => {
    setManualChecked(false);
  }, [value]);

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
        {enabled && dupeMode === "manual" && value.trim().length > 0 && (
          <button
            type="button"
            className="capture-dupe-check"
            title="Check for duplicate tasks"
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => setManualChecked(true)}
          >
            <Search size={13} />
          </button>
        )}
      </div>
      {preview && (
        <div className="capture-preview">
          {preview.date}
          {preview.date && preview.time ? " · " : ""}
          {preview.time}
        </div>
      )}
      {duplicate && (
        <div className="capture-duplicate-warning">Similar to "{duplicate.title}" — already on your list</div>
      )}
      {manualChecked && !manualDuplicate && (
        <div className="capture-duplicate-clear">No similar tasks found</div>
      )}
    </motion.form>
  );
}
