//! Wired to Flow's real task store (`flow-data`, extracted from the GPUI
//! app's own `src/db.rs` — see
//! /Users/avi/Developer/vibe/flow/wayfinder/tickets/migrate-to-tauri.md),
//! not mock data, for whichever screens have been ported so far.
//!
//! **Deliberately a separate database file from GPUI Flow's own** —
//! `Db::open()` resolves to Flow's actual production data path, and
//! running both the GPUI dev app and a Tauri build at once against the
//! *same* SQLite file, from two independent OS processes, is a real risk
//! neither needs to take (explicit user decision, 2026-08-20: keep the
//! databases separate rather than unify on GPUI's own). Isolation comes
//! from `tauri.conf.json`'s own `identifier` (a distinct `app_data_dir`
//! per bundle identifier), not just the filename — the debug build
//! (`Flow Debug`, `com.avi.flow-tauri-prototype`) and the release build
//! (`Flow`, `com.avi.flow`, via `tauri.release.conf.json`) already get
//! separate directories on that basis alone; the filename split below is
//! for a human reading the two directories side by side, not a safety
//! requirement in itself.

mod ai;

use flow_data::calendar::{self, CalendarAuth, CalendarEvent, CalendarInfo};
use flow_data::db::{
    Area, Bucket, Db, Project, ProjectArea, SubtaskCount, Tag, Task, TaskProject, TaskTag, View,
};
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

fn database_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let filename = if cfg!(debug_assertions) {
        "flow-tauri-dev.db"
    } else {
        "flow.db"
    };
    app.path()
        .app_data_dir()
        .map(|dir| dir.join(filename))
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
        let date = parsed
            .date
            .or_else(|| parsed.time.is_some().then_some(today));
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
async fn set_completed(
    db: tauri::State<'_, Db>,
    id: String,
    completed: bool,
) -> Result<(), String> {
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

/// Renames a task (or subtask — `db::set_title` doesn't distinguish, same
/// as `set_note`). Direct user report: there was no way to rename a task
/// or subtask at all once created.
#[tauri::command]
async fn set_title(db: tauri::State<'_, Db>, id: String, title: String) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.set_title(id, title))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

/// The detail card's Today/Anytime/Someday/Clear quick-picks — a plain
/// bucket + optional date/time, no parsing involved. `bucket` arrives as
/// the same tag strings `Bucket`'s own Deserialize produces ("Inbox" for
/// Clear, "Active" for Today/Anytime, "Someday").
#[tauri::command]
async fn schedule_task(
    db: tauri::State<'_, Db>,
    id: String,
    bucket: Bucket,
    date: Option<String>,
    time: Option<String>,
) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.schedule(id, bucket, date, time))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

/// The picker's own free-text field ("Schedule…") — parses `text` the same
/// way Capture does and schedules with whatever it recognized. Mirrors
/// `capture_task`'s exact rules (a recognized date activates the task into
/// Active, a bare time with no date defaults to today), the one difference
/// being this schedules an *existing* task instead of creating one.
#[tauri::command]
async fn schedule_task_from_text(
    db: tauri::State<'_, Db>,
    id: String,
    text: String,
) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let today = chrono::Local::now().date_naive();
        let parsed = parse::parse(&text, today);
        let date = parsed
            .date
            .or_else(|| parsed.time.is_some().then_some(today));
        if date.is_none() {
            return Err("couldn't recognize a date or time in that".to_string());
        }
        db.schedule(
            id,
            Bucket::Active,
            date.map(|d| d.to_string()),
            parsed.time.map(|t| t.format("%H:%M").to_string()),
        )
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn list_subtasks(db: tauri::State<'_, Db>, parent_id: String) -> Result<Vec<Task>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.list_subtasks(parent_id))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

/// Every parent's open/total subtask counts in one call — a collapsed
/// row's own subtask-count badge (direct user request, matching Things
/// 3's row indicators), not gated to a particular view.
#[tauri::command]
async fn subtask_counts(db: tauri::State<'_, Db>) -> Result<Vec<SubtaskCount>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.subtask_counts())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_subtask(
    db: tauri::State<'_, Db>,
    parent_id: String,
    title: String,
) -> Result<Task, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.create_subtask(parent_id, title))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_tags(db: tauri::State<'_, Db>) -> Result<Vec<Tag>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.list_tags())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_task_tags(db: tauri::State<'_, Db>) -> Result<Vec<TaskTag>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.list_task_tags())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_task_tags(
    db: tauri::State<'_, Db>,
    task_id: String,
    names: Vec<String>,
) -> Result<Vec<TaskTag>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.set_task_tags(task_id, names))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_projects(db: tauri::State<'_, Db>) -> Result<Vec<Project>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.list_projects())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_project(db: tauri::State<'_, Db>, title: String) -> Result<Project, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.create_project(title))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_task_projects(db: tauri::State<'_, Db>) -> Result<Vec<TaskProject>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.list_task_projects())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_task_project(
    db: tauri::State<'_, Db>,
    task_id: String,
    project_id: Option<String>,
) -> Result<Option<TaskProject>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.set_task_project(task_id, project_id))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_areas(db: tauri::State<'_, Db>) -> Result<Vec<Area>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.list_areas())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_area(db: tauri::State<'_, Db>, title: String) -> Result<Area, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.create_area(title))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_project_areas(db: tauri::State<'_, Db>) -> Result<Vec<ProjectArea>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.list_project_areas())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_project_area(
    db: tauri::State<'_, Db>,
    project_id: String,
    area_id: Option<String>,
) -> Result<Option<ProjectArea>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || db.set_project_area(project_id, area_id))
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
    let start: chrono::DateTime<chrono::FixedOffset> = start
        .parse()
        .map_err(|error| format!("bad start date: {error}"))?;
    let end: chrono::DateTime<chrono::FixedOffset> = end
        .parse()
        .map_err(|error| format!("bad end date: {error}"))?;
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
            let path = database_path(&app.handle())?;
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
            set_title,
            schedule_task,
            schedule_task_from_text,
            list_subtasks,
            create_subtask,
            subtask_counts,
            list_tags,
            list_task_tags,
            set_task_tags,
            list_projects,
            create_project,
            list_task_projects,
            set_task_project,
            list_areas,
            create_area,
            list_project_areas,
            set_project_area,
            ai::ai_list_models,
            ai::ai_chat_completion,
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
