import { memo, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  ArrowLeftIcon as ArrowLeft,
  ArrowsClockwiseIcon as ArrowsClockwise,
  BookOpenIcon as BookOpen,
  FileTextIcon as FileText,
  CircleDashedIcon as CircleDashed,
  FolderSimpleIcon as FolderSimple,
  LockSimpleIcon as LockSimple,
  MagnifyingGlassIcon as MagnifyingGlass,
  ScrollIcon as Scroll,
  UserIcon as User,
  UsersIcon as Users,
  WarningCircleIcon as WarningCircle,
  type Icon,
} from "@phosphor-icons/react";
import { useApp } from "../../state/app";
import { Banner, IconButton, ProjectTile, VirtualList, ZeroState, useHotkey, useNow } from "../../ui";
import { SkillDocPane } from "../skills/SkillDocPane";
import { SkillsPanel } from "../skills/SkillsPanel";
import { skillsList } from "../skills/bridge";
import "../skills/skills.css";
import { memoryList, memoryRead, memorySearch, type MemoryDoc, type MemoryHit, type MemoryItem, type MemoryKind, type MemorySearch } from "./bridge";
import { reportsList, type ReportItem } from "../reports/bridge";
import { DocBody } from "./DocBody";
import { ReportPane, ReportsPane, reportTitle } from "./Reports";
import { FindField } from "./FindField";
import { plural, shortAge } from "./format";
import { useHomeKey } from "./useHomeKey";
import { useRefreshing } from "./useRefreshing";
import "./memory.css";

type GroupKind = "brief" | "project-guide";
type Sel = { t: "doc"; id: string } | { t: "group"; kind: GroupKind } | { t: "skill"; id: string } | { t: "reports" } | { t: "report"; id: string };

const KIND_ICON: Record<MemoryKind, Icon> = {
  preferences: User,
  "shared-preferences": Users,
  learnings: BookOpen,
  brief: Scroll,
  "project-guide": FolderSimple,
};

const KIND_HINT: Partial<Record<MemoryKind, string>> = {
  preferences: "How you like work delivered",
  "shared-preferences": "Inherited by second mates",
  learnings: "What the fleet has learned",
};

const GROUP_TITLE: Record<GroupKind, string> = { brief: "Instructions", "project-guide": "Project guides" };
const RECENT = 3;
const SEARCH_LIMIT_NOTE = "Showing the first matches; refine the search to see the rest.";

