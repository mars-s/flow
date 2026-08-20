// Mirrors flow-data's real `db::Task`/`Bucket`/`View` exactly (field names,
// enum tag strings) — this is what actually comes back over Tauri's IPC now,
// not a hand-picked shape. See /Users/avi/Developer/vibe/flow/crates/
// flow-data/src/db.rs for the source of truth; keep these in sync with it,
// not the other way around.
export type Bucket = "Inbox" | "Active" | "Someday";

export type View = "Inbox" | "Today" | "Upcoming" | "Anytime" | "Someday";

export type Task = {
  id: string;
  parent_id: string | null;
  title: string;
  note: string | null;
  bucket: Bucket;
  scheduled_date: string | null;
  scheduled_time: string | null;
  scheduled_timezone: string | null;
  position: number;
  completed_at: string | null;
  created_at: string;
  updated_at: string;
};

export type Destination = "inbox" | "today" | "upcoming" | "anytime" | "someday" | "calendar" | "settings";

export const VIEW_FOR: Partial<Record<Destination, View>> = {
  inbox: "Inbox",
  today: "Today",
  upcoming: "Upcoming",
  anytime: "Anytime",
  someday: "Someday",
};
