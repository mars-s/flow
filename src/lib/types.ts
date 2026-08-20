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

// Mirrors flow-data's real db::SubtaskCount exactly.
export type SubtaskCount = {
  parent_id: string;
  open: number;
  total: number;
};

export type Destination = "inbox" | "today" | "upcoming" | "anytime" | "someday" | "calendar" | "settings";

// Mirrors flow-data's real calendar::{CalendarAuth, CalendarEvent,
// CalendarInfo} exactly — the same EventKit-backed types the GPUI app's
// Calendar tab uses, not a Tauri-specific shape.
export type CalendarAuth = "Unavailable" | "NotDetermined" | "Denied" | "Granted";

export type CalendarEvent = {
  id: string;
  calendar_id: string;
  calendar_title: string;
  title: string;
  start: string;
  end: string;
  all_day: boolean;
  color: [number, number, number, number];
};

export type CalendarInfo = {
  id: string;
  title: string;
  source_title: string;
  color: [number, number, number, number];
};

// Mirrors src-tauri's ParsePreview DTO exactly.
export type ParsePreview = {
  date: string | null;
  time: string | null;
  highlight_start: number | null;
  highlight_end: number | null;
};

export const VIEW_FOR: Partial<Record<Destination, View>> = {
  inbox: "Inbox",
  today: "Today",
  upcoming: "Upcoming",
  anytime: "Anytime",
  someday: "Someday",
};
