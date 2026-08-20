//! Cross-platform calendar glance types and functions — extracted from the
//! GPUI app's own `src/platform.rs` (its `calendar_*` functions and the
//! `CalendarAuth`/`CalendarEvent`/`CalendarInfo` types), same reasoning as
//! `db.rs`/`parse.rs`'s own moves: none of this ever depended on `gpui`,
//! so it was already reusable by any frontend. `platform.rs`'s
//! `open_calendar_privacy_pane` (an `objc2-app-kit` deep link to System
//! Settings) deliberately stayed behind — a Settings-only nicety, not
//! worth pulling in `objc2-app-kit` here for yet.
//!
//! `Serialize`/`Deserialize` on the three public types are for the Tauri
//! migration's IPC boundary (`wayfinder/tickets/migrate-to-tauri.md`),
//! same as `db::Task`'s own derives — the GPUI app never serializes these.

use serde::{Deserialize, Serialize};

/// Calendar auth state Flow's UI cares about (PRD §6.5: EventKit on macOS
/// only — every other platform reports `Unavailable`, same shape as a
/// permission that was asked for and denied, since there's no calendar
/// backend wired up there yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalendarAuth {
    Unavailable,
    NotDetermined,
    Denied,
    Granted,
}

/// One calendar event, already converted to Rust-native types regardless of
/// platform — no frontend ever touches an Objective-C object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    /// Matched against `CalendarInfo::id` for the Calendar tab's
    /// per-calendar visibility toggles.
    pub calendar_id: String,
    pub calendar_title: String,
    pub title: String,
    pub start: chrono::DateTime<chrono::Local>,
    pub end: chrono::DateTime<chrono::Local>,
    pub all_day: bool,
    /// The source calendar's own color, straight sRGB with alpha, each
    /// 0.0–1.0 — never a Flow theme token (`DESIGN_DIRECTION.md`: "Calendar
    /// colors remain calendar colors, never Flow status colors").
    pub color: (f32, f32, f32, f32),
}

/// One calendar the user can see and toggle — the Calendar tab's own
/// sidebar list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarInfo {
    pub id: String,
    pub title: String,
    pub source_title: String,
    pub color: (f32, f32, f32, f32),
}

/// Local midnight at the start of `date` — the one non-obvious edge case is
/// a DST transition landing exactly at midnight, when `and_local_timezone`
/// can't resolve a single instant; falls back to `Local::now()` rather than
/// propagating an `Option` through every calendar-range caller for a case
/// that in practice never actually happens (DST transitions land at 2am/3am
/// in every zone Flow ships to, not midnight).
pub fn local_midnight(date: chrono::NaiveDate) -> chrono::DateTime<chrono::Local> {
    date.and_hms_opt(0, 0, 0)
        .and_then(|naive| naive.and_local_timezone(chrono::Local).single())
        .unwrap_or_else(chrono::Local::now)
}

/// Current calendar permission state without prompting. Safe to call at any
/// time — it's a synchronous in-process query on macOS, not I/O.
#[cfg(target_os = "macos")]
pub fn calendar_authorization_status() -> CalendarAuth {
    match crate::eventkit::authorization_status() {
        crate::eventkit::AuthStatus::Granted => CalendarAuth::Granted,
        crate::eventkit::AuthStatus::NotDetermined => CalendarAuth::NotDetermined,
        crate::eventkit::AuthStatus::Denied => CalendarAuth::Denied,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn calendar_authorization_status() -> CalendarAuth {
    CalendarAuth::Unavailable
}

/// Triggers the system permission prompt (Settings' "Connect Calendar"
/// button). A one-shot user action, not a render path.
#[cfg(target_os = "macos")]
pub async fn calendar_request_access() -> CalendarAuth {
    match crate::eventkit::request_access().await {
        crate::eventkit::AuthStatus::Granted => CalendarAuth::Granted,
        crate::eventkit::AuthStatus::NotDetermined => CalendarAuth::NotDetermined,
        crate::eventkit::AuthStatus::Denied => CalendarAuth::Denied,
    }
}

#[cfg(not(target_os = "macos"))]
pub async fn calendar_request_access() -> CalendarAuth {
    CalendarAuth::Unavailable
}

/// All events in `[start, end)` across every visible calendar. Returns an
/// empty `Vec` on anything short of a crash (no permission, a query
/// failure, a non-macOS platform) — PRD §6.5: calendar failures never
/// block task CRUD, and an empty glance is the correct degraded state.
#[cfg(target_os = "macos")]
pub fn calendar_events_between(
    start: chrono::DateTime<chrono::Local>,
    end: chrono::DateTime<chrono::Local>,
) -> Vec<CalendarEvent> {
    crate::eventkit::events_between(start, end)
        .into_iter()
        .map(|event| CalendarEvent {
            id: event.id,
            calendar_id: event.calendar_id,
            calendar_title: event.calendar_title,
            title: event.title,
            start: event.start,
            end: event.end,
            all_day: event.all_day,
            color: event.color,
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
pub fn calendar_events_between(
    _start: chrono::DateTime<chrono::Local>,
    _end: chrono::DateTime<chrono::Local>,
) -> Vec<CalendarEvent> {
    Vec::new()
}

/// Every calendar that supports events, for the Calendar tab's own sidebar.
#[cfg(target_os = "macos")]
pub fn calendar_list() -> Vec<CalendarInfo> {
    crate::eventkit::list_calendars()
        .into_iter()
        .map(|calendar| CalendarInfo {
            id: calendar.id,
            title: calendar.title,
            source_title: calendar.source_title,
            color: calendar.color,
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
pub fn calendar_list() -> Vec<CalendarInfo> {
    Vec::new()
}
