import { useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Tag, Plus, Trash2, Circle, CheckCircle2 } from "lucide-react";
import type { Task } from "../lib/types";
import { linkify } from "../lib/linkify";
import { formatSchedule } from "../lib/date";
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
  onRename: (id: string, title: string) => void;
  onNoteChange: (note: string) => void;
  onAddSubtask: (title: string) => void;
  onToggleSubtask: (id: string, completed: boolean) => void;
  onDelete: () => void;
  onScheduled: () => void;
  selected: boolean;
  onToggleSelected: () => void;
};

// A subtask row with its own click-to-edit title, same view/edit toggle
// shape the parent task's note field already uses — direct user report
// that there was no way to rename a subtask once created.
function SubtaskRow({
  subtask,
  onToggle,
  onRename,
}: {
  subtask: Task;
  onToggle: () => void;
  onRename: (title: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  return (
    <div className="subtask-row">
      <motion.button type="button" className="subtask-checkbox" whileTap={{ scale: 0.85 }} onClick={onToggle}>
        {subtask.completed_at ? (
          <CheckCircle2 size={15} className="subtask-checkbox-icon checked" strokeWidth={2} />
        ) : (
          <Circle size={15} className="subtask-checkbox-icon" strokeWidth={1.6} />
        )}
      </motion.button>
      {editing ? (
        <input
          ref={inputRef}
          className="subtask-title-input"
          defaultValue={subtask.title}
          autoFocus
          onFocus={(event) => event.currentTarget.select()}
          onBlur={(event) => {
            const value = event.target.value.trim();
            if (value && value !== subtask.title) onRename(value);
            setEditing(false);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.currentTarget.blur();
            } else if (event.key === "Escape") {
              event.stopPropagation();
              event.currentTarget.value = subtask.title;
              event.currentTarget.blur();
            }
          }}
        />
      ) : (
        <span
          className={subtask.completed_at ? "subtask-title done" : "subtask-title"}
          tabIndex={0}
          role="button"
          onClick={() => setEditing(true)}
          onKeyDown={(event) => {
            if (event.key === "Enter") setEditing(true);
          }}
        >
          {subtask.title}
        </span>
      )}
    </div>
  );
}

export function TaskRow({
  task,
  expanded,
  completing,
  subtasks,
  onToggleExpanded,
  onComplete,
  onRename,
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
  const [editingTitle, setEditingTitle] = useState(false);
  const subtaskInputRef = useRef<HTMLInputElement>(null);
  const noteRef = useRef<HTMLTextAreaElement>(null);
  const titleRef = useRef<HTMLInputElement>(null);

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
          onClick={editingTitle ? undefined : onToggleExpanded}
          tabIndex={editingTitle ? undefined : 0}
          role={editingTitle ? undefined : "button"}
          onKeyDown={(event) => {
            if (event.key === "Enter") onToggleExpanded();
          }}
        >
          {checkbox}
          {editingTitle ? (
            <input
              ref={titleRef}
              className="card-title-input"
              defaultValue={task.title}
              autoFocus
              onClick={(event) => event.stopPropagation()}
              onFocus={(event) => event.currentTarget.select()}
              onBlur={(event) => {
                const value = event.target.value.trim();
                if (value && value !== task.title) onRename(task.id, value);
                setEditingTitle(false);
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.currentTarget.blur();
                } else if (event.key === "Escape") {
                  event.stopPropagation();
                  event.currentTarget.value = task.title;
                  event.currentTarget.blur();
                }
              }}
            />
          ) : (
            // click-to-edit, same pattern the note field uses — direct
            // user report that there was no way to rename a task at all.
            // stopPropagation so entering edit mode doesn't also collapse
            // the card via card-header's own onToggleExpanded.
            <div
              className="card-title"
              onClick={(event) => {
                event.stopPropagation();
                setEditingTitle(true);
              }}
            >
              {linkify(task.title)}
            </div>
          )}
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

          {/* Things 3-style checklist: no section header/count label (the
              "+Subtask" pill's own label already carries the count when
              closed), no left-border rail — just checkbox+text rows sitting
              directly under the note, as flat and undecorated as the rest
              of the card. */}
          {subtasksOpen && (
            <div className="card-subtasks">
              {subtasks.map((subtask) => (
                <SubtaskRow
                  key={subtask.id}
                  subtask={subtask}
                  onToggle={() => onToggleSubtask(subtask.id, !subtask.completed_at)}
                  onRename={(title) => onRename(subtask.id, title)}
                />
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
                  placeholder="New Checklist Item"
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
                {task.scheduled_date ? formatSchedule(task.scheduled_date, task.scheduled_time) : "Schedule…"}
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
              {subtasks.length > 0 ? `Checklist (${subtasks.length})` : "Checklist"}
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
      {task.scheduled_date && (
        <div className="row-schedule">{formatSchedule(task.scheduled_date, task.scheduled_time)}</div>
      )}
    </motion.div>
  );
}
