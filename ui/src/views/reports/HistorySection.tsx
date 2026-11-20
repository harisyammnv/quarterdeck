import { useState } from "react";
import { ListBulletsIcon as ListBullets, WarningCircleIcon as WarningCircle } from "@phosphor-icons/react";
import { Button } from "../../ui";
import { useApp } from "../../state/app";
import { reportHistory, type ReportHistory, type StatusEvent } from "./bridge";
import { useLoad } from "./load";
import "./reports.css";

/** Events shown before "Show more". */
const FIRST = 5;
/** Events shown at most, so the list never needs virtualizing. */
const MOST = 100;

/** The inspector's status history: the task's status lines, newest first. */
export default function HistorySection() {
  const { live, selectedTask } = useApp();
  const row = live.view?.rows.find((r) => r.id === selectedTask) ?? null;
  if (!row) return null;
  const ev = row.latest_event;
  const eventKey = ev ? `${String(ev.at)}:${ev.word}:${ev.text}` : null;
  return <History key={row.id} taskId={row.id} eventKey={eventKey} />;
}

function History({ taskId, eventKey }: { taskId: string; eventKey: string | null }) {
  const load = useLoad(reportHistory, taskId, eventKey);
  const [all, setAll] = useState(false);
  return (
    <section aria-label="Status history">
      <h2 className="h2">Status history</h2>
      {load.kind === "loading" ? (
        <span className="mut t12">Reading the status record.</span>
      ) : load.kind === "error" ? (
        <span className="mut t12 row gap6">
          <WarningCircle size={14} />
          {load.message}
        </span>
      ) : load.value.events.length === 0 ? (
        <span className="mut t12 row gap6">
          <ListBullets size={14} />
          No status events yet.
        </span>
      ) : (
        <HistoryList history={load.value} all={all} onAll={setAll} />
      )}
      <span className="mut t12">An event is a moment, not the current state.</span>
    </section>
  );
}

function HistoryList({ history, all, onAll }: { history: ReportHistory; all: boolean; onAll: (v: boolean) => void }) {
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
        <Button
          size="sm"
          className="reports-more"
          aria-expanded={all}
          onClick={() => {
            onAll(!all);
          }}
        >
          {all ? "Show fewer" : `Show ${more} more`}
        </Button>
      )}
      {all && history.events.length > MOST && <span className="mut t12">Older events are in the task's status record.</span>}
    </>
  );
}

function EventRow({ event }: { event: StatusEvent }) {
  const word = event.word.toLowerCase();
  return (
    <li className="evt" title={event.at === null ? undefined : new Date(event.at * 1000).toLocaleString()}>
      <span className="tm">{event.at === null ? "" : hhmm(event.at)}</span>
      <span className="reports-evt">
        {word !== "working" && <span className={`reports-word ${wordTone(word)}`}>{word}</span>}
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
