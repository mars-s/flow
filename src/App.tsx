import { useCallback, useEffect, useMemo, useState } from "react";
import { AnimatePresence } from "framer-motion";
import { Sidebar } from "./components/Sidebar";
import { CaptureField } from "./components/CaptureField";
import { TaskList } from "./views/TaskList";
import { UpcomingList } from "./views/UpcomingList";
import { Settings } from "./views/Settings";
import { Calendar } from "./views/Calendar";
import { UndoToast, type UndoState } from "./components/UndoToast";
import { BulkActionBar, type BulkTarget } from "./components/BulkActionBar";
import { CalendarGlance } from "./components/CalendarGlance";
import { api } from "./lib/api";
import { todayIso } from "./lib/date";
import { usePersistedString } from "./lib/persisted";
import type { ThemeId } from "./lib/theme";
import { VIEW_FOR } from "./lib/types";
import type { Bucket, Destination, Task, View } from "./lib/types";
import "./theme.css";
import "./App.css";

const EMPTY_LABEL: Record<string, string> = {
  inbox: "Nothing to process. Capture the next thing.",
  today: "Your day is clear.",
  anytime: "Nothing here yet.",
  someday: "Nothing here yet.",
};

const VIEWS = ["Inbox", "Today", "Upcoming", "Anytime", "Someday"] as const;
const EMPTY_VIEW_TASKS: Record<View, Task[]> = { Inbox: [], Today: [], Upcoming: [], Anytime: [], Someday: [] };

