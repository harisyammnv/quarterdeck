import { memo, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowSquareOutIcon as ArrowSquareOut,
  CaretRightIcon as CaretRight,
  CheckCircleIcon as CheckCircle,
  CircleDashedIcon as CircleDashed,
  ClockIcon as Clock,
  FileTextIcon as FileText,
  GitPullRequestIcon as GitPullRequest,
  InfoIcon as Info,
  WarningCircleIcon as WarningCircle,
} from "@phosphor-icons/react";
import { Button, Composer, Keycap, Panel, PanelHead, SegmentedTabs, Tag, VirtualList, ZeroState, useHotkey, useNow } from "../../ui";
import { inApp, urlOpen } from "../../bridge";
import { useApp } from "../../state/app";
import { age, firstLine, prNumber } from "../board/format";
import { reportsArea, takeDecisionRequest } from "../board/nav";
import { CallPill, decisionTag, SnapshotBanner, Tile } from "../board/parts";
import { useBoard } from "../board/useBoard";
import { openPullRequest, type Decision } from "./bridge";
import "./decisions.css";

type Tab = "live" | "deferred";

/** One list row; rows have a fixed height so long lists stay windowed. */
const ROW = 58;

/** Needs your call: every open captain decision, with the reason Firstmate recorded and a reply box. */
export default function DecisionsArea() {
  const load = useBoard();
  const now = useNow(30_000);
  const [request] = useState(takeDecisionRequest);
  const [tab, setTab] = useState<Tab>("live");
  const [sel, setSel] = useState<string | null>(request?.id ?? null);
  const detailRef = useRef<HTMLElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const live = useMemo(() => (load.status === "ready" ? load.view.decisions.live : []), [load]);
  const deferred = useMemo(() => (load.status === "ready" ? load.view.decisions.deferred : []), [load]);
  // A call asked for by id opens on the tab that holds it, once the data is there.
  const effectiveTab: Tab = request && sel === request.id && deferred.some((d) => d.id === sel) ? "deferred" : tab;
  const items = effectiveTab === "live" ? live : deferred;
  const selected = items.find((d) => d.id === sel) ?? items[0] ?? null;

  const focusReply = () => detailRef.current?.querySelector<HTMLInputElement>(".composer input")?.focus();
  // "Reply" on a strip card opens this view with the reply box focused.
  const ready = load.status === "ready";
  useEffect(() => {
    if (ready && request?.reply) focusReply();
  }, [ready, request]);
  const move = (step: number) => {
    if (items.length === 0) return;
    const at = selected ? items.findIndex((d) => d.id === selected.id) : -1;
    const to = Math.min(items.length - 1, Math.max(0, at + step));
    const next = items[to];
    if (!next) return;
    setSel(next.id);
    // The list is windowed: scroll the row into view first, then focus it once it is mounted.
    const box = listRef.current?.querySelector<HTMLElement>(".decisions-vlist");
    if (box) {
      const top = to * ROW;
      if (top < box.scrollTop) box.scrollTop = top;
      else if (top + ROW > box.scrollTop + box.clientHeight) box.scrollTop = top + ROW - box.clientHeight;
    }
    requestAnimationFrame(() => {
      listRef.current?.querySelector<HTMLElement>(`[data-id="${CSS.escape(next.id)}"]`)?.focus({ preventScroll: true });
    });
  };
  useHotkey("j", () => {
    move(1);
  });
  useHotkey("k", () => {
    move(-1);
  });
  useHotkey("r", focusReply, selected !== null);

  if (load.status !== "ready" || !load.view.ready) {
    return (
      <Panel as="main" className="fill" label="Needs your call">
        <PanelHead title="Needs your call" />
        <div className="decisions-state">
          {load.status === "error" ? (
            <ZeroState icon={WarningCircle} title="Decisions could not be read">
              {load.message}
            </ZeroState>
          ) : (
            <ZeroState icon={CircleDashed} title={load.status === "no-home" ? "No Firstmate home" : "Reading the open decisions"}>
              {load.status === "no-home" ? "Choose a home in Settings to see its decisions." : "They appear with the first fleet snapshot."}
            </ZeroState>
          )}
        </div>
      </Panel>
    );
  }

  return (
    <div className="decisions">
      <Panel as="main" className="fill decisions-list" label="Needs your call">
        <div className="phead">
          <h1 className="ptitle">Needs your call</h1>
          <span className="qd-count">{live.length} live</span>
          <span className="grow" />
          <span className="decisions-keys row" aria-hidden="true">
            Move <Keycap>J</Keycap>
            <Keycap>K</Keycap> Open <Keycap>↵</Keycap> Reply <Keycap>R</Keycap>
          </span>
        </div>
        <SnapshotBanner />
        <div className="decisions-tabs">
          <SegmentedTabs
            label="Decisions"
            value={effectiveTab}
            onChange={(t) => {
              setTab(t);
              setSel(null);
            }}
            items={[
              { id: "live", label: "Live", count: live.length },
              { id: "deferred", label: "Deferred", count: load.view.decisions.deferred_total },
            ]}
          />
        </div>
        <div className="decisions-body-l" ref={listRef}>
          {items.length > 0 && (
            <VirtualList
              label={effectiveTab === "live" ? "Waiting on you" : "Not waiting on you right now"}
              className="decisions-vlist"
              items={items}
              rowHeight={ROW}
              rowKey={(d) => d.id}
              render={(d) => <Row d={d} sel={d.id === selected?.id} onSelect={setSel} onOpen={focusReply} now={now} />}
            />
          )}
          {effectiveTab === "live" && deferred.length > 0 && (
            <button
              type="button"
              className="decisions-grp"
              onClick={() => {
                setTab("deferred");
                setSel(null);
              }}
            >
              <Clock size={14} aria-hidden="true" />
              <span>{load.view.decisions.deferred_total} not waiting on you right now</span>
              <span className="decisions-grp-sub trunc">Firstmate brings these back on their date or when the blocker clears</span>
              <CaretRight size={14} aria-hidden="true" />
            </button>
          )}
          {effectiveTab === "deferred" && load.view.decisions.deferred_total > deferred.length && (
            <ZeroState icon={Clock} title={`${String(load.view.decisions.deferred_total - deferred.length)} newer holds not shown`}>
              The oldest {deferred.length} are listed; Firstmate keeps the rest in the backlog.
            </ZeroState>
          )}
          {items.length === 0 ? (
            <ZeroState icon={CheckCircle} title={effectiveTab === "live" ? "Nothing needs your call" : "Nothing is set aside"}>
              {effectiveTab === "live" ? "Firstmate adds a call here when a task waits on you." : "Dated, blocked and aged holds show here with their reason."}
            </ZeroState>
          ) : (
            effectiveTab === "live" &&
            items.length < 8 && (
              <ZeroState end icon={CheckCircle} title="That is everything for now">
                A new call appears here as soon as Firstmate records it.
              </ZeroState>
            )
          )}
        </div>
        <div className="row decisions-foot">
          <Info size={14} aria-hidden="true" />
          <span>Replies go to Firstmate as a note. This app never merges, dispatches or cleans up by itself.</span>
        </div>
      </Panel>
      <Detail key={selected?.id ?? ""} d={selected} now={now} panelRef={detailRef} />
    </div>
  );
}

