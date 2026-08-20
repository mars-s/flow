import { CalendarDays } from "lucide-react";
import "./Settings.css";

export function Settings() {
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
            </div>
          </div>
          <button className="settings-connect-button">Connect Calendar</button>
        </div>
      </div>
    </div>
  );
}
