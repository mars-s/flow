import { useState } from "react";
import { Boxes, Check, Folder } from "lucide-react";
import type { Area, Project } from "../lib/types";
import "./TaskProjects.css";

export function TaskProjectEditor({
  projects,
  projectId,
  onChange,
}: {
  projects: Project[];
  projectId: string | null;
  onChange: (projectId: string | null) => void;
}) {
  const [open, setOpen] = useState(false);
  const current = projects.find((project) => project.id === projectId);

  return (
    <div className="task-project-editor">
      <button type="button" className="pill" onClick={() => setOpen((value) => !value)} aria-expanded={open}>
        <Folder size={11} />
        {current?.title ?? "Project"}
      </button>
      {open && (
        <div className="task-project-popover">
          <button
            type="button"
            className={projectId === null ? "selected" : ""}
            onClick={() => {
              onChange(null);
              setOpen(false);
            }}
          >
            <span>No project</span>
            {projectId === null && <Check size={12} />}
          </button>
          {projects.map((project) => (
            <button
              type="button"
              key={project.id}
              className={project.id === projectId ? "selected" : ""}
              onClick={() => {
                onChange(project.id);
                setOpen(false);
              }}
            >
              <span>{project.title}</span>
              {project.id === projectId && <Check size={12} />}
            </button>
          ))}

        </div>
      )}
    </div>
  );
}

export function ProjectAreaEditor({
  areas,
  areaId,
  onChange,
}: {
  areas: Area[];
  areaId: string | null;
  onChange: (areaId: string | null) => void;
}) {
  const [open, setOpen] = useState(false);
  const current = areas.find((area) => area.id === areaId);
  return (
    <div className="task-project-editor">
      <button type="button" className="task-project-button" onClick={() => setOpen((value) => !value)} aria-expanded={open}>
        <Boxes size={11} />
        {current?.title ?? "Area"}
      </button>
      {open && (
        <div className="task-project-popover">
          <button
            type="button"
            className={areaId === null ? "selected" : ""}
            onClick={() => {
              onChange(null);
              setOpen(false);
            }}
          >
            <span>No area</span>
            {areaId === null && <Check size={12} />}
          </button>
          {areas.map((area) => (
            <button
              type="button"
              key={area.id}
              className={area.id === areaId ? "selected" : ""}
              onClick={() => {
                onChange(area.id);
                setOpen(false);
              }}
            >
              <span>{area.title}</span>
              {area.id === areaId && <Check size={12} />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
