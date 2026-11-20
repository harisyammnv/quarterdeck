import type { TaskState } from "./bindings";

/** How each task state is drawn: one chip form (soft fill, dot, word) and a dot class. */
export interface StateLook {
  word: string;
  chip: "success" | "warning" | "danger" | "info" | "neutral" | "accent";
  dot: "working" | "waiting" | "blocked" | "idle" | "";
}

export const STATE_LOOK: Record<TaskState, StateLook> = {
  working: { word: "Working", chip: "success", dot: "working" },
  "needs-decision": { word: "Needs you", chip: "warning", dot: "waiting" },
  blocked: { word: "Blocked", chip: "danger", dot: "blocked" },
  failed: { word: "Failed", chip: "danger", dot: "blocked" },
  paused: { word: "Paused", chip: "neutral", dot: "idle" },
  done: { word: "Done", chip: "accent", dot: "" },
  unknown: { word: "Unknown", chip: "neutral", dot: "idle" },
};

/** Monogram fills from the product palette. Teal is reserved for the brand disc. */
const TILE_FILLS = ["#5d49d9", "#0025ff", "#8a5a00", "#5c5c5c"] as const;

export function tileFill(name: string): string {
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return TILE_FILLS[h % TILE_FILLS.length] ?? TILE_FILLS[3];
}

/** Two letters for a project tile: initials of the first two words, or the first two letters. */
export function monogram(name: string): string {
  const words = name.split(/[^A-Za-z0-9]+/).filter(Boolean);
  const a = words[0] ?? "?";
  const b = words[1];
  return (b ? a.charAt(0) + b.charAt(0) : a.slice(0, 2)).toUpperCase();
}

/** "now", "42 s", "5 min", "3 h", "2 d" since `thenMs`. */
export function ago(thenMs: number, nowMs: number): string {
  const s = Math.max(0, Math.round((nowMs - thenMs) / 1000));
  if (s < 5) return "now";
  if (s < 60) return `${s} s ago`;
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 48) return `${h} h ago`;
  return `${Math.round(h / 24)} d ago`;
}
