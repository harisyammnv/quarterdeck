import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowSquareOutIcon, CaretRightIcon, CheckCircleIcon, ClockIcon, FileTextIcon, GitPullRequestIcon, InfoIcon, PaperPlaneTiltIcon, WarningIcon } from "@phosphor-icons/react";
import { age, firstLine, prNumber } from "../board/format";
import { decisionTag, holdWords, Tile } from "../board/parts";
import { useBoard, useNow } from "../board/useBoard";
import { openLink, openReport, replyToDecision, type Decision } from "./bridge";
import { takeDecisionRequest } from "./focus";
import "./decisions.css";

type Tab = "live" | "deferred";

/** Needs your call: every open captain decision, live first, with a reply box. Read-only. */
export default function DecisionsView() {
  const load = useBoard();
  const now = useNow();
  // A hand-off from the strip, taken once. The Live tab lists deferred holds too, so it
  // always shows the requested call.
  const [request] = useState(takeDecisionRequest);
  const [tab, setTab] = useState<Tab>("live");
  const [sel, setSel] = useState<string | null>(request?.id ?? null);
  const replyRef = useRef<HTMLTextAreaElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const live = useMemo(() => (load.status === "ready" ? load.view.decisions.live : []), [load]);
  const deferred = useMemo(() => (load.status === "ready" ? load.view.decisions.deferred : []), [load]);
  // The Live tab lists the live calls and, below them, what is set aside; Deferred only the latter.
  const order = useMemo(() => (tab === "live" ? [...live, ...deferred] : deferred), [tab, live, deferred]);
  const selected = order.find((d) => d.id === sel) ?? order[0] ?? null;

  const ready = load.status === "ready";
  useEffect(() => {
    if (ready && request?.reply) replyRef.current?.focus();
  }, [ready, request]);

  const move = useCallback(
    (step: number) => {
      if (order.length === 0) return;
      const at = selected ? order.findIndex((d) => d.id === selected.id) : -1;
      const next = order[Math.min(order.length - 1, Math.max(0, at + step))];
      if (next) {
        setSel(next.id);
        listRef.current?.querySelector<HTMLElement>(`[data-id="${CSS.escape(next.id)}"]`)?.focus();
      }
    },
    [order, selected],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const t = e.target;
      if (t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName))) return;
      if (e.key === "j") {
        e.preventDefault();
        move(1);
      } else if (e.key === "k") {
        e.preventDefault();
        move(-1);
      } else if (e.key === "r" && selected) {
        e.preventDefault();
        replyRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [move, selected]);

  if (load.status !== "ready" || !load.view.ready) {
    const err = load.status === "error";
    return (
      <section className="panel decisions-list">
        <div className="phead"><span className="ptitle">Needs your call</span></div>
        <div className="zero" role={err ? "alert" : "status"}>
          <span className="zi">{err ? <WarningIcon size={16} /> : <ClockIcon size={16} />}</span>
          <span>
            <b>{err ? "Decisions could not be read" : "Reading the open decisions"}</b>
            <span className="t12">{err ? load.message : "They appear when the first fleet snapshot lands."}</span>
          </span>
        </div>
      </section>
    );
  }

  return (
    <div className="decisions">
      <section className="panel decisions-list" aria-labelledby="decisions-title">
        <div className="phead">
          <span className="ptitle" id="decisions-title">Needs your call</span>
          <span className="pill accent tn">{live.length} live</span>
          <span className="grow" />
          <span className="t12 mut row gap6 decisions-keys">
            Move <span className="kbd">J</span><span className="kbd">K</span> Open <span className="kbd">↵</span> Reply <span className="kbd">R</span>
          </span>
        </div>
        <div className="decisions-tabs" role="tablist" aria-label="Decisions">
          <span className="segtabs">
            <TabButton on={tab === "live"} onClick={() => { setTab("live"); }} label="Live" count={live.length} />
            <TabButton on={tab === "deferred"} onClick={() => { setTab("deferred"); }} label="Deferred" count={deferred.length} />
          </span>
        </div>
        <div className="decisions-scroll" ref={listRef} role="tabpanel">
          {tab === "live" && (
            <List items={live} selected={selected?.id ?? null} onSelect={setSel} now={now} onOpen={() => replyRef.current?.focus()} />
          )}
          {tab === "live" && deferred.length > 0 && (
            <div className="decisions-grp">
              <ClockIcon size={14} aria-hidden="true" />
              <span>Not waiting on you right now</span>
              <span className="mut t12 trunc">Firstmate brings these back on their date or when the blocker clears</span>
            </div>
          )}
          <List items={deferred} selected={selected?.id ?? null} onSelect={setSel} now={now} onOpen={() => replyRef.current?.focus()} />
          {order.length > 0 && (
            <div className="decisions-end">
              <span className="zi"><CheckCircleIcon size={18} /></span>
              <span>
                <b>That is everything for now</b>
                <span className="t12">A new call appears here as soon as Firstmate records it.</span>
              </span>
            </div>
          )}
          {order.length === 0 && (
            <div className="decisions-end">
              <span className="zi"><CheckCircleIcon size={18} /></span>
              <span>
                <b>{tab === "live" ? "Nothing needs your call" : "Nothing is set aside"}</b>
                <span className="t12">{tab === "live" ? "Firstmate adds a call here when a task waits on you." : "Deferred, blocked and aged holds show here with their reason."}</span>
              </span>
            </div>
          )}
        </div>
        <div className="row t12 mut decisions-foot">
          <InfoIcon size={14} aria-hidden="true" />
          <span>Replies go to Firstmate as a note. This app never merges, dispatches or cleans up by itself.</span>
        </div>
      </section>
      <Detail key={selected?.id ?? ""} d={selected} now={now} replyRef={replyRef} />
    </div>
  );
}

