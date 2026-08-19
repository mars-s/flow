//! macOS EventKit bridge for the calendar glance and Calendar tab.
//!
//! PRD §6.5 (revised 2026-08-19): Flow reads the user's local macOS
//! Calendar app via EventKit instead of a Google OAuth connection —
//! whatever calendars are already configured in Apple Calendar (iCloud,
//! Google, Exchange, ...) become visible here automatically. Read-only:
//! Flow never requests write access and never calls a mutating EventKit
//! method.
//!
//! Every symbol here is macOS-only. Non-macOS builds don't get a cfg'd-out
//! version of this module at all — callers (`app.rs`, `app/settings.rs`)
//! are themselves `#[cfg(target_os = "macos")]`-gated at their EventKit
//! call sites, matching the existing convention in `platform.rs` (see
//! `show_task_notification`'s two platform-specific bodies) rather than
//! stubbing out a parallel do-nothing implementation here.

use block2::RcBlock;
use chrono::{DateTime, Local, TimeZone};
use futures::channel::oneshot;
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_event_kit::{EKAuthorizationStatus, EKEntityType, EKEventStore};
use objc2_foundation::{NSDate, NSError};

/// Mirrors `EKAuthorizationStatus`, collapsed to what Flow's UI actually
/// distinguishes. `Restricted` (parental controls / MDM) and `WriteOnly`
/// (a state Flow never asks for, but the system can still report if some
/// other app previously requested write-only access) both fold into
/// `Denied` — Flow's UI has nothing different to say for either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStatus {
    NotDetermined,
    Denied,
    Granted,
}

impl AuthStatus {
    fn from_ek(status: EKAuthorizationStatus) -> Self {
        match status {
            EKAuthorizationStatus::FullAccess => Self::Granted,
            EKAuthorizationStatus::NotDetermined => Self::NotDetermined,
            _ => Self::Denied,
        }
    }
}

/// One calendar event, already converted to Rust-native types — nothing
/// downstream of this module touches an Objective-C object directly.
#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub id: String,
    /// `EKCalendar.calendarIdentifier` — matched against `CalendarInfo::id`
    /// for the Calendar tab's per-calendar visibility toggles. Empty when
    /// the event's calendar couldn't be read (rare; means the toggle can't
    /// hide this event, not a crash).
    pub calendar_id: String,
    pub calendar_title: String,
    pub title: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub all_day: bool,
    /// The source calendar's own color, straight (non-premultiplied) sRGB
    /// with alpha, each 0.0–1.0 — never a Flow theme token.
    /// `DESIGN_DIRECTION.md`: "Calendar colors remain calendar colors,
    /// never Flow status colors."
    pub color: (f32, f32, f32, f32),
}

/// Current permission state without prompting. Safe to call at any time —
/// on launch, from Settings, from the Today glance's own render path (it's
/// a synchronous in-process query, not I/O, so this doesn't reopen the
/// render-path-I/O question `CLAUDE.md` cares about) — to decide what to
/// show before ever asking the user for anything.
pub fn authorization_status() -> AuthStatus {
    // Safety: a plain C enum FFI call with no arguments to retain/release.
    AuthStatus::from_ek(unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) })
}

/// Triggers the system permission prompt. A no-op re-prompt if the user has
/// already decided (granted or denied) — EventKit only ever prompts once
/// per app per Apple's own docs; a later change of mind has to go through
/// System Settings, which is why Settings' own "Connect Calendar" affordance
/// also needs a way to deep-link there (see `platform.rs` for the existing
/// `open_system_settings_pane`-shaped helpers this should reuse).
///
/// This is a one-shot user action (a Settings button click), not a render
/// path, so bridging the completion-handler callback into an `.await`-able
/// future here is the documented `CLAUDE.md` carve-out for one-shot
/// actions, not a repeat of the render-path-I/O mistake.
pub async fn request_access() -> AuthStatus {
    let (tx, rx) = oneshot::channel::<bool>();
    // The completion block runs on a queue EventKit itself owns, not
    // necessarily this thread — `Mutex` just makes the one-time `take()`
    // sound across that boundary; there's never real contention on it.
    let tx = std::sync::Mutex::new(Some(tx));
    let completion = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
        if let Some(tx) = tx.lock().unwrap_or_else(|poison| poison.into_inner()).take() {
            let _ = tx.send(granted.as_bool());
        }
    });
    let store = unsafe { EKEventStore::init(EKEventStore::alloc()) };
    // Safety: `completion` outlives the call — it's held on this stack
    // frame across the `.await` below, and the block's own retain count
    // keeps it alive for EventKit's async callback regardless.
    unsafe {
        store.requestFullAccessToEventsWithCompletion(RcBlock::as_ptr(&completion) as *mut _);
    }
    match rx.await {
        Ok(true) => AuthStatus::Granted,
        // A dropped sender (the block never fired) reads the same as a
        // denial: either way there's nothing to show.
        Ok(false) | Err(_) => authorization_status(),
    }
}

