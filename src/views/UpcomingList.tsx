import { AnimatePresence, motion } from "framer-motion";
import { TaskRow } from "../components/TaskRow";
import type { Task } from "../lib/types";
import "./TaskList.css";
import "./UpcomingList.css";

const spring = { type: "spring" as const, stiffness: 520, damping: 34, mass: 0.7 };

type Props = {
  tasks: Task[];
  expanded: string | null;
  completing: Set<string>;
  onToggleExpanded: (id: string) => void;
  onComplete: (id: string) => void;
  onNoteChange: (id: string, note: string) => void;
};

function groupByDate(tasks: Task[]): [string, Task[]][] {
  const groups = new Map<string, Task[]>();
  for (const task of tasks) {
    const key = task.scheduledDate ?? "Later";
    const bucket = groups.get(key) ?? [];
    bucket.push(task);
    groups.set(key, bucket);
  }
  return [...groups.entries()];
}

export function UpcomingList({ tasks, expanded, completing, onToggleExpanded, onComplete, onNoteChange }: Props) {
  const groups = groupByDate(tasks);

  return (
    <div className="task-list-view">
      <div className="view-header">
        <h1>Upcoming</h1>
      </div>
      {groups.length === 0 ? (
        <div className="empty-state">Nothing scheduled yet.</div>
      ) : (
        <div className="list upcoming-list">
          {groups.map(([date, group]) => (
            <div className="upcoming-group" key={date}>
              <div className="upcoming-group-label">{date}</div>
              <AnimatePresence initial={false}>
                {group.map((task) => {
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
          ))}
        </div>
      )}
    </div>
  );
}