/** Memory and skills: the documents Firstmate keeps, one open, and the installed skills. */
export default function MemoryView() {
  const homeKey = useHomeKey();
  const [list, reloadList] = useRefreshing(memoryList, homeKey);
  const [skills, reloadSkills] = useRefreshing(skillsList, homeKey);
  const [reports, reloadReports] = useRefreshing(reportsList, homeKey);
  const [sel, setSel] = useState<Sel | null>(null);
  const [query, setQuery] = useState("");
  const [showHits, setShowHits] = useState(false);
  const find = useRef<HTMLInputElement>(null);
  const now = useNow(60_000);
  const { setArea } = useApp();
  useHotkey("/", () => find.current?.focus());

  const items = list.data?.items;
  const groups = useMemo(() => {
    const all = items ?? [];
    return {
      core: all.filter((i) => i.kind !== "brief" && i.kind !== "project-guide"),
      brief: all.filter((i) => i.kind === "brief"),
      "project-guide": all.filter((i) => i.kind === "project-guide"),
    };
  }, [items]);

  // Until something is chosen, open the learnings (the mock's default), else the first document.
  const fallback = groups.core.find((i) => i.kind === "learnings") ?? groups.core.at(0);
  const current: Sel | null = sel ?? (fallback ? { t: "doc", id: fallback.id } : null);

  const onQuery = useCallback((q: string) => {
    setQuery(q);
    setShowHits(q.trim().length >= 2);
  }, []);
  const openDoc = useCallback((id: string) => {
    setSel({ t: "doc", id });
    setShowHits(false);
  }, []);
  const openSkill = useCallback((id: string) => {
    setSel({ t: "skill", id });
    setShowHits(false);
  }, []);

  const findField = <FindField ref={find} value={query} onChange={onQuery} placeholder="Find in memory" />;
  const docItem = current?.t === "doc" ? items?.find((i) => i.id === current.id) : undefined;
  const back =
    docItem && (docItem.kind === "brief" || docItem.kind === "project-guide") ? (
      <IconButton
        icon={ArrowLeft}
        label={`Back to ${GROUP_TITLE[docItem.kind]}`}
        onClick={() => {
          setSel({ t: "group", kind: docItem.kind === "brief" ? "brief" : "project-guide" });
        }}
      />
    ) : null;

  let main: ReactNode;
  if (showHits) {
    main = <SearchPane query={query} onOpen={openDoc} />;
  } else if (!current) {
    main = (
      <>
        <MainHead icon={BookOpen} title="Memory" />
        {list.error ? (
          <ZeroState icon={WarningCircle} title="Memory could not be read">
            {list.error}
          </ZeroState>
        ) : list.loading || !items ? (
          <ZeroState icon={CircleDashed} title="Reading memory">
            Looking at the home's documents.
          </ZeroState>
        ) : (
          <ZeroState icon={BookOpen} title="Nothing written yet">
            Preferences and learnings appear here once Firstmate records them.
          </ZeroState>
        )}
      </>
    );
  } else if (current.t === "group") {
    main = <GroupPane kind={current.kind} items={groups[current.kind]} capped={current.kind === "brief" && (list.data?.capped ?? false)} now={now} onOpen={openDoc} />;
  } else if (current.t === "reports") {
    main = reports.data ? (
      <ReportsPane list={reports.data} now={now} onOpen={(id) => { setSel({ t: "report", id }); }} />
    ) : (
      <ZeroState icon={reports.error ? WarningCircle : CircleDashed} title={reports.error ? "Reports could not be listed" : "Listing reports"}>
        {reports.error ?? undefined}
      </ZeroState>
    );
  } else if (current.t === "report") {
    const item = reports.data?.items.find((r) => r.task_id === current.id);
    main = (
      <ReportPane
        taskId={current.id}
        title={item ? reportTitle(item) : current.id}
        find={query}
        back={<IconButton icon={ArrowLeft} label="Back to Reports" onClick={() => { setSel({ t: "reports" }); }} />}
      />
    );
  } else if (current.t === "skill") {
    main = <SkillDocPane id={current.id} find={query} />;
  } else {
    main = <DocPane id={current.id} back={back} query={query} />;
  }

  return (
    <div className="memory-cols">
      <MemoryNav
        groups={groups}
        sel={current}
        now={now}
        error={list.data ? list.error : null}
        onDoc={openDoc}
        onGroup={(kind) => {
          setSel({ t: "group", kind });
          setShowHits(false);
        }}
        onFind={() => find.current?.focus()}
        refreshing={list.loading || skills.loading || reports.loading}
        reports={reports.data?.items ?? []}
        onReports={() => {
          setSel({ t: "reports" });
          setShowHits(false);
        }}
        onReport={(id) => {
          setSel({ t: "report", id });
          setShowHits(false);
        }}
        onRefresh={() => {
          reloadList();
          reloadSkills();
          reloadReports();
        }}
      />
      <section className="qd-panel ui-panel memory-main" aria-label="Document">
        {main}
        {/* One find field for every pane, outside them so switching panes never remounts it. */}
        <div className="memory-findslot">{findField}</div>
      </section>
      <SkillsPanel
        skills={skills}
        selected={current?.t === "skill" ? current.id : null}
        onOpen={openSkill}
        compact
        onMore={() => {
          setArea("skills");
        }}
      />
    </div>
  );
}

function MainHead({ icon: I, title, meta, back }: { icon: Icon; title: string; meta?: string; back?: ReactNode }) {
  return (
    <div className="phead memory-phead">
      {back ?? <I size={18} className="mut" aria-hidden="true" />}
      <h1 className="ptitle trunc">{title}</h1>
      {meta && <span className="mut t12 tn">{meta}</span>}
      <span className="grow" />
    </div>
  );
}