function TabButton({ on, onClick, label, count }: { on: boolean; onClick: () => void; label: string; count: number }) {
  return (
    <button type="button" role="tab" aria-selected={on} className={`tab${on ? " on" : ""}`} onClick={onClick}>
      {label}
      <span className="count tn">{count}</span>
    </button>
  );
}

const List = memo(function List({
  items,
  selected,
  onSelect,
  onOpen,
  now,
}: {
  items: Decision[];
  selected: string | null;
  onSelect: (id: string) => void;
  onOpen: () => void;
  now: number;
}) {
  return (
    <ul className="decisions-ul">
      {items.map((d) => (
        <Row key={d.id} d={d} sel={d.id === selected} onSelect={onSelect} onOpen={onOpen} now={now} />
      ))}
    </ul>
  );
});

const Row = memo(function Row({
  d,
  sel,
  onSelect,
  onOpen,
  now,
}: {
  d: Decision;
  sel: boolean;
  onSelect: (id: string) => void;
  onOpen: () => void;
  now: number;
}) {
  const hold = holdWords(d.bucket, d.until, d.blocked_by, d.age_days);
  const sub = [prNumber(d.pr_url), firstLine(d.detail)].filter(Boolean).join(", ");
  return (
    <li>
      <button
        type="button"
        data-id={d.id}
        className={`decisions-li${sel ? " sel" : ""}`}
        aria-current={sel ? "true" : undefined}
        onClick={() => { onSelect(d.id); }}
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
          <span className="row gap6 t12 mut decisions-li-sub">
            <span className="trunc decisions-li-proj">{d.project ?? "No project"}</span>
            {sub && <span className="trunc">{sub}</span>}
          </span>
        </span>
        <span className="pill neutral">{decisionTag(d)}</span>
        {hold ? <span className={`pill ${hold.chip}`}>{hold.text}</span> : <span className="pill warning">Needs you</span>}
        <span className="mut t12 tn decisions-li-age">{age(d.raised_at, now)}</span>
        <CaretRightIcon size={14} className="decisions-chev" aria-hidden="true" />
      </button>
    </li>
  );
});

type SendState = { kind: "idle" } | { kind: "sending" } | { kind: "sent"; text: string } | { kind: "error"; text: string };

