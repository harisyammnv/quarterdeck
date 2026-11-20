import { memo } from "react";
import {
  ArrowSquareOutIcon as ArrowSquareOut,
  CircleDashedIcon as CircleDashed,
  FileTextIcon as FileText,
  KanbanIcon as Kanban,
  WarningCircleIcon as WarningCircle,
} from "@phosphor-icons/react";
import { Banner, Count, Panel, PanelHead, StateDot, Tag, VirtualList, ZeroState, ago, useNow, type DotKind } from "../../ui";
import { useApp } from "../../state/app";
import { openPullRequest } from "./bridge";
import { age, firstLine, prNumber } from "./format";
import { requestDecision, useReportOpener } from "./nav";
import { CallPill, decisionTag, LiveChip, SnapshotBanner, Tile } from "./parts";
import type { BoardCard, BoardView, Decision } from "./types";
import { useBoard } from "./useBoard";
import "./board.css";

/** One card's slot in a lane: the card plus the gap below it. Cards have a fixed height. */
const CARD_SLOT = 136;

/** The board: Queued, In flight, Waiting on you and Done, over the backlog. Read-only. */
export default function BoardArea() {
  const load = useBoard();
  const now = useNow(30_000);
  if (load.status !== "ready") {
    return (
      <Panel as="main" className="fill" label="Board">
        <PanelHead title="Board" />
        <div className="board-state">
          {load.status === "error" ? (
            <ZeroState icon={WarningCircle} title="The board could not be read">
              {load.message}
            </ZeroState>
          ) : load.status === "no-home" ? (
            <ZeroState icon={Kanban} title="No Firstmate home">
              Choose a home in Settings to see its backlog.
            </ZeroState>
          ) : (
            <ZeroState icon={CircleDashed} title="Reading the backlog">
              The board appears with the first fleet snapshot.
            </ZeroState>
          )}
        </div>
      </Panel>
    );
  }
  return <Board v={load.view} now={now} />;
}

function Board({ v, now }: { v: BoardView; now: number }) {
  const live = v.decisions.live;
  const doneMore = v.done_total - v.done.length;
  return (
    <>
      <Panel as="main" className="fill board" label="Board">
        <PanelHead title="Board" meta={v.ready ? `${String(v.queued_total)} queued, ${String(v.in_flight.length)} in flight, ${String(live.length)} waiting on you` : undefined}>
          {v.generated_at !== null && <span className="meta tn">Snapshot {ago(v.generated_at * 1000, now)}</span>}
        </PanelHead>
        <SnapshotBanner />
        {v.other.length > 0 && (
          <Banner title={v.other.map((o) => `${o.id}: ${o.why}`).join("\n")}>
            {v.other.length === 1 ? "One backlog record" : `${String(v.other.length)} backlog records`} could not be placed on the board: {v.other.map((o) => o.id).join(", ")}
          </Banner>
        )}
        {!v.ready ? (
          <div className="board-state">
            <ZeroState icon={CircleDashed} title="Reading the backlog">
              The board appears with the first fleet snapshot.
            </ZeroState>
          </div>
        ) : (
          <div className="board-cols">
            <Lane title="Queued" dot="idle" count={v.queued_total} empty="Nothing queued." foot={queuedFoot(v)}>
              <Cards label="Queued" items={v.queued} now={now} />
            </Lane>
            <Lane title="In flight" dot="working" count={v.in_flight.length} empty="No task is running.">
              <Cards label="In flight" items={v.in_flight} now={now} />
            </Lane>
            <Lane title="Waiting on you" dot="waiting" count={live.length} empty="Nothing needs your call.">
              <VirtualList label="Waiting on you" className="board-cards" items={live} rowHeight={CARD_SLOT} rowKey={(d) => d.id} render={(d) => <CallCard d={d} now={now} />} />
            </Lane>
            <Lane title="Done" dot={null} count={v.done_total} empty="Nothing finished yet." foot={doneMore > 0 ? `${String(doneMore)} older not shown` : undefined}>
              <Cards label="Done" items={v.done} now={now} />
            </Lane>
          </div>
        )}
      </Panel>
    </>
  );
}

