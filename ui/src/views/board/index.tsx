import { memo, useMemo } from "react";
import { ArrowSquareOutIcon, FileTextIcon, KanbanIcon, WarningIcon } from "@phosphor-icons/react";
import { openLink, openReport } from "./bridge";
import { age, firstLine, prNumber } from "./format";
import { decisionTag, HoldPill, StateChip, Tile } from "./parts";
import type { BoardCard, BoardView, Decision } from "./types";
import { useBoard, useNow } from "./useBoard";
import "./board.css";

/** The board: Queued, In flight, Waiting on you and Done, over the backlog. Read-only. */
export default function BoardViewPanel() {
  const load = useBoard();
  const now = useNow();
  if (load.status === "loading") {
    return <BoardShell><Zero title="Reading the backlog" text="The first snapshot takes a few seconds." /></BoardShell>;
  }
  if (load.status === "error") {
    return <BoardShell><Zero warn title="The board could not be read" text={load.message} /></BoardShell>;
  }
  const v = load.view;
  if (!v.ready) {
    return <BoardShell><Zero title="Waiting for the first snapshot" text="Cards appear as soon as Firstmate's fleet snapshot lands." /></BoardShell>;
  }
  return <Board v={v} now={now} />;
}

function BoardShell({ children, meta }: { children: React.ReactNode; meta?: React.ReactNode }) {
  return (
    <section className="panel board" aria-labelledby="board-title">
      <div className="phead">
        <span className="ptitle" id="board-title">Board</span>
        {meta}
      </div>
      {children}
    </section>
  );
}

function Zero({ title, text, warn = false }: { title: string; text: string; warn?: boolean }) {
  return (
    <div className="zero board-zero" role={warn ? "alert" : "status"}>
      <span className="zi">{warn ? <WarningIcon size={16} /> : <KanbanIcon size={16} />}</span>
      <span>
        <b>{title}</b>
        <span className="t12">{text}</span>
      </span>
    </div>
  );
}

function Board({ v, now }: { v: BoardView; now: number }) {
  const live = v.decisions.live;
  const doneMore = v.done_total - v.done.length;
  const meta = (
    <>
      <span className="mut board-meta tn">
        {v.queued.length} queued, {v.in_flight.length} in flight, {live.length} waiting on you
      </span>
      <span className="grow" />
      {v.other.length > 0 && (
        <span className="pill warning" title={v.other.map((o) => `${o.id}: ${o.why}`).join("\n")}>
          {v.other.length} not placed
        </span>
      )}
      {v.generated_at !== null && <span className="mut board-meta tn">Snapshot {age(v.generated_at, now)} ago</span>}
    </>
  );
  return (
    <BoardShell meta={meta}>
      <div className="board-cols">
        <Lane title="Queued" count={v.queued.length} dot="idle" empty="Nothing queued.">
          {v.queued.map((c) => <Card key={c.id} card={c} now={now} />)}
        </Lane>
        <Lane title="In flight" count={v.in_flight.length} dot="working" empty="No task is running.">
          {v.in_flight.map((c) => <Card key={c.id} card={c} now={now} />)}
        </Lane>
        <Lane title="Waiting on you" count={live.length} dot="waiting" empty="Nothing needs your call.">
          {live.map((d) => <CallCard key={d.id} d={d} now={now} />)}
        </Lane>
        <Lane
          title="Done"
          count={v.done_total}
          dot=""
          empty="Nothing finished yet."
          foot={doneMore > 0 ? `${String(doneMore)} older not shown` : undefined}
        >
          {v.done.map((c) => <Card key={c.id} card={c} now={now} />)}
        </Lane>
      </div>
    </BoardShell>
  );
}

function Lane({
  title,
  count,
  dot,
  empty,
  foot,
  children,
}: {
  title: string;
  count: number;
  dot: "idle" | "working" | "waiting" | "";
  empty: string;
  foot?: string;
  children: React.ReactNode[];
}) {
  const id = `board-lane-${title.toLowerCase().replace(/\s+/g, "-")}`;
  return (
    <section className="board-lane" aria-labelledby={id}>
      <header className="board-lane-h">
        {dot ? <span className={`dot ${dot}`} aria-hidden="true" /> : <span className="board-lane-check" aria-hidden="true" />}
        <span className="board-lane-t" id={id}>{title}</span>
        <span className="count tn">{count}</span>
      </header>
      {children.length === 0 ? (
        <p className="board-empty mut">{empty}</p>
      ) : (
        <ul className="board-cards">{children}</ul>
      )}
      {foot && <p className="board-foot mut">{foot}</p>}
    </section>
  );
}

const Card = memo(function Card({ card, now }: { card: BoardCard; now: number }) {
  const pr = prNumber(card.pr_url);
  const latest = card.live?.latest;
  return (
    <li className="board-card">
      <div className="row board-card-top">
        <Tile name={card.project} />
        <span className="mut trunc board-proj">{card.project ?? "No project"}</span>
        <span className="grow" />
        <span className="mut tn board-age" title={card.since !== null ? new Date(card.since * 1000).toLocaleString() : undefined}>
          {age(card.since, now)}
        </span>
      </div>
      <div className="board-title">{card.title}</div>
      {latest && <div className="mut trunc board-latest" title={latest}>{latest}</div>}
      <div className="row board-chips">
        <StateChip card={card} />
        <HoldPill card={card} />
        {card.kind && <span className="pill neutral">{card.kind}</span>}
        <span className="grow" />
        {card.has_report && (
          <button type="button" className="board-ib" aria-label={`Read the report for ${card.title}`} title="Read the report" onClick={() => { openReport(card.id); }}>
            <FileTextIcon size={14} />
          </button>
        )}
        {card.pr_url && (
          <button type="button" className="board-link mono" title={card.pr_url} aria-label={`Open pull request ${card.pr_url}`} onClick={() => void openLink(card.pr_url ?? "")}>
            {pr ?? "PR"}
            <ArrowSquareOutIcon size={12} />
          </button>
        )}
      </div>
    </li>
  );
});

const CallCard = memo(function CallCard({ d, now }: { d: Decision; now: number }) {
  const summary = useMemo(() => firstLine(d.detail), [d.detail]);
  return (
    <li className="board-card board-call">
      <div className="row board-card-top">
        <Tile name={d.project} />
        <span className="mut trunc board-proj">{d.project ?? "No project"}</span>
        <span className="grow" />
        <span className="mut tn board-age">{age(d.raised_at, now)}</span>
      </div>
      <div className="board-title">{d.question}</div>
      {summary && <div className="mut trunc board-latest" title={summary}>{summary}</div>}
      <div className="row board-chips">
        <span className="pill warning">Needs you</span>
        <span className="pill neutral">{decisionTag(d)}</span>
      </div>
    </li>
  );
});
