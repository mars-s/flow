import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import { CompletedSection } from "./components/CompletedSection";
import { StaleTaskNudges } from "./components/StaleTaskNudges";
import { OverdueReschedule } from "./components/OverdueReschedule";
import { api } from "./lib/api";
import { todayIso } from "./lib/date";
import { usePersistedString } from "./lib/persisted";
import type { ThemeId } from "./lib/theme";
import { VIEW_FOR } from "./lib/types";
import type { Bucket, Destination, SubtaskCount, Task, View } from "./lib/types";
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
  // Real gap found by re-checking tasks.rs's own completed_section against
  // the Tauri app: list_completed was already wired into api.ts back when
  // Capture/CRUD first landed but never actually called — a completed
  // task simply vanished from view with no way to see it again. Fetched
  // alongside the active lists, same "resolve the whole collection up
  // front" reasoning refresh() already uses for those.
  const [completedTasks, setCompletedTasks] = useState<Record<View, Task[]>>(EMPTY_VIEW_TASKS);
  const [completedOpen, setCompletedOpen] = useState<Record<View, boolean>>({
    Inbox: false,
    Today: false,
    Upcoming: false,
    Anytime: false,
    Someday: false,
  });
  // A collapsed row's own subtask-count badge (direct user request,
  // matching Things 3's row indicators) — keyed by parent id so any row,
  // in any view, can look its own count up without a per-row fetch.
  const [subtaskCounts, setSubtaskCounts] = useState<Record<string, SubtaskCount>>({});
  const [theme, setTheme] = usePersistedString("flow.theme", "default");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [mode, setMode] = useState<"tasks" | "calendar">("tasks");
  const [destination, setDestination] = useState<Destination>("inbox");
  const lastNonSettingsDestination = useRef<Destination>("inbox");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [completing, setCompleting] = useState<Set<string>>(new Set());
  const [capturing, setCapturing] = useState(false);
  const [subtasks, setSubtasks] = useState<Task[]>([]);
  const [undoToast, setUndoToast] = useState<UndoState | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  // A collapsed row's own subtask-count badge, kept separate from refresh()
  // below (though refresh() also calls it) so addSubtask/toggleSubtask/
  // deleteSubtask can update just this without re-fetching all ten view
  // lists for what's usually a single-row change — those three previously
  // only called refreshSubtasks (the expanded card's own subtask list),
  // leaving the collapsed row's badge stale until some unrelated refresh
  // happened to fire, the same class of bug the note-persistence one was.
  const refreshSubtaskCounts = useCallback(() => {
    api
      .subtaskCounts()
      .then((counts) => {
        const byParent: Record<string, SubtaskCount> = {};
        for (const count of counts) byParent[count.parent_id] = count;
        setSubtaskCounts(byParent);
      })
      .catch((error) => setLoadError(String(error)));
  }, []);

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
    Promise.all(VIEWS.map((view) => api.listCompleted(view)))
      .then((lists) => {
        const next = { ...EMPTY_VIEW_TASKS };
        VIEWS.forEach((view, i) => {
          next[view] = lists[i];
        });
        setCompletedTasks(next);
      })
      .catch((error) => setLoadError(String(error)));
    refreshSubtaskCounts();
  }, [refreshSubtaskCounts]);

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

  // Tracks whatever destination was showing before Settings, so Cmd+,
  // can toggle back to it — same as clicking Settings again in the
  // sidebar returned you nowhere before; a real "close preferences"
  // needs somewhere to close back to.
  useEffect(() => {
    if (destination !== "settings") lastNonSettingsDestination.current = destination;
  }, [destination]);

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
      // Cmd+, is the standard macOS "open preferences" shortcut — every
      // native app honors it, so Settings shouldn't be sidebar-click-only.
      // Pressing it again while already in Settings closes back to
      // wherever you were, matching how real macOS Preferences windows
      // toggle rather than just re-opening themselves.
      if (event.metaKey && event.key === ",") {
        event.preventDefault();
        setDestination(destination === "settings" ? lastNonSettingsDestination.current : "settings");
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
  // Flattened once per viewTasks change — used by the Capture field's
  // duplicate-detection check, which needs every active task regardless
  // of which view is currently on screen.
  const allTasks = useMemo(() => Object.values(viewTasks).flat(), [viewTasks]);
  // Falls back to "Inbox" only in the impossible case destination is
  // Settings/Calendar here — TaskList never renders for those (guarded in
  // the JSX below), this just satisfies VIEW_FOR's Partial<> type.
  const currentView: View = VIEW_FOR[destination] ?? "Inbox";
  const visibleTasks = useMemo(() => {
    const view = VIEW_FOR[destination];
    return view ? viewTasks[view] : [];
  }, [viewTasks, destination]);
  const upcomingTasks = viewTasks.Upcoming;

  // PRD §6.2/§11: "Completing a parent with incomplete children asks:
  // 'Complete parent and all subtasks' or 'Cancel.' It never leaves a
  // completed parent with open children" — real gap found by re-checking
  // tasks.rs's own request_complete/confirm_complete_with_subtasks
  // against Tauri: nothing here checked for open subtasks at all before
  // completing a parent outright. subtaskCounts already has each task's
  // open count with no extra fetch (unlike the GPUI app's own
  // request_complete_from_row, which has to background-fetch subtasks
  // per click since its compact row never loads them).
  const [pendingCompleteConfirm, setPendingCompleteConfirm] = useState<string | null>(null);

  const requestComplete = (id: string) => {
    const openCount = subtaskCounts[id]?.open ?? 0;
    if (openCount > 0) {
      // Expands the card (same as the GPUI app's own request_complete_
      // from_row) so the confirm banner has somewhere to live and its
      // own subtask list loads for confirmCompleteWithSubtasks to use.
      setExpanded(id);
      setPendingCompleteConfirm(id);
    } else {
      complete(id);
    }
  };

  const cancelCompleteConfirm = () => setPendingCompleteConfirm(null);

  const confirmCompleteWithSubtasks = (id: string) => {
    setPendingCompleteConfirm(null);
    const openIds = subtasks.filter((task) => !task.completed_at).map((task) => task.id);
    Promise.all(openIds.map((subtaskId) => api.setCompleted(subtaskId, true)))
      .then(() => {
        refreshSubtasks(id);
        refreshSubtaskCounts();
        complete(id);
      })
      .catch((error) => setLoadError(String(error)));
  };

  const complete = (id: string) => {
    if (completing.has(id)) return;
    const title = Object.values(viewTasks).flat().find((task) => task.id === id)?.title ?? "task";
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
          // PRD §6.1's 10s undo window covers completion too, not just
          // delete — matches the GPUI app's own toggle_completed, which
          // shows this after the write lands (same reasoning: showing it
          // before the animation finishes would let a second undo-click
          // race the row still being on screen).
          setUndoToast({
            message: `Completed "${title}"`,
            onUndo: () => api.setCompleted(id, false).then(refresh).catch((error) => setLoadError(String(error))),
          });
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

  // A completed-section row's checkbox: unlike `complete`, this writes
  // immediately with no animation delay and no undo toast — same
  // asymmetry the GPUI app's own toggle_completed has (its `if
  // !completed` branch returns early, straight to write_completed).
  const uncomplete = (id: string) => {
    api.setCompleted(id, false).then(refresh).catch((error) => setLoadError(String(error)));
  };

  const toggleCompletedOpen = (view: View) => {
    setCompletedOpen((prev) => ({ ...prev, [view]: !prev[view] }));
  };

  // The Completed section's "Clear" button — soft-deletes every completed
  // task shown there, same loop-per-id pattern as bulkDelete but with no
  // undo toast, matching the GPUI app's own clear_completed (PRD's undo
  // guarantee covers deleting an active task, not bulk-clearing ones
  // already completed).
  const clearCompleted = (view: View) => {
    const ids = completedTasks[view].map((task) => task.id);
    if (ids.length === 0) return;
    Promise.all(ids.map((id) => api.deleteTask(id)))
      .then(refresh)
      .catch((error) => setLoadError(String(error)));
  };

  const capture = (title: string) => {
    setCapturing(false);
    api.captureTask(title).then(refresh).catch((error) => setLoadError(String(error)));
  };

  // Real bug, direct user report: the write succeeded in the database the
  // whole time, but nothing here ever called refresh() afterward, so the
  // note view (which reads task.note straight off viewTasks state) kept
  // showing the stale pre-edit value until some unrelated refresh
  // happened to fire — indistinguishable from "it doesn't persist" from
  // the user's side of the screen, since clicking out of the field
  // visibly reverted to the old text.
  const changeNote = (id: string, note: string) => {
    api.setNote(id, note).then(refresh).catch((error) => setLoadError(String(error)));
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

  // Renaming a task with a recognized date/time phrase reschedules it too
  // — "look out for the nlp input similar to adding a task" (direct user
  // request). Always bucket Active, same as capture_task's own rule for
  // a task that gets a date or time attached to it.
  const rescheduleTask = (id: string, date: string, time: string | null) => {
    api.scheduleTask(id, "Active", date, time).then(refresh).catch((error) => setLoadError(String(error)));
  };

  // Overdue batch reschedule's own bulk write — same shape as bulkProcess
  // below, just driven by the AI block's own suggested date rather than
  // one of the bulk-action bar's fixed targets.
  const rescheduleMany = (ids: string[], date: string) => {
    Promise.all(ids.map((id) => api.scheduleTask(id, "Active", date, null)))
      .then(refresh)
      .catch((error) => setLoadError(String(error)));
  };

  const addSubtask = (parentId: string, title: string) => {
    api
      .createSubtask(parentId, title)
      .then(() => {
        refreshSubtasks(parentId);
        refreshSubtaskCounts();
      })
      .catch((error) => setLoadError(String(error)));
  };

  const toggleSubtask = (id: string, completed: boolean) => {
    if (!expanded) return;
    api
      .setCompleted(id, completed)
      .then(() => {
        refreshSubtasks(expanded);
        refreshSubtaskCounts();
      })
      .catch((error) => setLoadError(String(error)));
  };

  // Backspace-on-empty in the checklist — direct user request to delete
  // a subtask that way instead of needing a dedicated delete affordance
  // per row. No undo toast, matching the "gets rid of unnecessary ui"
  // spirit of the whole redesign; it's one keystroke to remove, not a
  // destructive bulk action.
  const deleteSubtask = (id: string) => {
    if (!expanded) return;
    api
      .deleteTask(id)
      .then(() => {
        refreshSubtasks(expanded);
        refreshSubtaskCounts();
      })
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
      {/* No stopPropagation here on purpose — a click anywhere in the main
          pane that isn't absorbed by something more specific (a row's own
          click-to-expand, the expanded card's own stopPropagation, a
          pill/input) bubbles up to the root div's own handler and
          collapses the expanded task, matching the direct user report
          that clicking elsewhere while a task is expanded didn't unfocus
          it — main-pane's own blind stopPropagation was swallowing that
          bubble before it ever reached the root. */}
      <div className="main-pane">
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
              <CaptureField
                open={capturing}
                onSubmit={capture}
                onClose={() => setCapturing(false)}
                existingTasks={allTasks}
              />
            </div>
            {destination === "upcoming" ? (
              <UpcomingList
                tasks={upcomingTasks}
                expanded={expanded}
                completing={completing}
                subtasks={subtasks}
                subtaskCounts={subtaskCounts}
                pendingCompleteConfirm={pendingCompleteConfirm}
                onConfirmComplete={confirmCompleteWithSubtasks}
                onCancelCompleteConfirm={cancelCompleteConfirm}
                selectedIds={selectedIds}
                onToggleExpanded={(id) => setExpanded((current) => (current === id ? null : id))}
                onComplete={requestComplete}
                onRename={renameTask}
                onReschedule={rescheduleTask}
                onNoteChange={changeNote}
                onAddSubtask={addSubtask}
                onToggleSubtask={toggleSubtask}
                onDeleteSubtask={deleteSubtask}
                onDelete={deleteTask}
                onScheduled={refresh}
                onToggleSelected={toggleSelected}
                bottomSlot={
                  <CompletedSection
                    tasks={completedTasks.Upcoming}
                    open={completedOpen.Upcoming}
                    onToggleOpen={() => toggleCompletedOpen("Upcoming")}
                    onUncomplete={uncomplete}
                    onClear={() => clearCompleted("Upcoming")}
                  />
                }
              />
            ) : (
              <TaskList
                title={destination[0].toUpperCase() + destination.slice(1)}
                tasks={visibleTasks}
                expanded={expanded}
                completing={completing}
                subtasks={subtasks}
                subtaskCounts={subtaskCounts}
                pendingCompleteConfirm={pendingCompleteConfirm}
                onConfirmComplete={confirmCompleteWithSubtasks}
                onCancelCompleteConfirm={cancelCompleteConfirm}
                selectedIds={selectedIds}
                onToggleExpanded={(id) => setExpanded((current) => (current === id ? null : id))}
                onComplete={requestComplete}
                onRename={renameTask}
                onReschedule={rescheduleTask}
                onNoteChange={changeNote}
                onAddSubtask={addSubtask}
                onToggleSubtask={toggleSubtask}
                onDeleteSubtask={deleteSubtask}
                onDelete={deleteTask}
                onScheduled={refresh}
                onToggleSelected={toggleSelected}
                bottomSlot={
                  <CompletedSection
                    tasks={completedTasks[currentView]}
                    open={completedOpen[currentView]}
                    onToggleOpen={() => toggleCompletedOpen(currentView)}
                    onUncomplete={uncomplete}
                    onClear={() => clearCompleted(currentView)}
                  />
                }
                emptyLabel={EMPTY_LABEL[destination] ?? "Nothing here yet."}
                topSlot={
                  destination === "today" ? (
                    <>
                      <CalendarGlance />
                      <OverdueReschedule tasks={visibleTasks} onRescheduleAll={rescheduleMany} />
                    </>
                  ) : destination === "inbox" || destination === "anytime" ? (
                    <StaleTaskNudges tasks={visibleTasks} />
                  ) : undefined
                }
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
