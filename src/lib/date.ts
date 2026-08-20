// Local calendar day as "YYYY-MM-DD" — matches flow_data::db's own
// scheduled_date format exactly (chrono::Local::now().date_naive().
// to_string()), which is what a task's real scheduled_date actually
// contains. Deliberately not `new Date().toISOString().slice(0, 10)`:
// that's UTC, and would read as tomorrow (or yesterday) for a chunk of
// every day depending on the user's own timezone offset from UTC.
export function todayIso(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

// Parses a "YYYY-MM-DD" string as a local calendar date, not UTC — same
// reasoning todayIso() itself has: `new Date("2026-08-22")` parses as UTC
// midnight, which reads as the previous day for a chunk of every 24 hours
// depending on the user's own offset.
function parseIsoLocal(date: string): Date | null {
  const match = date.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (!match) return null;
  const [, y, m, d] = match;
  return new Date(Number(y), Number(m) - 1, Number(d));
}

// Mirrors the GPUI app's own day_label: "Today"/"Tomorrow" for the two
// nearest days, a bare weekday name for the rest of the week, else a
// short date ("Aug 23") — direct user report that the raw "2026-08-22"
// format shown everywhere (Upcoming's own date headers, a row's trailing
// schedule badge) was worse than this.
export function dayLabel(date: string): string {
  const parsed = parseIsoLocal(date);
  if (!parsed) return date;
  const today = parseIsoLocal(todayIso())!;
  const days = Math.round((parsed.getTime() - today.getTime()) / 86_400_000);
  if (days === 0) return "Today";
  if (days === 1) return "Tomorrow";
  if (days >= 2 && days <= 6) return parsed.toLocaleDateString(undefined, { weekday: "long" });
  return parsed.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

// Mirrors the GPUI app's own format_schedule: day_label with a 12-hour
// time appended when present ("Tomorrow 6:00 PM").
export function formatSchedule(date: string, time: string | null): string {
  const dayPart = dayLabel(date);
  if (!time) return dayPart;
  const match = time.match(/^(\d{2}):(\d{2})$/);
  if (!match) return dayPart;
  const [, h, m] = match;
  const asDate = new Date(2000, 0, 1, Number(h), Number(m));
  const timePart = asDate.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  return `${dayPart} ${timePart}`;
}
