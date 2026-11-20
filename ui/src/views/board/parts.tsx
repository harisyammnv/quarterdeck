import { memo } from "react";
import { Banner, ProjectTile, StatePill, StatusPill, Tag } from "../../ui";
import { useApp } from "../../state/app";
import { shortDate } from "./format";
import type { BoardCard, Decision, HoldBucket } from "./types";

/** A project tile, or a neutral one when the record names no project. */
export function Tile({ name, size = 16 }: { name: string | null; size?: number }) {
  return <ProjectTile name={name ?? "?"} size={size} />;
}

/** Why a held or blocked item is not waiting on you: the snapshot's own reason. */
export function holdWords(
  bucket: HoldBucket | null,
  until: string | null,
  blockedBy: string[],
  ageDays: number | null,
): { text: string; warn: boolean } | null {
  const first = blockedBy[0];
  const waitingOn = first ? `Waiting on ${first}${blockedBy.length > 1 ? ` +${String(blockedBy.length - 1)}` : ""}` : null;
  switch (bucket) {
    case "dated":
      return { text: until ? `Deferred to ${shortDate(until)}` : "Deferred", warn: false };
    case "blocked":
      return { text: waitingOn ?? "Waiting on other work", warn: true };
    case "aged":
      return { text: ageDays !== null ? `Set aside, ${String(ageDays)} d old` : "Set aside", warn: false };
    case "live":
      return null;
    case null:
      return waitingOn ? { text: waitingOn, warn: true } : null;
  }
}

/** The chip that says where a call stands: Needs you, Deferred to a date, Waiting on work. */
export function CallPill({ bucket, until, blockedBy, ageDays }: { bucket: HoldBucket | null; until: string | null; blockedBy: string[]; ageDays: number | null }) {
  if (bucket === "live") return <StatusPill tone="warning">Needs you</StatusPill>;
  const w = holdWords(bucket, until, blockedBy, ageDays);
  if (!w) return null;
  return w.warn ? <StatusPill tone="warning">{w.text}</StatusPill> : <Tag>{w.text}</Tag>;
}

export const LiveChip = memo(function LiveChip({ card }: { card: BoardCard }) {
  if (!card.live) return null;
  const { state, word, source } = card.live;
  return (
    <StatePill
      state={state}
      word={state === "unknown" && word !== "unknown" ? word : undefined}
      title={source === "event" ? "From the latest status event" : "From the fleet snapshot"}
    />
  );
});

/** A short tag for what kind of call a decision is, from structured facts only. */
export function decisionTag(d: Decision): string {
  if (d.pr_url) return "PR";
  if (d.has_report) return "Report";
  return d.kind === "decision" ? "Decision" : "Approval";
}

/** The fleet snapshot's own state: a failed run, or a saved snapshot shown while a fresh one is read. */
export function SnapshotBanner() {
  const snap = useApp().live.view?.snapshot;
  if (!snap) return null;
  if (snap.error) {
    return <Banner title={snap.error}>Snapshot failed, showing the last known backlog: {snap.error}</Banner>;
  }
  if (snap.cached) {
    return <Banner>{snap.running ? "Showing the saved snapshot while a fresh one is read." : "Showing the saved snapshot."}</Banner>;
  }
  return null;
}
