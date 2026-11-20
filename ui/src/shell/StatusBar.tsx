import { ArrowsClockwiseIcon as ArrowsClockwise, HandPalmIcon as HandPalm, LockSimpleIcon as LockSimple, UsersIcon as Users } from "@phosphor-icons/react";
import { inApp } from "../bridge";
import { StateDot, ago, useNow } from "../ui";
import { useApp } from "../state/app";
import { views } from "../registry";

function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** The 28px status bar: what is watched, how many workers and calls, area items, freshness. */
export function StatusBar() {
  const { info, live } = useApp();
  const now = useNow(1000);
  const view = live.view;
  const rows = view?.rows ?? [];
  const workers = rows.filter((r) => r.state !== "done").length;
  const decisions = rows.filter((r) => r.state === "needs-decision").length;
  const snap = view?.snapshot;
  const home = info?.home ?? null;
  const items = views.flatMap((v) => (v.statusItem ? [v.statusItem] : []));
  return (
    <footer className="statusbar">
      <span className="row">
        <StateDot kind={!home ? "idle" : snap?.error ? "blocked" : view ? "working" : "idle"} />
        {home ? `Watching ${home}` : "No Firstmate home"}
      </span>
      {home && (
        <>
          <span className="row tn">
            <Users size={14} />
            {plural(workers, "worker", "workers")}
          </span>
          <span className="row tn">
            <HandPalm size={14} />
            {plural(decisions, "decision", "decisions")}
          </span>
        </>
      )}
      {items.map((Item, i) => (
        <Item key={i} />
      ))}
      <span className="grow" />
      {!inApp && <span className="row">Preview data</span>}
      <span className="row">
        <LockSimple size={14} />
        Read-only companion
      </span>
      {snap && (
        <span className="row tn">
          <ArrowsClockwise size={14} />
          {snap.running ? "Reading records" : snap.generated_at ? `Records refreshed ${ago(snap.generated_at * 1000, now)}` : "Records not read yet"}
        </span>
      )}
    </footer>
  );
}
