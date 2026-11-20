import type { TaskState } from "../../bindings";
import { stateDot, type DotKind } from "../../ui";
import type { ProjectRow } from "./bridge";

/**
 * The sidebar dot for a row: the shared mapping (`stateDot`), with "investigating" for a
 * working scout, and the idle ring for settled states that have no dot of their own.
 */
export function dotKind(row: Pick<ProjectRow, "state" | "kind">): DotKind {
  const kind = stateDot(row.state) ?? "idle";
  return kind === "working" && row.kind === "scout" ? "investigating" : kind;
}

/** Words for screen readers, matching the legend. */
export function stateWord(row: Pick<ProjectRow, "state" | "kind" | "state_word">): string {
  const k = dotKind(row);
  if (k === "investigating") return "Investigating";
  const words: Record<TaskState, string> = {
    working: "Working",
    "needs-decision": "Needs you",
    blocked: "Blocked",
    failed: "Failed",
    paused: "Paused",
    done: "Done",
    unknown: row.state_word === "unknown" ? "Unknown" : row.state_word,
  };
  return words[row.state];
}

/** "direct-PR" -> "Direct pull request"; unknown modes are shown as written. */
export function deliveryLabel(mode: string | null): string | null {
  if (!mode) return null;
  const known: Record<string, string> = {
    "direct-PR": "Direct pull request",
    "no-mistakes": "Full pipeline, then PR",
    "local-only": "Local only, no push",
  };
  return known[mode] ?? mode;
}

/** "claude" -> "Claude". */
export function agentLabel(harness: string | null): string | null {
  if (!harness) return null;
  return harness.charAt(0).toUpperCase() + harness.slice(1);
}

/** "14:02, 18 min ago" within the day, else "Nov 3, 2 d ago". */
export function startedLabel(atSec: number, nowMs: number, ago: (t: number, n: number) => string): string {
  const d = new Date(atSec * 1000);
  const recent = nowMs - atSec * 1000 < 20 * 3600 * 1000;
  const when = recent
    ? d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false })
    : d.toLocaleDateString([], { month: "short", day: "numeric" });
  return `${when}, ${ago(atSec * 1000, nowMs)}`;
}
