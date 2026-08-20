//! Commands for the AI settings' "test connection" model picker and any
//! AI block (Today briefing first) that needs to call a user-configured
//! OpenAI-compatible endpoint. Done here in Rust via reqwest rather than
//! a plain frontend `fetch()` — see the Cargo.toml dependency comment:
//! most third-party OpenAI-compatible providers don't set permissive CORS
//! headers for an arbitrary app origin, and Tauri's webview still
//! enforces CORS on cross-origin fetch even with `security.csp: null`.
//! `wayfinder/tickets/migrate-to-tauri.md` has the full feature context.

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

/// The "Test connection" button's own call: GETs `{base_url}/models`
/// (the OpenAI API's own listing endpoint, which every OpenAI-compatible
/// provider this is meant to support — Groq, Together, a local Ollama
/// server, etc. — implements the same way), returning just the model
/// ids for the Settings dropdown to populate from.
#[tauri::command]
pub async fn ai_list_models(base_url: String, api_key: String) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let response = client
        .get(&url)
        .bearer_auth(&api_key)
        .send()
        .await
        .map_err(|error| format!("couldn't reach {url}: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("{url} returned {status}: {body}"));
    }
    let parsed: ModelsResponse = response
        .json()
        .await
        .map_err(|error| format!("unexpected response shape from {url}: {error}"))?;
    Ok(parsed.data.into_iter().map(|entry| entry.id).collect())
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

/// One-shot, non-streaming chat completion against the same configured
/// endpoint — every AI block (Today briefing first) shares this instead
/// of each hand-rolling its own HTTP call.
#[tauri::command]
pub async fn ai_chat_completion(
    base_url: String,
    api_key: String,
    model: String,
    system: String,
    user: String,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = ChatRequest {
        model: &model,
        messages: vec![
            ChatMessage {
                role: "system",
                content: &system,
            },
            ChatMessage {
                role: "user",
                content: &user,
            },
        ],
    };
    let response = client
        .post(&url)
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("couldn't reach {url}: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{url} returned {status}: {text}"));
    }
    let parsed: ChatResponse = response
        .json()
        .await
        .map_err(|error| format!("unexpected response shape from {url}: {error}"))?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| "the model returned no choices".to_string())
}
