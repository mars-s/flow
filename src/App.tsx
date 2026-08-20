import { useCallback, useEffect, useMemo, useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { CaptureField } from "./components/CaptureField";
import { TaskList } from "./views/TaskList";
import { UpcomingList } from "./views/UpcomingList";
import { Settings } from "./views/Settings";
import { Calendar } from "./views/Calendar";
import { UndoToast, type UndoState } from "./components/UndoToast";
import { api } from "./lib/api";
import { todayIso } from "./lib/date";
import { VIEW_FOR } from "./lib/types";
import type { Destination, Task } from "./lib/types";
import "./theme.css";
import "./App.css";

const EMPTY_LABEL: Record<string, string> = {
  inbox: "Nothing to process. Capture the next thing.",
  today: "Your day is clear.",
  anytime: "Nothing here yet.",
  someday: "Nothing here yet.",
};

export default function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [mode, setMode] = useState<"tasks" | "calendar">("tasks");
  const [destination, setDestination] = useState<Destination>("inbox");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [completing, setCompleting] = useState<Set<string>>(new Set());
  const [capturing, setCapturing] = useState(false);
  const [subtasks, setSubtasks] = useState<Task[]>([]);
  const [undoToast, setUndoToast] = useState<UndoState | null>(null);

  // Every task view Flow actually has lives under one of these five real
  // View values — fetched together on mount/refresh rather than one at a
  // time per destination, the same "resolve the whole collection up front"
  // reasoning the GPUI app's own render_task_view comment gives for why it
  // doesn't gate every view behind its own fetch.
  const refresh = useCallback(() => {
    Promise.all(
      (["Inbox", "Today", "Upcoming", "Anytime", "Someday"] as const).map((view) => api.listView(view)),
    )
      .then((lists) => {
        const merged = new Map<string, Task>();
        for (const list of lists) for (const task of list) merged.set(task.id, task);
        setTasks([...merged.values()]);
        setLoadError(null);
      })
      .catch((error) => setLoadError(String(error)));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // A window-level listener, not the root div's own onKeyDown (used below
  // for Escape) — Cmd+N has to open Capture from anywhere in the app,
  // including when focus is inside a task's note field or nowhere at all,
  // not just when the shell div itself happens to have focus.
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.metaKey && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setCapturing(true);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // Only the expanded task's card ever needs its subtasks — same reasoning
  // the GPUI app's own subtask_context comment gives for fetching this only
  // when something is actually expanded, not per row.
  const refreshSubtasks = useCallback((parentId: string) => {
    api
      .listSubtasks(parentId)
      .then(setSubtasks)
      .catch((error) => setLoadError(String(error)));
  }, []);

  useEffect(() => {
    if (expanded) refreshSubtasks(expanded);
    else setSubtasks([]);
  }, [expanded, refreshSubtasks]);

  const inboxCount = useMemo(() => tasks.filter((task) => task.bucket === "Inbox").length, [tasks]);

  const visibleTasks = useMemo(() => {
    const view = VIEW_FOR[destination];
    if (!view) return [];
    const today = todayIso();
    if (view === "Inbox") return tasks.filter((task) => task.bucket === "Inbox");
    if (view === "Someday") return tasks.filter((task) => task.bucket === "Someday");
    if (view === "Today") return tasks.filter((task) => task.bucket === "Active" && task.scheduled_date === today);
    if (view === "Anytime") return tasks.filter((task) => task.bucket === "Active" && !task.scheduled_date);
    return [];
  }, [tasks, destination]);

  const upcomingTasks = useMemo(() => {
    const today = todayIso();
    return tasks.filter((task) => task.bucket === "Active" && task.scheduled_date && task.scheduled_date !== today);
  }, [tasks]);

  const complete = (id: string) => {
    if (completing.has(id)) return;
    setCompleting((prev) => new Set(prev).add(id));
    setExpanded((current) => (current === id ? null : current));
    api
      .setCompleted(id, true)
      .then(() => {
        setTimeout(() => {
          setCompleting((prev) => {
            const next = new Set(prev);
            next.delete(id);
            return next;
          });
          refresh();
        }, 260);
      })
      .catch((error) => {
        setLoadError(String(error));
        setCompleting((prev) => {
          const next = new Set(prev);
          next.delete(id);
          return next;
        });
      });
  };

  const capture = (title: string) => {
    setCapturing(false);
    api.captureTask(title).then(refresh).catch((error) => setLoadError(String(error)));
  };

  const changeNote = (id: string, note: string) => {
    api.setNote(id, note).catch((error) => setLoadError(String(error)));
  };

  const addSubtask = (parentId: string, title: string) => {
    api
      .createSubtask(parentId, title)
      .then(() => refreshSubtasks(parentId))
      .catch((error) => setLoadError(String(error)));
  };

  const toggleSubtask = (id: string, completed: boolean) => {
    if (!expanded) return;
    api
      .setCompleted(id, completed)
      .then(() => refreshSubtasks(expanded))
      .catch((error) => setLoadError(String(error)));
  };

  const deleteTask = (id: string) => {
    const title = tasks.find((task) => task.id === id)?.title ?? "task";
    setExpanded((current) => (current === id ? null : current));
    api
      .deleteTask(id)
      .then(() => {
        refresh();
        setUndoToast({
          message: `Deleted "${title}"`,
          onUndo: () => api.restoreTask(id).then(refresh).catch((error) => setLoadError(String(error))),
        });
      })
      .catch((error) => setLoadError(String(error)));
  };

  return (
    <div
      className="app"
      onClick={() => setExpanded(null)}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          setExpanded(null);
          setCapturing(false);
        }
      }}
    >
      <Sidebar
        destination={destination}
        onNavigate={setDestination}
        mode={mode}
        onModeChange={setMode}
        inboxCount={inboxCount}
        onCapture={() => setCapturing(true)}
      />
      <div className="main-pane" onClick={(event) => event.stopPropagation()}>
        {loadError && <div className="load-error">Couldn't reach the local task store: {loadError}</div>}
        <UndoToast toast={undoToast} onDismiss={() => setUndoToast(null)} />
        {mode === "tasks" && destination !== "settings" && destination !== "calendar" ? (
          <div className="task-list-shell">
            <div className="capture-slot">
              <CaptureField open={capturing} onSubmit={capture} onClose={() => setCapturing(false)} />
            </div>
            {destination === "upcoming" ? (
              <UpcomingList
                tasks={upcomingTasks}
                expanded={expanded}
                completing={completing}
                subtasks={subtasks}
                onToggleExpanded={(id) => setExpanded((current) => (current === id ? null : id))}
                onComplete={complete}
                onNoteChange={changeNote}
                onAddSubtask={addSubtask}
                onToggleSubtask={toggleSubtask}
                onDelete={deleteTask}
                onScheduled={refresh}
              />
            ) : (
              <TaskList
                title={destination[0].toUpperCase() + destination.slice(1)}
                tasks={visibleTasks}
                expanded={expanded}
                completing={completing}
                subtasks={subtasks}
                onToggleExpanded={(id) => setExpanded((current) => (current === id ? null : id))}
                onComplete={complete}
                onNoteChange={changeNote}
                onAddSubtask={addSubtask}
                onToggleSubtask={toggleSubtask}
                onDelete={deleteTask}
                onScheduled={refresh}
                emptyLabel={EMPTY_LABEL[destination] ?? "Nothing here yet."}
              />
            )}
          </div>
        ) : destination === "settings" ? (
          <Settings />
        ) : (
          <Calendar />
        )}
      </div>
    </div>
  );
}
