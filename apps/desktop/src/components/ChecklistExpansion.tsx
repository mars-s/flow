import { useEffect, useState } from "react";
import { Sparkles, Loader2 } from "lucide-react";
import { ai } from "../lib/ai";
import { useAiConfig, useAiFeatureState } from "../lib/aiConfig";
import "./ChecklistExpansion.css";

type Props = {
  taskId: string;
  title: string;
  note: string | null;
  hasSubtasks: boolean;
  onAddSubtask: (title: string) => void;
};

const SYSTEM_PROMPT =
  "Given a task's title and optional note, suggest a short checklist to break it down into concrete steps. " +
  "Respond with ONLY a JSON array of short strings (2 to 6 items), no prose, no markdown, no numbering — " +
  'just e.g. ["Book flights", "Pack bags", "Print boarding passes"]. If the task is already a single ' +
  "concrete step that doesn't need breaking down, respond with an empty array: [].";

// Tries a plain JSON.parse first (the prompt asks for exactly this); falls
// back to reading one item per non-empty line, stripping a leading
// bullet/number, for a model that ignores the "JSON only" instruction.
function parseSuggestions(raw: string): string[] {
  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed.filter((item): item is string => typeof item === "string");
  } catch {
    // fall through to the line-based fallback below
  }
  return raw
    .split("\n")
    .map((line) => line.replace(/^[\s\-*\d.)]+/, "").trim())
    .filter((line) => line.length > 0);
}

// The second real AI block (Today briefing was the first): suggests a
// subtask breakdown for a task's own title/note. Manual mode shows the
// suggestions as a preview the user explicitly adds; Auto mode writes
// them straight in — the same Off/Manual/Auto distinction every block
// shares, and for this one specifically "Auto... writes immediately" is
// a real, deliberate behavior difference the user spelled out directly
// (checklist expansion is the one block where "auto" actually creates
// data, not just displays it, unlike Today briefing).
export function ChecklistExpansion({ taskId, title, note, hasSubtasks, onAddSubtask }: Props) {
  const { enabled, baseUrl, apiKey, model } = useAiConfig();
  const [mode] = useAiFeatureState("checklist-expansion");
  const [suggestions, setSuggestions] = useState<string[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [autoRanFor, setAutoRanFor] = useState<string | null>(null);

  const generate = () => {
    setLoading(true);
    setError(null);
    const user = note ? `Title: ${title}\nNote: ${note}` : `Title: ${title}`;
    ai.chatCompletion(baseUrl, apiKey, model, SYSTEM_PROMPT, user)
      .then((result) => {
        setLoading(false);
        setSuggestions(parseSuggestions(result));
      })
      .catch((err) => {
        setLoading(false);
        setError(String(err));
      });
  };

  // Auto mode runs once per task, the first time its card is expanded
  // with no subtasks yet — the closest available proxy for "a task was
  // just created" without a separate creation-time hook — and writes
  // the suggestions straight in with no confirmation step, per the
  // user's own explicit spec for what "fully auto" means. Requires
  // apiKey/model actually set — same real bug as Today briefing's own
  // auto mode: without this, expanding any task with Auto on but
  // nothing configured fired a doomed API call every time.
  useEffect(() => {
    if (enabled && mode === "auto" && apiKey && model && !hasSubtasks && autoRanFor !== taskId && !loading) {
      setAutoRanFor(taskId);
      setLoading(true);
      setError(null);
      const user = note ? `Title: ${title}\nNote: ${note}` : `Title: ${title}`;
      ai.chatCompletion(baseUrl, apiKey, model, SYSTEM_PROMPT, user)
        .then((result) => {
          setLoading(false);
          for (const item of parseSuggestions(result)) onAddSubtask(item);
        })
        .catch((err) => {
          setLoading(false);
          setError(String(err));
        });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, mode, apiKey, model, hasSubtasks, taskId]);

  if (!enabled || mode === "off" || hasSubtasks) return null;
  // Auto mode has nothing of its own to show — it already wrote the
  // subtasks in, which render through the normal checklist UI.
  if (mode === "auto") return loading ? <div className="checklist-expansion-auto-status">Suggesting…</div> : null;

  return (
    <div className="checklist-expansion">
      {suggestions === null ? (
        <button type="button" className="pill" onClick={generate} disabled={loading}>
          {loading ? <Loader2 size={11} className="ai-spin" /> : <Sparkles size={11} />}
          Suggest checklist
        </button>
      ) : suggestions.length === 0 ? (
        <div className="checklist-expansion-empty">No breakdown suggested — looks like one step already.</div>
      ) : (
        <div className="checklist-expansion-preview">
          <div className="checklist-expansion-preview-label">Suggested checklist</div>
          {suggestions.map((item, index) => (
            <div className="checklist-expansion-item" key={index}>
              {item}
            </div>
          ))}
          <div className="checklist-expansion-preview-actions">
            <button
              type="button"
              className="checklist-expansion-dismiss"
              onClick={() => setSuggestions(null)}
            >
              Dismiss
            </button>
            <button
              type="button"
              className="checklist-expansion-add"
              onClick={() => {
                for (const item of suggestions) onAddSubtask(item);
                setSuggestions(null);
              }}
            >
              Add all
            </button>
          </div>
        </div>
      )}
      {error && <div className="checklist-expansion-error">{error}</div>}
    </div>
  );
}
