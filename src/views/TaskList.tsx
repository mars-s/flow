import { AnimatePresence, motion } from "framer-motion";
import { TaskRow } from "../components/TaskRow";
import type { Task } from "../lib/types";
import "./TaskList.css";

const spring = { type: "spring" as const, stiffness: 520, damping: 34, mass: 0.7 };

type Props = {
  title: string;
  tasks: Task[];
  expanded: string | null;
  completing: Set<string>;
  onToggleExpanded: (id: string) => void;
  onComplete: (id: string) => void;
  onNoteChange: (id: string, note: string) => void;
  emptyLabel: string;
};

export function TaskList({ title, tasks, expanded, completing, onToggleExpanded, onComplete, onNoteChange, emptyLabel }: Props) {
  return (
    <div className="task-list-view">
      <div className="view-header">
        <h1>{title}</h1>
      </div>
      {tasks.length === 0 ? (
        <div className="empty-state">{emptyLabel}</div>
      ) : (
        <div className="list">
          <AnimatePresence initial={false}>
            {tasks.map((task) => {
              const isCompleting = completing.has(task.id);
              return (
                <motion.div
                  key={task.id}
                  layout
                  initial={{ opacity: 0, y: -4 }}
                  animate={{ opacity: isCompleting ? 0 : 1, y: 0, height: isCompleting ? 0 : "auto" }}
                  exit={{ opacity: 0, height: 0 }}
                  transition={spring}
                  style={{ overflow: "hidden" }}
                >
                  <TaskRow
                    task={task}
                    expanded={expanded === task.id}
                    completing={isCompleting}
                    onToggleExpanded={() => onToggleExpanded(task.id)}
                    onComplete={() => onComplete(task.id)}
                    onNoteChange={(note) => onNoteChange(task.id, note)}
                  />
                </motion.div>
              );
            })}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
}
