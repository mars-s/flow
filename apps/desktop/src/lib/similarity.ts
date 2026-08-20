import type { Task } from "./types";

// Word-overlap (Jaccard) similarity on normalized tokens — deliberately
// not a model call. Catching "near-duplicate" is a string-comparison
// problem, not a language-understanding one: a threshold on shared
// words is instant, needs no API key, and is exactly as correct as an
// LLM would be here, so routing it through chat completion would only
// add latency and cost for the same answer. Still gated behind the
// feature's own Off/Manual/Auto toggle like every other AI-labeled
// block, since it lives in the same Settings section and shares the
// same on/off framing even though nothing here is generative.
function normalize(title: string): string[] {
  return title
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s]/gu, "")
    .split(/\s+/)
    .filter((word) => word.length > 0);
}

function jaccard(a: string[], b: string[]): number {
  if (a.length === 0 || b.length === 0) return 0;
  const setA = new Set(a);
  const setB = new Set(b);
  let intersection = 0;
  for (const word of setA) if (setB.has(word)) intersection++;
  const union = setA.size + setB.size - intersection;
  return union === 0 ? 0 : intersection / union;
}

const DUPLICATE_THRESHOLD = 0.6;

// Finds the closest existing task to `title` among `tasks`, if any clears
// the similarity threshold. Titles under 3 words are skipped — short
// titles ("Call mum", "Buy milk") share enough common words to false-
// positive constantly without carrying much real signal either way.
export function findDuplicate(title: string, tasks: Task[]): Task | null {
  const tokens = normalize(title);
  if (tokens.length < 3) return null;

  let best: Task | null = null;
  let bestScore = 0;
  for (const task of tasks) {
    const score = jaccard(tokens, normalize(task.title));
    if (score > bestScore) {
      bestScore = score;
      best = task;
    }
  }
  return bestScore >= DUPLICATE_THRESHOLD ? best : null;
}
