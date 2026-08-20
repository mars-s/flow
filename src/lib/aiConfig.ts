import { usePersistedBoolean, usePersistedString } from "./persisted";

// The one shared config every AI block and the Settings model picker
// read from — centralized here instead of each owning its own
// usePersistedString calls, so a block and Settings never drift out of
// sync on what "the configured model" even means.
export function useAiConfig() {
  const [enabled, setEnabled] = usePersistedBoolean("flow.ai.enabled", false);
  const [baseUrl, setBaseUrl] = usePersistedString("flow.ai.openaiBaseUrl", "https://api.openai.com/v1");
  const [apiKey, setApiKey] = usePersistedString("flow.ai.openaiApiKey", "");
  const [model, setModel] = usePersistedString("flow.ai.openaiModel", "");
  return { enabled, setEnabled, baseUrl, setBaseUrl, apiKey, setApiKey, model, setModel };
}

export type AiFeatureState = "off" | "manual" | "auto";

// Every AI block gets its own independent three-state control (direct
// user request: "every ai feature is toggleable in the settings, there
// should be a vertical slider for off, on (trigger/approval), fully
// auto") — Off renders nothing, Manual shows a button that generates on
// click, Auto generates on its own. `featureId` namespaces the persisted
// key per block ("today-briefing", etc.) so adding a new block never
// collides with an existing one's stored state.
export function useAiFeatureState(featureId: string): [AiFeatureState, (state: AiFeatureState) => void] {
  const [raw, setRaw] = usePersistedString(`flow.ai.feature.${featureId}`, "off");
  const value: AiFeatureState = raw === "manual" || raw === "auto" ? raw : "off";
  return [value, setRaw];
}
