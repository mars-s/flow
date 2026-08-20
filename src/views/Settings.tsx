import { useEffect, useState } from "react";
import { CalendarDays } from "lucide-react";
import { AISettingsSection } from "../components/AISettingsSection";
import { api } from "../lib/api";
import { openCalendarPrivacyPane } from "../lib/system";
import type { CalendarAuth } from "../lib/types";
import "./Settings.css";

const STATUS_LABEL: Record<CalendarAuth, string> = {
  Granted: "Connected",
  Denied: "Access denied",
  NotDetermined: "Not connected",
  Unavailable: "Unavailable",
};

export function Settings() {
  const [auth, setAuth] = useState<CalendarAuth | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.calendarAuthStatus().then(setAuth);
  }, []);

  const connect = () => {
    setConnecting(true);
    api
      .calendarConnect()
      .then(setAuth)
      .finally(() => setConnecting(false));
  };

  return (
    <div className="settings-view">
      <div className="view-header">
        <h1>Settings</h1>
      </div>
      <div className="settings-section">
        <div className="settings-row">
          <div className="settings-row-icon">
            <CalendarDays size={16} />
          </div>
          <div className="settings-row-body">
            <div className="settings-row-title">Calendar</div>
            <div className="settings-row-note">
              Read-only. Flow never creates, edits, or deletes anything in your calendar.
              {auth && <> · {STATUS_LABEL[auth]}</>}
            </div>
          </div>
          {auth === "Denied" ? (
            <button
              className="settings-connect-button"
              onClick={() => openCalendarPrivacyPane().catch((err) => setError(String(err)))}
            >
              Open System Settings
            </button>
          ) : (
            auth !== "Granted" && (
              <button className="settings-connect-button" onClick={connect} disabled={connecting}>
                {connecting ? "Requesting…" : "Connect Calendar"}
              </button>
            )
          )}
        </div>
        {auth === "Denied" && (
          <div className="settings-row-error">
            Calendar access was denied. Flow can't ask again — grant it from System Settings, then relaunch Flow.
          </div>
        )}
        {error && <div className="settings-row-error">{error}</div>}
      </div>
      <AISettingsSection />
    </div>
  );
}
