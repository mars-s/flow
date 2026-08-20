import { useEffect, useState } from "react";
import { Sparkles, Loader2, RotateCw } from "lucide-react";
import { api } from "../lib/api";
import { ai } from "../lib/ai";
import { useAiConfig, useAiFeatureState } from "../lib/aiConfig";
import { usePersistedString } from "../lib/persisted";
import { todayIso } from "../lib/date";
import "./TodayBriefing.css";

type Cache = { date: string; text: string };

function readCache(raw: string): Cache | null {
  try {
    return JSON.parse(raw) as Cache;
  } catch {
    return null;
  }
}

// The first real (non-scaffolding) AI block: a short summary of today's
// tasks + calendar events, gated behind the master AI switch and its own
// per-feature Off/Manual/Auto state (Settings → AI features → Calendar).
// Lives in the Calendar tab itself — direct user placement ("today
// summary should live under calendar block"), not the Today task view
// (CalendarGlance already covers that with real, non-AI data).
export function TodayBriefing() {
  const { enabled, baseUrl, apiKey, model } = useAiConfig();
  const [mode] = useAiFeatureState("today-briefing");
  const [cacheRaw, setCacheRaw] = usePersistedString("flow.ai.todayBriefing.cache", "");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const cache = readCache(cacheRaw);
  const today = todayIso();
  const text = cache?.date === today ? cache.text : null;

  const generate = () => {
    setLoading(true);
    setError(null);
    Promise.all([api.listView("Today"), api.calendarAuthStatus()])
      .then(async ([tasks, auth]) => {
        let eventLines: string[] = [];
        if (auth === "Granted") {
          const start = new Date();
          start.setHours(0, 0, 0, 0);
          const end = new Date(start);
          end.setDate(start.getDate() + 1);
          const events = await api.calendarEvents(start, end);
          eventLines = events.map(
            (event) =>
              `- ${event.all_day ? "All day" : new Date(event.start).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}: ${event.title}`,
          );
        }
        const taskLines = tasks.map((task) => `- ${task.title}`);
        const user = [
          `Today's tasks (${tasks.length}):`,
          taskLines.length > 0 ? taskLines.join("\n") : "(none)",
          "",
          `Today's calendar events (${eventLines.length}):`,
          eventLines.length > 0 ? eventLines.join("\n") : "(none)",
        ].join("\n");
        return ai.chatCompletion(
          baseUrl,
          apiKey,
          model,
          "You write a short, calm, two-to-three sentence briefing for someone's day ahead, given their task " +
            "list and calendar events. No greeting, no sign-off, no markdown — plain prose. Mention real " +
            "conflicts or a packed day if there genuinely is one; otherwise just orient them.",
          user,
        );
      })
      .then((result) => {
        setCacheRaw(JSON.stringify({ date: today, text: result } satisfies Cache));
        setLoading(false);
      })
      .catch((err) => {
        setError(String(err));
        setLoading(false);
      });
  };

  // Auto mode generates itself once per day — a cache hit for today
  // means it already ran, so this only ever fires the first time the
  // Calendar tab is visited on a given day. Requires apiKey/model to
  // actually be set, not just baseUrl (which always has a default) —
  // real bug found by re-checking this against a fresh "AI on, Today
  // briefing set to Auto, nothing configured yet" state: without this
  // guard, every single visit to the Calendar tab fired a doomed API
  // call against api.openai.com with no key, failing the same way
  // every time with no way to stop it short of turning the feature off.
  useEffect(() => {
    if (enabled && mode === "auto" && apiKey && model && !text && !loading) generate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, mode, apiKey, model, text]);

  if (!enabled || mode === "off") return null;

  return (
    <div className="today-briefing">
      <Sparkles size={13} className="today-briefing-icon" />
      <div className="today-briefing-body">
        {text ? (
          <div className="today-briefing-text">{text}</div>
        ) : loading ? (
          <div className="today-briefing-text muted">Thinking…</div>
        ) : error ? (
          <div className="today-briefing-text error">{error}</div>
        ) : (
          <div className="today-briefing-text muted">No briefing yet today.</div>
        )}
      </div>
      {mode === "manual" && (
        <button type="button" className="today-briefing-button" onClick={generate} disabled={loading}>
          {loading ? <Loader2 size={12} className="ai-spin" /> : <RotateCw size={12} />}
          {text ? "Regenerate" : "Generate"}
        </button>
      )}
    </div>
  );
}
