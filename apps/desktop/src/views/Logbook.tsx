import { motion } from "framer-motion";
import { RotateCcw, SquareCheckBig, Tag as TagIcon } from "lucide-react";
import type { Project, Task, TaskTag } from "../lib/types";
import "./TaskList.css";
import "./Logbook.css";

function completionDay(timestamp: string): string {
  const date = new Date(timestamp);
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return "Today";
  if (date.toDateString() === yesterday.toDateString()) return "Yesterday";
  return date.toLocaleDateString(undefined, { weekday: "long", month: "short", day: "numeric" });
}

export function Logbook({
  tasks,
  taskTags,
  projects,
  taskProjects,
  onReopen,
}: {
  tasks: Task[];
  taskTags: Record<string, TaskTag[]>;
  projects: Project[];
  taskProjects: Record<string, string>;
  onReopen: (id: string) => void;
}) {
  const groups = new Map<string, Task[]>();
  for (const task of tasks) {
    if (!task.completed_at) continue;
    const label = completionDay(task.completed_at);
    const group = groups.get(label) ?? [];
    group.push(task);
    groups.set(label, group);
  }

  return (
    <div className="task-list-view logbook-view">
      <div className="view-header">
        <h1>Logbook</h1>
      </div>
      {groups.size === 0 ? (
        <div className="empty-state">Completed tasks will collect here.</div>
      ) : (
        <div className="logbook-list">
          {[...groups.entries()].map(([label, group]) => (
            <section className="logbook-group" key={label}>
              <div className="logbook-group-heading">
                <span>{label}</span>
                <span>{group.length}</span>
              </div>
              {group.map((task) => {
                const project = projects.find((item) => item.id === taskProjects[task.id]);
                return (
                  <motion.div className="logbook-row" layout key={task.id}>
                    <SquareCheckBig size={15} className="logbook-check" />
                    <div className="logbook-row-body">
                      <div className="logbook-row-title">{task.title}</div>
                      <div className="logbook-row-meta">
                        {project && <span>{project.title}</span>}
                        {(taskTags[task.id] ?? []).map((tag) => (
                          <span className="logbook-tag" key={tag.tag_id}>
                            <TagIcon size={9} />
                            {tag.name}
                          </span>
                        ))}
                        {task.completed_at && (
                          <span>
                            {new Date(task.completed_at).toLocaleTimeString(undefined, {
                              hour: "numeric",
                              minute: "2-digit",
                            })}
                          </span>
                        )}
                      </div>
                    </div>
                    <button type="button" className="logbook-reopen" onClick={() => onReopen(task.id)}>
                      <RotateCcw size={12} />
                      Reopen
                    </button>
                  </motion.div>
                );
              })}
            </section>
          ))}
        </div>
      )}
    </div>
  );
}
