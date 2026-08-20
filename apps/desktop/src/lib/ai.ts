import { invoke } from "@tauri-apps/api/core";

// Thin wrapper around src-tauri/src/ai.rs's own two commands — every AI
// block (Today briefing first) and the Settings model picker share these
// instead of each hand-rolling an HTTP call.
export const ai = {
  listModels: (baseUrl: string, apiKey: string) => invoke<string[]>("ai_list_models", { baseUrl, apiKey }),
  chatCompletion: (baseUrl: string, apiKey: string, model: string, system: string, user: string) =>
    invoke<string>("ai_chat_completion", { baseUrl, apiKey, model, system, user }),
};
