import { memo, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  ArrowsClockwiseIcon as ArrowsClockwise,
  CircleDashedIcon as CircleDashed,
  EyeIcon as Eye,
  TerminalWindowIcon as TerminalWindow,
  WarningCircleIcon as WarningCircle,
} from "@phosphor-icons/react";
import { terminalTail, type Span, type TerminalTail } from "./bridge";
import "./terminal.css";

export interface TerminalViewProps {
  /** The task whose terminal is shown. */
  taskId: string;
  /**
   * Changes whenever that task's latest status event changes (for example its time and word).
   * Each change reads the tail again; there is no timer.
   */
  eventKey?: string | null;
}

type Load =
  | { kind: "loading" }
  | { kind: "ready"; tail: TerminalTail }
  | { kind: "error"; message: string; last: TerminalTail | null };

/**
 * A read-only tail of a worker's terminal, read through Firstmate's peek command. It reads on
 * open, when the task reports a status event, and when asked; one read is in flight at a time,
 * and a request made meanwhile runs once after it. Mount it with `key={taskId}`, so another
 * task starts from an empty buffer; the buffer is released when the view unmounts.
 */
export function TerminalView({ taskId, eventKey = null }: TerminalViewProps) {
  const [load, setLoad] = useState<Load>({ kind: "loading" });
  const [busy, setBusy] = useState(false);
  const inFlight = useRef(false);
  const again = useRef(false);
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const readOnce = useCallback(async () => {
    try {
      const tail = await terminalTail(taskId);
      if (!alive.current) return;
      setLoad((cur) => (cur.kind === "ready" && cur.tail.digest === tail.digest ? cur : { kind: "ready", tail }));
    } catch (e) {
      if (!alive.current) return;
      const message = e instanceof Error ? e.message : String(e);
      setLoad((cur) => ({
        kind: "error",
        message,
        last: cur.kind === "ready" ? cur.tail : cur.kind === "error" ? cur.last : null,
      }));
    }
  }, [taskId]);

  const read = useCallback(async () => {
    if (inFlight.current) {
      again.current = true;
      return;
    }
    inFlight.current = true;
    setBusy(true);
    try {
      await readOnce();
      while (takeAgain(again)) await readOnce();
    } finally {
      inFlight.current = false;
      if (alive.current) setBusy(false);
    }
  }, [readOnce]);

  useEffect(() => {
    void read();
  }, [read, eventKey]);

  const tail = load.kind === "ready" ? load.tail : load.kind === "error" ? load.last : null;

  return (
    <div className="terminal">
      {load.kind === "error" && (
        <div className="banner" role="status">
          <WarningCircle size={16} />
          <span className="trunc grow" title={load.message}>
            {load.message}
          </span>
        </div>
      )}
      {load.kind === "loading" ? (
        <TerminalZero icon={CircleDashed} title="Reading the terminal" text="Asking Firstmate for the latest output." />
      ) : tail === null ? (
        <TerminalZero icon={TerminalWindow} title="No terminal output" text="The terminal tail is shown here when it can be read." />
      ) : tail.lines.length === 0 ? (
        <TerminalZero icon={TerminalWindow} title="No output yet" text="The terminal is open but has not printed anything." />
      ) : (
        <TailPre tail={tail} taskId={taskId} />
      )}
      <div className="terminal-foot">
        <span className="mut row gap6">
          <Eye size={14} />
          Recent output, read when the task reports. Read-only in this version.
        </span>
        <span className="grow" />
        {tail && (
          <span className="mut tn" title={`Captured at ${clock(tail.captured_at)}`}>
            {tail.truncated ? `Last ${tail.lines.length} lines` : lineCount(tail.lines.length)}
          </span>
        )}
        <button
          type="button"
          className="btn sm"
          disabled={busy}
          onClick={() => void read()}
          aria-label="Read the terminal again"
        >
          <ArrowsClockwise size={14} />
          {busy ? "Reading" : "Refresh"}
        </button>
      </div>
    </div>
  );
}

/** Reads and clears the "read again" flag. */
function takeAgain(flag: { current: boolean }): boolean {
  const v = flag.current;
  flag.current = false;
  return v;
}

function lineCount(n: number): string {
  return n === 1 ? "1 line" : `${n} lines`;
}

function clock(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function TailPre({ tail, taskId }: { tail: TerminalTail; taskId: string }) {
  const ref = useRef<HTMLPreElement>(null);
  const pinned = useRef(true);

  // Follow new output while the reader is at the bottom; leave them be when they scrolled up.
  useLayoutEffect(() => {
    const el = ref.current;
    if (el && pinned.current) el.scrollTop = el.scrollHeight;
  }, [tail.digest]);

  return (
    <pre
      ref={ref}
      className="terminal-pre"
      tabIndex={0}
      role="log"
      aria-label={`Recent terminal output of ${taskId}`}
      onScroll={(e) => {
        const el = e.currentTarget;
        pinned.current = el.scrollHeight - el.scrollTop - el.clientHeight < 8;
      }}
    >
      {tail.lines.map((line, i) => (
        // Lines are positional: the tail is replaced as a whole, never edited in place.
        <Line key={i} spans={line} />
      ))}
    </pre>
  );
}

const Line = memo(function Line({ spans }: { spans: Span[] }) {
  return (
    <span className="terminal-line">
      {spans.map((s, i) => (
        <span key={i} className={`terminal-${s.tone}${s.bold ? " terminal-bold" : ""}`}>
          {s.text}
        </span>
      ))}
      {"\n"}
    </span>
  );
});

function TerminalZero({ icon: I, title, text }: { icon: typeof Eye; title: string; text: string }) {
  return (
    <div className="zero terminal-zero">
      <span className="zi">
        <I size={18} />
      </span>
      <span>
        <b>{title}</b>
        <span className="mut">{text}</span>
      </span>
    </div>
  );
}