type Groups = { core: MemoryItem[]; brief: MemoryItem[]; "project-guide": MemoryItem[] };

const MemoryNav = memo(function MemoryNav({
  groups,
  sel,
  now,
  error,
  onDoc,
  onGroup,
  onFind,
  refreshing,
  onRefresh,
  reports,
  onReports,
  onReport,
}: {
  groups: Groups;
  sel: Sel | null;
  now: number;
  error: string | null;
  onDoc: (id: string) => void;
  onGroup: (kind: GroupKind) => void;
  onFind: () => void;
  refreshing: boolean;
  onRefresh: () => void;
  reports: ReportItem[];
  onReports: () => void;
  onReport: (taskId: string) => void;
}) {
  const docId = sel?.t === "doc" ? sel.id : null;
  const reportId = sel?.t === "report" ? sel.id : null;
  const briefs = groups.brief;
  // The mock's "Recent reports"; recent instructions stand in when there are no reports.
  const recentReports = reports.slice(0, RECENT);
  const recent = new Set(recentReports.length ? [] : briefs.slice(0, RECENT).map((b) => b.id));
  const reportsOn = sel?.t === "reports" || (reportId !== null && !recentReports.some((r) => r.task_id === reportId));
  // A group row stays marked while one of its documents is open, unless that document has
  // its own row under "Recent instructions".
  const groupOn = (k: GroupKind) =>
    (sel?.t === "group" && sel.kind === k) ||
    (docId !== null && groups[k].some((i) => i.id === docId) && !recent.has(docId));
  const guides = groups["project-guide"];
  return (
    <section className="qd-panel ui-panel memory-nav" aria-label="Memory">
      <div className="memory-sh">
        <h2 className="memory-h2">Memory</h2>
        <span className="grow" />
        <IconButton icon={MagnifyingGlass} label="Find in memory (/)" onClick={onFind} />
        <IconButton icon={ArrowsClockwise} label="Read memory and skills again" onClick={onRefresh} disabled={refreshing} />
      </div>
      {error && <Banner title={error}>Memory could not be refreshed: {error}</Banner>}
      <div className="memory-list" role="list">
        {groups.core.map((i) => (
          <NavRow key={i.id} icon={KIND_ICON[i.kind]} title={i.title} hint={KIND_HINT[i.kind] ?? i.label} age={shortAge(i.modified, now)} on={docId === i.id} onClick={() => {
            onDoc(i.id);
          }} />
        ))}
        {briefs.length > 0 && (
          <NavRow icon={Scroll} title="Instructions" hint={`${plural(briefs.length, "set", "sets")} for current and past work`} age={shortAge(briefs[0]?.modified ?? null, now)} on={groupOn("brief")} onClick={() => {
            onGroup("brief");
          }} />
        )}
        <NavRow icon={FileText} title="Reports" hint={reports.length ? plural(reports.length, "investigation report", "investigation reports") : "None written yet"} age={shortAge(reports[0]?.modified ?? null, now)} on={reportsOn} onClick={onReports} />
        {guides.length > 0 && (
          <NavRow icon={FolderSimple} title="Project guides" hint={`Agent guides for ${plural(guides.length, "project", "projects")}`} age={shortAge(Math.max(...guides.map((g) => g.modified ?? 0)) || null, now)} on={groupOn("project-guide")} onClick={() => {
            onGroup("project-guide");
          }} />
        )}
      </div>
      {recentReports.length > 0 && (
        <>
          <div className="memory-sh memory-sect">
            <h3 className="sect">Recent reports</h3>
          </div>
          <div className="memory-list" role="list">
            {recentReports.map((r) => (
              <NavRow key={r.task_id} tile={r.task_id} title={reportTitle(r)} hint={r.title ? r.task_id : (r.kind ?? "Report")} age={shortAge(r.modified, now, true)} on={reportId === r.task_id} recent onClick={() => {
                onReport(r.task_id);
              }} />
            ))}
          </div>
        </>
      )}
      {recentReports.length === 0 && briefs.length > 0 && (
        <>
          <div className="memory-sh memory-sect">
            <h3 className="sect">Recent instructions</h3>
          </div>
          <div className="memory-list" role="list">
            {briefs.slice(0, RECENT).map((b) => (
              <NavRow key={b.id} tile={b.title} title={b.title} hint="Brief" age={shortAge(b.modified, now, true)} on={docId === b.id} recent onClick={() => {
                onDoc(b.id);
              }} />
            ))}
          </div>
        </>
      )}
      <span className="grow" />
      <div className="memory-foot">
        <LockSimple size={16} aria-hidden="true" />
        <span>Read-only. Firstmate owns edits.</span>
      </div>
    </section>
  );
});

