/**
 * The projects and worktrees tree (design 5.1), drawn inside the shell's project frame.
 *
 * Projects come from the core's projects view, grouped by the project's directory name, each
 * with its icon (from the clone, else a monogram) and its worktrees with a state dot and kind
 * glyph. Selecting a worktree sets the task in focus for every area and the inspector.
 * Keyboard: arrows move, Right and Left open and close a project, Home and End jump.
 */
import { memo, useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import {
  BinocularsIcon as Binoculars,
  CaretDownIcon as CaretDown,
  CaretRightIcon as CaretRight,
  CircleDashedIcon as CircleDashed,
  SailboatIcon as Sailboat,
  UsersThreeIcon as UsersThree,
  WarningCircleIcon as WarningCircle,
  type Icon,
} from "@phosphor-icons/react";
import { Count, ProjectTile, StateDot } from "../../ui";
import { useApp } from "../../state/app";
import { projectIcon, useProjects, type ProjectRow } from "./bridge";
import { dotKind, stateWord } from "./look";
import "./projects.css";

const COLLAPSED_KEY = "quarterdeck.projects.collapsed";

function readCollapsed(): Set<string> {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(COLLAPSED_KEY) ?? "[]");
    return new Set(Array.isArray(raw) ? raw.filter((x): x is string => typeof x === "string") : []);
  } catch {
    return new Set();
  }
}

const KIND_ICON: Record<string, Icon> = { ship: Sailboat, scout: Binoculars, secondmate: UsersThree };

export function KindGlyph({ kind }: { kind: string | null }) {
  const I: Icon = (kind ? KIND_ICON[kind] : undefined) ?? CircleDashed;
  return <I size={14} aria-hidden="true" />;
}

/** The project's icon from the core, once it answers; a monogram until then and without one. */
export function ProjectIcon({ name, size = 20 }: { name: string; size?: number }) {
  const [icon, setIcon] = useState<{ name: string; src: string | null } | null>(null);
  useEffect(() => {
    let live = true;
    void projectIcon(name).then((src) => {
      if (live) setIcon({ name, src });
    });
    return () => {
      live = false;
    };
  }, [name]);
  return <ProjectTile name={name} src={icon?.name === name ? icon.src : null} size={size} />;
}

type Item = { type: "project"; key: string; group: string } | { type: "task"; key: string; id: string; group: string };

const NO_PROJECT = "No project";

