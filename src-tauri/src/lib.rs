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

use flow_data::calendar::{self, CalendarAuth, CalendarEvent, CalendarInfo};
use flow_data::db::{Db, Task, View};
use flow_data::parse;
use serde::Serialize;
use tauri::Manager;

/// What the Capture field's live preview shows as you type — the read-only
/// half of `flow_data::parse::parse`, called on every keystroke (it's a
/// regex match against a short string, not I/O), never writing anything.
/// `highlight_start`/`highlight_end` are UTF-16 code-unit offsets (not
/// `ParsedTitle::source_range`'s Rust byte offsets) because that's what a
/// JS `<input>`'s own selection/slicing API speaks — converted here once,
/// at the one place that has both the original title and the byte range,
/// rather than asking the frontend to do UTF-8/UTF-16 arithmetic itself.
#[derive(Serialize)]
struct ParsePreview {
    date: Option<String>,
    time: Option<String>,
    highlight_start: Option<usize>,
    highlight_end: Option<usize>,
}

#[tauri::command]
fn preview_capture(title: String) -> ParsePreview {
    parse_preview(&title, chrono::Local::now().date_naive())
}

/// The actual logic, factored out of the `#[tauri::command]` wrapper so it
/// has a plain signature `cargo test -p flow-tauri-prototype` can exercise
/// directly — a fixed `today` matters here for the same determinism reason
/// `flow_data::parse::parse` itself takes one instead of reading the clock.
fn parse_preview(title: &str, today: chrono::NaiveDate) -> ParsePreview {
    let parsed = parse::parse(title, today);
    let (highlight_start, highlight_end) = match parsed.source_range {
        Some(range) => (
            Some(title[..range.start].encode_utf16().count()),
            Some(title[..range.end].encode_utf16().count()),
        ),
        None => (None, None),
    };
    ParsePreview {
        date: parsed.date.map(|d| d.format("%a, %b %-d").to_string()),
        time: parsed.time.map(|t| t.format("%-I:%M %p").to_string()),
        highlight_start,
        highlight_end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap() // a Thursday
    }

    #[test]
    fn a_bare_numeric_time_is_recognized_and_highlighted() {
        let preview = parse_preview("call the dentist 8am", today());
        assert_eq!(preview.time.as_deref(), Some("8:00 AM"));
        assert_eq!(preview.date.as_deref(), None);
        // "8am" starts right after "call the dentist " (17 UTF-16 units in).
        assert_eq!(preview.highlight_start, Some(17));
        assert_eq!(preview.highlight_end, Some(20));
    }

    #[test]
    fn a_relative_date_word_is_recognized() {
        let preview = parse_preview("take out laundry tomorrow", today());
        assert_eq!(preview.date.as_deref(), Some("Fri, Aug 21"));
        assert_eq!(preview.time, None);
    }

    #[test]
    fn date_and_time_together_are_both_recognized() {
        let preview = parse_preview("take out laundry 8am tomorrow", today());
        assert_eq!(preview.date.as_deref(), Some("Fri, Aug 21"));
        assert_eq!(preview.time.as_deref(), Some("8:00 AM"));
    }

    #[test]
    fn an_explicit_iso_date_is_recognized() {
        let preview = parse_preview("renew passport 2026-09-01", today());
        assert_eq!(preview.date.as_deref(), Some("Tue, Sep 1"));
    }

    #[test]
    fn plain_text_with_no_recognizable_phrase_highlights_nothing() {
        let preview = parse_preview("buy milk", today());
        assert_eq!(preview.date, None);
        assert_eq!(preview.time, None);
        assert_eq!(preview.highlight_start, None);
        assert_eq!(preview.highlight_end, None);
    }

    #[test]
    fn highlight_offsets_are_utf16_code_units_not_byte_offsets() {
        // "café" has a 2-byte 'é' in UTF-8 but is still 4 UTF-16 code units —
        // this is the one case a naive byte-offset pass-through would get
        // wrong, so it's the one worth a dedicated test rather than trusting
        // the ASCII-only cases above to cover it.
        let preview = parse_preview("café meeting tomorrow", today());
        assert_eq!(preview.date.as_deref(), Some("Fri, Aug 21"));
        // "tomorrow" starts after "café meeting " — 4 (café) + 1 (space) +
        // 7 (meeting) + 1 (space) = 13 UTF-16 units in.
        assert_eq!(preview.highlight_start, Some(13));
    }
}

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

#[tauri::command]
async fn restore_task(db: tauri::State<'_, Db>, id: String) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.restore_task(id))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn calendar_auth_status() -> CalendarAuth {
    calendar::calendar_authorization_status()
}

#[tauri::command]
async fn calendar_connect() -> CalendarAuth {
    // calendar::calendar_request_access()'s future bridges an EventKit
    // completion block (an Objective-C `RcBlock`) into an .await-able
    // future — real, useful async, but that block can never be `Send`,
    // and Tauri's async commands require their whole future to be `Send`
    // (it can move between tokio worker threads between polls). Driving
    // it to completion inside spawn_blocking's own dedicated thread, on a
    // throwaway single-thread runtime that never needs to move the future
    // anywhere, sidesteps that — the same "give the non-Send thing its
    // own thread" shape flow_data::db already uses for its own connection.
    tokio::task::spawn_blocking(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("building a runtime for the calendar permission thread")
            .block_on(calendar::calendar_request_access())
    })
    .await
    .unwrap_or(CalendarAuth::Unavailable)
}

/// The frontend sends RFC3339 strings (`Date.prototype.toISOString()`,
/// always UTC/`Z`-suffixed) — parsed as `DateTime<FixedOffset>` first since
/// that's the form chrono's `FromStr` actually supports, then converted to
/// `Local` (what `calendar::calendar_events_between` takes), rather than
/// parsing straight into `DateTime<Local>`, which isn't derivable from a
/// string alone (it needs the system's own timezone database, not just
/// what's written in the string).
#[tauri::command]
fn calendar_events(start: String, end: String) -> Result<Vec<CalendarEvent>, String> {
    let start: chrono::DateTime<chrono::FixedOffset> =
        start.parse().map_err(|error| format!("bad start date: {error}"))?;
    let end: chrono::DateTime<chrono::FixedOffset> =
        end.parse().map_err(|error| format!("bad end date: {error}"))?;
    Ok(calendar::calendar_events_between(
        start.with_timezone(&chrono::Local),
        end.with_timezone(&chrono::Local),
    ))
}

#[tauri::command]
fn calendar_list() -> Vec<CalendarInfo> {
    calendar::calendar_list()
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
            preview_capture,
            set_completed,
            set_note,
            list_subtasks,
            create_subtask,
            delete_task,
            restore_task,
            calendar_auth_status,
            calendar_connect,
            calendar_events,
            calendar_list,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
