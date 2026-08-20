import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { CalendarAuth, CalendarEvent } from "../lib/types";
import "./CalendarGlance.css";

function colorCss([r, g, b, a]: [number, number, number, number]): string {
  return `rgba(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)}, ${a})`;
}

// PRD §6.3: "A compact calendar-glance card precedes the tasks" in Today
// specifically. PRD §6.5 (revised 2026-08-19): hidden entirely until
// EventKit permission is granted, rather than showing an empty or
// "not connected" state — mirrors the GPUI app's own components::
// calendar_glance and its render_task_view gating exactly.
export function CalendarGlance() {
  const [auth, setAuth] = useState<CalendarAuth | null>(null);
  const [events, setEvents] = useState<CalendarEvent[]>([]);

  useEffect(() => {
    api.calendarAuthStatus().then(setAuth).catch(() => setAuth("Unavailable"));
  }, []);

  useEffect(() => {
    if (auth !== "Granted") return;
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    const end = new Date(start);
    end.setDate(start.getDate() + 1);
    api.calendarEvents(start, end).then(setEvents).catch(() => setEvents([]));
  }, [auth]);

  if (auth !== "Granted") return null;

  // Timed events by start time first, then all-day events after — the
  // GPUI app's own sort_by_key((event.all_day, event.start)) order
  // (false < true in a bool sort), not the DayColumn convention elsewhere
  // of putting all-day first.
  const sorted = [...events].sort(
    (a, b) => Number(a.all_day) - Number(b.all_day) || a.start.localeCompare(b.start),
  );

  return (
    <div className="calendar-glance">
      <div className="calendar-glance-date">
        {new Date().toLocaleDateString(undefined, { weekday: "long", month: "short", day: "numeric" })}
      </div>
      {sorted.length === 0 ? (
        <div className="calendar-glance-empty">No events today</div>
      ) : (
        sorted.map((event) => (
          <div className="calendar-glance-row" key={event.id}>
            <span className="calendar-glance-dot" style={{ background: colorCss(event.color) }} />
            <span className="calendar-glance-time">
              {event.all_day
                ? "All day"
                : new Date(event.start).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}
            </span>
            <span className="calendar-glance-title">{event.title}</span>
          </div>
        ))
      )}
    </div>
  );
}