/** What the Queued lane holds beyond plain queued work, and what it leaves out. */
function queuedFoot(v: BoardView): string | undefined {
  const parts: string[] = [];
  if (v.queued_held > 0) parts.push(`Includes ${String(v.queued_held)} deferred ${v.queued_held === 1 ? "hold" : "holds"}, also under Needs your call`);
  const more = v.queued_total - v.queued.length;
  if (more > 0) parts.push(`${String(more)} further down the backlog not shown`);
  return parts.length > 0 ? parts.join(". ") + "." : undefined;
}

function Cards({ label, items, now }: { label: string; items: BoardCard[]; now: number }) {
  return <VirtualList label={label} className="board-cards" items={items} rowHeight={CARD_SLOT} rowKey={(c) => c.id} render={(c) => <Card card={c} now={now} />} />;
}

function Lane({ title, dot, count, empty, foot, children }: { title: string; dot: DotKind | null; count: number; empty: string; foot?: string; children: React.ReactNode }) {
  const id = `board-lane-${title.toLowerCase().replace(/\s+/g, "-")}`;
  return (
    <section className="board-lane" aria-labelledby={id}>
      <header className="board-lane-h">
        {dot ? <StateDot kind={dot} /> : <span className="board-lane-ring" aria-hidden="true" />}
        <h2 className="board-lane-t" id={id}>
          {title}
        </h2>
        <Count n={count} label={`${String(count)} ${title.toLowerCase()}`} />
      </header>
      {count === 0 ? <p className="board-empty">{empty}</p> : children}
      {foot && <p className="board-foot">{foot}</p>}
    </section>
  );
}

function Top({ project, since, now }: { project: string | null; since: number | null; now: number }) {
  return (
    <div className="row board-card-top">
      <Tile name={project} />
      <span className="trunc board-proj">{project ?? "No project"}</span>
      <span className="grow" />
      <span className="tn board-age" title={since !== null ? new Date(since * 1000).toLocaleString() : undefined}>
        {age(since, now)}
      </span>
    </div>
  );
}

const Card = memo(function Card({ card, now }: { card: BoardCard; now: number }) {
  const readReport = useReportOpener()(card.id, card.has_report);
  const pr = prNumber(card.pr_url);
  const latest = card.live?.latest ?? null;
  const hold = card.hold;
  return (
    <article className="board-card" aria-label={card.title}>
      <Top project={card.project} since={card.since} now={now} />
      <div className="board-title" title={card.title}>
        {card.title}
      </div>
      <div className={`trunc board-sub${latest ? "" : " mono"}`} title={latest ?? card.id}>
        {latest ?? card.id}
      </div>
      <div className="row board-chips">
        <LiveChip card={card} />
        {(hold || card.blocked_by.length > 0) && <CallPill bucket={hold} until={card.until} blockedBy={card.blocked_by} ageDays={null} />}
        {card.kind && <Tag>{card.kind}</Tag>}
        <span className="grow" />
        {readReport && (
          <button type="button" className="board-ib" aria-label={`Read the report for ${card.title}`} title="Read the report" onClick={readReport}>
            <FileText size={14} />
          </button>
        )}
        {card.pr_url && (
          <button type="button" className="board-link" title={card.pr_url} aria-label={`Open pull request ${card.pr_url}`} onClick={() => void openPullRequest(card.id).catch(() => undefined)}>
            <span className="mono">{pr ?? "PR"}</span>
            <ArrowSquareOut size={12} />
          </button>
        )}
      </div>
    </article>
  );
});

const CallCard = memo(function CallCard({ d, now }: { d: Decision; now: number }) {
  const { setArea } = useApp();
  return (
    <article className="board-card board-call" aria-label={d.question}>
      <Top project={d.project} since={d.raised_at} now={now} />
      <button
        type="button"
        className="board-title board-title-btn"
        title={d.question}
        onClick={() => {
          requestDecision(d.id);
          setArea("decisions");
        }}
      >
        {d.question}
      </button>
      <div className="trunc board-sub" title={firstLine(d.detail)}>
        {firstLine(d.detail) || "No reason recorded."}
      </div>
      <div className="row board-chips">
        <CallPill bucket="live" until={null} blockedBy={[]} ageDays={null} />
        <Tag>{decisionTag(d)}</Tag>
      </div>
    </article>
  );
});
