import { invoke } from "@tauri-apps/api/core";
import type { Task, View } from "./types";

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
  setCompleted: (id: string, completed: boolean) => invoke<void>("set_completed", { id, completed }),
  setNote: (id: string, note: string) => invoke<void>("set_note", { id, note }),
  deleteTask: (id: string) => invoke<void>("delete_task", { id }),
};
