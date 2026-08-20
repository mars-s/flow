import { ChevronDown, ChevronRight, CheckCircle2 } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { formatSchedule } from "../lib/date";
import type { Task } from "../lib/types";
import "./CompletedSection.css";

type Props = {
  tasks: Task[];
  open: boolean;
  onToggleOpen: () => void;
  onUncomplete: (id: string) => void;
  onClear: () => void;
};

// Mirrors the GPUI app's own completed_section: a disclosure row pinned at
// the bottom of the list ("Completed (N)"), a scrollable capped-height
// list of completed tasks when open, and a "Clear" button that soft-
// deletes all of them (no undo — same as the GPUI app's own
// clear_completed, PRD's undo guarantee covers single/bulk delete and
// completion, not this bulk clear-of-already-completed-items action).
// Real gap found by re-checking tasks.rs: list_completed was already
// wired into api.ts back when Capture/CRUD first landed but never
// actually called — completed tasks were simply invisible once done.
export function CompletedSection({ tasks, open, onToggleOpen, onUncomplete, onClear }: Props) {
  if (tasks.length === 0) return null;

  return (
    <div className="completed-section">
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            className="completed-rows"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.14 }}
          >
            {tasks.map((task) => (
              <div className="completed-row" key={task.id}>
                <motion.button
                  type="button"
                  className="subtask-checkbox"
                  whileTap={{ scale: 0.85 }}
                  onClick={() => onUncomplete(task.id)}
                  aria-label="Mark not done"
                >
                  <CheckCircle2 size={15} className="subtask-checkbox-icon checked" strokeWidth={2} />
                </motion.button>
                <span className="completed-row-title">{task.title}</span>
                {task.scheduled_date && (
                  <span className="completed-row-date">{formatSchedule(task.scheduled_date, task.scheduled_time)}</span>
                )}
              </div>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
      <div className="completed-toggle-row">
        <button type="button" className="completed-toggle" onClick={onToggleOpen}>
          {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          Completed ({tasks.length})
        </button>
        {open && (
          <button type="button" className="completed-clear" onClick={onClear}>
            Clear
          </button>
        )}
      </div>
    </div>
  );
}

