import { motion } from "framer-motion";
import {
  Inbox as InboxIcon,
  Sun,
  CalendarDays,
  Layers,
  Moon,
  Calendar as CalendarIcon,
  Settings as SettingsIcon,
  Plus,
} from "lucide-react";
import type { Destination } from "../lib/types";
import "./Sidebar.css";

const TASK_NAV: { id: Destination; label: string; icon: React.ReactNode }[] = [
  { id: "inbox", label: "Inbox", icon: <InboxIcon size={15} /> },
  { id: "today", label: "Today", icon: <Sun size={15} /> },
  { id: "upcoming", label: "Upcoming", icon: <CalendarDays size={15} /> },
  { id: "anytime", label: "Anytime", icon: <Layers size={15} /> },
  { id: "someday", label: "Someday", icon: <Moon size={15} /> },
];

type Props = {
  destination: Destination;
  onNavigate: (destination: Destination) => void;
  mode: "tasks" | "calendar";
  onModeChange: (mode: "tasks" | "calendar") => void;
  inboxCount: number;
  onCapture: () => void;
};

export function Sidebar({ destination, onNavigate, mode, onModeChange, inboxCount, onCapture }: Props) {
  return (
    <div className="sidebar">
      <div className="sidebar-wordmark">Flow</div>

      <button className="capture-row" onClick={onCapture}>
        <Plus size={14} />
        <span>Capture</span>
        <span className="capture-shortcut">⌘N</span>
      </button>

      <div className="mode-switch">
        <button className={mode === "tasks" ? "mode-tab active" : "mode-tab"} onClick={() => onModeChange("tasks")}>
          {mode === "tasks" && <motion.div layoutId="mode-thumb" className="mode-thumb" transition={{ type: "spring", stiffness: 500, damping: 40 }} />}
          <span className="mode-tab-label">Tasks</span>
        </button>
        <button
          className={mode === "calendar" ? "mode-tab active" : "mode-tab"}
          onClick={() => onModeChange("calendar")}
        >
          {mode === "calendar" && <motion.div layoutId="mode-thumb" className="mode-thumb" transition={{ type: "spring", stiffness: 500, damping: 40 }} />}
          <span className="mode-tab-label">Calendar</span>
        </button>
      </div>

      {mode === "tasks" && (
        <nav className="nav-list">
          {TASK_NAV.map((item) => (
            <button
              key={item.id}
              className={destination === item.id ? "nav-row active" : "nav-row"}
              onClick={() => onNavigate(item.id)}
            >
              {destination === item.id && (
                <motion.div
                  layoutId="nav-active"
                  className="nav-row-bg"
                  transition={{ type: "spring", stiffness: 480, damping: 38 }}
                />
              )}
              <span className="nav-row-icon">{item.icon}</span>
              <span className="nav-row-label">{item.label}</span>
              {item.id === "inbox" && inboxCount > 0 && <span className="nav-badge">{inboxCount}</span>}
            </button>
          ))}
        </nav>
      )}

      {mode === "calendar" && (
        <nav className="nav-list">
          <div className="nav-row active" style={{ cursor: "default" }}>
            <span className="nav-row-icon">
              <CalendarIcon size={15} />
            </span>
            <span className="nav-row-label">Calendar</span>
          </div>
        </nav>
      )}

      <div className="sidebar-spacer" />

      <button
        className={destination === "settings" ? "nav-row active" : "nav-row"}
        onClick={() => onNavigate("settings")}
      >
        <span className="nav-row-icon">
          <SettingsIcon size={15} />
        </span>
        <span className="nav-row-label">Settings</span>
      </button>
    </div>
  );
}
