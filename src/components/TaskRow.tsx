import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Tag, Trash2, Square, SquareCheckBig, FileText, ListChecks } from "lucide-react";
import type { SubtaskCount, Task } from "../lib/types";
import { linkify } from "../lib/linkify";
import { formatSchedule } from "../lib/date";
import { splitHighlight, stripHighlight, useNlpPreview } from "../lib/nlpPreview";
import { SchedulePicker } from "./SchedulePicker";
import { ChecklistExpansion } from "./ChecklistExpansion";
import { DraftFromTask } from "./DraftFromTask";
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
  subtaskCount?: SubtaskCount;
  pendingCompleteConfirm: boolean;
  onConfirmComplete: () => void;
  onCancelCompleteConfirm: () => void;
  onToggleExpanded: () => void;
  onComplete: () => void;
  onRename: (id: string, title: string) => void;
  onReschedule: (id: string, date: string, time: string | null) => void;
  onNoteChange: (note: string) => void;
  onAddSubtask: (title: string) => void;
  onToggleSubtask: (id: string, completed: boolean) => void;
  onDeleteSubtask: (id: string) => void;
  onDelete: () => void;
  onScheduled: () => void;
  selected: boolean;
  onToggleSelected: () => void;
};

// A checklist row with its own click-to-edit title. Enter both commits a
// rename (if the text changed) and tells the parent to open a fresh
// draft row right after it — chaining "type, Enter, type, Enter..." is
// the only way to grow the list past the first item (direct user
// request: no standing "add" affordance once there's already at least
// one subtask). Backspace on an empty title deletes the row entirely and
// hands focus back — same as Notion/Things's own block-editor feel.
function SubtaskRow({
  subtask,
  onToggle,
  onRename,
  onEnter,
  onDelete,
}: {
  subtask: Task;
  onToggle: () => void;
  onRename: (title: string) => void;
  onEnter: () => void;
  onDelete: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  return (
    <div className="subtask-row">
      <motion.button type="button" className="subtask-checkbox" whileTap={{ scale: 0.85 }} onClick={onToggle}>
        {subtask.completed_at ? (
          <SquareCheckBig size={15} className="subtask-checkbox-icon checked" strokeWidth={2} />
        ) : (
          <Square size={15} className="subtask-checkbox-icon" strokeWidth={1.6} />
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
              const value = event.currentTarget.value.trim();
              if (value && value !== subtask.title) onRename(value);
              setEditing(false);
              onEnter();
            } else if (event.key === "Backspace" && event.currentTarget.value === "") {
              event.preventDefault();
              onDelete();
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

// The trailing draft row — not yet a real subtask. Enter commits it (via
// onAddSubtask) and clears + refocuses itself for the next one, so the
// chain keeps going until the user stops typing. Empty Enter/Backspace/
// blur closes the draft instead of adding a blank item.
function DraftSubtaskRow({ onAdd, onClose }: { onAdd: (title: string) => void; onClose: () => void }) {
  const inputRef = useRef<HTMLInputElement>(null);

  return (
    <div className="subtask-row">
      <Square size={15} className="subtask-checkbox-icon draft-icon" strokeWidth={1.6} />
      <input
        ref={inputRef}
        className="subtask-title-input"
        placeholder="New checklist item"
        autoFocus
        onKeyDown={(event) => {
          const input = event.currentTarget;
          if (event.key === "Enter") {
            const value = input.value.trim();
            if (value) {
              onAdd(value);
              input.value = "";
              input.focus();
            } else {
              onClose();
            }
          } else if (event.key === "Backspace" && input.value === "") {
            event.preventDefault();
            onClose();
          } else if (event.key === "Escape") {
            event.stopPropagation();
            onClose();
          }
        }}
        onBlur={(event) => {
          const value = event.target.value.trim();
          if (value) onAdd(value);
          onClose();
        }}
      />
    </div>
  );
}

// A rename input that "looks out for the nlp input similar to adding a
// task" (direct user request): the same live highlight + date/time
// preview Capture's own field has. Committing strips the recognized
// phrase out of the title (same shape flow_data::parse::parse itself
// does) and, when a date or time was found, reschedules the task too —
// renaming "call mom tomorrow" behaves like capturing it fresh, not just
// a plain text replace.
function TitleEditInput({
  title,
  onCommit,
  onCancel,
}: {
  title: string;
  onCommit: (title: string, date: string | null, time: string | null) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(title);
  const inputRef = useRef<HTMLInputElement>(null);
  const { highlight, preview } = useNlpPreview(value);
  const { before, matched, after } = splitHighlight(value, highlight);

  useEffect(() => {
    inputRef.current?.select();
  }, []);

  const commit = () => {
    const cleaned = stripHighlight(value, highlight);
    if (cleaned && (cleaned !== title || preview)) {
      onCommit(cleaned, preview?.date ?? null, preview?.time ?? null);
    } else {
      onCancel();
    }
  };

  return (
    <div className="card-title-edit" onClick={(event) => event.stopPropagation()}>
      <div className="card-title-input-wrap">
        <div className="card-title-highlight-layer" aria-hidden="true">
          {before}
          {matched && <mark>{matched}</mark>}
          {after}
          {"​"}
        </div>
        <input
          ref={inputRef}
          className="card-title-input"
          value={value}
          onChange={(event) => setValue(event.target.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.currentTarget.blur();
            } else if (event.key === "Escape") {
              event.stopPropagation();
              onCancel();
            }
          }}
        />
      </div>
      {preview && (
        <div className="card-title-preview">
          {preview.date}
          {preview.date && preview.time ? " · " : ""}
          {preview.time}
        </div>
      )}
    </div>
  );
}

export function TaskRow({
  task,
  expanded,
  completing,
  subtasks,
  subtaskCount,
  pendingCompleteConfirm,
  onConfirmComplete,
  onCancelCompleteConfirm,
  onToggleExpanded,
  onComplete,
  onRename,
  onReschedule,
  onNoteChange,
  onAddSubtask,
  onToggleSubtask,
  onDeleteSubtask,
  onDelete,
  onScheduled,
  selected,
  onToggleSelected,
}: Props) {
  const [pressed, setPressed] = useState(false);
  const [draftOpen, setDraftOpen] = useState(false);
  const [schedulingOpen, setSchedulingOpen] = useState(false);
  const [editingNote, setEditingNote] = useState(false);
  const [editingTitle, setEditingTitle] = useState(false);
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
    const showChecklist = subtasks.length > 0 || draftOpen;
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
            <TitleEditInput
              title={task.title}
              onCancel={() => setEditingTitle(false)}
              onCommit={(title, date, time) => {
                onRename(task.id, title);
                if (date) onReschedule(task.id, date, time);
                setEditingTitle(false);
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

          {/* Things 3-style checklist: a thin horizontal divider under
              each row (a real Things 3 screenshot showed these as row
              borders, not a vertical connector between checkboxes — the
              first attempt at this got that wrong), no section header —
              the "Checklist" pill itself only exists while the list is
              empty, gone the moment a subtask exists (direct user
              request: "gets rid of unnecessary ui"). */}
          {showChecklist && (
            <div className="card-subtasks">
              {subtasks.map((subtask) => (
                <SubtaskRow
                  key={subtask.id}
                  subtask={subtask}
                  onToggle={() => onToggleSubtask(subtask.id, !subtask.completed_at)}
                  onRename={(title) => onRename(subtask.id, title)}
                  onEnter={() => setDraftOpen(true)}
                  onDelete={() => onDeleteSubtask(subtask.id)}
                />
              ))}
              {draftOpen && (
                <DraftSubtaskRow
                  onAdd={(title) => onAddSubtask(title)}
                  onClose={() => setDraftOpen(false)}
                />
              )}
            </div>
          )}

          <ChecklistExpansion
            taskId={task.id}
            title={task.title}
            note={task.note}
            hasSubtasks={subtasks.length > 0}
            onAddSubtask={onAddSubtask}
          />

          <DraftFromTask taskId={task.id} title={task.title} note={task.note} />

          {/* PRD §6.2/§11: "Completing a parent with incomplete children
              asks: 'Complete parent and all subtasks' or 'Cancel.' It
              never leaves a completed parent with open children" — same
              copy the GPUI app's own confirm banner uses. */}
          {pendingCompleteConfirm && (
            <div className="card-complete-confirm">
              <span className="card-complete-confirm-text">Complete parent and all subtasks?</span>
              <div className="card-complete-confirm-actions">
                <button type="button" className="card-complete-confirm-cancel" onClick={onCancelCompleteConfirm}>
                  Cancel
                </button>
                <button type="button" className="card-complete-confirm-yes" onClick={onConfirmComplete}>
                  Complete all
                </button>
              </div>
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
            {subtasks.length === 0 && !draftOpen && (
              <motion.button
                type="button"
                className="pill"
                whileHover={{ y: -1 }}
                whileTap={{ scale: 0.96 }}
                onClick={() => setDraftOpen(true)}
              >
                <Square size={11} />
                Checklist
              </motion.button>
            )}
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
        // stopPropagation so this doesn't also bubble to the app root's
        // own "click elsewhere collapses the expanded task" handler,
        // which would otherwise immediately undo the expand this click
        // just triggered (both set the same `expanded` state; without
        // this the root's handler runs right after and wins).
        event.stopPropagation();
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
      {/* Direct user request, matching Things 3's own row indicators: a
          note icon when the task has one, a checklist icon + open/total
          count when it has subtasks — both readable without opening the
          card. task.note is already on every row (no extra fetch); the
          subtask count comes from the one-shot subtask_counts map
          (App.tsx), not a per-row fetch. */}
      <div className="row-indicators">
        {task.note && <FileText size={12} className="row-indicator-icon" />}
        {subtaskCount && subtaskCount.total > 0 && (
          <span className="row-indicator">
            <ListChecks size={12} className="row-indicator-icon" />
            {subtaskCount.total - subtaskCount.open}/{subtaskCount.total}
          </span>
        )}
      </div>
      {task.scheduled_date && (
        <div className="row-schedule">{formatSchedule(task.scheduled_date, task.scheduled_time)}</div>
      )}
    </motion.div>
  );
}