export default function ProjectsTree() {
  const { view, error } = useProjects();
  const { selectedTask, setSelectedTask } = useApp();
  const [collapsed, setCollapsed] = useState<Set<string>>(readCollapsed);
  const [focusKey, setFocusKey] = useState<string | null>(null);
  const treeRef = useRef<HTMLDivElement>(null);

  const toggle = useCallback((group: string, open?: boolean) => {
    setCollapsed((cur) => {
      const isOpen = !cur.has(group);
      if (open !== undefined && open === isOpen) return cur;
      const next = new Set(cur);
      if (isOpen) next.add(group);
      else next.delete(group);
      localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...next]));
      return next;
    });
  }, []);

  const items = useMemo<Item[]>(() => {
    const out: Item[] = [];
    for (const g of view?.projects ?? []) {
      const group = g.name ?? "";
      out.push({ type: "project", key: `p:${group}`, group });
      if (!collapsed.has(group)) {
        for (const t of g.tasks) out.push({ type: "task", key: `t:${t.id}`, id: t.id, group });
      }
    }
    return out;
  }, [view, collapsed]);

  const has = (key: string | null) => key !== null && items.some((i) => i.key === key);
  // The one tab stop: the focused item, else the selected worktree, else the first project.
  const tabKey = has(focusKey) ? focusKey : has(`t:${selectedTask ?? ""}`) ? `t:${selectedTask ?? ""}` : items[0]?.key;

  const focusItem = (key: string) => {
    setFocusKey(key);
    treeRef.current?.querySelector<HTMLElement>(`[data-key="${CSS.escape(key)}"]`)?.focus();
  };

  const onKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const i = items.findIndex((it) => it.key === tabKey);
    const it = items[i];
    if (!it) return;
    const go = (j: number) => {
      const target = items[Math.max(0, Math.min(items.length - 1, j))];
      if (!target) return;
      focusItem(target.key);
      if (target.type === "task") setSelectedTask(target.id);
    };
    switch (e.key) {
      case "ArrowDown":
        go(i + 1);
        break;
      case "ArrowUp":
        go(i - 1);
        break;
      case "Home":
        go(0);
        break;
      case "End":
        go(items.length - 1);
        break;
      case "ArrowRight":
        if (it.type === "project") {
          if (collapsed.has(it.group)) toggle(it.group, true);
          else go(i + 1);
        }
        break;
      case "ArrowLeft":
        if (it.type === "project") toggle(it.group, false);
        else focusItem(`p:${it.group}`);
        break;
      case "Enter":
      case " ":
        if (it.type === "project") toggle(it.group);
        else setSelectedTask(it.id);
        break;
      default:
        return;
    }
    e.preventDefault();
  };

  const select = useCallback(
    (id: string) => {
      setFocusKey(`t:${id}`);
      setSelectedTask(id);
    },
    [setSelectedTask],
  );

  if (error) return <Note icon={WarningCircle} text={`Projects could not be read: ${error}`} />;
  if (!view || !view.ready) return <Note icon={CircleDashed} text="Reading projects from the first fleet snapshot." />;
  if (view.projects.length === 0) return <Note icon={Sailboat} text="No worktrees yet. Projects appear when Firstmate starts work in them." />;

  return (
    <div className="qd-tree projects-tree" role="tree" aria-label="Projects and worktrees" ref={treeRef} onKeyDown={onKey}>
      {view.projects.map((g) => {
        const group = g.name ?? "";
        const open = !collapsed.has(group);
        const label = g.name ?? NO_PROJECT;
        return (
          <div role="none" key={`p:${group}`}>
            <div
              role="treeitem"
              aria-expanded={open}
              aria-level={1}
              aria-label={`${label}, ${String(g.tasks.length)} ${g.tasks.length === 1 ? "worktree" : "worktrees"}`}
              tabIndex={tabKey === `p:${group}` ? 0 : -1}
              data-key={`p:${group}`}
              className="qd-proj projects-proj"
              onClick={() => {
                setFocusKey(`p:${group}`);
                toggle(group);
              }}
            >
              {open ? <CaretDown size={14} aria-hidden="true" /> : <CaretRight size={14} aria-hidden="true" />}
              {g.name ? <ProjectIcon name={g.name} /> : <span className="projects-none" aria-hidden="true" />}
              <span className="qd-trunc projects-grow">{label}</span>
              <Count n={g.tasks.length} />
            </div>
            {open && (
              <div role="group">
                {g.tasks.map((t) => (
                  <Worktree key={t.id} row={t} selected={t.id === selectedTask} tabbable={tabKey === `t:${t.id}`} onSelect={select} />
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

const Worktree = memo(function Worktree({ row, selected, tabbable, onSelect }: { row: ProjectRow; selected: boolean; tabbable: boolean; onSelect: (id: string) => void }) {
  return (
    <div
      role="treeitem"
      aria-level={2}
      aria-selected={selected}
      aria-label={`${row.id}, ${stateWord(row)}, ${row.kind ?? "task"}`}
      tabIndex={tabbable ? 0 : -1}
      data-key={`t:${row.id}`}
      className="qd-wt"
      title={row.title ?? row.id}
      onClick={() => {
        onSelect(row.id);
      }}
    >
      <StateDot kind={dotKind(row)} />
      <span className="qd-trunc">{row.id}</span>
      <span className="qd-wt__kind">
        <KindGlyph kind={row.kind} />
      </span>
    </div>
  );
});

function Note({ icon: I, text }: { icon: Icon; text: string }) {
  return (
    <div className="side-empty" role="status">
      <I size={18} />
      <span>{text}</span>
    </div>
  );
}
