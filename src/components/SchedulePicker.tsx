import { useRef, useState } from "react";
import { motion } from "framer-motion";
import { api } from "../lib/api";
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
// takes for its equivalent picker.
export function SchedulePicker({ taskId, onScheduled, onClose }: Props) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

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
      <form
        onSubmit={(event) => {
          event.preventDefault();
          submitText();
        }}
      >
        <input
          ref={inputRef}
          className="schedule-picker-input"
          placeholder="Schedule…"
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
      </form>
      {error && <div className="schedule-picker-error">{error}</div>}
      <div className="schedule-picker-quick">
        <button type="button" onClick={() => quickPick("Active", todayIso())}>
          Today
        </button>
        <button type="button" onClick={() => quickPick("Active", null)}>
          Anytime
        </button>
        <button type="button" onClick={() => quickPick("Someday", null)}>
          Someday
        </button>
        <button type="button" className="schedule-picker-clear" onClick={() => quickPick("Inbox", null)}>
          Clear
        </button>
      </div>
    </motion.div>
  );
}
