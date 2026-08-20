import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import { CalendarDays, ChevronLeft, ChevronRight } from "lucide-react";
import { api } from "../lib/api";
import type { CalendarAuth, CalendarEvent } from "../lib/types";
import "./Calendar.css";

type Mode = "day" | "week" | "month" | "year";

function startOfWeek(date: Date): Date {
  const day = date.getDay(); // 0 = Sunday
  const diff = day === 0 ? -6 : 1 - day; // Monday-start, matching the GPUI app's own week
  const start = new Date(date);
  start.setDate(date.getDate() + diff);
  start.setHours(0, 0, 0, 0);
  return start;
}

function addDays(date: Date, days: number): Date {
  const next = new Date(date);
  next.setDate(date.getDate() + days);
  return next;
}

function addMonths(date: Date, months: number): Date {
  const next = new Date(date);
  next.setDate(1);
  next.setMonth(date.getMonth() + months);
  return next;
}

function addYears(date: Date, years: number): Date {
  const next = new Date(date);
  next.setDate(1);
  next.setFullYear(date.getFullYear() + years);
  return next;
}

// A single month's own grid range (Monday-start, spilling into neighboring
// months to fill whole weeks) — same math monthGridRange uses, just for an
// arbitrary month rather than the cursor's own.
function gridRangeFor(year: number, month: number): { start: Date; end: Date } {
  const firstOfMonth = new Date(year, month, 1);
  const lastOfMonth = new Date(year, month + 1, 0);
  return { start: startOfWeek(firstOfMonth), end: startOfWeek(addDays(lastOfMonth, 7)) };
}

function daysBetween(start: Date, count: number): Date[] {
  return Array.from({ length: count }, (_, i) => addDays(start, i));
}

function sameDay(a: Date, b: Date): boolean {
  return a.toDateString() === b.toDateString();
}

function colorCss([r, g, b, a]: [number, number, number, number]): string {
  return `rgba(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)}, ${a})`;
}

// The Month grid's own range: the full calendar weeks (Monday-start) that
// cover the month, same "grid_start"/"grid_end" shape the GPUI app's
// render_calendar_year_grid uses per-month — a month grid always shows
// whole weeks, so the first/last visible day can spill into the
// neighboring month.
function monthGridRange(cursor: Date): { start: Date; end: Date } {
  const firstOfMonth = new Date(cursor.getFullYear(), cursor.getMonth(), 1);
  const lastOfMonth = new Date(cursor.getFullYear(), cursor.getMonth() + 1, 0);
  return { start: startOfWeek(firstOfMonth), end: startOfWeek(addDays(lastOfMonth, 7)) };
}

function headerLabel(mode: Mode, cursor: Date): string {
  if (mode === "day") return cursor.toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" });
  if (mode === "year") return String(cursor.getFullYear());
  if (mode === "month") return cursor.toLocaleDateString(undefined, { month: "long", year: "numeric" });
  const start = startOfWeek(cursor);
  const end = addDays(start, 6);
  const sameMonth = start.getMonth() === end.getMonth();
  const startLabel = start.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  const endLabel = end.toLocaleDateString(undefined, sameMonth ? { day: "numeric" } : { month: "short", day: "numeric" });
  return `${startLabel} – ${endLabel}`;
}