fn to_local(date: &NSDate) -> DateTime<Local> {
    Local
        .timestamp_opt(date.timeIntervalSince1970() as i64, 0)
        .single()
        .unwrap_or_else(Local::now)
}

fn from_local(date: DateTime<Local>) -> Retained<NSDate> {
    NSDate::dateWithTimeIntervalSince1970(date.timestamp() as f64)
}

/// All events in `[start, end)` across every calendar the user hasn't
/// hidden, ordered as EventKit returns them (already chronological in
/// practice, but callers that care about strict ordering should still sort
/// — EventKit doesn't document a stable order guarantee).
///
/// Returns an empty `Vec` rather than an error on anything short of a
/// crash: PRD §6.5 says "an EventKit permission denial or query failure ...
/// never exposes anything credential-shaped" and never blocks task CRUD —
/// an empty glance is the correct degraded state, not a popped error.
pub fn events_between(start: DateTime<Local>, end: DateTime<Local>) -> Vec<CalendarEvent> {
    if authorization_status() != AuthStatus::Granted {
        return Vec::new();
    }
    let store = unsafe { EKEventStore::init(EKEventStore::alloc()) };
    let start_ns = from_local(start);
    let end_ns = from_local(end);
    // Safety: plain FFI calls on freshly retained objects; no aliasing.
    unsafe {
        let predicate = store.predicateForEventsWithStartDate_endDate_calendars(&start_ns, &end_ns, None);
        let events = store.eventsMatchingPredicate(&predicate);
        events.iter().map(|event| convert_event(&event)).collect()
    }
}

/// # Safety
/// `event` must be a valid, retained `EKEvent`.
unsafe fn convert_event(event: &objc2_event_kit::EKEvent) -> CalendarEvent {
    unsafe {
        let calendar = event.calendar();
        let calendar_id = calendar
            .as_ref()
            .map(|calendar| calendar.calendarIdentifier().to_string())
            .unwrap_or_default();
        let calendar_title = calendar
            .as_ref()
            .map(|calendar| calendar.title().to_string())
            .unwrap_or_default();
        let color = calendar
            .as_ref()
            .map(|calendar| {
                let color = calendar.color();
                (
                    color.redComponent() as f32,
                    color.greenComponent() as f32,
                    color.blueComponent() as f32,
                    color.alphaComponent() as f32,
                )
            })
            .unwrap_or((0.6, 0.6, 0.6, 1.0));
        CalendarEvent {
            id: event.eventIdentifier().map(|id| id.to_string()).unwrap_or_default(),
            calendar_id,
            calendar_title,
            title: event.title().to_string(),
            start: to_local(&event.startDate()),
            end: to_local(&event.endDate()),
            all_day: event.isAllDay(),
            color,
        }
    }
}

/// One calendar the user can see and toggle — the Calendar tab's own
/// sidebar list (PRD §6.5's Milestone 3 note: modeled on Apple Calendar's
/// own per-calendar color toggles).
#[derive(Debug, Clone)]
pub struct CalendarInfo {
    pub id: String,
    pub title: String,
    pub source_title: String,
    pub color: (f32, f32, f32, f32),
}

/// Every calendar that supports events, grouped by nothing in particular —
/// callers group by `source_title` themselves if they want Apple Calendar's
/// own per-account sections.
pub fn list_calendars() -> Vec<CalendarInfo> {
    if authorization_status() != AuthStatus::Granted {
        return Vec::new();
    }
    let store = unsafe { EKEventStore::init(EKEventStore::alloc()) };
    // Safety: plain FFI calls on freshly retained objects; no aliasing.
    unsafe {
        let calendars = store.calendarsForEntityType(EKEntityType::Event);
        calendars
            .iter()
            .map(|calendar| {
                let color = calendar.color();
                CalendarInfo {
                    id: calendar.calendarIdentifier().to_string(),
                    title: calendar.title().to_string(),
                    source_title: calendar
                        .source()
                        .map(|source| source.title().to_string())
                        .unwrap_or_default(),
                    color: (
                        color.redComponent() as f32,
                        color.greenComponent() as f32,
                        color.blueComponent() as f32,
                        color.alphaComponent() as f32,
                    ),
                }
            })
            .collect()
    }
}