function NavRow({
  icon: I,
  tile,
  title,
  hint,
  age,
  on,
  recent,
  onClick,
}: {
  icon?: Icon;
  tile?: string;
  title: string;
  hint: string;
  age: string;
  on: boolean;
  recent?: boolean;
  onClick: () => void;
}) {
  return (
    <div role="listitem">
      <button type="button" className={`memory-li${on ? " sel" : ""}${recent ? " recent" : ""}`} aria-current={on ? "true" : undefined} onClick={onClick}>
        {I ? (
          <I size={18} className="mut" aria-hidden="true" />
        ) : (
          <ProjectTile name={tile ?? title} size={18} />
        )}
        <span className="memory-li-text">
          <span className="t13 trunc">{title}</span>
          <span className="t12 mut trunc">{hint}</span>
        </span>
        <span className="t12 mut tn">{age}</span>
      </button>
    </div>
  );
}

type DocLoad = { id: string; doc: MemoryDoc | null; error: string | null };

function DocPane({ id, back, query }: { id: string; back: ReactNode; query: string }) {
  const [load, setLoad] = useState<DocLoad>({ id, doc: null, error: null });
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const again = () => {
      setTick((t) => t + 1);
    };
    window.addEventListener("focus", again);
    return () => {
      window.removeEventListener("focus", again);
    };
  }, []);
  useEffect(() => {
    let live = true;
    memoryRead(id)
      .then((doc) => {
        if (live) setLoad({ id, doc, error: null });
      })
      .catch((e: unknown) => {
        if (live) setLoad({ id, doc: null, error: e instanceof Error ? e.message : String(e) });
      });
    return () => {
      live = false;
    };
  }, [id, tick]);
  const doc = load.id === id ? load.doc : null;
  const error = load.id === id ? load.error : null;
  const item = doc?.item;
  const counted = item && (item.kind === "preferences" || item.kind === "shared-preferences" || item.kind === "learnings");
  const meta = item ? (counted ? plural(doc.entries, "entry", "entries") : item.label) : undefined;
  return (
    <>
      <MainHead icon={item ? KIND_ICON[item.kind] : BookOpen} title={item?.title ?? "Reading"} meta={meta} back={back} />
      {error ? (
        <ZeroState icon={WarningCircle} title="This document could not be read">
          {error}
        </ZeroState>
      ) : !doc ? (
        <ZeroState icon={CircleDashed} title="Reading the document" />
      ) : doc.text.trim() === "" ? (
        <ZeroState icon={BookOpen} title="This document is empty" />
      ) : (
        <DocBody text={doc.text} truncated={doc.truncated} find={query} />
      )}
    </>
  );
}

function GroupPane({ kind, items, capped, now, onOpen }: { kind: GroupKind; items: MemoryItem[]; capped: boolean; now: number; onOpen: (id: string) => void }) {
  const I = KIND_ICON[kind];
  const meta = kind === "brief" ? plural(items.length, "set", "sets") : plural(items.length, "project", "projects");
  return (
    <>
      <MainHead icon={I} title={GROUP_TITLE[kind]} meta={meta} />
      {capped && <Banner>Only the newest {items.length} sets are listed; older ones stay in the home.</Banner>}
      {items.length === 0 ? (
        <ZeroState icon={I} title={`No ${GROUP_TITLE[kind].toLowerCase()} yet`} />
      ) : (
        <VirtualList
          className="memory-group"
          label={GROUP_TITLE[kind]}
          items={items}
          rowHeight={56}
          rowKey={(i) => i.id}
          render={(i) => (
            <button
              type="button"
              className="memory-li memory-group-row"
              onClick={() => {
                onOpen(i.id);
              }}
            >
              <ProjectTile name={i.title} size={20} />
              <span className="memory-li-text">
                <span className="t13 trunc">{i.title}</span>
                <span className="t12 mut trunc">{i.label}</span>
              </span>
              <span className="t12 mut tn">{shortAge(i.modified, now, true)}</span>
            </button>
          )}
        />
      )}
    </>
  );
}

