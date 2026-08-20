import { useMemo, useState } from "react";
import { Plus, Tag as TagIcon, X } from "lucide-react";
import type { Tag, TaskTag } from "../lib/types";
import "./TaskTags.css";

export function TaskTagEditor({
  available,
  assigned,
  onChange,
}: {
  available: Tag[];
  assigned: TaskTag[];
  onChange: (names: string[]) => void;
}) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState("");
  const assignedNames = assigned.map((tag) => tag.name);
  const suggestions = useMemo(() => {
    const query = draft.trim().toLowerCase();
    const selected = new Set(assignedNames.map((name) => name.toLowerCase()));
    return available
      .filter((tag) => !selected.has(tag.name.toLowerCase()))
      .filter((tag) => !query || tag.name.toLowerCase().includes(query))
      .slice(0, 6);
  }, [available, assignedNames, draft]);

  const add = (name: string) => {
    const trimmed = name.trim();
    if (!trimmed || assignedNames.some((current) => current.toLowerCase() === trimmed.toLowerCase())) return;
    onChange([...assignedNames, trimmed]);
    setDraft("");
    setOpen(false);
  };

  return (
    <div className="task-tags-editor">
      <div className="task-tags-list">
        {assigned.map((tag) => (
          <span className="task-tag-chip" key={tag.tag_id}>
            <TagIcon size={10} />
            {tag.name}
            <button
              type="button"
              onClick={() => onChange(assignedNames.filter((name) => name !== tag.name))}
              aria-label={`Remove ${tag.name} tag`}
            >
              <X size={10} />
            </button>
          </span>
        ))}
        <button type="button" className="pill" onClick={() => setOpen(true)} aria-expanded={open}>
          <Plus size={11} />
          Tag
        </button>
      </div>
      {open && (
        <div className="task-tag-popover">
          <input
            autoFocus
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") add(draft || suggestions[0]?.name || "");
              if (event.key === "Escape") {
                setDraft("");
                setOpen(false);
              }
            }}
            placeholder="Find or create a tag"
            aria-label="Tag name"
          />
          {suggestions.length > 0 && (
            <div className="task-tag-suggestions">
              {suggestions.map((tag) => (
                <button
                  type="button"
                  key={tag.id}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => add(tag.name)}
                >
                  <TagIcon size={11} />
                  {tag.name}
                </button>
              ))}
            </div>
          )}
          {draft.trim() && !suggestions.some((tag) => tag.name.toLowerCase() === draft.trim().toLowerCase()) && (
            <button type="button" className="task-tag-create" onClick={() => add(draft)}>
              Create “{draft.trim()}”
            </button>
          )}
        </div>
      )}
    </div>
  );
}

export function TaskTagFilter({
  tags,
  value,
  onChange,
}: {
  tags: Tag[];
  value: string | null;
  onChange: (name: string | null) => void;
}) {
  if (tags.length === 0) return null;
  return (
    <div className="task-tag-filter" aria-label="Filter tasks by tag">
      <button type="button" className={value === null ? "active" : ""} onClick={() => onChange(null)}>
        All
      </button>
      {tags.map((tag) => (
        <button
          type="button"
          key={tag.id}
          className={value?.toLowerCase() === tag.name.toLowerCase() ? "active" : ""}
          onClick={() => onChange(value?.toLowerCase() === tag.name.toLowerCase() ? null : tag.name)}
        >
          {tag.name}
        </button>
      ))}
    </div>
  );
}
