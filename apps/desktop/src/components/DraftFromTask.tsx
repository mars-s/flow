import { useEffect, useState } from "react";
import { Sparkles, Loader2, Copy, Check } from "lucide-react";
import { ai } from "../lib/ai";
import { useAiConfig, useAiFeatureState } from "../lib/aiConfig";
import "./DraftFromTask.css";

const SYSTEM_PROMPT =
  "Given a task's title and optional note, draft a short message someone could send about it — e.g. asking " +
  "someone else to do it, following up, or letting someone know it's done. Judge the likeliest intent from " +
  "the title/note. Plain text, no subject line, no signature, 1-4 sentences. Respond with ONLY the message.";

// Sixth AI block, and — like Today briefing — display-only: there's no
// task field a drafted message writes into, so unlike Checklist
// expansion or Overdue reschedule, "Auto" here just means "generate it
// without being asked" rather than "write it in with no confirmation."
// Nothing to confirm because nothing gets saved; the user copies it out
// by hand if they want it.
export function DraftFromTask({ taskId, title, note }: { taskId: string; title: string; note: string | null }) {
  const { enabled, baseUrl, apiKey, model } = useAiConfig();
  const [mode] = useAiFeatureState("draft-from-task");
  const [draft, setDraft] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [autoRanFor, setAutoRanFor] = useState<string | null>(null);

  const generate = () => {
    setLoading(true);
    setError(null);
    setCopied(false);
    const user = note ? `Title: ${title}\nNote: ${note}` : `Title: ${title}`;
    ai.chatCompletion(baseUrl, apiKey, model, SYSTEM_PROMPT, user)
      .then((result) => {
        setLoading(false);
        setDraft(result.trim());
      })
      .catch((err) => {
        setLoading(false);
        setError(String(err));
      });
  };

  // Resets whenever the card switches to a different task, so a stale
  // draft from the previously expanded task doesn't linger under a new
  // one's title.
  useEffect(() => {
    setDraft(null);
    setError(null);
    setCopied(false);
  }, [taskId]);

  useEffect(() => {
    if (enabled && mode === "auto" && apiKey && model && autoRanFor !== taskId && !loading) {
      setAutoRanFor(taskId);
      generate();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, mode, apiKey, model, taskId]);

  if (!enabled || mode === "off") return null;

  const copy = () => {
    if (!draft) return;
    navigator.clipboard.writeText(draft).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };

  return (
    <div className="draft-from-task">
      {draft ? (
        <div className="draft-from-task-preview">
          <div className="draft-from-task-text">{draft}</div>
          <button type="button" className="draft-from-task-copy" onClick={copy}>
            {copied ? <Check size={12} /> : <Copy size={12} />}
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
      ) : (
        <button type="button" className="pill" onClick={generate} disabled={loading}>
          {loading ? <Loader2 size={11} className="ai-spin" /> : <Sparkles size={11} />}
          Draft a message
        </button>
      )}
      {error && <div className="draft-from-task-error">{error}</div>}
    </div>
  );
}
