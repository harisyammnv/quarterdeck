import { memo, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  ArrowLeftIcon as ArrowLeft,
  BookOpenIcon as BookOpen,
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
import { Banner, EmptyState, IconButton, monogram, tileFill, useHotkey, useNow } from "../../components";
import { SkillDocPane } from "../skills/SkillDocPane";
import { SkillsPanel } from "../skills/SkillsPanel";
import { skillsList } from "../skills/bridge";
import "../skills/skills.css";
import { memoryList, memoryRead, memorySearch, type MemoryDoc, type MemoryHit, type MemoryItem, type MemoryKind, type MemorySearch } from "./bridge";
import { DocBody } from "./DocBody";
import { FindField } from "./FindField";
import { plural, shortAge } from "./format";
import { useRefreshing } from "./useRefreshing";
import { Windowed } from "./Windowed";
import "./memory.css";

type GroupKind = "brief" | "project-guide";
type Sel = { t: "doc"; id: string } | { t: "group"; kind: GroupKind } | { t: "skill"; id: string };

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
  const [list] = useRefreshing(memoryList);
  const [skills] = useRefreshing(skillsList);
  const [sel, setSel] = useState<Sel | null>(null);
  const [query, setQuery] = useState("");
  const [showHits, setShowHits] = useState(false);
  const find = useRef<HTMLInputElement>(null);
  const now = useNow(60_000);
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
  const current: Sel | null =
    sel ?? (groups.core.length ? { t: "doc", id: (groups.core.find((i) => i.kind === "learnings") ?? groups.core[0]).id } : null);

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
          setSel({ t: "group", kind: docItem.kind as GroupKind });
        }}
      />
    ) : null;

  let main: ReactNode;
  if (showHits) {
    main = <SearchPane query={query} onOpen={openDoc} find={findField} />;
  } else if (!current) {
    main = (
      <>
        <MainHead icon={BookOpen} title="Memory" find={findField} />
        {list.error ? (
          <EmptyState icon={WarningCircle} title="Memory could not be read" dashed={false}>
            {list.error}
          </EmptyState>
        ) : list.loading || !items ? (
          <EmptyState icon={CircleDashed} title="Reading memory" dashed={false}>
            Looking at the home's documents.
          </EmptyState>
        ) : (
          <EmptyState icon={BookOpen} title="Nothing written yet" dashed={false}>
            Preferences and learnings appear here once Firstmate records them.
          </EmptyState>
        )}
      </>
    );
  } else if (current.t === "group") {
    main = <GroupPane kind={current.kind} items={groups[current.kind]} now={now} onOpen={openDoc} find={findField} />;
  } else if (current.t === "skill") {
    main = <SkillDocPane id={current.id} find={query} actions={findField} />;
  } else {
    main = <DocPane id={current.id} back={back} find={findField} query={query} />;
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
      />
      <section className="panel memory-main" aria-label="Document">
        {main}
      </section>
      <SkillsPanel skills={skills} selected={current?.t === "skill" ? current.id : null} onOpen={openSkill} compact />
    </div>
  );
}

