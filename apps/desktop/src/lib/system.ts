import { openUrl } from "@tauri-apps/plugin-opener";

// PRD §6.5: "a way to reach macOS's own System Settings → Privacy &
// Security → Calendars pane to revoke it — Flow cannot revoke a system
// permission grant programmatically." Same deep link the GPUI app's own
// platform::open_calendar_privacy_pane uses. `x-apple.systempreferences:`
// is private, undocumented API that's already changed format once
// (Ventura's move off System Preferences); if it ever breaks, openUrl's
// own rejection surfaces to the caller instead of doing nothing silently.
export function openCalendarPrivacyPane(): Promise<void> {
  return openUrl("x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars");
}