const Row = memo(function Row({ d, sel, onSelect, onOpen, now }: { d: Decision; sel: boolean; onSelect: (id: string) => void; onOpen: () => void; now: number }) {
  const sub = [prNumber(d.pr_url), firstLine(d.detail)].filter(Boolean).join(", ");
  return (
    <button
      type="button"
      data-id={d.id}
      className={`decisions-li${sel ? " sel" : ""}`}
      aria-current={sel ? "true" : undefined}
      onClick={() => {
        onSelect(d.id);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          onSelect(d.id);
          onOpen();
        }
      }}
    >
      <Tile name={d.project} size={24} />
      <span className="decisions-li-main">
        <span className="decisions-li-t trunc">{d.question}</span>
        <span className="row decisions-li-sub">
          <span className="trunc decisions-li-proj">{d.project ?? "No project"}</span>
          {sub && <span className="trunc">{sub}</span>}
        </span>
      </span>
      <Tag>{decisionTag(d)}</Tag>
      <CallPill bucket={d.bucket} until={d.until} blockedBy={d.blocked_by} ageDays={d.age_days} />
      <span className="tn decisions-li-age">{age(d.raised_at, now)}</span>
      <CaretRight size={14} className="decisions-chev" aria-hidden="true" />
    </button>
  );
});

