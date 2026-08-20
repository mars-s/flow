import { invoke } from "@tauri-apps/api/core";
import type {
  Area,
  Bucket,
  CalendarAuth,
  CalendarEvent,
  CalendarInfo,
  ParsePreview,
  Project,
  ProjectArea,
  SubtaskCount,
  Tag,
  Task,
  TaskTag,
  TaskProject,
  View,
} from "./types";

// Thin wrapper around the real Tauri commands (src-tauri/src/lib.rs), which
// call straight into flow-data::db — the same store the GPUI app itself
// runs on, just pointed at a separate dev database file so the two don't
// fight over the same SQLite file while both exist during the migration.
export const api = {
  listView: (view: View) => invoke<Task[]>("list_view", { view }),
  listCompleted: (view: View) => invoke<Task[]>("list_completed", { view }),
  createTask: (title: string) => invoke<Task>("create_task", { title }),
  // Real date/time parsing (flow_data::parse), not a bare title — see the
  // Rust command's own doc for the exact GPUI-matching capture rules.
  captureTask: (title: string) => invoke<Task>("capture_task", { title }),
  previewCapture: (title: string) => invoke<ParsePreview>("preview_capture", { title }),
  setCompleted: (id: string, completed: boolean) => invoke<void>("set_completed", { id, completed }),
  setNote: (id: string, note: string) => invoke<void>("set_note", { id, note }),
  setTitle: (id: string, title: string) => invoke<void>("set_title", { id, title }),
  scheduleTask: (id: string, bucket: Bucket, date: string | null, time: string | null) =>
    invoke<void>("schedule_task", { id, bucket, date, time }),
  scheduleTaskFromText: (id: string, text: string) => invoke<void>("schedule_task_from_text", { id, text }),
  listSubtasks: (parentId: string) => invoke<Task[]>("list_subtasks", { parentId }),
  createSubtask: (parentId: string, title: string) => invoke<Task>("create_subtask", { parentId, title }),
  subtaskCounts: () => invoke<SubtaskCount[]>("subtask_counts"),
  listTags: () => invoke<Tag[]>("list_tags"),
  listTaskTags: () => invoke<TaskTag[]>("list_task_tags"),
  setTaskTags: (taskId: string, names: string[]) => invoke<TaskTag[]>("set_task_tags", { taskId, names }),
  listProjects: () => invoke<Project[]>("list_projects"),
  createProject: (title: string) => invoke<Project>("create_project", { title }),
  listTaskProjects: () => invoke<TaskProject[]>("list_task_projects"),
  setTaskProject: (taskId: string, projectId: string | null) =>
    invoke<TaskProject | null>("set_task_project", { taskId, projectId }),
  listAreas: () => invoke<Area[]>("list_areas"),
  createArea: (title: string) => invoke<Area>("create_area", { title }),
  listProjectAreas: () => invoke<ProjectArea[]>("list_project_areas"),
  setProjectArea: (projectId: string, areaId: string | null) =>
    invoke<ProjectArea | null>("set_project_area", { projectId, areaId }),
  deleteTask: (id: string) => invoke<void>("delete_task", { id }),
  restoreTask: (id: string) => invoke<void>("restore_task", { id }),
  calendarAuthStatus: () => invoke<CalendarAuth>("calendar_auth_status"),
  calendarConnect: () => invoke<CalendarAuth>("calendar_connect"),
  calendarEvents: (start: Date, end: Date) =>
    invoke<CalendarEvent[]>("calendar_events", { start: start.toISOString(), end: end.toISOString() }),
  calendarList: () => invoke<CalendarInfo[]>("calendar_list"),
};