function DayColumn({ day, events, isToday }: { day: Date; events: CalendarEvent[]; isToday: boolean }) {
  const dayEvents = events
    .filter((event) => sameDay(new Date(event.start), day))
    .sort((a, b) => Number(b.all_day) - Number(a.all_day) || a.start.localeCompare(b.start));
  return (
    <div className="calendar-day-column">
      <div className="calendar-day-header">
        <span className="calendar-day-name">{day.toLocaleDateString(undefined, { weekday: "short" })}</span>
        <span className={isToday ? "calendar-day-number today" : "calendar-day-number"}>{day.getDate()}</span>
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
}

function MonthGrid({
  cursor,
  events,
  today,
  onPickDay,
}: {
  cursor: Date;
  events: CalendarEvent[];
  today: Date;
  onPickDay: (day: Date) => void;
}) {
  const { start } = monthGridRange(cursor);
  const weeks = Math.ceil((monthGridRange(cursor).end.getTime() - start.getTime()) / (7 * 86400000));
  const cells = daysBetween(start, weeks * 7);

  return (
    <div className="calendar-month">
      <div className="calendar-month-weekday-row">
        {cells.slice(0, 7).map((day) => (
          <div className="calendar-month-weekday" key={day.getDay()}>
            {day.toLocaleDateString(undefined, { weekday: "short" })}
          </div>
        ))}
      </div>
      <div className="calendar-month-grid">
        {cells.map((day) => {
          const inMonth = day.getMonth() === cursor.getMonth();
          const dayEvents = events.filter((event) => sameDay(new Date(event.start), day));
          const overflow = dayEvents.length - 3;
          return (
            <button
              type="button"
              key={day.toISOString()}
              className={`calendar-month-cell ${inMonth ? "" : "outside"}`}
              onClick={() => onPickDay(day)}
            >
              <span className={sameDay(day, today) ? "calendar-month-day-number today" : "calendar-month-day-number"}>
                {day.getDate()}
              </span>
              <div className="calendar-month-events">
                {dayEvents.slice(0, 3).map((event) => (
                  <div className="calendar-month-event" key={event.id}>
                    <span className="calendar-event-dot" style={{ background: colorCss(event.color) }} />
                    <span className="calendar-month-event-title">{event.title}</span>
                  </div>
                ))}
                {overflow > 0 && <div className="calendar-month-overflow">+{overflow} more</div>}
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}

// Mirrors the GPUI app's own render_calendar_year_grid: a 4-column grid of
// 12 mini months, each day cell just a number (a dot marks "has an event",
// not the events themselves — a full agenda per day would be unreadable at
// this size), clicking a month jumps to Month mode for it.
function YearGrid({
  year,
  events,
  today,
  onPickMonth,
}: {
  year: number;
  events: CalendarEvent[];
  today: Date;
  onPickMonth: (month: Date) => void;
}) {
  const eventDates = useMemo(() => new Set(events.map((event) => new Date(event.start).toDateString())), [events]);

  return (
    <div className="calendar-year">
      {Array.from({ length: 12 }, (_, month) => {
        const firstOfMonth = new Date(year, month, 1);
        const { start, end } = gridRangeFor(year, month);
        const weeks = Math.round((end.getTime() - start.getTime()) / (7 * 86400000));
        const cells = daysBetween(start, weeks * 7);
        return (
          <button
            type="button"
            key={month}
            className="calendar-year-month"
            onClick={() => onPickMonth(firstOfMonth)}
          >
            <div className="calendar-year-month-name">
              {firstOfMonth.toLocaleDateString(undefined, { month: "long" })}
            </div>
            <div className="calendar-year-month-days">
              {cells.map((day) => {
                const inMonth = day.getMonth() === month;
                return (
                  <div className={`calendar-year-day ${inMonth ? "" : "outside"}`} key={day.toISOString()}>
                    <span className={sameDay(day, today) ? "calendar-year-day-number today" : "calendar-year-day-number"}>
                      {day.getDate()}
                    </span>
                    {inMonth && eventDates.has(day.toDateString()) && <span className="calendar-year-day-dot" />}
                  </div>
                );
              })}
            </div>
          </button>
        );
      })}
    </div>
  );
}

export function Calendar() {
  const [auth, setAuth] = useState<CalendarAuth | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<Mode>("week");
  const [cursor, setCursor] = useState(() => new Date());

  const range = useMemo(() => {
    if (mode === "day") return { start: cursor, end: addDays(cursor, 1) };
    if (mode === "month") return monthGridRange(cursor);
    if (mode === "year") return { start: new Date(cursor.getFullYear(), 0, 1), end: new Date(cursor.getFullYear() + 1, 0, 1) };
    const start = startOfWeek(cursor);
    return { start, end: addDays(start, 7) };
  }, [mode, cursor]);

  useEffect(() => {
    api
      .calendarAuthStatus()
      .then(setAuth)
      .catch((err) => setError(String(err)));
  }, []);

  useEffect(() => {
    if (auth !== "Granted") return;
    api
      .calendarEvents(range.start, range.end)
      .then(setEvents)
      .catch((err) => setError(String(err)));
  }, [auth, range]);

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
  const step = mode === "day" ? 1 : mode === "week" ? 7 : 0;
  const goPrev = () =>
    setCursor((c) => (mode === "month" ? addMonths(c, -1) : mode === "year" ? addYears(c, -1) : addDays(c, -step)));
  const goNext = () =>
    setCursor((c) => (mode === "month" ? addMonths(c, 1) : mode === "year" ? addYears(c, 1) : addDays(c, step)));
  const goToday = () => setCursor(new Date());

  return (
    <div className="calendar-view">
      <div className="view-header calendar-header">
        <h1>{headerLabel(mode, cursor)}</h1>
        <div className="calendar-header-controls">
          <div className="calendar-nav">
            <button type="button" onClick={goPrev} aria-label="Previous">
              <ChevronLeft size={15} />
            </button>
            <button type="button" className="calendar-today-button" onClick={goToday}>
              Today
            </button>
            <button type="button" onClick={goNext} aria-label="Next">
              <ChevronRight size={15} />
            </button>
          </div>
          <div className="calendar-mode-toggle">
            {(["day", "week", "month", "year"] as const).map((m) => (
              <button
                type="button"
                key={m}
                className={mode === m ? "active" : ""}
                onClick={() => setMode(m)}
              >
                {m[0].toUpperCase() + m.slice(1)}
              </button>
            ))}
          </div>
        </div>
      </div>
      {error && <div className="calendar-error">{error}</div>}
      {mode === "year" ? (
        <YearGrid
          year={cursor.getFullYear()}
          events={events}
          today={today}
          onPickMonth={(month) => {
            setCursor(month);
            setMode("month");
          }}
        />
      ) : mode === "month" ? (
        <MonthGrid
          cursor={cursor}
          events={events}
          today={today}
          onPickDay={(day) => {
            setCursor(day);
            setMode("day");
          }}
        />
      ) : (
        <div className="calendar-week">
          {(mode === "day" ? [cursor] : daysBetween(startOfWeek(cursor), 7)).map((day) => (
            <DayColumn key={day.toISOString()} day={day} events={events} isToday={sameDay(day, today)} />
          ))}
        </div>
      )}
    </div>
  );
}