export default function App() {
  // Kept as five separate per-view lists straight from list_view, not one
  // flat array re-filtered client-side — Today's real definition is
  // "scheduled_date <= today" (it includes overdue tasks, per the GPUI
  // app's own sidebar.rs description: "Overdue and today's active tasks"),
  // not "scheduled_date === today". Re-deriving that split in JS from a
  // merged list previously got it wrong: overdue tasks landed in Upcoming
  // instead of Today. Each view's own query already gets this right, so
  // reusing its result directly is both the fix and the simpler code.
  const [viewTasks, setViewTasks] = useState<Record<View, Task[]>>(EMPTY_VIEW_TASKS);
  const [theme, setTheme] = usePersistedString("flow.theme", "default");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [mode, setMode] = useState<"tasks" | "calendar">("tasks");
  const [destination, setDestination] = useState<Destination>("inbox");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [completing, setCompleting] = useState<Set<string>>(new Set());
  const [capturing, setCapturing] = useState(false);
  const [subtasks, setSubtasks] = useState<Task[]>([]);
  const [undoToast, setUndoToast] = useState<UndoState | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  // Every task view Flow actually has lives under one of these five real
  // View values — fetched together on mount/refresh rather than one at a
  // time per destination, the same "resolve the whole collection up front"
  // reasoning the GPUI app's own render_task_view comment gives for why it
  // doesn't gate every view behind its own fetch.
  const refresh = useCallback(() => {
    Promise.all(VIEWS.map((view) => api.listView(view)))
      .then((lists) => {
        const next = { ...EMPTY_VIEW_TASKS };
        VIEWS.forEach((view, i) => {
          next[view] = lists[i];
        });
        setViewTasks(next);
        setLoadError(null);
      })
      .catch((error) => setLoadError(String(error)));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // theme.css's [data-theme="river-cut"] block only applies once this
  // attribute is set on <html> — "default" needs no attribute at all
  // (the base :root tokens already are the default theme), so it's the
  // one value this clears rather than sets. Lives here rather than
  // inside Settings so switching theme applies immediately regardless of
  // which view is on screen, not just once Settings itself re-renders.
  useEffect(() => {
    if (theme === "default") document.documentElement.removeAttribute("data-theme");
    else document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  // A window-level listener, not the root div's own onKeyDown (used below
  // for Escape) — Cmd+N has to open Capture from anywhere in the app,
  // including when focus is inside a task's note field or nowhere at all,
  // not just when the shell div itself happens to have focus.
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.metaKey && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setCapturing(true);
        return;
      }
      // Bare space opens Capture too, scoped to task views only (not
      // Calendar/Settings) so it doesn't hijack space's native meaning
      // there — same handle_space_capture_action the GPUI app has. The
      // GPUI app's own keymap context skips this while a composer has
      // focus; here that's checking the real focused element, which also
      // covers the schedule picker's own auto-focused input (its state
      // lives inside TaskRow, not lifted up here, so there's no flag for
      // it to check directly).
      if (event.key === " " && !event.metaKey && !event.ctrlKey && !event.altKey) {
        const tag = document.activeElement?.tagName;
        const editable = document.activeElement?.getAttribute("contenteditable") === "true";
        if (tag === "INPUT" || tag === "TEXTAREA" || editable) return;
        if (capturing || expanded || mode !== "tasks" || destination === "settings" || destination === "calendar") {
          return;
        }
        event.preventDefault();
        setCapturing(true);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [capturing, expanded, mode, destination]);

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

  const inboxCount = viewTasks.Inbox.length;
  const visibleTasks = useMemo(() => {
    const view = VIEW_FOR[destination];
    return view ? viewTasks[view] : [];
  }, [viewTasks, destination]);
  const upcomingTasks = viewTasks.Upcoming;

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

  // Renames a task or a subtask — same command either way (set_title
  // doesn't distinguish), and this doesn't know which `id` is without
  // extra bookkeeping, so it just refreshes both: the task-view lists
  // (in case it renamed the expanded task itself, shown in the row/card
  // title) and the subtask list (in case it renamed one of those instead).
  const renameTask = (id: string, title: string) => {
    api
      .setTitle(id, title)
      .then(() => {
        refresh();
        if (expanded) refreshSubtasks(expanded);
      })
      .catch((error) => setLoadError(String(error)));
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
    const title = Object.values(viewTasks).flat().find((task) => task.id === id)?.title ?? "task";
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

  // Cmd+click toggles a row into the multi-select set instead of expanding
  // it — same interaction as the GPUI app's own toggle_selected.
  const toggleSelected = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  };

  // The bulk-action bar's Today/Anytime/Someday buttons — same
  // schedule-every-selected-task-then-clear-selection shape as the GPUI
  // app's own bulk_process.
  const bulkProcess = (target: BulkTarget) => {
    const ids = [...selectedIds];
    if (ids.length === 0) return;
    setSelectedIds(new Set());
    const bucket: Bucket = target === "someday" ? "Someday" : "Active";
    const date = target === "today" ? todayIso() : null;
    Promise.all(ids.map((id) => api.scheduleTask(id, bucket, date, null)))
      .then(refresh)
      .catch((error) => setLoadError(String(error)));
  };

  // The bulk-action bar's Delete button — same Undo-toast affordance as a
  // single row's delete (PRD §6.1's undo-delete criterion doesn't
  // distinguish single from bulk).
  const bulkDelete = () => {
    const ids = [...selectedIds];
    if (ids.length === 0) return;
    setSelectedIds(new Set());
    Promise.all(ids.map((id) => api.deleteTask(id)))
      .then(() => {
        refresh();
        const label = ids.length === 1 ? "1 task" : `${ids.length} tasks`;
        setUndoToast({
          message: `Deleted ${label}`,
          onUndo: () => Promise.all(ids.map((id) => api.restoreTask(id))).then(refresh).catch((error) => setLoadError(String(error))),
        });
      })
      .catch((error) => setLoadError(String(error)));
  };

  return (
    <div
      className="app"
      onClick={() => {
        setExpanded(null);
        setSelectedIds(new Set());
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          setExpanded(null);
          setCapturing(false);
          setSelectedIds(new Set());
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
        <AnimatePresence>
          {selectedIds.size > 0 && (
            <BulkActionBar count={selectedIds.size} onProcess={bulkProcess} onDelete={bulkDelete} />
          )}
        </AnimatePresence>
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
                selectedIds={selectedIds}
                onToggleExpanded={(id) => setExpanded((current) => (current === id ? null : id))}
                onComplete={complete}
                onRename={renameTask}
                onNoteChange={changeNote}
                onAddSubtask={addSubtask}
                onToggleSubtask={toggleSubtask}
                onDelete={deleteTask}
                onScheduled={refresh}
                onToggleSelected={toggleSelected}
              />
            ) : (
              <TaskList
                title={destination[0].toUpperCase() + destination.slice(1)}
                tasks={visibleTasks}
                expanded={expanded}
                completing={completing}
                subtasks={subtasks}
                selectedIds={selectedIds}
                onToggleExpanded={(id) => setExpanded((current) => (current === id ? null : id))}
                onComplete={complete}
                onRename={renameTask}
                onNoteChange={changeNote}
                onAddSubtask={addSubtask}
                onToggleSubtask={toggleSubtask}
                onDelete={deleteTask}
                onScheduled={refresh}
                onToggleSelected={toggleSelected}
                emptyLabel={EMPTY_LABEL[destination] ?? "Nothing here yet."}
                topSlot={destination === "today" ? <CalendarGlance /> : undefined}
              />
            )}
          </div>
        ) : destination === "settings" ? (
          <Settings theme={theme as ThemeId} onThemeChange={setTheme} />
        ) : (
          <Calendar />
        )}
      </div>
    </div>
  );
}
