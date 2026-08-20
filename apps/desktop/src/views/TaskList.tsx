import type { ReactNode } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { TaskRow } from "../components/TaskRow";
import type { Project, SubtaskCount, Tag, Task, TaskTag } from "../lib/types";
import { TaskTagFilter } from "../components/TaskTags";
import "./TaskList.css";

const spring = { type: "spring" as const, stiffness: 520, damping: 34, mass: 0.7 };

type Props = {
  title: string;
  tasks: Task[];
  sections?: { label: string | null; tone?: "danger" | "quiet"; tasks: Task[] }[];
  expanded: string | null;
  completing: Set<string>;
  subtasks: Task[];
  subtaskCounts: Record<string, SubtaskCount>;
  pendingCompleteConfirm: string | null;
  tags: Tag[];
  taskTags: Record<string, TaskTag[]>;
  activeTag: string | null;
  onTagFilterChange: (name: string | null) => void;
  onTaskTagsChange: (taskId: string, names: string[]) => void;
  projects: Project[];
  taskProjects: Record<string, string>;
  onTaskProjectChange: (taskId: string, projectId: string | null) => void;
  onConfirmComplete: (id: string) => void;
  onCancelCompleteConfirm: () => void;
  selectedIds: Set<string>;
  onToggleExpanded: (id: string) => void;
  onComplete: (id: string) => void;
  onRename: (id: string, title: string) => void;
  onReschedule: (id: string, date: string, time: string | null) => void;
  onNoteChange: (id: string, note: string) => void;
  onAddSubtask: (parentId: string, title: string) => void;
  onToggleSubtask: (id: string, completed: boolean) => void;
  onDeleteSubtask: (id: string) => void;
  onDelete: (id: string) => void;
  onScheduled: () => void;
  onToggleSelected: (id: string) => void;
  emptyLabel: string;
  topSlot?: ReactNode;
  headerSlot?: ReactNode;
  showProjectContext?: boolean;
  bottomSlot?: ReactNode;
};

export function TaskList({
  title,
  tasks,
  sections,
  expanded,
  completing,
  subtasks,
  subtaskCounts,
  pendingCompleteConfirm,
  tags,
  taskTags,
  activeTag,
  onTagFilterChange,
  onTaskTagsChange,
  projects,
  taskProjects,
  onTaskProjectChange,
  onConfirmComplete,
  onCancelCompleteConfirm,
  selectedIds,
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
  onToggleSelected,
  emptyLabel,
  topSlot,
  bottomSlot,
  headerSlot,
  showProjectContext = true,
}: Props) {
  const displaySections = sections ?? [{ label: null, tasks }];
  return (
    <div className="task-list-view">
      <div className="view-header">
        <h1>{title}</h1>
        <div className="view-header-actions">
          {headerSlot}
          <TaskTagFilter tags={tags} value={activeTag} onChange={onTagFilterChange} />
        </div>
      </div>
      {topSlot}
      {tasks.length === 0 ? (
        <div className="empty-state">{emptyLabel}</div>
      ) : (
        <div className="list">
          {displaySections.map((section, sectionIndex) => (
            <section className="task-section" key={section.label ?? `tasks-${sectionIndex}`}>
              {section.label && (
                <div className={`task-section-heading ${section.tone ?? ""}`.trim()}>{section.label}</div>
              )}
              <AnimatePresence initial={false}>
                {section.tasks.map((task) => {
                  const isCompleting = completing.has(task.id);
                  const isExpanded = expanded === task.id;
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
                        expanded={isExpanded}
                        completing={isCompleting}
                        subtasks={isExpanded ? subtasks : []}
                        showProjectContext={showProjectContext}
                        subtaskCount={subtaskCounts[task.id]}
                        availableTags={tags}
                        tags={taskTags[task.id] ?? []}
                        onTagsChange={(names) => onTaskTagsChange(task.id, names)}
                        projects={projects}
                        projectId={taskProjects[task.id] ?? null}
                        onProjectChange={(projectId) => onTaskProjectChange(task.id, projectId)}
                        pendingCompleteConfirm={pendingCompleteConfirm === task.id}
                        onConfirmComplete={() => onConfirmComplete(task.id)}
                        onCancelCompleteConfirm={onCancelCompleteConfirm}
                        selected={selectedIds.has(task.id)}
                        onToggleExpanded={() => onToggleExpanded(task.id)}
                        onComplete={() => onComplete(task.id)}
                        onRename={onRename}
                        onReschedule={onReschedule}
                        onNoteChange={(note) => onNoteChange(task.id, note)}
                        onAddSubtask={(title) => onAddSubtask(task.id, title)}
                        onToggleSubtask={onToggleSubtask}
                        onDeleteSubtask={onDeleteSubtask}
                        onDelete={() => onDelete(task.id)}
                        onScheduled={onScheduled}
                        onToggleSelected={() => onToggleSelected(task.id)}
                      />
                    </motion.div>
                  );
                })}
              </AnimatePresence>
            </section>
          ))}
        </div>
      )}
      {bottomSlot}
    </div>
  );
}
