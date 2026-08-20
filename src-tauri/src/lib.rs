//! Wired to Flow's real task store (`flow-data`, extracted from the GPUI
//! app's own `src/db.rs` — see
//! /Users/avi/Developer/vibe/flow/wayfinder/tickets/migrate-to-tauri.md),
//! not mock data, for whichever screens have been ported so far.
//!
//! **Deliberately a separate database file from the real app**
//! (`flow-tauri-dev.db`, not `flow.db`) — `Db::open()` resolves to Flow's
//! actual production data path, and running both the GPUI dev app and this
//! prototype at once against the *same* SQLite file, from two independent
//! OS processes, is a real risk this repo doesn't need to take just to
//! develop the UI. Point this at the real path once the migration is
//! actually cutting over, not before.

use flow_data::db::{Db, Task, View};
use flow_data::parse;
use tauri::Manager;

fn dev_database_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("flow-tauri-dev.db"))
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_view(db: tauri::State<'_, Db>, view: View) -> Result<Vec<Task>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.list_view(view))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_completed(db: tauri::State<'_, Db>, view: View) -> Result<Vec<Task>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.list_completed(view))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_task(db: tauri::State<'_, Db>, title: String) -> Result<Task, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.create_task(title))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

/// Capture's real path — `flow_data::parse::parse` against the raw typed
/// text, then `create_task_scheduled` with whatever it recognized, not a
/// bare `create_task`. Mirrors the GPUI app's own `submit_capture`
/// (`src/app.rs`) exactly, including its one deliberate override of a
/// literal PRD §14 reading: a recognized date activates the task straight
/// into Today/Upcoming rather than leaving it sitting in Inbox with a
/// schedule attached, and a bare recognized time with no date phrase
/// ("3pm") defaults to today rather than being rejected — `Db::schedule`'s
/// own guard requires a date whenever a time is present.
#[tauri::command]
async fn capture_task(db: tauri::State<'_, Db>, title: String) -> Result<Task, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let today = chrono::Local::now().date_naive();
        let parsed = parse::parse(&title, today);
        let date = parsed.date.or_else(|| parsed.time.is_some().then_some(today));
        db.create_task_scheduled(
            parsed.cleaned_title,
            date.map(|d| d.to_string()),
            parsed.time.map(|t| t.format("%H:%M").to_string()),
        )
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_completed(db: tauri::State<'_, Db>, id: String, completed: bool) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.set_completed(id, completed))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_note(db: tauri::State<'_, Db>, id: String, note: String) -> Result<(), String> {
    let db = db.inner().clone();
    let note = if note.is_empty() { None } else { Some(note) };
    tokio::task::spawn_blocking(move || db.set_note(id, note))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_subtasks(db: tauri::State<'_, Db>, parent_id: String) -> Result<Vec<Task>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.list_subtasks(parent_id))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_subtask(db: tauri::State<'_, Db>, parent_id: String, title: String) -> Result<Task, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.create_subtask(parent_id, title))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_task(db: tauri::State<'_, Db>, id: String) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.delete_task(id))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let path = dev_database_path(&app.handle())?;
            let db = Db::open_at(path)?;
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_view,
            list_completed,
            create_task,
            capture_task,
            set_completed,
            set_note,
            list_subtasks,
            create_subtask,
            delete_task,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
