import { useEffect, useMemo, useState } from "react";
import { Search, Tag as TagIcon } from "lucide-react";
import { formatSchedule } from "../lib/date";
import type { Task, TaskTag } from "../lib/types";
import "./TaskSearch.css";

export function TaskSearch({
  open,
  tasks,
  taskTags,
  onClose,
  onOpenTask,
}: {
  open: boolean;
  tasks: Task[];
  taskTags: Record<string, TaskTag[]>;
  onClose: () => void;
  onOpenTask: (task: Task) => void;
}) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const results = useMemo(() => {
    const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
    if (terms.length === 0) return tasks.slice(0, 12);
    return tasks
      .filter((task) => {
        const searchable = [task.title, task.note ?? "", ...(taskTags[task.id] ?? []).map((tag) => tag.name)]
          .join(" ")
          .toLowerCase();
        return terms.every((term) => searchable.includes(term));
      })
      .slice(0, 50);
  }, [query, taskTags, tasks]);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setSelectedIndex(0);
  }, [open]);

  useEffect(() => {
    if (selectedIndex >= results.length) setSelectedIndex(Math.max(0, results.length - 1));
  }, [results.length, selectedIndex]);

  if (!open) return null;

  return (
    <div className="task-search-backdrop" onMouseDown={onClose}>
      <div
        className="task-search-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Search tasks"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="task-search-input-row">
          <Search size={15} />
          <input
            autoFocus
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setSelectedIndex(0);
            }}
            onKeyDown={(event) => {
              if (event.key === "Escape") onClose();
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setSelectedIndex((index) => Math.min(index + 1, results.length - 1));
              }
              if (event.key === "ArrowUp") {
                event.preventDefault();
                setSelectedIndex((index) => Math.max(0, index - 1));
              }
              if (event.key === "Enter" && results[selectedIndex]) onOpenTask(results[selectedIndex]);
            }}
            placeholder="Search titles, notes, and tags"
            aria-label="Search tasks"
          />
          <kbd>esc</kbd>
        </div>
        <div className="task-search-results" role="listbox">
          {results.length === 0 ? (
            <div className="task-search-empty">No matching tasks.</div>
          ) : (
            results.map((task, index) => (
              <button
                type="button"
                role="option"
                aria-selected={index === selectedIndex}
                className={index === selectedIndex ? "task-search-result selected" : "task-search-result"}
                key={task.id}
                onMouseEnter={() => setSelectedIndex(index)}
                onClick={() => onOpenTask(task)}
              >
                <span className="task-search-result-title">{task.title}</span>
                <span className="task-search-result-meta">
                  {task.scheduled_date && <span>{formatSchedule(task.scheduled_date, task.scheduled_time)}</span>}
                  {(taskTags[task.id] ?? []).map((tag) => (
                    <span className="task-search-result-tag" key={tag.tag_id}>
                      <TagIcon size={9} />
                      {tag.name}
                    </span>
                  ))}
                </span>
              </button>
            ))
          )}
        </div>
        <div className="task-search-footer">
          <span>↑↓ navigate</span>
          <span>↵ open</span>
        </div>
      </div>
    </div>
  );
}
