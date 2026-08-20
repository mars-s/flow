import { useState } from "react";
import { Sparkles } from "lucide-react";
import { usePersistedBoolean, usePersistedString } from "../lib/persisted";
import "./AISettingsSection.css";

// Scaffolding for AI-related settings, direct user request — no AI feature
// actually calls any of this yet (no Anthropic OAuth client is registered
// for this app, no OpenAI-compatible request is ever sent). This section
// exists so the controls and their persisted values are in place before
// the features that read them are built: a master on/off switch (checked
// everywhere a future AI feature would gate itself), a Claude sign-in
// button (present, but honest about not being wired to a real OAuth flow
// yet rather than faking a signed-in state), and custom OpenAI-compatible
// endpoint/key fields for a self-hosted or third-party model later.
export function AISettingsSection() {
  const [enabled, setEnabled] = usePersistedBoolean("flow.ai.enabled", false);
  const [claudeNote, setClaudeNote] = useState(false);
  const [baseUrl, setBaseUrl] = usePersistedString("flow.ai.openaiBaseUrl", "https://api.openai.com/v1");
  const [apiKey, setApiKey] = usePersistedString("flow.ai.openaiApiKey", "");
  const [savedFlash, setSavedFlash] = useState(false);

  return (
    <div className="settings-section">
      <div className="settings-row">
        <div className="settings-row-icon">
          <Sparkles size={16} />
        </div>
        <div className="settings-row-body">
          <div className="settings-row-title">AI features</div>
          <div className="settings-row-note">
            Off by default. Nothing here is wired to a real AI feature yet — these are the controls future
            AI-related features will read and gate themselves on.
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
                onChange={(event) => setBaseUrl(event.target.value)}
                placeholder="https://api.openai.com/v1"
              />
            </label>
            <label className="ai-settings-field">
              <span>API key</span>
              <input
                type="password"
                value={apiKey}
                onChange={(event) => setApiKey(event.target.value)}
                placeholder="sk-…"
                autoComplete="off"
              />
            </label>
            <div className="ai-settings-save-row">
              {/* usePersistedString already writes to localStorage on
                  every keystroke — this button is confirmation feedback
                  for the user, not what actually saves the value. */}
              <button
                type="button"
                className="settings-connect-button"
                onClick={() => {
                  setSavedFlash(true);
                  setTimeout(() => setSavedFlash(false), 1400);
                }}
              >
                {savedFlash ? "Saved" : "Save"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
