import { useEffect, useState } from "react";
import { CalendarDays } from "lucide-react";
import { api } from "../lib/api";
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
          {auth !== "Granted" && (
            <button className="settings-connect-button" onClick={connect} disabled={connecting || auth === "Denied"}>
              {connecting ? "Requesting…" : "Connect Calendar"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
