import { useEffect, useState } from "react";
import { Sparkles, Loader2, Clock } from "lucide-react";
import { api } from "../lib/api";
import { ai } from "../lib/ai";
import { useAiConfig, useAiFeatureState } from "../lib/aiConfig";
import { computeFreeSlots, formatSlotLabel, minutesToTime, type FreeSlot } from "../lib/freeSlots";
import { todayIso } from "../lib/date";
import type { CalendarAuth } from "../lib/types";
import "./SmartScheduling.css";

const SYSTEM_PROMPT =
  "Given a task's title and a numbered list of free calendar slots today, pick the single best slot for " +
  "it — consider what the task likely needs (a quick errand fits a short gap, focused work fits a long one, " +
  "an early task might suit a morning slot). Respond with ONLY JSON, no prose, no markdown: " +
  '{"slot_index": 0, "reason": "one short sentence"}. slot_index must be one of the listed numbers.';

type Suggestion = { slot: FreeSlot; reason: string };

function parseSuggestion(raw: string, slots: FreeSlot[]): Suggestion | null {
  try {
    const parsed = JSON.parse(raw) as { slot_index?: unknown; reason?: unknown };
    const index = Number(parsed.slot_index);
    if (!Number.isInteger(index) || index < 0 || index >= slots.length) return null;
    return { slot: slots[index], reason: typeof parsed.reason === "string" ? parsed.reason : "" };
  } catch {
    return null;
  }
}

// Seventh and final AI-backlog block, and the last remaining one from
// Settings' original list. Only shown for a task with no scheduled_date
// yet — scheduling one that's already scheduled would be a rescheduling
// decision this block isn't built for (Overdue reschedule already owns
// pushing a date; this one owns picking a same-day time from real
// gaps). The free-slot math itself is exact local interval arithmetic
// (lib/freeSlots.ts); the model's only job is judgment — which slot
// actually fits the task — same "model judges, code computes" split
// Overdue reschedule already established, just picking an index into a
// known list instead of a day offset, so a hallucinated response can't
// produce a time that was never actually free.
export function SmartScheduling({
  taskId,
  title,
  scheduledDate,
  onSchedule,
}: {
  taskId: string;
  title: string;
  scheduledDate: string | null;
  onSchedule: (id: string, date: string, time: string) => void;
}) {
  const { enabled, baseUrl, apiKey, model } = useAiConfig();
  const [mode] = useAiFeatureState("smart-scheduling");
  const [auth, setAuth] = useState<CalendarAuth | null>(null);
  const [slots, setSlots] = useState<FreeSlot[] | null>(null);
  const [suggestion, setSuggestion] = useState<Suggestion | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [autoRanFor, setAutoRanFor] = useState<string | null>(null);

  const active = enabled && mode !== "off" && !scheduledDate;

  useEffect(() => {
    if (!active) return;
    api.calendarAuthStatus().then(setAuth).catch(() => setAuth("Unavailable"));
  }, [active]);

  useEffect(() => {
    if (!active || auth !== "Granted") return;
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    const end = new Date(start);
    end.setDate(start.getDate() + 1);
    api
      .calendarEvents(start, end)
      .then((events) => setSlots(computeFreeSlots(events)))
      .catch(() => setSlots([]));
  }, [active, auth]);

  const generate = () => {
    if (!slots || slots.length === 0) return;
    setLoading(true);
    setError(null);
    const list = slots.map((slot, index) => `${index}. ${formatSlotLabel(slot)}`).join("\n");
    ai.chatCompletion(baseUrl, apiKey, model, SYSTEM_PROMPT, `Task: ${title}\n\nFree slots today:\n${list}`)
      .then((result) => {
        setLoading(false);
        const parsed = parseSuggestion(result, slots);
        if (!parsed) {
          setError("Couldn't parse a suggestion");
          return;
        }
        setSuggestion(parsed);
      })
      .catch((err) => {
        setLoading(false);
        setError(String(err));
      });
  };

  useEffect(() => {
    if (
      active &&
      mode === "auto" &&
      apiKey &&
      model &&
      slots &&
      slots.length > 0 &&
      autoRanFor !== taskId &&
      !loading
    ) {
      setAutoRanFor(taskId);
      setLoading(true);
      setError(null);
      const list = slots.map((slot, index) => `${index}. ${formatSlotLabel(slot)}`).join("\n");
      ai.chatCompletion(baseUrl, apiKey, model, SYSTEM_PROMPT, `Task: ${title}\n\nFree slots today:\n${list}`)
        .then((result) => {
          setLoading(false);
          const parsed = parseSuggestion(result, slots);
          if (parsed) onSchedule(taskId, todayIso(), minutesToTime(parsed.slot.startMinutes));
        })
        .catch((err) => {
          setLoading(false);
          setError(String(err));
        });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, mode, apiKey, model, slots, taskId]);

  if (!active || auth !== "Granted" || !slots || slots.length === 0) return null;
  if (mode === "auto") return loading ? <div className="smart-scheduling-auto-status">Finding a time…</div> : null;

  return (
    <div className="smart-scheduling">
      {suggestion ? (
        <div className="smart-scheduling-preview ai-surface">
          <div className="smart-scheduling-slot">
            <Clock size={12} />
            Today, {formatSlotLabel(suggestion.slot)}
          </div>
          {suggestion.reason && <div className="smart-scheduling-reason">{suggestion.reason}</div>}
          <div className="smart-scheduling-actions">
            <button type="button" className="smart-scheduling-dismiss" onClick={() => setSuggestion(null)}>
              Dismiss
            </button>
            <button
              type="button"
              className="smart-scheduling-accept"
              onClick={() => {
                onSchedule(taskId, todayIso(), minutesToTime(suggestion.slot.startMinutes));
                setSuggestion(null);
              }}
            >
              Schedule
            </button>
          </div>
        </div>
      ) : (
        <button type="button" className="pill" onClick={generate} disabled={loading}>
          {loading ? <Loader2 size={11} className="ai-spin" /> : <Sparkles size={11} />}
          Suggest a time
        </button>
      )}
      {error && <div className="smart-scheduling-error">{error}</div>}
    </div>
  );
}