type SearchLoad = { query: string; result: MemorySearch | null; error: string | null };

function SearchPane({ query, onOpen }: { query: string; onOpen: (id: string) => void }) {
  const [load, setLoad] = useState<SearchLoad>({ query: "", result: null, error: null });
  const q = query.trim();
  useEffect(() => {
    let live = true;
    const t = setTimeout(() => {
      memorySearch(q)
        .then((result) => {
          if (live) setLoad({ query: q, result, error: null });
        })
        .catch((e: unknown) => {
          if (live) setLoad({ query: q, result: null, error: e instanceof Error ? e.message : String(e) });
        });
    }, 150);
    return () => {
      live = false;
      clearTimeout(t);
    };
  }, [q]);
  const r = load.result;
  const groups = useMemo(() => {
    const out: { id: string; title: string; kind: MemoryKind; hits: MemoryHit[] }[] = [];
    for (const h of r?.hits ?? []) {
      const last = out.at(-1);
      if (last?.id === h.id) last.hits.push(h);
      else out.push({ id: h.id, title: h.title, kind: h.kind, hits: [h] });
    }
    return out;
  }, [r]);
  const stale = load.query !== q;
  const meta = r ? `${plural(r.hits.length, "line", "lines")} in ${plural(r.documents, "document", "documents")}` : undefined;
  return (
    <>
      <MainHead icon={MagnifyingGlass} title={`Matches for “${q}”`} meta={stale ? "Searching" : meta} />
      {load.error ? (
        <ZeroState icon={WarningCircle} title="The search failed">
          {load.error}
        </ZeroState>
      ) : !r ? (
        <ZeroState icon={CircleDashed} title="Searching memory" />
      ) : r.hits.length === 0 ? (
        <ZeroState icon={MagnifyingGlass} title="No matches">
          Nothing in preferences, learnings, instructions or project guides mentions “{r.query}”.
        </ZeroState>
      ) : (
        <div className="memory-hits" aria-busy={stale}>
          {r.truncated && <Banner>{SEARCH_LIMIT_NOTE}</Banner>}
          {groups.map((g) => {
            const I = KIND_ICON[g.kind];
            return (
              <div key={g.id} className="memory-hitgroup">
                <div className="memory-hit-doc">
                  <I size={16} className="mut" aria-hidden="true" />
                  <span className="t13 b7 trunc">{g.title}</span>
                  <span className="t12 mut tn">{plural(g.hits.length, "match", "matches")}</span>
                </div>
                {g.hits.map((h) => (
                  <button
                    key={`${h.id}:${h.line}`}
                    type="button"
                    className="memory-hit"
                    onClick={() => {
                      onOpen(h.id);
                    }}
                  >
                    <span className="t12 mut tn memory-hit-line">{h.line}</span>
                    <span className="trunc">
                      <Snippet text={h.snippet} ranges={h.ranges} />
                    </span>
                  </button>
                ))}
              </div>
            );
          })}
        </div>
      )}
    </>
  );
}

/** A snippet with its matched ranges marked; ranges are UTF-16 offsets, as JS strings use. */
function Snippet({ text, ranges }: { text: string; ranges: [number, number][] }) {
  const parts: ReactNode[] = [];
  let at = 0;
  ranges.forEach(([a, b], i) => {
    if (a < at || b > text.length || a >= b) return;
    if (a > at) parts.push(text.slice(at, a));
    parts.push(<mark key={i}>{text.slice(a, b)}</mark>);
    at = b;
  });
  if (at < text.length) parts.push(text.slice(at));
  return <>{parts}</>;
}
