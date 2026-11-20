import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore, type KeyboardEvent, type ReactNode } from "react";
import {
  ArrowSquareOutIcon as ArrowSquareOut,
  ArrowsClockwiseIcon as ArrowsClockwise,
  CaretDownIcon as CaretDown,
  CircleDashedIcon as CircleDashed,
  CubeIcon as Cube,
  DatabaseIcon as Database,
  InfoIcon as Info,
  MagnifyingGlassIcon as MagnifyingGlass,
  PlugsIcon as Plugs,
  ShareNetworkIcon as ShareNetwork,
  StackIcon as Stack,
  WarningCircleIcon as WarningCircle,
  type Icon,
} from "@phosphor-icons/react";
import {
  Banner,
  Button,
  Facts,
  IconButton,
  Kbd,
  Panel,
  PanelHead,
  ProjectTile,
  StateDot,
  StatusChip,
  Tabs,
  Tag,
  VirtualList,
  Zero,
  stateDot,
  useNow,
  type Segment,
} from "../../ui";
import { STATE_LOOK } from "../../states";
import {
  containerInspect,
  containerLogs,
  containerStats,
  kiriOpen,
  lastContainers,
  loadContainers,
  onContainers,
  type ContainerDetail,
  type ContainerGroup,
  type ContainerLogs,
  type ContainerRow,
  type ContainersView,
  type LogLine,
} from "./bridge";
import { useApp } from "../../state/app";
import "./containers.css";

type Tab = "containers" | "images" | "volumes" | "networks";

/** Stats refresh for the selected container while the view is on screen (contract section 7). */
const STATS_INTERVAL_MS = 2000;
/** Samples kept for each sparkline. */
const SPARK_SAMPLES = 20;
const LOG_MODES: Segment<"latest" | "search">[] = [
  { id: "latest", label: "Latest" },
  { id: "search", label: "Search" },
];

/** Lists longer than this are windowed. */
const VIRTUAL_FROM = 100;
/** Row height of a windowed list. */
const ROW_H = 36;

export default function ContainersScreen() {
  const view = useSyncExternalStore(onContainers, lastContainers);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [kiriError, setKiriError] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("containers");
  const [selected, setSelected] = useState<string | null>(null);

  const fetchList = useCallback(
    () =>
      loadContainers()
        .then(() => {
          setError(null);
        })
        .catch((e: unknown) => {
          setError(e instanceof Error ? e.message : String(e));
        }),
    [],
  );

  const load = () => {
    setLoading(true);
    void fetchList().finally(() => {
      setLoading(false);
    });
  };

  // Grouping needs the fleet snapshot's worktrees: list again once the first one lands.
  const hasSnapshot = (useApp().live.view?.snapshot.generated_at ?? null) !== null;

  // Read on mount, when the first snapshot lands and whenever the window regains focus; the
  // list itself is not polled.
  useEffect(() => {
    const read = () => {
      void fetchList();
    };
    read();
    window.addEventListener("focus", read);
    return () => {
      window.removeEventListener("focus", read);
    };
  }, [fetchList, hasSnapshot]);

  const rows = useMemo(() => orderedRows(view), [view]);
  // Without a (still listed) choice, the first running container is shown, else the first one.
  const current = rows.find((r) => r.id === selected) ?? rows.find((r) => r.state === "running") ?? rows[0] ?? null;
  const ready = view?.status === "ready";
  const now = useNow(30_000);

  const openKiri = () => {
    setKiriError(null);
    kiriOpen().catch((e: unknown) => {
      setKiriError(e instanceof Error ? e.message : String(e));
    });
  };

  const segments: Segment<Tab>[] = [
    { id: "containers", label: "Containers", icon: Cube, count: ready ? view.counts.containers : undefined },
    { id: "images", label: "Images", icon: Stack, count: ready ? view.counts.images : undefined },
    { id: "volumes", label: "Volumes", icon: Database, count: ready ? view.counts.volumes : undefined },
    { id: "networks", label: "Networks", icon: ShareNetwork, count: ready ? view.counts.networks : undefined },
  ];

  return (
    <div className="containers-root">
      <Panel className="containers-head" label="Docker engine">
        <PanelHead
          title="Containers"
          meta={
            view ? (
              <span className="containers-meta">
                <EnginePill view={view} />
                {ready && <span>{engineMeta(view)}</span>}
              </span>
            ) : undefined
          }
        >
          <Button size="sm" icon={ArrowsClockwise} onClick={load} disabled={loading}>
            Refresh
          </Button>
          {view?.kiri_version && (
            <Button size="sm" icon={ArrowSquareOut} onClick={openKiri}>
              Open Kiri
            </Button>
          )}
        </PanelHead>
        <div className="containers-tabs">
          <Tabs items={segments} value={tab} onChange={setTab} label="Docker objects" />
        </div>
      </Panel>

      {error && <Banner title={error}>The window could not read containers: {error}</Banner>}
      {kiriError && <Banner title={kiriError}>Kiri did not open: {kiriError}</Banner>}

      <div className="containers-cols">
        <Panel className="containers-list" label={segments.find((s) => s.id === tab)?.label ?? "Containers"}>
          {!view ? (
            <Zero icon={CircleDashed} title="Reading Docker">
              Asking the active Docker context for its containers.
            </Zero>
          ) : view.status !== "ready" ? (
            <EngineZero view={view} />
          ) : tab === "containers" ? (
            <ContainersTab view={view} rows={rows} selected={current?.id ?? null} onSelect={setSelected} />
          ) : tab === "images" ? (
            <ImagesTab view={view} now={now} />
          ) : tab === "volumes" ? (
            <VolumesTab view={view} now={now} />
          ) : (
            <NetworksTab view={view} />
          )}
        </Panel>
        {ready && tab === "containers" && current && <Inspector key={current.id} row={current} group={groupOf(view, current.id)} />}
      </div>
    </div>
  );
}

