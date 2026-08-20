import type { CalendarEvent } from "./types";

// Working-hours window Smart scheduling looks for gaps inside — a task
// suggested at 11pm because the calendar happened to be empty then isn't
// actually useful. Matches the GPUI app's own daily-view working hours.
const WORK_START_MINUTES = 9 * 60;
const WORK_END_MINUTES = 18 * 60;
const MIN_GAP_MINUTES = 30;

export type FreeSlot = { startMinutes: number; endMinutes: number };

function minutesOfDay(iso: string): number {
  const date = new Date(iso);
  return date.getHours() * 60 + date.getMinutes();
}

// Computes free gaps inside today's working hours, given the day's real
// calendar events. Pure interval math — no model call, same reasoning
// Overdue reschedule's own addDaysIso split uses: the arithmetic is
// exact by construction, so there's nothing an LLM could get more right.
export function computeFreeSlots(events: CalendarEvent[]): FreeSlot[] {
  const busy = events
    .filter((event) => !event.all_day)
    .map((event) => ({
      start: Math.max(WORK_START_MINUTES, minutesOfDay(event.start)),
      end: Math.min(WORK_END_MINUTES, minutesOfDay(event.end)),
    }))
    .filter((interval) => interval.end > interval.start)
    .sort((a, b) => a.start - b.start);

  const merged: FreeSlot[] = [];
  for (const interval of busy) {
    const last = merged[merged.length - 1];
    if (last && interval.start <= last.endMinutes) {
      last.endMinutes = Math.max(last.endMinutes, interval.end);
    } else {
      merged.push({ startMinutes: interval.start, endMinutes: interval.end });
    }
  }

  const free: FreeSlot[] = [];
  let cursor = WORK_START_MINUTES;
  for (const busySlot of merged) {
    if (busySlot.startMinutes - cursor >= MIN_GAP_MINUTES) {
      free.push({ startMinutes: cursor, endMinutes: busySlot.startMinutes });
    }
    cursor = Math.max(cursor, busySlot.endMinutes);
  }
  if (WORK_END_MINUTES - cursor >= MIN_GAP_MINUTES) {
    free.push({ startMinutes: cursor, endMinutes: WORK_END_MINUTES });
  }
  return free;
}

export function minutesToTime(minutes: number): string {
  const hour = String(Math.floor(minutes / 60)).padStart(2, "0");
  const minute = String(minutes % 60).padStart(2, "0");
  return `${hour}:${minute}`;
}

export function formatSlotLabel(slot: FreeSlot): string {
  const format = (minutes: number) => {
    const date = new Date(2000, 0, 1, Math.floor(minutes / 60), minutes % 60);
    return date.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  };
  return `${format(slot.startMinutes)}–${format(slot.endMinutes)}`;
}
