import { useCallback, useEffect, useRef, useState } from "react";
import {
  AnchorSimpleIcon as AnchorSimple,
  ArrowsClockwiseIcon as ArrowsClockwise,
  BinocularsIcon as Binoculars,
  CheckIcon as Check,
  CircleDashedIcon as CircleDashed,
  DesktopIcon as Desktop,
  HandPalmIcon as HandPalm,
  MoonIcon as Moon,
  PaperPlaneTiltIcon as PaperPlaneTilt,
  PauseIcon as Pause,
  QuestionIcon as Question,
  SailboatIcon as Sailboat,
  SunIcon as Sun,
  WarningCircleIcon as WarningCircle,
  type Icon,
} from "@phosphor-icons/react";
import {
  appInfo,
  fleetView,
  inApp,
  onFleetUpdated,
  refreshFleet,
  sendNote,
  viewRendered,
  type AppInfo,
  type FleetRow,
  type FleetView,
} from "./bridge";
import { STATE_LOOK, ago, monogram, tileFill } from "./states";

type ThemePref = "system" | "light" | "dark";
const THEME_KEY = "quarterdeck.theme";

function readThemePref(): ThemePref {
  const v = localStorage.getItem(THEME_KEY);
  return v === "light" || v === "dark" ? v : "system";
}

function useNow(intervalMs: number): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = setInterval(() => {
      setNow(Date.now());
    }, intervalMs);
    return () => {
      clearInterval(t);
    };
  }, [intervalMs]);
  return now;
}

export function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [live, setLive] = useState<Live>({ view: null, changed: new Map() });
  const [theme, setTheme] = useState<ThemePref>(readThemePref);
  const [ipcError, setIpcError] = useState<string | null>(null);
  const now = useNow(1000);
  const { view, changed } = live;

  const accept = useCallback((next: FleetView) => {
    setLive((cur) => nextLive(cur, next));
  }, []);

  useEffect(() => {
    const stop = onFleetUpdated(accept);
    const fail = (e: unknown) => {
      setIpcError(e instanceof Error ? e.message : String(e));
    };
    appInfo()
      .then((i) => {
        setInfo(i);
        if (i.theme === "light" || i.theme === "dark") setTheme(i.theme);
      })
      .catch(fail);
    fleetView()
      .then((v) => {
        if (v) accept(v);
      })
      .catch(fail);
    return stop;
  }, [accept]);

  // Theme: pinned light or dark sets data-theme; system leaves it to prefers-color-scheme.
  useEffect(() => {
    const root = document.documentElement;
    if (theme === "system") root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", theme);
  }, [theme]);

  const chooseTheme = (t: ThemePref) => {
    setTheme(t);
    if (t === "system") localStorage.removeItem(THEME_KEY);
    else localStorage.setItem(THEME_KEY, t);
  };

  // A view caused by a status change reports when it reached the screen (latency probe).
  useEffect(() => {
    if (view?.cause) {
      const seq = view.seq;
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          viewRendered(seq, Date.now());
        });
      });
    }
  }, [view]);

  const rows = view?.rows ?? [];
  const needYou = rows.filter((r) => r.state === "needs-decision").length;
  const home = info?.home ?? view?.home ?? null;

  return (
    <div className="app">
      <header className="titlebar" data-tauri-drag-region>
        <span className="wtitle" data-tauri-drag-region>
          Quarterdeck
        </span>
        <span className="crumb" data-tauri-drag-region>
          {home ? `${home} / Fleet` : "Fleet"}
        </span>
        <span className="grow" data-tauri-drag-region />
        <ThemeSwitch value={theme} onChange={chooseTheme} />
      </header>

      <div className="shell">
        <nav className="rail" aria-label="Areas">
          <span className="brand" aria-hidden="true">
            <AnchorSimple size={22} />
          </span>
          <button type="button" className="ri" aria-current="page" aria-label={`Fleet${needYou ? `, ${needYou} need you` : ""}`}>
            <span className="ic">
              <Sailboat size={18} weight="duotone" />
              {needYou > 0 && <span className="badge tn">{needYou}</span>}
            </span>
            <span className="lb">Fleet</span>
          </button>
        </nav>

        <main className="panel">
          <div className="phead">
            <span className="ptitle">Fleet</span>
            {view && <span className="meta tn">{taskCount(rows.length)}</span>}
            <span className="grow" />
            {view && <SnapshotMeta view={view} now={now} />}
            <button
              type="button"
              className="btn sm"
              disabled={!view || view.snapshot.running}
              onClick={() => void refreshFleet()}
            >
              <ArrowsClockwise size={14} />
              Refresh
            </button>
          </div>

          {ipcError && (
            <div className="banner" role="status">
              <WarningCircle size={16} />
              <span className="trunc grow" title={ipcError}>
                The window could not reach the core: {ipcError}
              </span>
            </div>
          )}
          {view?.snapshot.error && (
            <div className="banner" role="status">
              <WarningCircle size={16} />
              <span className="trunc grow" title={view.snapshot.error}>
                Snapshot failed, showing the last known fleet: {view.snapshot.error}
              </span>
            </div>
          )}
          {view && view.snapshot.omitted.length > 0 && (
            <div className="banner" role="status">
              <WarningCircle size={16} />
              <span className="trunc grow">The snapshot left tasks out: {view.snapshot.omitted.join(", ")}</span>
            </div>
          )}

          <div className="fleet">
            {info?.home_error ? (
              <Zero icon={WarningCircle} title="No Firstmate home" text={sentence(info.home_error)} />
            ) : !view ? (
              <Zero icon={CircleDashed} title="Reading the fleet" text="Looking at the home's records." />
            ) : rows.length === 0 ? (
              <Zero icon={Sailboat} title="No tasks in flight" text="New tasks appear here as soon as Firstmate starts them." />
            ) : (
              <FleetTable rows={rows} now={now} changed={changed} />
            )}
          </div>

          <Composer enabled={inApp && !info?.home_error && view !== null} />
        </main>
      </div>

      <footer className="statusbar">
        <span className="row">
          <span className={`dot ${view?.snapshot.error ? "blocked" : view ? "working" : "idle"}`} />
          {home ? `Watching ${home}` : "Not watching"}
        </span>
        <span className="row tn">
          <Sailboat size={14} />
          {taskCount(rows.length)}
        </span>
        <span className="row tn">
          <HandPalm size={14} />
          {needYou} need you
        </span>
        <span className="grow" />
        {!inApp && <span className="row">Preview data</span>}
        <span className="row">Read-only companion</span>
      </footer>
    </div>
  );
}

