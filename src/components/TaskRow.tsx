import { useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Tag, Plus, Trash2 } from "lucide-react";
import type { Task } from "../lib/types";
import { SchedulePicker } from "./SchedulePicker";
import "./TaskRow.css";

const spring = { type: "spring" as const, stiffness: 520, damping: 34, mass: 0.7 };
const softSpring = { type: "spring" as const, stiffness: 420, damping: 32 };

function Check() {
  return (
    <svg className="check-icon" viewBox="0 0 12 12" fill="none">
      <path d="M2 6.2L4.7 9L10 3" stroke="var(--accent)" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

type Props = {
  task: Task;
  expanded: boolean;
  completing: boolean;
  subtasks: Task[];
  onToggleExpanded: () => void;
  onComplete: () => void;
  onNoteChange: (note: string) => void;
  onAddSubtask: (title: string) => void;
  onToggleSubtask: (id: string, completed: boolean) => void;
  onDelete: () => void;
  onScheduled: () => void;
  selected: boolean;
  onToggleSelected: () => void;
};

export function TaskRow({
  task,
  expanded,
  completing,
  subtasks,
  onToggleExpanded,
  onComplete,
  onNoteChange,
  onAddSubtask,
  onToggleSubtask,
  onDelete,
  onScheduled,
  selected,
  onToggleSelected,
}: Props) {
  const [pressed, setPressed] = useState(false);
  const [addingSubtask, setAddingSubtask] = useState(false);
  const [schedulingOpen, setSchedulingOpen] = useState(false);
  const subtaskInputRef = useRef<HTMLInputElement>(null);

  const checkbox = (
    <motion.div
      className={`checkbox ${completing ? "checked" : ""}`}
      animate={pressed ? { scale: 0.82 } : completing ? { scale: [1, 1.15, 1] } : { scale: 1 }}
      transition={pressed ? { duration: 0.08 } : { duration: 0.22 }}
      onMouseDown={() => setPressed(true)}
      onMouseUp={() => setPressed(false)}
      onMouseLeave={() => setPressed(false)}
      onClick={(event) => {
        event.stopPropagation();
        onComplete();
      }}
    >
      <AnimatePresence>
        {completing && (
          <motion.div initial={{ scale: 0, opacity: 0 }} animate={{ scale: 1, opacity: 1 }} transition={{ duration: 0.15 }}>
            <Check />
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );

  if (expanded) {
    const openCount = subtasks.filter((subtask) => !subtask.completed_at).length;
    return (
      <motion.div
        layoutId={`row-${task.id}`}
        className="card"
        initial={{ opacity: 0, scale: 0.96 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={{ opacity: 0, scale: 0.97 }}
        transition={spring}
        onClick={(event) => event.stopPropagation()}
      >
        <div
          className="card-header"
          onClick={onToggleExpanded}
          tabIndex={0}
          role="button"
          onKeyDown={(event) => {
            if (event.key === "Enter") onToggleExpanded();
          }}
        >
          {checkbox}
          <div className="card-title">{task.title}</div>
        </div>
        <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.05, duration: 0.14 }}>
          <textarea
            className="card-note"
            placeholder="Notes"
            rows={1}
            defaultValue={task.note ?? ""}
            onBlur={(event) => onNoteChange(event.target.value)}
          />

          {(subtasks.length > 0 || addingSubtask) && (
            <div className="card-subtasks">
              {subtasks.length > 0 && (
                <div className="subtasks-header">
                  Subtasks ({subtasks.length - openCount}/{subtasks.length})
                </div>
              )}
              {subtasks.map((subtask) => (
                <div className="subtask-row" key={subtask.id}>
                  <motion.div
                    className={`checkbox small ${subtask.completed_at ? "checked" : ""}`}
                    whileTap={{ scale: 0.82 }}
                    onClick={() => onToggleSubtask(subtask.id, !subtask.completed_at)}
                  >
                    {subtask.completed_at && <Check />}
                  </motion.div>
                  <span className={subtask.completed_at ? "subtask-title done" : "subtask-title"}>
                    {subtask.title}
                  </span>
                </div>
              ))}
              {addingSubtask && (
                <form
                  className="subtask-add-row"
                  onSubmit={(event) => {
                    event.preventDefault();
                    const value = subtaskInputRef.current?.value.trim();
                    if (value) onAddSubtask(value);
                    if (subtaskInputRef.current) subtaskInputRef.current.value = "";
                    setAddingSubtask(false);
                  }}
                >
                  <div className="checkbox small" />
                  <input
                    ref={subtaskInputRef}
                    className="subtask-add-input"
                    placeholder="New subtask"
                    autoFocus
                    onBlur={() => setAddingSubtask(false)}
                    onKeyDown={(event) => {
                      if (event.key === "Escape") {
                        // Cancels just the subtask-add row, not the whole
                        // card — stopPropagation so this doesn't also
                        // bubble to the app root's own Escape handler
                        // (which collapses the expanded task entirely).
                        event.stopPropagation();
                        setAddingSubtask(false);
                      }
                    }}
                  />
                </form>
              )}
            </div>
          )}

          <div className="card-pills">
            {/* Real <button>s, not styled <div>s — free keyboard operability
                (native tabIndex, Enter/Space activation) instead of
                hand-rolling onKeyDown on each one. */}
            <div className="pill-anchor">
              <motion.button
                type="button"
                className="pill"
                whileHover={{ y: -1 }}
                whileTap={{ scale: 0.96 }}
                onClick={() => setSchedulingOpen(true)}
              >
                <Tag size={11} />
                {task.scheduled_date ?? "Schedule…"}
              </motion.button>
              <AnimatePresence>
                {schedulingOpen && (
                  <SchedulePicker
                    taskId={task.id}
                    onScheduled={() => {
                      setSchedulingOpen(false);
                      onScheduled();
                    }}
                    onClose={() => setSchedulingOpen(false)}
                  />
                )}
              </AnimatePresence>
            </div>
            <motion.button
              type="button"
              className="pill"
              whileHover={{ y: -1 }}
              whileTap={{ scale: 0.96 }}
              onClick={() => setAddingSubtask(true)}
            >
              <Plus size={11} />
              Subtask
            </motion.button>
            <motion.button
              type="button"
              className="pill danger"
              whileHover={{ y: -1 }}
              whileTap={{ scale: 0.96 }}
              onClick={onDelete}
            >
              <Trash2 size={11} />
              Delete
            </motion.button>
          </div>
        </motion.div>
      </motion.div>
    );
  }

  return (
    <motion.div
      layoutId={`row-${task.id}`}
      className={`row ${selected ? "selected" : ""}`}
      onClick={(event) => {
        // Cmd+click toggles multi-select instead of opening the row — same
        // interaction as the GPUI app's own toggle_selected.
        if (event.metaKey) onToggleSelected();
        else onToggleExpanded();
      }}
      whileHover={{ x: 1 }}
      transition={softSpring}
      // Enter opens the row, Space completes it — no separate tab stop for
      // the checkbox itself, matching the GPUI app's own deliberate choice
      // (its own tasks.rs comment: doubling tab stops across a long list
      // is a real cost, not just a style call). preventDefault on Space
      // stops the page from scrolling on it, the same reason a plain
      // <button> needs it for a bare Space press.
      tabIndex={0}
      role="button"
      aria-pressed={completing}
      onKeyDown={(event) => {
        if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
        if (event.key === "Enter") {
          onToggleExpanded();
        } else if (event.key === " ") {
          event.preventDefault();
          if (!completing) onComplete();
        }
      }}
    >
      {checkbox}
      <div className={`row-title ${completing ? "checked" : ""}`}>{task.title}</div>
      {task.scheduled_date && <div className="row-schedule">{task.scheduled_date}</div>}
    </motion.div>
  );
}
