import { useEffect, useMemo, useRef, useState } from "react";
import { motion } from "framer-motion";
import { CalendarDays, ChevronLeft, ChevronRight } from "lucide-react";
import { api } from "../lib/api";
import { openCalendarPrivacyPane } from "../lib/system";
import type { CalendarAuth, CalendarEvent, CalendarInfo } from "../lib/types";
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

// Picking white or black off the color's own HSL lightness — cheap and
// correct for the common case (some real calendar colors, pale yellow or
// light green, are too light for a fixed white label), without pulling in
// real WCAG contrast math for a text/background pair that's always exactly
// this one accent color underneath. Same rule the GPUI app's own
// readable_text_on uses.
function readableTextOn([r, g, b]: [number, number, number, number]): string {
  const lightness = (Math.max(r, g, b) + Math.min(r, g, b)) / 2;
  return lightness > 0.6 ? "#000" : "#fff";
}

const HOUR_HEIGHT = 48;
const DEFAULT_START_HOUR = 7;

function hourLabel(hour: number): string {
  if (hour === 0) return "12 AM";
  if (hour < 12) return `${hour} AM`;
  if (hour === 12) return "12 PM";
  return `${hour - 12} PM`;
}

// Greedy lane sweep: give each event the first lane whose previous occupant
// already ended by this event's start, else open a new lane. Same
// simplification the GPUI app's own render_calendar_grid_day_column keeps
// deliberate — overlapping events share a uniform lane width, not Apple's
// true interval-packing layout.
function assignLanes(events: CalendarEvent[]): { event: CalendarEvent; lane: number; laneCount: number }[] {
  const sorted = [...events].sort((a, b) => a.start.localeCompare(b.start));
  const laneEnd: number[] = [];
  const placed = sorted.map((event) => {
    const start = new Date(event.start).getTime();
    const end = new Date(event.end).getTime();
    let lane = laneEnd.findIndex((laneEndTime) => laneEndTime <= start);
    if (lane === -1) {
      lane = laneEnd.length;
      laneEnd.push(end);
    } else {
      laneEnd[lane] = end;
    }
    return { event, lane };
  });
  const laneCount = Math.max(laneEnd.length, 1);
  return placed.map(({ event, lane }) => ({ event, lane, laneCount }));
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

// A real time-grid week: a fixed hour gutter, one column per day, all-day
// events in their own strip above the grid, timed events absolutely
// positioned by time-of-day and duration — mirrors the GPUI app's own
// render_calendar_week_grid. Day mode deliberately keeps the simpler
// agenda-per-day DayColumn instead (the GPUI app's own comment: Day kept
// its Kanban-board look on purpose when Week moved to a real grid).
function WeekTimeGrid({ days, events, today }: { days: Date[]; events: CalendarEvent[]; today: Date }) {
  const scrollRef = useRef<HTMLDivElement>(null);

  // Jump to a sensible starting hour on mount instead of opening on
  // midnight — mostly empty for almost everyone. Only once per mount,
  // same "seed it, then let the user's own scrolling take over" reasoning
  // the GPUI app's own calendar_week_scrolled_once flag uses.
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: DEFAULT_START_HOUR * HOUR_HEIGHT });
  }, []);

  const allDayByDay = days.map((day) => events.filter((event) => event.all_day && sameDay(new Date(event.start), day)));
  const hasAllDay = allDayByDay.some((dayEvents) => dayEvents.length > 0);

  return (
    <div className="calendar-grid">
      <div className="calendar-grid-header">
        <div className="calendar-grid-gutter" />
        {days.map((day) => (
          <div className="calendar-grid-header-cell" key={day.toISOString()}>
            <span className="calendar-grid-header-weekday">{day.toLocaleDateString(undefined, { weekday: "short" })}</span>
            <span className={sameDay(day, today) ? "calendar-grid-header-number today" : "calendar-grid-header-number"}>
              {day.getDate()}
            </span>
          </div>
        ))}
      </div>
      {hasAllDay && (
        <div className="calendar-grid-all-day">
          <div className="calendar-grid-gutter" />
          {allDayByDay.map((dayEvents, i) => (
            <div className="calendar-grid-all-day-cell" key={days[i].toISOString()}>
              {dayEvents.map((event) => (
                <div
                  className="calendar-grid-all-day-event"
                  key={event.id}
                  style={{ background: colorCss(event.color), color: readableTextOn(event.color) }}
                >
                  {event.title}
                </div>
              ))}
            </div>
          ))}
        </div>
      )}
      <div className="calendar-grid-body" ref={scrollRef}>
        <div className="calendar-grid-gutter">
          {Array.from({ length: 24 }, (_, hour) => (
            <div className="calendar-grid-hour-row" key={hour}>
              <span className="calendar-grid-hour-label">{hourLabel(hour)}</span>
            </div>
          ))}
        </div>
        {days.map((day) => {
          const dayEvents = events.filter((event) => !event.all_day && sameDay(new Date(event.start), day));
          const midnight = new Date(day);
          midnight.setHours(0, 0, 0, 0);
          return (
            <div className="calendar-grid-day-column" key={day.toISOString()}>
              {Array.from({ length: 24 }, (_, hour) => (
                <div className="calendar-grid-hour-row" key={hour} />
              ))}
              {assignLanes(dayEvents).map(({ event, lane, laneCount }) => {
                const startMinutes = Math.max(0, (new Date(event.start).getTime() - midnight.getTime()) / 60000);
                const durationMinutes = Math.max(15, (new Date(event.end).getTime() - new Date(event.start).getTime()) / 60000);
                const top = (startMinutes / 60) * HOUR_HEIGHT;
                const height = Math.max(18, (durationMinutes / 60) * HOUR_HEIGHT);
                const textColor = readableTextOn(event.color);
                return (
                  <div
                    className="calendar-grid-event"
                    key={event.id}
                    style={{
                      top,
                      height,
                      left: `${(lane / laneCount) * 100}%`,
                      width: `${(1 / laneCount) * 100}%`,
                    }}
                  >
                    <div className="calendar-grid-event-card" style={{ background: colorCss(event.color), color: textColor }}>
                      <span className="calendar-grid-event-title">{event.title}</span>
                      {height >= 32 && (
                        <span className="calendar-grid-event-time">
                          {new Date(event.start).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          );
        })}
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

// Mirrors the GPUI app's own render_calendar_sidebar: grouped by account
// (source_title), matching Apple Calendar's own sidebar sectioning. Each
// row toggles that calendar's events on/off — shown = filled dot, hidden =
// hollow (a shape change, not just a dimmer color, so on/off reads without
// relying on contrast sensitivity — CLAUDE.md: "never encode meaning in
// color alone"). The calendar's own color stays the dot's border either
// way, so which calendar this is never disappears with it.
function CalendarSidebar({
  calendars,
  hiddenIds,
  onToggle,
}: {
  calendars: CalendarInfo[];
  hiddenIds: Set<string>;
  onToggle: (id: string) => void;
}) {
  const groups = useMemo(() => {
    const map = new Map<string, CalendarInfo[]>();
    for (const calendar of calendars) {
      const key = calendar.source_title || "Other";
      const list = map.get(key) ?? [];
      list.push(calendar);
      map.set(key, list);
    }
    return [...map.entries()];
  }, [calendars]);

  return (
    <div className="calendar-sidebar">
      {groups.map(([source, group]) => (
        <div className="calendar-sidebar-group" key={source}>
          <div className="calendar-sidebar-group-label">{source}</div>
          {group.map((calendar) => {
            const hidden = hiddenIds.has(calendar.id);
            return (
              <button
                type="button"
                key={calendar.id}
                className="calendar-sidebar-row"
                onClick={() => onToggle(calendar.id)}
              >
                <span
                  className={`calendar-sidebar-dot ${hidden ? "" : "filled"}`}
                  style={{ borderColor: colorCss(calendar.color), background: hidden ? "transparent" : colorCss(calendar.color) }}
                />
                <span className={hidden ? "calendar-sidebar-title hidden" : "calendar-sidebar-title"}>
                  {calendar.title}
                </span>
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
}

export function Calendar() {
  const [auth, setAuth] = useState<CalendarAuth | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [calendars, setCalendars] = useState<CalendarInfo[]>([]);
  const [hiddenIds, setHiddenIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<Mode>("week");
  const [cursor, setCursor] = useState(() => new Date());

  const visibleEvents = useMemo(
    () => events.filter((event) => !hiddenIds.has(event.calendar_id)),
    [events, hiddenIds],
  );

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

  useEffect(() => {
    if (auth !== "Granted") return;
    api
      .calendarList()
      .then(setCalendars)
      .catch((err) => setError(String(err)));
  }, [auth]);

  const toggleCalendar = (id: string) => {
    setHiddenIds((prev) => {
      const next = new Set(prev);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  };

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
        {auth === "Denied" ? (
          <button
            className="calendar-connect-button"
            onClick={() => openCalendarPrivacyPane().catch((err) => setError(String(err)))}
          >
            Open System Settings
          </button>
        ) : (
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
    <div className="calendar-shell">
      <CalendarSidebar calendars={calendars} hiddenIds={hiddenIds} onToggle={toggleCalendar} />
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
            events={visibleEvents}
            today={today}
            onPickMonth={(month) => {
              setCursor(month);
              setMode("month");
            }}
          />
        ) : mode === "month" ? (
          <MonthGrid
            cursor={cursor}
            events={visibleEvents}
            today={today}
            onPickDay={(day) => {
              setCursor(day);
              setMode("day");
            }}
          />
        ) : mode === "week" ? (
          <WeekTimeGrid days={daysBetween(startOfWeek(cursor), 7)} events={visibleEvents} today={today} />
        ) : (
          <div className="calendar-week">
            <DayColumn day={cursor} events={visibleEvents} isToday={sameDay(cursor, today)} />
          </div>
        )}
      </div>
    </div>
  );
}
