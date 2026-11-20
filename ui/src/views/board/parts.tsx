import { memo } from "react";
import { monogram, STATE_LOOK, tileFill } from "../../states";
import { shortDate } from "./format";
import type { BoardCard, Decision, HoldBucket } from "./types";

/** A project tile: monogram on a fill derived from the name. */
export function Tile({ name, size = 16 }: { name: string | null; size?: number }) {
  const label = name ?? "?";
  return (
    <span
      className="mono-ic"
      aria-hidden="true"
      style={{ background: tileFill(label), width: size, height: size, fontSize: size <= 16 ? 8.5 : 10 }}
    >
      {monogram(label)}
    </span>
  );
}

/** Why a held or blocked item is not waiting on you: the snapshot's own reason. */
export function holdWords(
  bucket: HoldBucket | null,
  until: string | null,
  blockedBy: string[],
  ageDays: number | null,
): { text: string; chip: "neutral" | "warning" } | null {
  const first = blockedBy[0];
  const waitingOn = first ? `Waiting on ${first}${blockedBy.length > 1 ? ` +${String(blockedBy.length - 1)}` : ""}` : null;
  switch (bucket) {
    case "dated":
      return { text: until ? `Deferred to ${shortDate(until)}` : "Deferred", chip: "neutral" };
    case "blocked":
      return { text: waitingOn ?? "Waiting on other work", chip: "warning" };
    case "aged":
      return { text: ageDays !== null ? `Set aside, ${String(ageDays)} d old` : "Set aside", chip: "neutral" };
    case "live":
      return null;
    case null:
      return waitingOn ? { text: waitingOn, chip: "warning" } : null;
  }
}

export function HoldPill({ card }: { card: Pick<BoardCard, "hold" | "until" | "blocked_by"> & { age_days?: number | null } }) {
  const w = holdWords(card.hold, card.until, card.blocked_by, card.age_days ?? null);
  if (!w) return null;
  return <span className={`pill ${w.chip}`}>{w.text}</span>;
}

export const StateChip = memo(function StateChip({ card }: { card: BoardCard }) {
  if (!card.live) return null;
  const look = STATE_LOOK[card.live.state];
  const word = card.live.state === "unknown" && card.live.word !== "unknown" ? card.live.word : look.word;
  return (
    <span className={`pill ${look.chip}`} title={card.live.source === "event" ? "From the latest event" : "From the snapshot"}>
      {word}
    </span>
  );
});

/** A short tag for what kind of call a decision is, from structured facts only. */
export function decisionTag(d: Decision): string {
  if (d.pr_url) return "PR";
  if (d.has_report) return "Report";
  return d.kind === "decision" ? "Decision" : "Approval";
}
