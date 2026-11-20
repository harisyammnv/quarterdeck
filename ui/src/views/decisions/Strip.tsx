import { useEffect, useRef } from "react";
import { firstLine, prNumber } from "../board/format";
import { decisionTag, Tile } from "../board/parts";
import { useBoard } from "../board/useBoard";
import { openLink, openReport, type Decision } from "./bridge";
import "./decisions.css";

/** How many calls the strip shows as cards; the count beside the title is never capped. */
const STRIP_CARDS = 3;

/**
 * The teal "Needs your call" room above the fleet. It shows only live calls
 * (`captain_actionable`), oldest first, and renders nothing when there are none.
 */
export function DecisionsStrip({
  onOpen,
  onReply,
}: {
  /** Opens the decisions view, at a call when an id is given. */
  onOpen: (id?: string) => void;
  /** Opens the decisions view with the reply box for this call focused. */
  onReply: (id: string) => void;
}) {
  const load = useBoard();
  const ref = useRef<HTMLDivElement>(null);
  const live = load.status === "ready" ? load.view.decisions.live : [];

  // J moves focus to the next call card, wrapping; typing in a field is left alone.
  useEffect(() => {
    if (live.length === 0) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "j" || e.metaKey || e.ctrlKey || e.altKey) return;
      const t = e.target;
      if (t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName))) return;
      const cards = Array.from(ref.current?.querySelectorAll<HTMLElement>("[data-call]") ?? []);
      if (cards.length === 0) return;
      e.preventDefault();
      const at = cards.findIndex((c) => c.contains(document.activeElement));
      cards[(at + 1) % cards.length]?.focus();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [live.length]);

  if (live.length === 0) return null;
  const shown = live.slice(0, STRIP_CARDS);
  const more = live.length - shown.length;
  return (
    <section className="decisions-strip" aria-label={`Needs your call, ${String(live.length)}`} ref={ref}>
      <div className="row">
        <h2 className="decisions-strip-t">Needs your call</h2>
        <span className="decisions-strip-n tn" aria-hidden="true">{live.length}</span>
        <span className="grow" />
        {more > 0 && (
          <button type="button" className="decisions-strip-more" onClick={() => { onOpen(); }}>
            {more} more
          </button>
        )}
        <span className="decisions-strip-hint row">
          Jump to next <span className="kbd">J</span>
        </span>
      </div>
      <div className="decisions-cards">
        {shown.map((d) => (
          <CallCard key={d.id} d={d} onOpen={onOpen} onReply={onReply} />
        ))}
      </div>
    </section>
  );
}

function CallCard({ d, onOpen, onReply }: { d: Decision; onOpen: (id?: string) => void; onReply: (id: string) => void }) {
  const pr = prNumber(d.pr_url);
  const tag = decisionTag(d);
  const primary = d.pr_url
    ? { label: "Open pull request", run: () => void openLink(d.pr_url ?? "") }
    : d.has_report
      ? { label: "Read the report", run: () => { openReport(d.id); } }
      : { label: "Read the options", run: () => { onOpen(d.id); } };
  return (
    <article className="decisions-dcard" data-call tabIndex={-1} aria-label={d.question}>
      <div className="row gap6">
        <Tile name={d.project} />
        <span className="mut trunc decisions-dcard-proj">{d.project ?? "No project"}</span>
        {pr && <span className="mut mono decisions-dcard-pr">{pr}</span>}
        <span className="grow" />
        <span className="pill neutral">{tag}</span>
      </div>
      <button type="button" className="decisions-dcard-t trunc" onClick={() => { onOpen(d.id); }} title={d.question}>
        {d.question}
      </button>
      <div className="mut trunc decisions-dcard-sub">{firstLine(d.detail) || "No reason recorded."}</div>
      <div className="row decisions-dcard-acts">
        <button type="button" className="btn sm primary" onClick={primary.run}>
          {primary.label}
        </button>
        <button type="button" className="btn sm" onClick={() => { onReply(d.id); }}>
          Reply
        </button>
      </div>
    </article>
  );
}
