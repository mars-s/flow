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
