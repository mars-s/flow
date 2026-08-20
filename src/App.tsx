import { useMemo, useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { CaptureField } from "./components/CaptureField";
import { TaskList } from "./views/TaskList";
import { UpcomingList } from "./views/UpcomingList";
import { Settings } from "./views/Settings";
import { initialTasks } from "./lib/mockData";
import type { Bucket, Destination, Task } from "./lib/types";
import "./theme.css";
import "./App.css";

const BUCKET_FOR: Partial<Record<Destination, Bucket>> = {
  inbox: "inbox",
  today: "today",
  anytime: "anytime",
  someday: "someday",
};

const EMPTY_LABEL: Record<string, string> = {
  inbox: "Nothing to process. Capture the next thing.",
  today: "Your day is clear.",
  anytime: "Nothing here yet.",
  someday: "Nothing here yet.",
};

export default function App() {
  const [tasks, setTasks] = useState<Task[]>(initialTasks);
  const [mode, setMode] = useState<"tasks" | "calendar">("tasks");
  const [destination, setDestination] = useState<Destination>("inbox");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [completing, setCompleting] = useState<Set<string>>(new Set());
  const [capturing, setCapturing] = useState(false);

  const inboxCount = useMemo(() => tasks.filter((task) => task.bucket === "inbox" && !task.completed).length, [tasks]);

  const visibleTasks = useMemo(() => {
    const bucket = BUCKET_FOR[destination];
    if (!bucket) return [];
    return tasks.filter((task) => task.bucket === bucket && !task.completed);
  }, [tasks, destination]);

  // Upcoming groups by scheduled date across every bucket, not one bucket's
  // own tasks — matches Flow's real PRD §6.3 semantics ("groups active
  // tasks by local date from tomorrow onward"), not a per-bucket filter.
  const upcomingTasks = useMemo(
    () => tasks.filter((task) => !task.completed && task.scheduledDate),
    [tasks],
  );

  const complete = (id: string) => {
    if (completing.has(id)) return;
    setCompleting((prev) => new Set(prev).add(id));
    setExpanded((current) => (current === id ? null : current));
    setTimeout(() => {
      setTasks((prev) => prev.map((task) => (task.id === id ? { ...task, completed: true } : task)));
      setCompleting((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    }, 260);
  };

  const capture = (title: string) => {
    setTasks((prev) => [{ id: crypto.randomUUID(), title, note: "", bucket: "inbox", completed: false, subtasks: [] }, ...prev]);
    setCapturing(false);
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
                onToggleExpanded={(id) => setExpanded((current) => (current === id ? null : id))}
                onComplete={complete}
                onNoteChange={(id, note) =>
                  setTasks((prev) => prev.map((task) => (task.id === id ? { ...task, note } : task)))
                }
              />
            ) : (
              <TaskList
                title={destination[0].toUpperCase() + destination.slice(1)}
                tasks={visibleTasks}
                expanded={expanded}
                completing={completing}
                onToggleExpanded={(id) => setExpanded((current) => (current === id ? null : id))}
                onComplete={complete}
                onNoteChange={(id, note) =>
                  setTasks((prev) => prev.map((task) => (task.id === id ? { ...task, note } : task)))
                }
                emptyLabel={EMPTY_LABEL[destination] ?? "Nothing here yet."}
              />
            )}
          </div>
        ) : destination === "settings" ? (
          <Settings />
        ) : (
          <div className="placeholder-pane">
            Calendar
            <span className="placeholder-note">Not built in this prototype yet.</span>
          </div>
        )}
      </div>
    </div>
  );
}
