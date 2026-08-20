import { useEffect, useState } from "react";
import { Sparkles, Loader2, CalendarClock } from "lucide-react";
import { ai } from "../lib/ai";
import { useAiConfig, useAiFeatureState } from "../lib/aiConfig";
import { addDaysIso, dayLabel, todayIso } from "../lib/date";
import type { Task } from "../lib/types";
import "./OverdueReschedule.css";

const SYSTEM_PROMPT =
  "Given a batch of overdue tasks, each with how many days overdue it is, suggest a single day offset " +
  "from today (an integer from 1 to 14) to reschedule the whole batch to at once, plus a short one-sentence " +
  "note explaining the choice — favor tomorrow for a small, recent batch and a bit further out for a large " +
  'or very stale one. Respond with ONLY JSON, no prose, no markdown: {"days_from_today": 1, "note": "..."}';

type Suggestion = { date: string; note: string };

function parseSuggestion(raw: string): Suggestion | null {
  try {
    const parsed = JSON.parse(raw) as { days_from_today?: unknown; note?: unknown };
    const days = Number(parsed.days_from_today);
    if (!Number.isFinite(days) || days < 1 || days > 30) return null;
    return { date: addDaysIso(Math.round(days)), note: typeof parsed.note === "string" ? parsed.note : "" };
  } catch {
    return null;
  }
}

function daysOverdue(scheduledDate: string): number {
  const today = todayIso();
  return Math.max(1, Math.round((Date.parse(today) - Date.parse(scheduledDate)) / 86_400_000));
}

// Fifth AI block: overdue tasks sit in Today's own list (PRD: Today
// shows overdue first, then today's own tasks) rather than a separate
// view, so this renders inline above them instead of needing its own
// page. Manual mode previews the suggested date and requires an
// explicit "Reschedule all"; Auto writes the new date to every overdue
// task the first time the batch is seen, same "auto writes immediately"
// behavior Checklist expansion and Stale task nudges already establish
// for blocks that change data rather than just display it.
export function OverdueReschedule({
  tasks,
  onRescheduleAll,
}: {
  tasks: Task[];
  onRescheduleAll: (ids: string[], date: string) => void;
}) {
  const { enabled, baseUrl, apiKey, model } = useAiConfig();
  const [mode] = useAiFeatureState("overdue-reschedule");
  const [suggestion, setSuggestion] = useState<Suggestion | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [autoRanFor, setAutoRanFor] = useState<string | null>(null);

  const overdue = tasks.filter((task) => task.scheduled_date && task.scheduled_date < todayIso());
  const idsKey = overdue
    .map((task) => task.id)
    .sort()
    .join(",");

  const generate = (onDone?: (result: Suggestion) => void) => {
    setLoading(true);
    setError(null);
    const lines = overdue.map((task) => `- "${task.title}" (${daysOverdue(task.scheduled_date!)} days overdue)`).join("\n");
    ai.chatCompletion(baseUrl, apiKey, model, SYSTEM_PROMPT, `Overdue tasks:\n${lines}`)
      .then((result) => {
        setLoading(false);
        const parsed = parseSuggestion(result);
        if (!parsed) {
          setError("Couldn't parse a suggestion");
          return;
        }
        setSuggestion(parsed);
        onDone?.(parsed);
      })
      .catch((err) => {
        setLoading(false);
        setError(String(err));
      });
  };

  useEffect(() => {
    if (enabled && mode === "auto" && apiKey && model && overdue.length > 0 && autoRanFor !== idsKey && !loading) {
      setAutoRanFor(idsKey);
      generate((result) => onRescheduleAll(overdue.map((task) => task.id), result.date));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, mode, apiKey, model, idsKey]);

  if (!enabled || mode === "off" || overdue.length === 0) return null;
  if (mode === "auto") return loading ? <div className="overdue-reschedule-auto-status">Rescheduling…</div> : null;

  return (
    <div className="overdue-reschedule">
      <CalendarClock size={13} className="overdue-reschedule-icon" />
      <div className="overdue-reschedule-body">
        <div className="overdue-reschedule-title">
          {overdue.length} overdue task{overdue.length === 1 ? "" : "s"}
        </div>
        {suggestion ? (
          <div className="overdue-reschedule-preview">
            <span className="overdue-reschedule-date">Move all to {dayLabel(suggestion.date)}</span>
            {suggestion.note && <span className="overdue-reschedule-note">{suggestion.note}</span>}
          </div>
        ) : error ? (
          <div className="overdue-reschedule-text error">{error}</div>
        ) : (
          <div className="overdue-reschedule-text muted">No suggestion yet.</div>
        )}
      </div>
      {suggestion ? (
        <button
          type="button"
          className="overdue-reschedule-button"
          onClick={() => {
            onRescheduleAll(overdue.map((task) => task.id), suggestion.date);
            setSuggestion(null);
          }}
        >
          Reschedule all
        </button>
      ) : (
        <button type="button" className="overdue-reschedule-button" onClick={() => generate()} disabled={loading}>
          {loading ? <Loader2 size={12} className="ai-spin" /> : <Sparkles size={12} />}
          Suggest
        </button>
      )}
    </div>
  );
}
