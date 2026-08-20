import { useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Tag, Plus, Trash2, Circle, CheckCircle2 } from "lucide-react";
import type { Task } from "../lib/types";
import { linkify } from "../lib/linkify";
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
  const [subtasksOpen, setSubtasksOpen] = useState(false);
  const [schedulingOpen, setSchedulingOpen] = useState(false);
  const [editingNote, setEditingNote] = useState(false);
  const subtaskInputRef = useRef<HTMLInputElement>(null);
  const noteRef = useRef<HTMLTextAreaElement>(null);

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
          <div className="card-title">{linkify(task.title)}</div>
        </div>
        <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.05, duration: 0.14 }}>
          {editingNote ? (
            <textarea
              ref={noteRef}
              className="card-note"
              placeholder="Notes"
              rows={1}
              autoFocus
              defaultValue={task.note ?? ""}
              onBlur={(event) => {
                onNoteChange(event.target.value);
                setEditingNote(false);
              }}
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  event.stopPropagation();
                  noteRef.current?.blur();
                }
              }}
            />
          ) : (
            // A rendered view mode, not the textarea itself — a native
            // <textarea> can't render some of its content as a clickable
            // link, so notes containing a URL (PRD-adjacent user request:
            // "links should highlight") show here instead, and clicking
            // into it switches to the real editable textarea above.
            <div
              className={task.note ? "card-note-view" : "card-note-view placeholder"}
              tabIndex={0}
              role="button"
              onClick={() => setEditingNote(true)}
              onKeyDown={(event) => {
                if (event.key === "Enter") setEditingNote(true);
              }}
            >
              {task.note ? linkify(task.note) : "Notes"}
            </div>
          )}

          {subtasksOpen && (
            <div className="card-subtasks">
              {subtasks.length > 0 && (
                <div className="subtasks-header">
                  Subtasks ({subtasks.length - openCount}/{subtasks.length})
                </div>
              )}
              {subtasks.map((subtask) => (
                <div className="subtask-row" key={subtask.id}>
                  <motion.button
                    type="button"
                    className="subtask-checkbox"
                    whileTap={{ scale: 0.85 }}
                    onClick={() => onToggleSubtask(subtask.id, !subtask.completed_at)}
                  >
                    {subtask.completed_at ? (
                      <CheckCircle2 size={15} className="subtask-checkbox-icon checked" strokeWidth={2} />
                    ) : (
                      <Circle size={15} className="subtask-checkbox-icon" strokeWidth={1.6} />
                    )}
                  </motion.button>
                  <span className={subtask.completed_at ? "subtask-title done" : "subtask-title"}>
                    {subtask.title}
                  </span>
                </div>
              ))}
              {/* A real checklist-entry flow, not a single-shot add form:
                  Enter commits the current line as a subtask and clears +
                  refocuses the same input for the next one, so typing a
                  short checklist is "type, Enter, type, Enter..." without
                  re-opening anything in between. Escape closes the whole
                  section instead of just this row — there's no longer a
                  separate "list" vs "add row" state to fall back to. */}
              <form
                className="subtask-add-row"
                onSubmit={(event) => {
                  event.preventDefault();
                  const input = subtaskInputRef.current;
                  const value = input?.value.trim();
                  if (value) onAddSubtask(value);
                  if (input) {
                    input.value = "";
                    input.focus();
                  }
                }}
              >
                <Circle size={15} className="subtask-checkbox-icon add-row-icon" strokeWidth={1.6} />
                <input
                  ref={subtaskInputRef}
                  className="subtask-add-input"
                  placeholder="New subtask"
                  autoFocus
                  onKeyDown={(event) => {
                    if (event.key === "Escape") {
                      // stopPropagation so this doesn't also bubble to the
                      // app root's own Escape handler (which collapses
                      // the expanded task entirely).
                      event.stopPropagation();
                      setSubtasksOpen(false);
                    }
                  }}
                />
              </form>
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
              onClick={() => setSubtasksOpen((open) => !open)}
            >
              <Plus size={11} />
              {subtasks.length > 0 ? `Subtasks (${subtasks.length})` : "Subtask"}
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
      <div className={`row-title ${completing ? "checked" : ""}`}>{linkify(task.title)}</div>
      {task.scheduled_date && <div className="row-schedule">{task.scheduled_date}</div>}
    </motion.div>
  );
}
