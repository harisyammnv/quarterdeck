import { useRef } from "react";
import { Button, Kbd, Tag, useHotkey } from "../../ui";
import { useApp } from "../../state/app";
import { openPullRequest } from "../board/bridge";
import { firstLine, prNumber } from "../board/format";
import { reportsArea, requestDecision } from "../board/nav";
import { decisionTag, Tile } from "../board/parts";
import type { Decision } from "../board/types";
import { useBoard } from "../board/useBoard";
import "./decisions.css";

/** How many calls the strip shows as cards; the count beside the title is never capped. */
const STRIP_CARDS = 3;

/**
 * The teal "Needs your call" room. It shows only live calls (`captain_actionable`), oldest
 * first, and renders nothing when there are none. J moves focus to the next call card.
 * Self-contained: mount it with `<DecisionsStrip />` at the top of a view.
 */
export function DecisionsStrip() {
  const load = useBoard();
  const { setArea } = useApp();
  const ref = useRef<HTMLElement>(null);
  const live = load.status === "ready" ? load.view.decisions.live : [];

  useHotkey(
    "j",
    () => {
      const cards = Array.from(ref.current?.querySelectorAll<HTMLElement>("[data-call]") ?? []);
      const at = cards.findIndex((c) => c.contains(document.activeElement));
      cards[(at + 1) % cards.length]?.focus();
    },
    live.length > 0,
  );

  if (live.length === 0) return null;
  const open = (id: string | null, reply = false) => {
    requestDecision(id, reply);
    setArea("decisions");
  };
  const shown = live.slice(0, STRIP_CARDS);
  const more = live.length - shown.length;
  return (
    <section className="decisions-strip" aria-labelledby="decisions-strip-t" ref={ref}>
      <div className="row">
        <h2 className="decisions-strip-t" id="decisions-strip-t">
          Needs your call
        </h2>
        <span className="decisions-strip-n tn" aria-label={`${String(live.length)} live`}>
          {live.length}
        </span>
        <span className="grow" />
        {more > 0 && (
          <button
            type="button"
            className="decisions-strip-more"
            onClick={() => {
              open(null);
            }}
          >
            {more} more
          </button>
        )}
        <span className="decisions-strip-hint row">
          Jump to next <Kbd>J</Kbd>
        </span>
      </div>
      <div className="decisions-cards">
        {shown.map((d) => (
          <CallCard key={d.id} d={d} open={open} />
        ))}
      </div>
    </section>
  );
}

function CallCard({ d, open }: { d: Decision; open: (id: string | null, reply?: boolean) => void }) {
  const { setArea, setSelectedTask } = useApp();
  const pr = prNumber(d.pr_url);
  const reports = reportsArea;
  const primary = d.pr_url
    ? { label: "Open pull request", run: () => void openPullRequest(d.id).catch(() => undefined) }
    : d.has_report && reports
      ? {
          label: "Read the report",
          run: () => {
            setSelectedTask(d.id);
            setArea(reports);
          },
        }
      : {
          label: "Read the options",
          run: () => {
            open(d.id);
          },
        };
  return (
    <article className="decisions-dcard" data-call tabIndex={-1} aria-label={d.question}>
      <div className="row gap6">
        <Tile name={d.project} />
        <span className="trunc decisions-dcard-proj">{d.project ?? "No project"}</span>
        {pr && <span className="mono decisions-dcard-pr">{pr}</span>}
        <span className="grow" />
        <Tag>{decisionTag(d)}</Tag>
      </div>
      <button
        type="button"
        className="decisions-dcard-t trunc"
        title={d.question}
        onClick={() => {
          open(d.id);
        }}
      >
        {d.question}
      </button>
      <div className="trunc decisions-dcard-sub">{firstLine(d.detail) || "No reason recorded."}</div>
      <div className="row decisions-dcard-acts">
        <Button size="sm" variant="primary" onClick={primary.run}>
          {primary.label}
        </Button>
        <Button
          size="sm"
          onClick={() => {
            open(d.id, true);
          }}
        >
          Reply
        </Button>
      </div>
    </article>
  );
}
