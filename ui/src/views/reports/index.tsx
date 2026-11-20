import { useEffect, useState } from "react";
import {
  CircleDashedIcon as CircleDashed,
  FileTextIcon as FileText,
  ListBulletsIcon as ListBullets,
  WarningCircleIcon as WarningCircle,
  type Icon,
} from "@phosphor-icons/react";
import { reportRead, statusHistory, type ReportDoc, type StatusEvent, type StatusHistory } from "./bridge";
import "./reports.css";

type Load<T> = { kind: "loading" } | { kind: "ready"; value: T } | { kind: "error"; message: string };

function errorText(e: unknown): string {
  const m = e instanceof Error ? e.message : String(e);
  return m.charAt(0).toUpperCase() + m.slice(1);
}

/**
 * Loads `fetch(taskId)` and reloads it whenever `reloadKey` changes. Mount the host with
 * `key={taskId}` so another task starts from the loading state.
 */
function useLoad<T>(fetch: (taskId: string) => Promise<T>, taskId: string, reloadKey: string | null): Load<T> {
  const [load, setLoad] = useState<Load<T>>({ kind: "loading" });
  useEffect(() => {
    let live = true;
    fetch(taskId).then(
      (value) => {
        if (live) setLoad({ kind: "ready", value });
      },
      (e: unknown) => {
        if (live) setLoad({ kind: "error", message: errorText(e) });
      },
    );
    return () => {
      live = false;
    };
  }, [fetch, taskId, reloadKey]);
  return load;
}

export interface ReportViewProps {
  taskId: string;
  /** Changes whenever the task's latest status event changes; each change reads again. */
  eventKey?: string | null;
}

/** A task's `report.md`, rendered as sanitized Markdown. */
export function ReportView({ taskId, eventKey = null }: ReportViewProps) {
  const load = useLoad(reportRead, taskId, eventKey);
  if (load.kind === "loading") return <ReportsZero icon={CircleDashed} title="Reading the report" text="Opening the task's report." />;
  if (load.kind === "error") return <ReportsZero icon={WarningCircle} title="Report not available" text={load.message} />;
  if (load.value === null)
    return <ReportsZero icon={FileText} title="No report yet" text="The task writes its report when it finishes; it appears here." />;
  return <ReportBody doc={load.value} />;
}

function ReportBody({ doc }: { doc: ReportDoc }) {
  return (
    <div className="reports">
      <div className="reports-meta mut tn">
        <FileText size={14} />
        <span>report.md</span>
        <span aria-hidden="true">·</span>
        <span>{size(doc.bytes)}</span>
        {doc.modified !== null && (
          <>
            <span aria-hidden="true">·</span>
            <span>Updated {stamp(doc.modified)}</span>
          </>
        )}
      </div>
      {doc.truncated && (
        <div className="banner" role="status">
          <WarningCircle size={16} />
          <span className="grow">The report is longer than 1 MB; only its start is shown.</span>
        </div>
      )}
      <article className="reports-doc" tabIndex={0} aria-label="Task report">
        <pre className="reports-src">{doc.markdown}</pre>
      </article>
    </div>
  );
}

/** Events shown before "Show all". */
const FIRST = 5;
/** Events shown at most, so the list never needs virtualizing. */
const MOST = 100;

export interface StatusHistoryProps {
  taskId: string;
  eventKey?: string | null;
}

/** The inspector's "Latest events": the task's status lines, newest first. */
export function StatusHistorySection({ taskId, eventKey = null }: StatusHistoryProps) {
  const load = useLoad(statusHistory, taskId, eventKey);
  const [all, setAll] = useState(false);
  return (
    <section className="reports-history" aria-label="Latest events">
      <span className="reports-h2">Latest events</span>
      {load.kind === "loading" ? (
        <span className="mut reports-note">Reading the status record.</span>
      ) : load.kind === "error" ? (
        <span className="mut reports-note row gap6">
          <WarningCircle size={14} />
          {load.message}
        </span>
      ) : load.value.events.length === 0 ? (
        <span className="mut reports-note row gap6">
          <ListBullets size={14} />
          No status events yet.
        </span>
      ) : (
        <HistoryList history={load.value} all={all} onAll={setAll} />
      )}
      <span className="mut reports-note">An event is a moment, not the current state.</span>
    </section>
  );
}

function HistoryList({ history, all, onAll }: { history: StatusHistory; all: boolean; onAll: (v: boolean) => void }) {
  const shown = history.events.slice(0, all ? MOST : FIRST);
  const more = Math.min(history.events.length, MOST) - FIRST;
  return (
    <>
      <ol className={`reports-events${all ? " all" : ""}`}>
        {shown.map((e, i) => (
          // Events have no id; the list is replaced as a whole on each read.
          <EventRow key={`${String(e.at)}:${String(i)}`} event={e} />
        ))}
      </ol>
      {(more > 0 || all) && (
        <button
          type="button"
          className="btn sm reports-more"
          aria-expanded={all}
          onClick={() => {
            onAll(!all);
          }}
        >
          {all ? "Show fewer" : `Show ${more} more`}
        </button>
      )}
      {all && (history.truncated || history.events.length > MOST) && (
        <span className="mut reports-note">Older events are in the task's status record.</span>
      )}
    </>
  );
}

function EventRow({ event }: { event: StatusEvent }) {
  const word = event.word.toLowerCase();
  const tagged = word !== "working";
  return (
    <li className="reports-evt-row" title={event.at === null ? undefined : new Date(event.at * 1000).toLocaleString()}>
      <span className="reports-tm tn">{event.at === null ? "" : hhmm(event.at)}</span>
      <span className="reports-evt">
        {tagged && <span className={`reports-word ${wordTone(word)}`}>{word}</span>}
        {event.text || <span className="mut">(no text)</span>}
      </span>
    </li>
  );
}

function wordTone(word: string): string {
  switch (word) {
    case "needs-decision":
    case "blocked":
    case "failed":
      return "warn";
    case "done":
    case "resolved":
      return "ok";
    default:
      return "";
  }
}

function hhmm(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
}

function stamp(unixSeconds: number): string {
  const d = new Date(unixSeconds * 1000);
  const today = new Date().toDateString() === d.toDateString();
  return today ? hhmm(unixSeconds) : d.toLocaleDateString([], { day: "numeric", month: "short" }) + " " + hhmm(unixSeconds);
}

function size(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function ReportsZero({ icon: I, title, text }: { icon: Icon; title: string; text: string }) {
  return (
    <div className="reports">
      <div className="zero">
      <span className="zi">
        <I size={18} />
      </span>
      <span>
        <b>{title}</b>
        <span className="mut">{text}</span>
      </span>
      </div>
    </div>
  );
}