function Detail({ d, now, panelRef }: { d: Decision | null; now: number; panelRef: React.RefObject<HTMLElement | null> }) {
  const { info, setArea, setSelectedTask } = useApp();
  const [linkError, setLinkError] = useState<string | null>(null);
  const [prError, setPrError] = useState<string | null>(null);
  if (!d) {
    return (
      <Panel as="aside" className="decisions-detail" label="Decision">
        <div className="decisions-state">
          <ZeroState icon={CheckCircle} title="No call selected">
            Pick one on the left to read it.
          </ZeroState>
        </div>
      </Panel>
    );
  }
  const pr = prNumber(d.pr_url);
  const reports = reportsArea;
  return (
    <aside className="qd-panel ui-panel decisions-detail" aria-labelledby="decisions-q" ref={panelRef}>
      <div className="phead">
        <Tile name={d.project} size={22} />
        <h1 className="decisions-q trunc" id="decisions-q" title={d.question}>
          {d.question}
        </h1>
        <span className="grow" />
        <CallPill bucket={d.bucket} until={d.until} blockedBy={d.blocked_by} ageDays={d.age_days} />
      </div>
      <div className="decisions-body">
        <dl className="decisions-facts">
          <dt>Task</dt>
          <dd className="mono">{d.id}</dd>
          {d.project && (
            <>
              <dt>Project</dt>
              <dd>{d.project}</dd>
            </>
          )}
          <dt>Raised</dt>
          <dd className="tn">{d.raised_at !== null ? `${age(d.raised_at, now)} ago` : "Unknown"}</dd>
        </dl>
        {d.pr_url && (
          <div className="decisions-inner">
            <div className="row">
              <GitPullRequest size={16} aria-hidden="true" />
              <span className="decisions-inner-t grow">Pull request</span>
              {pr && <span className="mono">{pr}</span>}
            </div>
            <div className="mono decisions-url">{d.pr_url}</div>
          </div>
        )}
        <section className="decisions-sect" aria-labelledby="decisions-recorded">
          <h2 className="sect" id="decisions-recorded">
            What Firstmate recorded
          </h2>
          <p className="decisions-text">{d.detail ?? "No reason was recorded with this hold."}</p>
        </section>
        {d.blocked_by.length > 0 && (
          <section className="decisions-sect" aria-labelledby="decisions-waiting">
            <h2 className="sect" id="decisions-waiting">
              Waiting on
            </h2>
            <p className="decisions-text mono">{d.blocked_by.join(", ")}</p>
          </section>
        )}
        {d.links.length > 0 && (
          <section className="decisions-sect" aria-labelledby="decisions-links">
            <h2 className="sect" id="decisions-links">
              Links
            </h2>
            <ul className="decisions-links">
              {d.links.map((u) => (
                <li key={u}>
                  <button
                    type="button"
                    className="decisions-linkbtn"
                    title={u}
                    onClick={() => {
                      setLinkError(null);
                      // The core shows the full address in a native confirmation before it opens.
                      urlOpen(u).catch((e: unknown) => {
                        setLinkError(e instanceof Error ? e.message : String(e));
                      });
                    }}
                  >
                    <span className="mono trunc">{u}</span>
                    <ArrowSquareOut size={12} aria-hidden="true" />
                  </button>
                </li>
              ))}
            </ul>
          </section>
        )}
        {(d.pr_url || (d.has_report && reports)) && (
          <div className="row decisions-acts">
            {d.pr_url && (
              <Button
                icon={ArrowSquareOut}
                onClick={() => {
                  setPrError(null);
                  openPullRequest(d.id).catch((e: unknown) => {
                    setPrError(e instanceof Error ? e.message : String(e));
                  });
                }}
              >
                Open pull request
              </Button>
            )}
            {d.has_report && reports && (
              <Button
                icon={FileText}
                onClick={() => {
                  setSelectedTask(d.id);
                  setArea(reports);
                }}
              >
                Read the report
              </Button>
            )}
          </div>
        )}
        {linkError && (
          <p className="decisions-err" role="alert">
            {linkError}
          </p>
        )}
        {prError && (
          <p className="decisions-err" role="alert">
            {prError}
          </p>
        )}
      </div>
      <Composer enabled={inApp && !info?.home_error} about={d.id} placeholder="Write your answer for Firstmate" initialHint="Your answer goes to Firstmate as a note about this task." />
    </aside>
  );
}
