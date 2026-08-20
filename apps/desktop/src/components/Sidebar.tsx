import { useState } from "react";
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
  Search,
  Folder,
  BookCheck,
} from "lucide-react";
import type { Area, Destination, Project } from "../lib/types";
import "./Sidebar.css";

const TASK_NAV: { id: Destination; label: string; icon: React.ReactNode }[] = [
  { id: "inbox", label: "Inbox", icon: <InboxIcon size={15} /> },
  { id: "today", label: "Today", icon: <Sun size={15} /> },
  { id: "upcoming", label: "Upcoming", icon: <CalendarDays size={15} /> },
  { id: "anytime", label: "Anytime", icon: <Layers size={15} /> },
  { id: "someday", label: "Someday", icon: <Moon size={15} /> },
  { id: "logbook", label: "Logbook", icon: <BookCheck size={15} /> },
];

type Props = {
  destination: Destination;
  onNavigate: (destination: Destination) => void;
  mode: "tasks" | "calendar";
  onModeChange: (mode: "tasks" | "calendar") => void;
  inboxCount: number;
  onCapture: () => void;
  onSearch: () => void;
  projects: Project[];
  activeProjectId: string | null;
  onProjectNavigate: (projectId: string) => void;
  onCreateProject: (title: string) => void;
  areas: Area[];
  projectAreas: Record<string, string>;
  onCreateArea: (title: string) => void;
};

function ProjectNavRow({
  project,
  active,
  onClick,
}: {
  project: Project;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button type="button" className={active ? "nav-row active" : "nav-row"} onClick={onClick}>
      {active && (
        <motion.div
          layoutId="nav-active"
          className="nav-row-bg"
          transition={{ type: "spring", stiffness: 480, damping: 38 }}
        />
      )}
      <span className="nav-row-icon">
        <Folder size={14} />
      </span>
      <span className="nav-row-label">{project.title}</span>
    </button>
  );
}

export function Sidebar({
  destination,
  onNavigate,
  mode,
  onModeChange,
  inboxCount,
  onCapture,
  onSearch,
  projects,
  activeProjectId,
  onProjectNavigate,
  onCreateProject,
  areas,
  projectAreas,
  onCreateArea,
}: Props) {
  const [creatingProject, setCreatingProject] = useState(false);
  const [creatingArea, setCreatingArea] = useState(false);
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

      <div className="sidebar-scroll">
      {mode === "tasks" && (
        <nav className="nav-list">
          {TASK_NAV.map((item) => (
            <button
              key={item.id}
              className={!activeProjectId && destination === item.id ? "nav-row active" : "nav-row"}
              onClick={() => onNavigate(item.id)}
            >
              {!activeProjectId && destination === item.id && (
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
      {mode === "tasks" && (
        <div className="sidebar-projects">
          <div className="sidebar-projects-heading">
            <span>Projects</span>
            <button type="button" onClick={() => setCreatingProject(true)} aria-label="New project">
              <Plus size={12} />
            </button>
          </div>
          {projects
            .filter((project) => !projectAreas[project.id])
            .map((project) => (
              <ProjectNavRow
                key={project.id}
                project={project}
                active={activeProjectId === project.id}
                onClick={() => onProjectNavigate(project.id)}
              />
            ))}
          {creatingProject && (
            <input
              className="sidebar-project-input"
              autoFocus
              placeholder="Project name"
              onBlur={() => setCreatingProject(false)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  const title = event.currentTarget.value.trim();
                  if (title) onCreateProject(title);
                  setCreatingProject(false);
                }
                if (event.key === "Escape") setCreatingProject(false);
              }}
            />
          )}
          <div className="sidebar-projects-heading sidebar-areas-heading">
            <span>Areas</span>
            <button type="button" onClick={() => setCreatingArea(true)} aria-label="New area">
              <Plus size={12} />
            </button>
          </div>
          {areas.map((area) => (
            <div className="sidebar-area" key={area.id}>
              <div className="sidebar-area-label">{area.title}</div>
              {projects
                .filter((project) => projectAreas[project.id] === area.id)
                .map((project) => (
                  <ProjectNavRow
                    key={project.id}
                    project={project}
                    active={activeProjectId === project.id}
                    onClick={() => onProjectNavigate(project.id)}
                  />
                ))}
            </div>
          ))}
          {creatingArea && (
            <input
              className="sidebar-project-input"
              autoFocus
              placeholder="Area name"
              onBlur={() => setCreatingArea(false)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  const title = event.currentTarget.value.trim();
                  if (title) onCreateArea(title);
                  setCreatingArea(false);
                }
                if (event.key === "Escape") setCreatingArea(false);
              }}
            />
          )}
        </div>
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
      </div>

      <button className="nav-row" onClick={onSearch}>
        <span className="nav-row-icon">
          <Search size={15} />
        </span>
        <span className="nav-row-label">Search</span>
        <span className="nav-shortcut">⌘K</span>
      </button>


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