function Detail({ d, now, replyRef }: { d: Decision | null; now: number; replyRef: React.RefObject<HTMLTextAreaElement | null> }) {
  const [text, setText] = useState("");
  const [send, setSend] = useState<SendState>({ kind: "idle" });

  if (!d) {
    return (
      <section className="panel decisions-detail">
        <div className="zero">
          <span className="zi"><CheckCircleIcon size={16} /></span>
          <span><b>No call selected</b><span className="t12">Pick one on the left to read it.</span></span>
        </div>
      </section>
    );
  }
  const hold = holdWords(d.bucket, d.until, d.blocked_by, d.age_days);
  const pr = prNumber(d.pr_url);
  const submit = () => {
    if (!text.trim() || send.kind === "sending") return;
    setSend({ kind: "sending" });
    replyToDecision(d.id, text).then(
      () => {
        setSend({ kind: "sent", text: "Sent. Firstmate reads it at its next turn." });
        setText("");
      },
      (e: unknown) => {
        setSend({ kind: "error", text: e instanceof Error ? e.message : String(e) });
      },
    );
  };
  return (
    <section className="panel decisions-detail" aria-labelledby="decisions-q">
      <div className="phead">
        <Tile name={d.project} size={22} />
        <span className="decisions-q trunc" id="decisions-q" title={d.question}>{d.question}</span>
        <span className="grow" />
        {hold ? <span className={`pill ${hold.chip}`}>{hold.text}</span> : <span className="pill warning">Needs you</span>}
      </div>
      <div className="decisions-body">
        <dl className="decisions-facts t12">
          <dt className="mut">Task</dt>
          <dd className="mono">{d.id}</dd>
          {d.project && (<><dt className="mut">Project</dt><dd>{d.project}</dd></>)}
          <dt className="mut">Raised</dt>
          <dd className="tn">{d.raised_at !== null ? `${age(d.raised_at, now)} ago` : "Unknown"}</dd>
        </dl>
        {d.pr_url && (
          <div className="decisions-inner">
            <div className="row">
              <GitPullRequestIcon size={16} aria-hidden="true" />
              <span className="decisions-inner-t grow">Pull request</span>
              {pr && <span className="mono t12">{pr}</span>}
            </div>
            <div className="mut t12 mono decisions-url">{d.pr_url}</div>
          </div>
        )}
        <div className="decisions-sect">
          <span className="sect">What Firstmate recorded</span>
          <p className="decisions-detail-text">{d.detail ?? "No reason was recorded with this hold."}</p>
        </div>
        {d.blocked_by.length > 0 && (
          <div className="decisions-sect">
            <span className="sect">Waiting on</span>
            <p className="decisions-detail-text mono">{d.blocked_by.join(", ")}</p>
          </div>
        )}
        {d.links.length > 0 && (
          <div className="decisions-sect">
            <span className="sect">Links</span>
            <ul className="decisions-links">
              {d.links.map((u) => (
                <li key={u}>
                  <button type="button" className="decisions-linkbtn" onClick={() => void openLink(u)} title={u}>
                    <span className="mono trunc">{u}</span>
                    <ArrowSquareOutIcon size={12} aria-hidden="true" />
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
      <div className="decisions-compose">
        <label className="sr-only" htmlFor="decisions-reply">Reply to Firstmate about {d.id}</label>
        <textarea
          id="decisions-reply"
          ref={replyRef}
          className="decisions-reply"
          rows={2}
          maxLength={4000}
          placeholder="Write your answer for Firstmate"
          value={text}
          onChange={(e) => { setText(e.target.value); }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              submit();
            }
          }}
        />
        {send.kind === "sent" && <p className="t12 decisions-ok" role="status">{send.text}</p>}
        {send.kind === "error" && <p className="t12 decisions-err" role="alert">{send.text}</p>}
        <div className="row">
          <button type="button" className="btn primary" disabled={!text.trim() || send.kind === "sending"} onClick={submit}>
            <PaperPlaneTiltIcon size={14} aria-hidden="true" />
            {send.kind === "sending" ? "Sending" : "Send reply"}
            <span className="kbd">⌘↵</span>
          </button>
          {d.pr_url && (
            <button type="button" className="btn" onClick={() => void openLink(d.pr_url ?? "")}>
              <ArrowSquareOutIcon size={14} aria-hidden="true" />
              Open pull request
            </button>
          )}
          {d.has_report && (
            <button type="button" className="btn" onClick={() => { openReport(d.id); }}>
              <FileTextIcon size={14} aria-hidden="true" />
              Read the report
            </button>
          )}
        </div>
      </div>
    </section>
  );
}
