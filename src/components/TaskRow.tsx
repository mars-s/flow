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
            defaultValue={task.note ?? ""}
            onBlur={(event) => onNoteChange(event.target.value)}
          />
          {/* Subtasks aren't wired yet — flow-data's Task has no embedded
              subtasks field (they're a separate list_subtasks call per
              parent, matching the real GPUI app's own architecture), and
              this prototype doesn't call it yet. Open work, not a design
              decision to drop them. */}
          <div className="card-pills">
            <motion.div className="pill" whileHover={{ y: -1 }} whileTap={{ scale: 0.96 }}>
              <Tag size={11} />
              {task.scheduled_date ?? "Schedule…"}
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
      {task.scheduled_date && <div className="row-schedule">{task.scheduled_date}</div>}
    </motion.div>
  );
}