interface Live {
  view: FleetView | null;
  /** Rows whose state word changed, keyed by id, with the view seq that changed them. */
  changed: ReadonlyMap<string, number>;
}

/** Accepts a newer view and marks the rows whose state changed, so they pulse once. */
function nextLive(cur: Live, next: FleetView): Live {
  if (cur.view && cur.view.seq >= next.seq) return cur;
  if (!cur.view) return { view: next, changed: cur.changed };
  const before = new Map(cur.view.rows.map((r) => [r.id, r.state_word]));
  const changed = new Map(cur.changed);
  for (const r of next.rows) {
    const was = before.get(r.id);
    if (was !== undefined && was !== r.state_word) changed.set(r.id, next.seq);
  }
  return { view: next, changed };
}

/** Capitalises the first letter, for messages that come from the core in lower case. */
function sentence(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function taskCount(n: number): string {
  return n === 1 ? "1 task" : `${n} tasks`;
}

function SnapshotMeta({ view, now }: { view: FleetView; now: number }) {
  const s = view.snapshot;
  const text = s.running
    ? "Reading snapshot"
    : s.generated_at
      ? `Snapshot ${ago(s.generated_at * 1000, now)}`
      : "No snapshot yet";
  return <span className="meta tn">{text}</span>;
}

function ThemeSwitch({ value, onChange }: { value: ThemePref; onChange: (t: ThemePref) => void }) {
  const opts: [ThemePref, Icon, string][] = [
    ["light", Sun, "Light"],
    ["dark", Moon, "Dark"],
    ["system", Desktop, "Match the system"],
  ];
  return (
    <span className="seg" role="group" aria-label="Theme">
      {opts.map(([t, I, label]) => (
        <button
          key={t}
          type="button"
          aria-pressed={value === t}
          aria-label={label}
          title={label}
          onClick={() => {
            onChange(t);
          }}
        >
          <I size={14} />
        </button>
      ))}
    </span>
  );
}

function Zero({ icon: I, title, text }: { icon: Icon; title: string; text: string }) {
  return (
    <div className="zero">
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

function KindPill({ kind }: { kind: string | null }) {
  const k = kind ?? "task";
  const I = k === "ship" ? Sailboat : k === "scout" ? Binoculars : CircleDashed;
  return (
    <span className="pill neutral">
      <I size={12} />
      {k}
    </span>
  );
}

function StatePill({ row }: { row: FleetRow }) {
  const look = STATE_LOOK[row.state];
  const word = row.state === "unknown" && row.state_word !== "unknown" ? row.state_word : look.word;
  const I = row.state === "done" ? Check : row.state === "paused" ? Pause : row.state === "unknown" ? Question : null;
  const title =
    row.source === "event"
      ? "From the latest status event: newer than the snapshot, or the snapshot could not tell"
      : row.source === "bootstrap"
        ? "From the task's status record, before the first snapshot"
        : "From the fleet snapshot";
  return (
    <span className={`pill ${look.chip}`} title={title}>
      {I && <I size={12} weight="bold" />}
      {word}
    </span>
  );
}

function FleetTable({ rows, now, changed }: { rows: FleetRow[]; now: number; changed: ReadonlyMap<string, number> }) {
  return (
    <table>
      <colgroup>
        <col style={{ width: "36%" }} />
        <col style={{ width: "19%" }} />
        <col style={{ width: "10%" }} />
        <col style={{ width: "13%" }} />
        <col />
      </colgroup>
      <thead>
        <tr>
          <th>Task</th>
          <th>Project</th>
          <th>Kind</th>
          <th>State</th>
          <th>Latest event</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r) => {
          const pulse = changed.get(r.id);
          const ev = r.latest_event;
          const latest = ev?.text ?? r.doing ?? "";
          return (
            <tr key={`${r.id}:${String(pulse ?? 0)}`} className={pulse ? "changed" : undefined}>
              <td>
                <span className="row">
                  <span className="task trunc" title={r.name}>
                    {r.name}
                  </span>
                  <span className="tid mono trunc" style={{ flex: "none", maxWidth: "40%" }}>
                    {r.id}
                  </span>
                </span>
              </td>
              <td>
                <span className="row">
                  <span className="mono-ic" style={{ background: tileFill(r.project ?? r.id) }} aria-hidden="true">
                    {monogram(r.project ?? r.id)}
                  </span>
                  <span className="trunc">{r.project ?? "unknown project"}</span>
                </span>
              </td>
              <td>
                <KindPill kind={r.kind} />
              </td>
              <td>
                <StatePill row={r} />
              </td>
              <td>
                <span className="row">
                  <span className="mut trunc grow" title={latest}>
                    {latest}
                  </span>
                  {ev?.at && <span className="when tn">{ago(ev.at * 1000, now)}</span>}
                </span>
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function newRequestId(): string {
  return `qd-${crypto.randomUUID()}`;
}

type Hint = { tone: "" | "ok" | "err"; text: string };

function Composer({ enabled }: { enabled: boolean }) {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [hint, setHint] = useState<Hint>({ tone: "", text: "One note goes to Firstmate's inbox. It is read at Firstmate's next check." });
  // One request id per draft, so a retried send replays instead of creating a second note.
  const requestId = useRef<string | null>(null);

  const send = async () => {
    const body = text.trim();
    if (!enabled || busy || !body) return;
    requestId.current ??= newRequestId();
    setBusy(true);
    try {
      const r = await sendNote(body, requestId.current);
      if (r.announced || r.acknowledged) {
        setHint({
          tone: "ok",
          text:
            r.outcome === "replay"
              ? `Already sent as note ${r.id}.`
              : `Sent as note ${r.id}. Firstmate will read it at its next check.`,
        });
        setText("");
        requestId.current = null;
      } else {
        setHint({
          tone: "err",
          text: `Saved as note ${r.id}, but Firstmate was not woken. Send again to retry the wake; it will not create a second note.`,
        });
      }
    } catch (e) {
      // The draft and its request id are kept: sending again replays the same note if the
      // intake saved it, and can never create a second one.
      setHint({
        tone: "err",
        text: `Firstmate's intake reported a problem: ${e instanceof Error ? e.message : String(e)}. Sending again is safe; it cannot create a second note.`,
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      className="composer"
      onSubmit={(e) => {
        e.preventDefault();
        void send();
      }}
    >
      <label className="input">
        <span className="sr-only">Note for Firstmate</span>
        <input
          value={text}
          disabled={!enabled}
          placeholder={
            enabled
              ? "Leave a note for Firstmate"
              : inApp
                ? "Notes need a Firstmate home to go to"
                : "Notes are sent from the app window"
          }
          maxLength={4000}
          onChange={(e) => {
            setText(e.target.value);
            requestId.current = null;
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.metaKey) e.preventDefault();
            if (e.key === "Enter" && e.metaKey) void send();
          }}
        />
        <button type="submit" className="btn sm primary" disabled={!enabled || busy || !text.trim()}>
          <PaperPlaneTilt size={14} weight="bold" />
          {busy ? "Sending" : "Send note"}
          <span className="kbd" aria-hidden="true">
            ⌘↵
          </span>
        </button>
      </label>
      <span className={`hint ${hint.tone}`} role="status">
        {hint.text}
      </span>
    </form>
  );
}
