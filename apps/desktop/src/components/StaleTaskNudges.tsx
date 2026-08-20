import { useEffect, useState } from "react";
import { Sparkles, Loader2, RotateCw } from "lucide-react";
import { ai } from "../lib/ai";
import { useAiConfig, useAiFeatureState } from "../lib/aiConfig";
import { usePersistedString } from "../lib/persisted";
import { todayIso } from "../lib/date";
import type { Task } from "../lib/types";
import "./StaleTaskNudges.css";

const STALE_DAYS = 14;

type Cache = { date: string; ids: string; text: string };

function readCache(raw: string): Cache | null {
  try {
    return JSON.parse(raw) as Cache;
  } catch {
    return null;
  }
}

function daysSince(iso: string): number {
  return Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
}

// Third real AI block: flags Inbox/Anytime tasks that have sat untouched
// for STALE_DAYS+ (via created_at — no separate "last touched" field
// exists, so creation date is the only signal available) and asks the
// model for a short nudge on what to do with them. Detection itself is a
// plain date comparison, not an AI call — only the one-line nudge is.
export function StaleTaskNudges({ tasks }: { tasks: Task[] }) {
  const { enabled, baseUrl, apiKey, model } = useAiConfig();
  const [mode] = useAiFeatureState("stale-task-nudges");
  const [cacheRaw, setCacheRaw] = usePersistedString("flow.ai.staleNudges.cache", "");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const stale = tasks.filter((task) => daysSince(task.created_at) >= STALE_DAYS);
  // Cache keyed by today's date + which tasks are actually stale right
  // now (not just a count) — completing or deleting a stale task, or a
  // new one aging in, invalidates it correctly instead of showing a
  // nudge about tasks that already got handled.
  const idsKey = stale
    .map((task) => task.id)
    .sort()
    .join(",");
  const cache = readCache(cacheRaw);
  const today = todayIso();
  const text = cache?.date === today && cache.ids === idsKey ? cache.text : null;

  const generate = () => {
    setLoading(true);
    setError(null);
    const lines = stale.map((task) => `- "${task.title}" (${daysSince(task.created_at)} days old)`).join("\n");
    ai.chatCompletion(
      baseUrl,
      apiKey,
      model,
      "You write a single short, calm sentence nudging someone about task-list tasks that have sat " +
        "untouched a long time — suggest whether to defer to Someday, delete, or actually schedule them. " +
        "No greeting, no markdown, just one plain sentence.",
      `These tasks have sat untouched for a while:\n${lines}`,
    )
      .then((result) => {
        setCacheRaw(JSON.stringify({ date: today, ids: idsKey, text: result } satisfies Cache));
        setLoading(false);
      })
      .catch((err) => {
        setError(String(err));
        setLoading(false);
      });
  };

  useEffect(() => {
    if (enabled && mode === "auto" && apiKey && model && stale.length > 0 && !text && !loading) generate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, mode, apiKey, model, idsKey, text]);

  if (!enabled || mode === "off" || stale.length === 0) return null;

  return (
    <div className={`stale-nudges ai-surface${loading ? " is-generating" : ""}`}>
      <span className="ai-glyph">
        <Sparkles size={13} className="stale-nudges-icon" />
      </span>
      <div className="stale-nudges-body">
        <div className="stale-nudges-title">
          {stale.length} task{stale.length === 1 ? "" : "s"} sitting {STALE_DAYS}+ days
        </div>
        {text ? (
          <div className="stale-nudges-text">{text}</div>
        ) : loading ? (
          <div className="stale-nudges-text muted">Thinking…</div>
        ) : error ? (
          <div className="stale-nudges-text error">{error}</div>
        ) : mode === "manual" ? (
          <div className="stale-nudges-text muted">No nudge yet.</div>
        ) : null}
      </div>
      {mode === "manual" && (
        <button type="button" className="stale-nudges-button ai-action" onClick={generate} disabled={loading}>
          {loading ? <Loader2 size={12} className="ai-spin" /> : <RotateCw size={12} />}
          {text ? "Regenerate" : "Generate"}
        </button>
      )}
    </div>
  );
}
