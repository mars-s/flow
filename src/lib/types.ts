// Mirrors the shape of Flow's real `db::Task` (src/db.rs) closely enough
// to swap in real data later without reshaping every component — not
// wired to a backend yet, see wayfinder/tickets/migrate-to-tauri.md in
// the Flow repo for that open question.
export type Bucket = "inbox" | "today" | "anytime" | "someday";

export type Task = {
  id: string;
  title: string;
  note: string;
  bucket: Bucket;
  scheduledDate?: string;
  scheduledTime?: string;
  completed: boolean;
  subtasks: Subtask[];
};

export type Subtask = {
  id: string;
  title: string;
  completed: boolean;
};

export type Destination = "inbox" | "today" | "upcoming" | "anytime" | "someday" | "calendar" | "settings";
