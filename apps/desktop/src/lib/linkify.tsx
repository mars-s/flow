import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

const URL_PATTERN = /https?:\/\/[^\s]+/g;

// A URL picked up from surrounding prose often carries trailing
// punctuation that belongs to the sentence, not the link ("...app
// https://example.com." shouldn't linkify the trailing period into the
// href) — strip it off the match instead of swallowing it.
function trimTrailingPunctuation(url: string): { url: string; trailing: string } {
  const match = url.match(/^(.*[^.,;:!?)\]'"])([.,;:!?)\]'"]*)$/);
  return match ? { url: match[1], trailing: match[2] } : { url, trailing: "" };
}

// Splits text on http(s) URLs and renders each as a clickable link that
// opens in the user's real browser via tauri-plugin-opener — not the
// app's own webview, which has no back button and no reason to navigate
// away from Flow. Plain text elsewhere is passed through unchanged.
export function linkify(text: string): ReactNode[] {
  const parts: ReactNode[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  const regex = new RegExp(URL_PATTERN);
  let key = 0;
  while ((match = regex.exec(text))) {
    const { url, trailing } = trimTrailingPunctuation(match[0]);
    if (!url) continue;
    if (match.index > lastIndex) parts.push(text.slice(lastIndex, match.index));
    parts.push(
      <a
        key={key++}
        className="task-link"
        href={url}
        onClick={(event) => {
          event.stopPropagation();
          event.preventDefault();
          openUrl(url).catch(() => {});
        }}
      >
        {url}
      </a>,
    );
    if (trailing) parts.push(trailing);
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < text.length) parts.push(text.slice(lastIndex));
  return parts.length > 0 ? parts : [text];
}
