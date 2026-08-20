import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import { CalendarDays } from "lucide-react";
import { api } from "../lib/api";
import type { CalendarAuth, CalendarEvent } from "../lib/types";
import "./Calendar.css";

function startOfWeek(date: Date): Date {
  const day = date.getDay(); // 0 = Sunday
  const diff = day === 0 ? -6 : 1 - day; // Monday-start, matching the GPUI app's own week
  const start = new Date(date);
  start.setDate(date.getDate() + diff);
  start.setHours(0, 0, 0, 0);
  return start;
}

function weekDays(start: Date): Date[] {
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    return d;
  });
}

function sameDay(a: Date, b: Date): boolean {
  return a.toDateString() === b.toDateString();
}

function colorCss([r, g, b, a]: [number, number, number, number]): string {
  return `rgba(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)}, ${a})`;
}

export function Calendar() {
  const [auth, setAuth] = useState<CalendarAuth | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [error, setError] = useState<string | null>(null);

  const weekStart = useMemo(() => startOfWeek(new Date()), []);
  const days = useMemo(() => weekDays(weekStart), [weekStart]);

  useEffect(() => {
    api
      .calendarAuthStatus()
      .then(setAuth)
      .catch((err) => setError(String(err)));
  }, []);

  useEffect(() => {
    if (auth !== "Granted") return;
    const end = new Date(weekStart);
    end.setDate(weekStart.getDate() + 7);
    api
      .calendarEvents(weekStart, end)
      .then(setEvents)
      .catch((err) => setError(String(err)));
  }, [auth, weekStart]);

  const connect = () => {
    setConnecting(true);
    api
      .calendarConnect()
      .then(setAuth)
      .catch((err) => setError(String(err)))
      .finally(() => setConnecting(false));
  };

  if (auth === null) {
    return <div className="calendar-loading" />;
  }

  if (auth !== "Granted") {
    return (
      <div className="calendar-not-connected">
        <CalendarDays size={28} />
        <div className="calendar-not-connected-title">
          {auth === "Denied" ? "Calendar access denied" : "Connect your calendar"}
        </div>
        <div className="calendar-not-connected-note">
          Read-only. Flow never creates, edits, or deletes anything in your calendar.
        </div>
        {auth !== "Denied" && (
          <button className="calendar-connect-button" onClick={connect} disabled={connecting}>
            {connecting ? "Requesting…" : "Connect Calendar"}
          </button>
        )}
        {error && <div className="calendar-error">{error}</div>}
      </div>
    );
  }

  const today = new Date();

  return (
    <div className="calendar-view">
      <div className="view-header">
        <h1>
          {weekStart.toLocaleDateString(undefined, { month: "short", day: "numeric" })} –{" "}
          {days[6].toLocaleDateString(undefined, { month: "short", day: "numeric" })}
        </h1>
      </div>
      {error && <div className="calendar-error">{error}</div>}
      <div className="calendar-week">
        {days.map((day) => {
          const dayEvents = events
            .filter((event) => sameDay(new Date(event.start), day))
            .sort((a, b) => Number(b.all_day) - Number(a.all_day) || a.start.localeCompare(b.start));
          const isToday = sameDay(day, today);
          return (
            <div className="calendar-day-column" key={day.toISOString()}>
              <div className="calendar-day-header">
                <span className="calendar-day-name">{day.toLocaleDateString(undefined, { weekday: "short" })}</span>
                <span className={isToday ? "calendar-day-number today" : "calendar-day-number"}>
                  {day.getDate()}
                </span>
              </div>
              <div className="calendar-day-events">
                {dayEvents.length === 0 && <div className="calendar-day-empty">No events</div>}
                {dayEvents.map((event) => (
                  <motion.div className="calendar-event-card" key={event.id} whileHover={{ y: -1 }}>
                    <div className="calendar-event-title-row">
                      <span className="calendar-event-dot" style={{ background: colorCss(event.color) }} />
                      <span className="calendar-event-title">{event.title}</span>
                    </div>
                    {!event.all_day && (
                      <div className="calendar-event-time">
                        {new Date(event.start).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}
                        {" – "}
                        {new Date(event.end).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}
                      </div>
                    )}
                  </motion.div>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
