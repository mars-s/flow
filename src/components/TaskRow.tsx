import { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Tag, ListTree, Flag } from "lucide-react";
import type { Task } from "../lib/types";
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
  onToggleExpanded: () => void;
  onComplete: () => void;
  onNoteChange: (note: string) => void;
};

export function TaskRow({ task, expanded, completing, onToggleExpanded, onComplete, onNoteChange }: Props) {
  const [pressed, setPressed] = useState(false);

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
        <div className="card-header" onClick={onToggleExpanded}>
          {checkbox}
          <div className="card-title">{task.title}</div>
        </div>
        <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.05, duration: 0.14 }}>
          <textarea
            className="card-note"
            placeholder="Notes"
            rows={1}
            value={task.note}
            onChange={(event) => onNoteChange(event.target.value)}
          />
          {task.subtasks.length > 0 && (
            <div className="card-subtasks">
              {task.subtasks.map((subtask) => (
                <div className="subtask-row" key={subtask.id}>
                  <div className={`checkbox small ${subtask.completed ? "checked" : ""}`}>
                    {subtask.completed && <Check />}
                  </div>
                  <span className={subtask.completed ? "subtask-title done" : "subtask-title"}>{subtask.title}</span>
                </div>
              ))}
            </div>
          )}
          <div className="card-pills">
            <motion.div className="pill" whileHover={{ y: -1 }} whileTap={{ scale: 0.96 }}>
              <Tag size={11} />
              {task.scheduledDate ?? "Schedule…"}
            </motion.div>
            <motion.div className="pill" whileHover={{ y: -1 }} whileTap={{ scale: 0.96 }}>
              <ListTree size={11} />
              Move
            </motion.div>
            <motion.div className="pill" whileHover={{ y: -1 }} whileTap={{ scale: 0.96 }}>
              <Flag size={11} />
              Flag
            </motion.div>
          </div>
        </motion.div>
      </motion.div>
    );
  }

  return (
    <motion.div layoutId={`row-${task.id}`} className="row" onClick={onToggleExpanded} whileHover={{ x: 1 }} transition={softSpring}>
      {checkbox}
      <div className={`row-title ${completing ? "checked" : ""}`}>{task.title}</div>
      {task.subtasks.length > 0 && (
        <div className="row-subtask-count">
          {task.subtasks.filter((subtask) => subtask.completed).length}/{task.subtasks.length}
        </div>
      )}
      {task.scheduledDate && <div className="row-schedule">{task.scheduledDate}</div>}
    </motion.div>
  );
}