/* ---------- head ---------- */

function EnginePill({ view }: { view: ContainersView }) {
  const ctx = view.context ?? "default";
  if (view.status === "ready")
    return (
      <StatusChip tone="success">
        Docker context {ctx}
      </StatusChip>
    );
  if (view.status === "unsupported") return <StatusChip tone="warning">Context {ctx} not supported</StatusChip>;
  return <StatusChip tone="danger">Docker not running</StatusChip>;
}

function engineMeta(view: ContainersView): string {
  const parts: string[] = [];
  if (view.engine_version) parts.push(`Engine ${view.engine_version}`);
  if (view.kiri_version) parts.push(`Kiri ${view.kiri_version}`);
  parts.push(plural(view.counts.containers, "container"));
  parts.push(`${String(view.counts.running)} running`);
  return parts.join(", ");
}

function EngineZero({ view }: { view: ContainersView }) {
  const detail = view.detail ?? "";
  if (view.status === "unsupported")
    return (
      <Zero icon={Plugs} title="This Docker context is not local">
        {detail}
      </Zero>
    );
  return (
    <Zero icon={Plugs} title="Docker is not running">
      Context {view.context ?? "default"}. {detail} Start Docker Desktop, Kiri or another engine, then refresh.
    </Zero>
  );
}

/* ---------- containers tab ---------- */

function orderedRows(view: ContainersView | null): ContainerRow[] {
  if (view?.status !== "ready") return [];
  return [...view.groups.flatMap((g) => g.containers), ...view.other];
}

function groupOf(view: ContainersView, id: string): ContainerGroup | null {
  return view.groups.find((g) => g.containers.some((c) => c.id === id)) ?? null;
}

/** One line of the grouped list. */
type Item =
  | { kind: "group"; key: string; group: ContainerGroup | null; count: number; open: boolean; empty: boolean }
  | { kind: "row"; key: string; row: ContainerRow }
  | { kind: "note"; key: string; icon: Icon; text: string };

const OTHER = "\u0000other";

