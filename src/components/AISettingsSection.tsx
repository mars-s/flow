import { useState } from "react";
import { Sparkles, Loader2, Check, X } from "lucide-react";
import { useAiConfig, useAiFeatureState } from "../lib/aiConfig";
import { ai } from "../lib/ai";
import { AIFeatureToggle } from "./AIFeatureToggle";
import "./AISettingsSection.css";

// Every AI feature this app has, grouped by the area of the app they
// show up in ("shiny blocks around the app," direct user framing) — one
// row per block, each with its own independent Off/Manual/Auto state.
// Today briefing, checklist expansion, stale task nudges, and overdue
// batch reschedule are wired to a real model call so far; duplicate
// detection is live too but runs as a local string-similarity check
// (see lib/similarity.ts) rather than a model call. The rest are the
// backlog the user asked to keep visible rather than dropped once
// brainstormed.
const FEATURE_GROUPS: { group: string; features: { id: string; name: string; description: string; live: boolean }[] }[] = [
  {
    group: "Calendar",
    features: [
      {
        id: "today-briefing",
        name: "Today briefing",
        description: "A short summary of today's tasks and calendar events.",
        live: true,
      },
    ],
  },
  {
    group: "Tasks",
    features: [
      {
        id: "checklist-expansion",
        name: "Checklist expansion",
        description: "Suggest a subtask breakdown for a vague task title.",
        live: true,
      },
      {
        id: "smart-scheduling",
        name: "Smart scheduling",
        description: "Suggest open slots for unscheduled tasks from real calendar gaps.",
        live: false,
      },
      {
        id: "stale-task-nudges",
        name: "Stale task nudges",
        description: "Flag Inbox/Anytime tasks that have sat untouched a long time.",
        live: true,
      },
      {
        id: "duplicate-detection",
        name: "Duplicate detection",
        description: "Catch near-duplicate task titles before they're created twice.",
        live: true,
      },
      {
        id: "overdue-reschedule",
        name: "Overdue batch reschedule",
        description: "One suggested new date for a pile of overdue tasks at once.",
        live: true,
      },
      {
        id: "draft-from-task",
        name: "Draft from task",
        description: "Draft a message from a task's title and note.",
        live: false,
      },
    ],
  },
];

function FeatureRow({ id, name, description, live }: { id: string; name: string; description: string; live: boolean }) {
  const [state, setState] = useAiFeatureState(id);
  return (
    <div className="ai-feature-row">
      <div className="ai-feature-row-body">
        <div className="ai-feature-row-name">
          {name}
          {!live && <span className="ai-feature-row-badge">Backlog</span>}
        </div>
        <div className="ai-settings-row-note">{description}</div>
      </div>
      <AIFeatureToggle value={state} onChange={setState} />
    </div>
  );
}

export function AISettingsSection() {
  const { enabled, setEnabled, baseUrl, setBaseUrl, apiKey, setApiKey, model, setModel } = useAiConfig();
  const [claudeNote, setClaudeNote] = useState(false);
  const [models, setModels] = useState<string[] | null>(null);
  const [testState, setTestState] = useState<"idle" | "testing" | "ok" | "error">("idle");
  const [testError, setTestError] = useState<string | null>(null);

  const testConnection = () => {
    setTestState("testing");
    setTestError(null);
    ai.listModels(baseUrl, apiKey)
      .then((list) => {
        setModels(list);
        setTestState("ok");
        if (!model && list.length > 0) setModel(list[0]);
      })
      .catch((error) => {
        setModels(null);
        setTestState("error");
        setTestError(String(error));
      });
  };

  return (
    <div className="settings-section">
      <div className="settings-row">
        <div className="settings-row-icon">
          <Sparkles size={16} />
        </div>
        <div className="settings-row-body">
          <div className="settings-row-title">AI features</div>
          <div className="settings-row-note">
            Off by default. "Today briefing," "Checklist expansion," "Stale task nudges," and "Overdue batch
            reschedule" are wired to a real model call; "Duplicate detection" runs as a local check instead —
            no model call needed for string similarity. Everything else below is the backlog, kept visible
            rather than dropped once brainstormed.
          </div>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={enabled}
          className={enabled ? "ai-switch on" : "ai-switch"}
          onClick={() => setEnabled(!enabled)}
        >
          <span className="ai-switch-knob" />
        </button>
      </div>

      {enabled && (
        <div className="ai-settings-body">
          <div className="ai-settings-row">
            <div className="ai-settings-row-body">
              <div className="ai-settings-row-title">Claude account</div>
              <div className="ai-settings-row-note">
                Sign in with your Claude account for AI features that use it directly.
              </div>
              {claudeNote && (
                <div className="ai-settings-row-note ai-settings-row-note-warn">
                  Sign-in isn't wired up yet — this button is a placeholder for the real OAuth flow.
                </div>
              )}
            </div>
            <button type="button" className="settings-connect-button" onClick={() => setClaudeNote(true)}>
              Sign in with Claude
            </button>
          </div>

          <div className="ai-settings-row column">
            <div className="ai-settings-row-title">Custom OpenAI-compatible API</div>
            <div className="ai-settings-row-note">
              For a self-hosted or third-party endpoint instead of Claude. Stored locally on this device only.
            </div>
            <label className="ai-settings-field">
              <span>Base URL</span>
              <input
                type="text"
                value={baseUrl}
                onChange={(event) => {
                  setBaseUrl(event.target.value);
                  setTestState("idle");
                }}
                placeholder="https://api.openai.com/v1"
              />
            </label>
            <label className="ai-settings-field">
              <span>API key</span>
              <input
                type="password"
                value={apiKey}
                onChange={(event) => {
                  setApiKey(event.target.value);
                  setTestState("idle");
                }}
                placeholder="sk-…"
                autoComplete="off"
              />
            </label>
            <div className="ai-settings-test-row">
              {/* apiKey not required to enable this — a local, no-auth
                  server (Ollama, LM Studio) is an explicit target for
                  "custom OpenAI-compatible" (see ai.rs's own doc
                  comment), and requiring a key blocked testing those
                  entirely. ai_list_models still sends an empty bearer
                  token either way; most local servers just ignore it. */}
              <button
                type="button"
                className="settings-connect-button"
                onClick={testConnection}
                disabled={testState === "testing" || !baseUrl}
              >
                {testState === "testing" && <Loader2 size={12} className="ai-spin" />}
                {testState === "ok" && <Check size={12} />}
                {testState === "error" && <X size={12} />}
                Test connection
              </button>
              {testState === "ok" && models && (
                <span className="ai-settings-row-note">
                  {models.length} model{models.length === 1 ? "" : "s"} found
                </span>
              )}
            </div>
            {testState === "error" && testError && (
              <div className="ai-settings-row-note ai-settings-row-note-warn">{testError}</div>
            )}
            {models && models.length > 0 && (
              <label className="ai-settings-field">
                <span>Model</span>
                <select value={model} onChange={(event) => setModel(event.target.value)}>
                  {models.map((id) => (
                    <option key={id} value={id}>
                      {id}
                    </option>
                  ))}
                </select>
              </label>
            )}
          </div>

          <div className="ai-settings-row column">
            <div className="ai-settings-row-title">Features</div>
            <div className="ai-settings-row-note">
              Off skips the block entirely. Manual shows a button that generates on click. Auto runs on its own.
            </div>
            {FEATURE_GROUPS.map(({ group, features }) => (
              <div className="ai-feature-group" key={group}>
                <div className="ai-feature-group-label">{group}</div>
                {features.map((feature) => (
                  <FeatureRow key={feature.id} {...feature} />
                ))}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
