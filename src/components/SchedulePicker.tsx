import { useRef, useState } from "react";
import { motion } from "framer-motion";
import { CalendarDays, Inbox, Archive, X } from "lucide-react";
import { api } from "../lib/api";
import { splitHighlight, useNlpPreview } from "../lib/nlpPreview";
import { todayIso } from "../lib/date";
import "./SchedulePicker.css";

type Props = {
  taskId: string;
  onScheduled: () => void;
  onClose: () => void;
};

// The detail card's own Today/Anytime/Someday/Clear quick-picks, plus a
// free-text field that reuses Capture's real NLP parsing (schedule_task_
// from_text) instead of a calendar widget — same "reuse parse.rs on
// whatever the user types" approach the GPUI app's own PRD-driven design
// takes for its equivalent picker. Redesigned (direct user report the
// previous version "was horrible"): the quick-picks are real labeled
// buttons in a row instead of a bare vertical text list, the free-text
// field gets the same live highlight/preview Capture's own field and
// rename now have, and the whole popover is properly anchored/sized
// instead of drifting from `position: absolute` with no top/left.
export function SchedulePicker({ taskId, onScheduled, onClose }: Props) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const { highlight, preview } = useNlpPreview(text);
  const { before, matched, after } = splitHighlight(text, highlight);

  const quickPick = (bucket: "Active" | "Inbox" | "Someday", date: string | null) => {
    api
      .scheduleTask(taskId, bucket, date, null)
      .then(onScheduled)
      .catch((err) => setError(String(err)));
  };

  const submitText = () => {
    const value = text.trim();
    if (!value) return;
    api
      .scheduleTaskFromText(taskId, value)
      .then(onScheduled)
      .catch((err) => setError(String(err)));
  };

  return (
    <motion.div
      className="schedule-picker"
      initial={{ opacity: 0, y: -4, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: -4, scale: 0.98 }}
      transition={{ type: "spring", stiffness: 500, damping: 36 }}
      onClick={(event) => event.stopPropagation()}
    >
      {/* onMouseDown preventDefault on every button here — the free-text
          input below has autoFocus, and its own onBlur closes the whole
          picker; without this, clicking any of these buttons blurs the
          input (closing/unmounting the picker) before the click's own
          onClick ever gets to fire, so nothing happens. Real bug, not
          hypothetical — this is most of why the picker "felt horrible." */}
      <div className="schedule-picker-quick">
        <button
          type="button"
          className="schedule-picker-quick-btn"
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => quickPick("Active", todayIso())}
        >
          <CalendarDays size={13} />
          Today
        </button>
        <button
          type="button"
          className="schedule-picker-quick-btn"
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => quickPick("Active", null)}
        >
          <Inbox size={13} />
          Anytime
        </button>
        <button
          type="button"
          className="schedule-picker-quick-btn"
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => quickPick("Someday", null)}
        >
          <Archive size={13} />
          Someday
        </button>
      </div>
      <form
        className="schedule-picker-form"
        onSubmit={(event) => {
          event.preventDefault();
          submitText();
        }}
      >
        <div className="schedule-picker-input-wrap">
          <div className="schedule-picker-highlight-layer" aria-hidden="true">
            {before}
            {matched && <mark>{matched}</mark>}
            {after}
            {"​"}
          </div>
          <input
            ref={inputRef}
            className="schedule-picker-input"
            placeholder="Or type a date, like “next Friday”…"
            autoFocus
            value={text}
            onChange={(event) => setText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.stopPropagation();
                onClose();
              }
            }}
            onBlur={onClose}
          />
        </div>
      </form>
      {preview && (
        <div className="schedule-picker-preview">
          {preview.date}
          {preview.date && preview.time ? " · " : ""}
          {preview.time}
        </div>
      )}
      {error && <div className="schedule-picker-error">{error}</div>}
      <button
        type="button"
        className="schedule-picker-clear"
        onMouseDown={(event) => event.preventDefault()}
        onClick={() => quickPick("Inbox", null)}
      >
        <X size={12} />
        Clear schedule
      </button>
    </motion.div>
  );
}