function items(view: ContainersView, collapsed: ReadonlySet<string>): Item[] {
  const out: Item[] = [];
  const full = view.groups.filter((g) => g.containers.length > 0);
  const empty = view.groups.filter((g) => g.containers.length === 0);
  for (const g of full) {
    const open = !collapsed.has(g.task_id);
    out.push({ kind: "group", key: `g:${g.task_id}`, group: g, count: g.containers.length, open, empty: false });
    if (open) for (const r of g.containers) out.push({ kind: "row", key: r.id, row: r });
  }
  if (view.other.length > 0) {
    const open = !collapsed.has(OTHER);
    out.push({ kind: "group", key: "g:other", group: null, count: view.other.length, open, empty: false });
    if (open) for (const r of view.other) out.push({ kind: "row", key: r.id, row: r });
  }
  for (const g of empty) out.push({ kind: "group", key: `g:${g.task_id}`, group: g, count: 0, open: true, empty: true });
  if (empty.length > 0) {
    out.push({
      kind: "note",
      key: "n:empty",
      icon: Cube,
      text: `Nothing running for ${empty.length === 1 ? "this task" : "these tasks"}. Containers show up here when a worker starts a compose project in its folder.`,
    });
  }
  if (view.truncated) {
    out.push({ kind: "note", key: "n:cut", icon: WarningCircle, text: "Some lists were cut to keep the window fast; the counts above are complete." });
  }
  return out;
}