function MainHead({ icon: I, title, meta, back, find }: { icon: Icon; title: string; meta?: string; back?: ReactNode; find: ReactNode }) {
  return (
    <div className="phead memory-phead">
      {back ?? <I size={18} className="mut" aria-hidden="true" />}
      <h1 className="ptitle trunc">{title}</h1>
      {meta && <span className="mut t12 tn">{meta}</span>}
      <span className="grow" />
      {find}
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
}: {
  groups: Groups;
  sel: Sel | null;
  now: number;
  error: string | null;
  onDoc: (id: string) => void;
  onGroup: (kind: GroupKind) => void;
  onFind: () => void;
}) {
  const docId = sel?.t === "doc" ? sel.id : null;
  const briefs = groups.brief;
  const recent = new Set(briefs.slice(0, RECENT).map((b) => b.id));
  // A group row stays marked while one of its documents is open, unless that document has
  // its own row under "Recent instructions".
  const groupOn = (k: GroupKind) =>
    (sel?.t === "group" && sel.kind === k) ||
    (docId !== null && groups[k].some((i) => i.id === docId) && !recent.has(docId));
  const guides = groups["project-guide"];
  return (
    <section className="panel memory-nav" aria-label="Memory">
      <div className="memory-sh">
        <h2 className="memory-h2">Memory</h2>
        <span className="grow" />
        <IconButton icon={MagnifyingGlass} label="Find in memory (/)" onClick={onFind} />
      </div>
      {error && <Banner title={error}>Memory could not be refreshed: {error}</Banner>}
      <div className="memory-list" role="list">
        {groups.core.map((i) => (
          <NavRow key={i.id} icon={KIND_ICON[i.kind]} title={i.title} hint={KIND_HINT[i.kind] ?? i.label} age={shortAge(i.modified, now)} on={docId === i.id} onClick={() => {
            onDoc(i.id);
          }} />
        ))}
        {briefs.length > 0 && (
          <NavRow icon={Scroll} title="Instructions" hint={`${plural(briefs.length, "set", "sets")} for current and past work`} age={shortAge(briefs[0].modified, now)} on={groupOn("brief")} onClick={() => {
            onGroup("brief");
          }} />
        )}
        {guides.length > 0 && (
          <NavRow icon={FolderSimple} title="Project guides" hint={`Agent guides for ${plural(guides.length, "project", "projects")}`} age={shortAge(Math.max(...guides.map((g) => g.modified ?? 0)) || null, now)} on={groupOn("project-guide")} onClick={() => {
            onGroup("project-guide");
          }} />
        )}
      </div>
      {briefs.length > 0 && (
        <>
          <div className="memory-sh memory-sect">
            <h3 className="sect">Recent instructions</h3>
          </div>
          <div className="memory-list" role="list">
            {briefs.slice(0, RECENT).map((b) => (
              <NavRow key={b.id} tile={b.title} title={b.title} hint="Brief" age={shortAge(b.modified, now)} on={docId === b.id} recent onClick={() => {
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
          <span className="mono-ic memory-tile" style={{ background: tileFill(tile ?? title) }} aria-hidden="true">
            {monogram(tile ?? title)}
          </span>
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

function DocPane({ id, back, find, query }: { id: string; back: ReactNode; find: ReactNode; query: string }) {
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
      <MainHead icon={item ? KIND_ICON[item.kind] : BookOpen} title={item?.title ?? "Reading"} meta={meta} back={back} find={find} />
      {error ? (
        <EmptyState icon={WarningCircle} title="This document could not be read" dashed={false}>
          {error}
        </EmptyState>
      ) : !doc ? (
        <EmptyState icon={CircleDashed} title="Reading the document" dashed={false} />
      ) : doc.text.trim() === "" ? (
        <EmptyState icon={BookOpen} title="This document is empty" dashed={false} />
      ) : (
        <DocBody text={doc.text} truncated={doc.truncated} find={query} />
      )}
    </>
  );
}

function GroupPane({ kind, items, now, onOpen, find }: { kind: GroupKind; items: MemoryItem[]; now: number; onOpen: (id: string) => void; find: ReactNode }) {
  const I = KIND_ICON[kind];
  const meta = kind === "brief" ? plural(items.length, "set", "sets") : plural(items.length, "project", "projects");
  return (
    <>
      <MainHead icon={I} title={GROUP_TITLE[kind]} meta={meta} find={find} />
      {items.length === 0 ? (
        <EmptyState icon={I} title={`No ${GROUP_TITLE[kind].toLowerCase()} yet`} dashed={false} />
      ) : (
        <Windowed
          className="memory-group"
          label={GROUP_TITLE[kind]}
          items={items}
          rowHeight={56}
          render={(i) => (
            <button
              type="button"
              className="memory-li memory-group-row"
              onClick={() => {
                onOpen(i.id);
              }}
            >
              <span className="mono-ic memory-tile" style={{ background: tileFill(i.title) }} aria-hidden="true">
                {monogram(i.title)}
              </span>
              <span className="memory-li-text">
                <span className="t13 trunc">{i.title}</span>
                <span className="t12 mut trunc">{i.label}</span>
              </span>
              <span className="t12 mut tn">{shortAge(i.modified, now)}</span>
            </button>
          )}
        />
      )}
    </>
  );
}

type SearchLoad = { query: string; result: MemorySearch | null; error: string | null };

function SearchPane({ query, onOpen, find }: { query: string; onOpen: (id: string) => void; find: ReactNode }) {
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
      const last = out[out.length - 1] as (typeof out)[number] | undefined;
      if (last?.id === h.id) last.hits.push(h);
      else out.push({ id: h.id, title: h.title, kind: h.kind, hits: [h] });
    }
    return out;
  }, [r]);
  const stale = load.query !== q;
  const meta = r ? `${plural(r.hits.length, "line", "lines")} in ${plural(r.documents, "document", "documents")}` : undefined;
  return (
    <>
      <MainHead icon={MagnifyingGlass} title={`Matches for “${q}”`} meta={stale ? "Searching" : meta} find={find} />
      {load.error ? (
        <EmptyState icon={WarningCircle} title="The search failed" dashed={false}>
          {load.error}
        </EmptyState>
      ) : !r ? (
        <EmptyState icon={CircleDashed} title="Searching memory" dashed={false} />
      ) : r.hits.length === 0 ? (
        <EmptyState icon={MagnifyingGlass} title="No matches" dashed={false}>
          Nothing in preferences, learnings, instructions or project guides mentions “{r.query}”.
        </EmptyState>
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
