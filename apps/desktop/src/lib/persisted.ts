import { useState } from "react";

function readJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw !== null ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

// A Set<string> that persists to localStorage under `key` and survives
// relaunches — UI preference state that belongs to this machine (which
// calendars are hidden, which account groups are folded), not task data
// that belongs in flow_data::db.
export function usePersistedSet(key: string): [Set<string>, (id: string) => void] {
  const [value, setValue] = useState<Set<string>>(() => new Set(readJson<string[]>(key, [])));

  const toggle = (id: string) => {
    setValue((prev) => {
      const next = new Set(prev);
      if (!next.delete(id)) next.add(id);
      localStorage.setItem(key, JSON.stringify([...next]));
      return next;
    });
  };

  return [value, toggle];
}

export function usePersistedBoolean(key: string, fallback = false): [boolean, (value: boolean) => void] {
  const [value, setValue] = useState<boolean>(() => readJson(key, fallback));

  const set = (next: boolean) => {
    setValue(next);
    localStorage.setItem(key, JSON.stringify(next));
  };

  return [value, set];
}

export function usePersistedString(key: string, fallback = ""): [string, (value: string) => void] {
  const [value, setValue] = useState<string>(() => readJson(key, fallback));

  const set = (next: string) => {
    setValue(next);
    localStorage.setItem(key, JSON.stringify(next));
  };

  return [value, set];
}