function ContainersTab({
  view,
  rows,
  selected,
  onSelect,
}: {
  view: ContainersView;
  rows: ContainerRow[];
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set());
  const boxRef = useRef<HTMLDivElement>(null);
  const list = useMemo(() => items(view, collapsed), [view, collapsed]);
  const virtual = list.length > VIRTUAL_FROM;

  const toggle = (id: string) => {
    setCollapsed((cur) => {
      const next = new Set(cur);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const onKey = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    const inRow = e.target instanceof HTMLElement && e.target.closest("[data-row]") !== null;
    if (e.target !== e.currentTarget && !inRow) return;
    e.preventDefault();
    const visible = list.flatMap((it) => (it.kind === "row" ? [it.row] : []));
    const i = visible.findIndex((r) => r.id === selected);
    const next = visible[Math.min(visible.length - 1, Math.max(0, i + (e.key === "ArrowDown" ? 1 : -1)))];
    if (!next) return;
    onSelect(next.id);
    const box = boxRef.current;
    if (!box) return;
    box.focus({ preventScroll: true });
    if (virtual) {
      const scroller = box.querySelector<HTMLElement>(".vlist");
      const at = list.findIndex((it) => it.key === next.id) * ROW_H;
      if (scroller && (at < scroller.scrollTop || at + ROW_H > scroller.scrollTop + scroller.clientHeight)) {
        scroller.scrollTop = at - scroller.clientHeight / 2;
      }
    } else {
      box.querySelector(`[data-row="${next.id}"]`)?.scrollIntoView({ block: "nearest" });
    }
  };

  if (rows.length === 0 && view.groups.length === 0)
    return (
      <Zero icon={Cube} title="No containers">
        Containers show up here when a worker starts a compose project or runs an image.
      </Zero>
    );

  const render = (it: Item) =>
    it.kind === "group" ? (
      <GroupLine item={it} onToggle={toggle} />
    ) : it.kind === "row" ? (
      <Row row={it.row} selected={it.row.id === selected} onSelect={onSelect} />
    ) : (
      <div className="containers-zrow">
        <it.icon size={14} />
        <span className="trunc">{it.text}</span>
      </div>
    );

  return (
    <>
      <div className="containers-grid containers-th" role="presentation">
        <span className="containers-first">Name</span>
        <span>Image</span>
        <span>State</span>
        <span>Port</span>
        <span>Up</span>
      </div>
      <div
        ref={boxRef}
        className={`containers-scroll${virtual ? " virtual" : ""}`}
        tabIndex={0}
        aria-label="Containers, grouped by worktree. Arrow keys move the selection."
        onKeyDown={onKey}
      >
        {virtual ? (
          <VirtualList items={list} rowHeight={ROW_H} rowKey={(it) => it.key} render={render} label="Containers" className="containers-vlist" />
        ) : (
          list.map((it) => <div key={it.key}>{render(it)}</div>)
        )}
      </div>
      <div className="containers-foot">
        <Info size={14} />
        <span className="trunc">Grouped by the worktree that holds the compose folder or a mounted path.</span>
        <span className="grow" />
        <span className="containers-keys">
          Select <Kbd>↑</Kbd>
          <Kbd>↓</Kbd>
        </span>
      </div>
    </>
  );
}

function GroupLine({ item, onToggle }: { item: Extract<Item, { kind: "group" }>; onToggle: (id: string) => void }) {
  const g = item.group;
  const n = item.count;
  const count = <span className="containers-gcount tn">{n === 0 ? "No containers" : plural(n, "container")}</span>;
  const caret = (
    <span className={`containers-caret${item.open ? "" : " shut"}`} aria-hidden="true">
      <CaretDown size={12} weight="bold" />
    </span>
  );
  if (!g) {
    return (
      <button
        type="button"
        className="containers-grp"
        aria-expanded={item.open}
        onClick={() => {
          onToggle(OTHER);
        }}
      >
        {caret}
        <span className="containers-othertile" aria-hidden="true">
          <Cube size={12} weight="bold" />
        </span>
        <span className="trunc">Not in a worktree</span>
        <span className="grow" />
        {count}
      </button>
    );
  }
  const project = g.project ?? g.task_id;
  const dot = stateDot(g.state);
  const label = (
    <>
      {caret}
      <ProjectTile name={project} size={18} />
      <span className="trunc containers-gname">
        {project}
        <span className="containers-slash"> / </span>
        {g.worktree}
      </span>
      {dot && <StateDot kind={dot} label={STATE_LOOK[g.state].word} />}
      <span className="grow" />
      {count}
    </>
  );
  if (item.empty) return <div className="containers-grp empty">{label}</div>;
  return (
    <button
      type="button"
      className="containers-grp"
      aria-expanded={item.open}
      onClick={() => {
        onToggle(g.task_id);
      }}
    >
      {label}
    </button>
  );
}

function Row({ row: r, selected, onSelect }: { row: ContainerRow; selected: boolean; onSelect: (id: string) => void }) {
  return (
    <button
      type="button"
      data-row={r.id}
      className={`containers-grid containers-row${selected ? " sel" : ""}`}
      aria-current={selected ? "true" : undefined}
      tabIndex={-1}
      onClick={() => {
        onSelect(r.id);
      }}
    >
      <span className="containers-first mono t12 trunc" title={r.name}>
        {r.service ?? r.name}
      </span>
      <span className="mono t12 mut trunc" title={r.image}>
        {r.image}
      </span>
      <span>
        <ContainerPill row={r} />
      </span>
      <span className="mono t12">{hostPort(r)}</span>
      <span className="mut trunc">{upText(r)}</span>
    </button>
  );
}

function ContainerPill({ row }: { row: ContainerRow }) {
  switch (row.state) {
    case "running":
      return <StatusChip tone="success">Running</StatusChip>;
    case "exited":
      return row.exit_code !== null && row.exit_code !== 0 ? <StatusChip tone="danger">Exited</StatusChip> : <Tag>Stopped</Tag>;
    case "paused":
      return <StatusChip tone="warning">Paused</StatusChip>;
    case "restarting":
      return <StatusChip tone="warning">Restarting</StatusChip>;
    case "dead":
      return <StatusChip tone="danger">Dead</StatusChip>;
    case "created":
      return <Tag>Created</Tag>;
    case "removing":
      return <Tag>Removing</Tag>;
    case "unknown":
      return <Tag>Unknown</Tag>;
  }
}

function hostPort(r: ContainerRow): string {
  const p = r.ports.find((x) => x.host !== null) ?? r.ports[0];
  if (!p) return "";
  return String(p.host ?? p.container);
}

/* ---------- inspector ---------- */

function Inspector({ row, group }: { row: ContainerRow; group: ContainerGroup | null }) {
  const running = row.state === "running";
  const [cpu, setCpu] = useState<number[]>([]);
  const [mem, setMem] = useState<number[]>([]);
  const [statsError, setStatsError] = useState<string | null>(null);
  const [logs, setLogs] = useState<ContainerLogs | null>(null);
  const [logsError, setLogsError] = useState<string | null>(null);
  const [search, setSearch] = useState<string | null>(null);
  const [detail, setDetail] = useState<ContainerDetail | null>(null);
  const now = useNow(30_000);

  // Times come from inspecting this one container, once per selection.
  useEffect(() => {
    let alive = true;
    containerInspect(row.id)
      .then((d) => {
        if (alive) setDetail(d);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [row.id]);

  // Stats for this container only, every 2 s while the window is visible; one call in flight.
  useEffect(() => {
    if (!running) return;
    let busy = false;
    let alive = true;
    const tick = () => {
      if (busy || document.visibilityState !== "visible") return;
      busy = true;
      containerStats(row.id)
        .then((s) => {
          if (!alive) return;
          setStatsError(null);
          if (s.cpu_percent !== null) setCpu((h) => [...h, s.cpu_percent ?? 0].slice(-SPARK_SAMPLES));
          if (s.memory_bytes !== null) setMem((h) => [...h, s.memory_bytes ?? 0].slice(-SPARK_SAMPLES));
        })
        .catch((e: unknown) => {
          if (alive) setStatsError(e instanceof Error ? e.message : String(e));
        })
        .finally(() => {
          busy = false;
        });
    };
    tick();
    const t = setInterval(tick, STATS_INTERVAL_MS);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, [row.id, running]);

  const loadLogs = useCallback(() => {
    containerLogs(row.id)
      .then((l) => {
        setLogs(l);
        setLogsError(null);
      })
      .catch((e: unknown) => {
        setLogsError(e instanceof Error ? e.message : String(e));
      });
  }, [row.id]);

  useEffect(() => {
    loadLogs();
  }, [loadLogs]);

  const dot = group ? stateDot(group.state) : null;
  const lastCpu = cpu.at(-1);
  const lastMem = mem.at(-1);
  const facts: [string, ReactNode][] = [
    ["Image", <span className="mono t12" title={row.image}>{row.image}</span>],
  ];
  if (row.service) facts.push(["Container", <span className="mono t12">{row.name}</span>]);
  facts.push(["Port", <span className="mono t12">{portText(row) || "None published"}</span>]);
  if (row.compose) facts.push(["Compose", <span className="mono t12" title={row.compose}>{row.compose}</span>]);
  facts.push([running ? "Started" : "Finished", <span className="tn">{detail ? clockAgo(running ? detail.started_at : (detail.finished_at ?? row.created_at), now) : "Reading"}</span>]);
  if (row.exit_code !== null) facts.push(["Exit code", <span className="tn">{row.exit_code}</span>]);
  if (detail?.restart_count) facts.push(["Restarts", <span className="tn">{detail.restart_count}</span>]);

  return (
    <Panel as="aside" className="containers-insp" label={`Container ${row.name}`}>
      <div className="phead">
        {dot && <StateDot kind={dot} label={group ? STATE_LOOK[group.state].word : undefined} />}
        <span className="mono containers-iname trunc" title={row.name}>
          {row.service ?? row.name}
        </span>
        <ContainerPill row={row} />
        <span className="grow" />
        <IconButton icon={ArrowsClockwise} label="Reload logs" onClick={loadLogs} />
      </div>
      <div className="containers-ibody">
        <div className="containers-stats">
          <Stat label="CPU" value={running && lastCpu !== undefined ? `${lastCpu.toFixed(1)}%` : "-"} series={running ? cpu : []} />
          <Stat label="Memory" value={running && lastMem !== undefined ? bytes(lastMem) : "-"} series={running ? mem : []} />
        </div>
        {statsError && <p className="containers-note">Stats unavailable: {statsError}</p>}
        <Facts items={facts} />
        <div className="containers-loghead">
          <span className="containers-h2 grow">Logs</span>
          <Tabs
            items={LOG_MODES}
            value={search === null ? "latest" : "search"}
            onChange={(m) => {
              setSearch(m === "latest" ? null : (search ?? ""));
            }}
            label="Log mode"
          />
        </div>
        {search !== null && (
          <label className="containers-search">
            <MagnifyingGlass size={14} />
            <input
              autoFocus
              value={search}
              placeholder="Filter lines"
              aria-label="Filter log lines"
              onChange={(e) => {
                setSearch(e.target.value);
              }}
            />
          </label>
        )}
        <LogBox logs={logs} error={logsError} filter={search ?? ""} />
        <p className="containers-caption">Last 500 lines, read when the container is selected or reloaded.</p>
      </div>
    </Panel>
  );
}

function Stat({ label, value, series }: { label: string; value: string; series: number[] }) {
  const max = Math.max(...series, 0);
  return (
    <div className="containers-stat">
      <div className="containers-statrow">
        <span className="mut t12">{label}</span>
        <span className="containers-statv tn">{value}</span>
      </div>
      <div className="containers-spark" aria-hidden="true">
        {Array.from({ length: SPARK_SAMPLES }, (_, i) => {
          const v = series[series.length - SPARK_SAMPLES + i];
          if (v === undefined || max === 0) return <i key={i} className="none" />;
          return <i key={i} style={{ height: `${String(Math.max(6, (v / (max * 1.6)) * 100))}%` }} />;
        })}
      </div>
    </div>
  );
}

function LogBox({ logs, error, filter }: { logs: ContainerLogs | null; error: string | null; filter: string }) {
  const ref = useRef<HTMLPreElement>(null);
  const lines = useMemo(() => {
    const all = logs?.lines ?? [];
    const f = filter.trim().toLowerCase();
    return f ? all.filter((l) => l.text.toLowerCase().includes(f)) : all;
  }, [logs, filter]);

  useEffect(() => {
    const el = ref.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines]);

  if (error) return <p className="containers-note">Logs unavailable: {error}</p>;
  if (!logs) return <pre className="qd-term containers-logs d">Reading logs</pre>;
  if (lines.length === 0)
    return <pre className="qd-term containers-logs">{filter ? "No line matches." : "This container has not written any output."}</pre>;
  return (
    <pre ref={ref} className="qd-term containers-logs" tabIndex={0} aria-label="Log tail">
      {logs.truncated && <span className="d">Older lines were left out.{"\n"}</span>}
      {lines.map((l, i) => (
        <Line key={i} line={l} />
      ))}
    </pre>
  );
}

function Line({ line }: { line: LogLine }) {
  const level = /\b(ERROR|FATAL|PANIC)\b/i.test(line.text) ? "err" : /\bWARN(ING)?\b/i.test(line.text) ? "warn" : "";
  return (
    <>
      {line.at !== null && <span className="d">{clock(line.at, true)} </span>}
      <span className={level === "err" ? "y containers-err" : level === "warn" ? "y" : undefined}>{line.text}</span>
      {"\n"}
    </>
  );
}

/* ---------- images, volumes, networks ---------- */

/** A plain or windowed list of grid rows under a header row. */
function Table<T>({ cols, head, rows, rowKey, cells, label }: { cols: string; head: ReactNode; rows: readonly T[]; rowKey: (r: T) => string; cells: (r: T) => ReactNode; label: string }) {
  const style = { gridTemplateColumns: cols };
  const render = (r: T) => (
    <div className="containers-grid containers-row static" style={style}>
      {cells(r)}
    </div>
  );
  return (
    <>
      <div className="containers-grid containers-th" style={style} role="presentation">
        {head}
      </div>
      {rows.length > VIRTUAL_FROM ? (
        <VirtualList items={rows} rowHeight={ROW_H} rowKey={rowKey} render={render} label={label} className="containers-vlist" />
      ) : (
        <div className="containers-scroll" role="list" aria-label={label}>
          {rows.map((r) => (
            <div key={rowKey(r)} role="listitem">
              {render(r)}
            </div>
          ))}
        </div>
      )}
    </>
  );
}

function ImagesTab({ view, now }: { view: ContainersView; now: number }) {
  if (view.images.length === 0)
    return (
      <Zero icon={Stack} title="No images">
        Pulled and built images show up here.
      </Zero>
    );
  return (
    <Table
      label="Images"
      cols="300px 150px 100px 140px 90px minmax(0, 1fr)"
      rows={view.images}
      rowKey={(i) => i.id}
      head={
        <>
          <span className="containers-first">Tag</span>
          <span>ID</span>
          <span className="num">Size</span>
          <span>Created</span>
          <span className="num">Used by</span>
        </>
      }
      cells={(i) => (
        <>
          <span className="containers-first mono t12 trunc" title={i.tag}>
            {i.tag}
            {i.more_tags > 0 && <span className="mut"> +{i.more_tags}</span>}
          </span>
          <span className="mono t12 mut">{i.id}</span>
          <span className="num">{i.size === null ? "" : bytes(i.size)}</span>
          <span className="mut">{agoText(i.created_at, now)}</span>
          <span className="num mut">{i.containers === null ? "" : i.containers}</span>
        </>
      )}
    />
  );
}

function VolumesTab({ view, now }: { view: ContainersView; now: number }) {
  if (view.volumes.length === 0)
    return (
      <Zero icon={Database} title="No volumes">
        Named volumes that containers create show up here.
      </Zero>
    );
  return (
    <Table
      label="Volumes"
      cols="360px 120px 140px minmax(0, 1fr)"
      rows={view.volumes}
      rowKey={(v) => v.name}
      head={
        <>
          <span className="containers-first">Name</span>
          <span>Driver</span>
          <span>Created</span>
        </>
      }
      cells={(v) => (
        <>
          <span className="containers-first mono t12 trunc" title={v.name}>
            {v.name}
          </span>
          <span className="mut">{v.driver}</span>
          <span className="mut">{agoText(v.created_at, now)}</span>
        </>
      )}
    />
  );
}

function NetworksTab({ view }: { view: ContainersView }) {
  if (view.networks.length === 0)
    return (
      <Zero icon={ShareNetwork} title="No networks">
        The engine reported no networks.
      </Zero>
    );
  return (
    <Table
      label="Networks"
      cols="260px 150px 110px 100px 110px minmax(0, 1fr)"
      rows={view.networks}
      rowKey={(n) => n.id}
      head={
        <>
          <span className="containers-first">Name</span>
          <span>ID</span>
          <span>Driver</span>
          <span>Scope</span>
          <span>Access</span>
        </>
      }
      cells={(n) => (
        <>
          <span className="containers-first mono t12 trunc">{n.name}</span>
          <span className="mono t12 mut">{n.id}</span>
          <span className="mut">{n.driver}</span>
          <span className="mut">{n.scope}</span>
          <span>{n.internal ? <Tag>Internal</Tag> : <span className="mut">External</span>}</span>
        </>
      )}
    />
  );
}

/* ---------- formatting ---------- */

function plural(n: number, word: string): string {
  return `${String(n)} ${word}${n === 1 ? "" : "s"}`;
}

function shortDuration(secs: number): string {
  const m = Math.floor(secs / 60);
  if (m < 60) return `${String(Math.max(1, m))} min`;
  const h = Math.floor(m / 60);
  return h < 48 ? `${String(h)} h` : `${String(Math.floor(h / 24))} d`;
}

/** The engine's status text, shortened: "Up 18 minutes" to "18 min", "Exited (0) 2 hours ago" to "Stopped 2 h". */
function upText(r: ContainerRow): string {
  const s = r.status.trim();
  if (r.state === "running" || r.state === "paused" || r.state === "restarting") return shortStatus(s.replace(/^Up\s+/i, ""));
  if (r.state === "exited" && r.exit_code !== null && r.exit_code !== 0) return `Exit code ${String(r.exit_code)}`;
  if (r.state === "exited") {
    const m = /^Exited \(\d+\)\s+(.*?)\s+ago$/i.exec(s);
    return m?.[1] ? `Stopped ${shortStatus(m[1])}` : "Stopped";
  }
  return s;
}

function shortStatus(t: string): string {
  return t
    .replace(/\s*\((healthy|unhealthy|health: starting|paused)\)\s*/i, "")
    .replace(/^about an? /i, "1 ")
    .replace(/^less than a second/i, "under a second")
    .replace(/\bseconds?\b/i, "s")
    .replace(/\bminutes?\b/i, "min")
    .replace(/\bhours?\b/i, "h")
    .replace(/\bdays?\b/i, "d")
    .replace(/\bweeks?\b/i, "w")
    .replace(/\bmonths?\b/i, "mo");
}

function portText(r: ContainerRow): string {
  return r.ports
    .map((p) => (p.host === null ? `${String(p.container)} (not published)` : `${String(p.host)} to ${String(p.container)}`))
    .join(", ");
}

function clock(t: number, seconds = false): string {
  return new Date(t * 1000).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    ...(seconds ? { second: "2-digit" } : {}),
    hour12: false,
  });
}

function agoText(t: number | null, now: number): string {
  if (t === null) return "";
  return `${shortDuration(Math.max(60, now / 1000 - t))} ago`;
}

function clockAgo(t: number | null, now: number): string {
  if (t === null) return "Unknown";
  return `${clock(t)}, ${agoText(t, now)}`;
}

function bytes(n: number): string {
  if (n < 1000) return `${String(n)} B`;
  if (n < 1e6) return `${String(Math.round(n / 1e3))} KB`;
  if (n < 1e9) return `${String(Math.round(n / 1e6))} MB`;
  return `${(n / 1e9).toFixed(1)} GB`;
}
