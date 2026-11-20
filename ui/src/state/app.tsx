import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import {
  appInfo,
  homePick,
  fleetView,
  inApp,
  onFleetUpdated,
  onHomeChanged,
  viewRendered,
  type AppInfo,
  type FleetView,
} from "../bridge";
import { previewParam } from "../preview";

export type ThemePref = "system" | "light" | "dark";
const THEME_KEY = "quarterdeck.theme";
const PROJECTS_KEY = "quarterdeck.projects";

function readThemePref(): ThemePref {
  const v = localStorage.getItem(THEME_KEY);
  return v === "light" || v === "dark" ? v : "system";
}

function readProjects(): Record<string, boolean> {
  try {
    const v: unknown = JSON.parse(localStorage.getItem(PROJECTS_KEY) ?? "{}");
    if (typeof v !== "object" || v === null) return {};
    return Object.fromEntries(Object.entries(v).filter((e): e is [string, boolean] => typeof e[1] === "boolean"));
  } catch {
    return {};
  }
}

function readArea(): string {
  return (inApp ? null : previewParam("area")) ?? "fleet";
}

/** The live fleet, plus the rows whose state word just changed (they pulse once). */
export interface Live {
  view: FleetView | null;
  changed: ReadonlyMap<string, number>;
}

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

export interface AppState {
  info: AppInfo | null;
  live: Live;
  /** The window could not reach the core. */
  ipcError: string | null;
  theme: ThemePref;
  setTheme: (t: ThemePref) => void;
  area: string;
  setArea: (a: string) => void;
  /** Whether the project list shows on `view`, given its default from the registry. */
  projectsShown: (view: string, byDefault: boolean) => boolean;
  /** Shows or hides the project list on `view` (⌘\\). */
  toggleProjects: (view: string, byDefault: boolean) => void;
  /** The task in focus across areas (project list, fleet, board, inspector), by id. */
  selectedTask: string | null;
  setSelectedTask: (id: string | null) => void;
  /** Opens the folder picker in the core. */
  chooseHome: () => Promise<void>;
  homeBusy: boolean;
  homeError: string | null;
}

const Ctx = createContext<AppState | null>(null);

export function useApp(): AppState {
  const v = useContext(Ctx);
  if (!v) throw new Error("useApp outside AppProvider");
  return v;
}

/** The live fleet view and its pulse marks. */
export function useFleet(): Live {
  return useApp().live;
}

export function AppProvider({ children }: { children: ReactNode }) {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [live, setLive] = useState<Live>({ view: null, changed: new Map() });
  const [ipcError, setIpcError] = useState<string | null>(null);
  const [theme, setThemeState] = useState<ThemePref>(readThemePref);
  const [area, setArea] = useState<string>(readArea);
  const [projects, setProjects] = useState<Record<string, boolean>>(() => readProjects());
  const [selectedTask, setSelectedTask] = useState<string | null>(null);
  const [homeBusy, setHomeBusy] = useState(false);
  const [homeError, setHomeError] = useState<string | null>(null);

  const accept = useCallback((next: FleetView) => {
    setLive((cur) => nextLive(cur, next));
  }, []);

  const adoptInfo = useCallback((i: AppInfo) => {
    setInfo(i);
    if (i.theme === "light" || i.theme === "dark") setThemeState(i.theme);
  }, []);

  useEffect(() => {
    const stopFleet = onFleetUpdated(accept);
    const stopHome = onHomeChanged((i) => {
      // A new home starts a new engine whose view numbers start again: drop the old view.
      setLive({ view: null, changed: new Map() });
      setSelectedTask(null);
      adoptInfo(i);
    });
    const fail = (e: unknown) => {
      setIpcError(e instanceof Error ? e.message : String(e));
    };
    appInfo().then(adoptInfo).catch(fail);
    fleetView()
      .then((v) => {
        if (v) accept(v);
      })
      .catch(fail);
    return () => {
      stopFleet();
      stopHome();
    };
  }, [accept, adoptInfo]);

  // Theme: <html> always carries data-theme, as the design system requires; "match the
  // system" resolves to light or dark and follows the system setting while it changes.
  useEffect(() => {
    const root = document.documentElement;
    if (theme !== "system") {
      root.setAttribute("data-theme", theme);
      return;
    }
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      root.setAttribute("data-theme", mq.matches ? "dark" : "light");
    };
    apply();
    mq.addEventListener("change", apply);
    return () => {
      mq.removeEventListener("change", apply);
    };
  }, [theme]);

  // A view caused by a status change reports when it reached the screen (latency probe).
  const view = live.view;
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

  const setTheme = useCallback((t: ThemePref) => {
    setThemeState(t);
    if (t === "system") localStorage.removeItem(THEME_KEY);
    else localStorage.setItem(THEME_KEY, t);
  }, []);

  const projectsShown = useCallback((view: string, byDefault: boolean) => projects[view] ?? byDefault, [projects]);
  const toggleProjects = useCallback((view: string, byDefault: boolean) => {
    setProjects((cur) => {
      const next = { ...cur, [view]: !(cur[view] ?? byDefault) };
      localStorage.setItem(PROJECTS_KEY, JSON.stringify(next));
      return next;
    });
  }, []);

  const chooseHome = useCallback(async () => {
    setHomeBusy(true);
    setHomeError(null);
    try {
      adoptInfo(await homePick());
      const v = await fleetView();
      if (v) setLive({ view: v, changed: new Map() });
    } catch (e) {
      setHomeError(e instanceof Error ? e.message : String(e));
    } finally {
      setHomeBusy(false);
    }
  }, [adoptInfo]);

  const value = useMemo<AppState>(
    () => ({
      info,
      live,
      ipcError,
      theme,
      setTheme,
      area,
      setArea,
      projectsShown,
      toggleProjects,
      selectedTask,
      setSelectedTask,
      chooseHome,
      homeBusy,
      homeError,
    }),
    [info, live, ipcError, theme, setTheme, area, projectsShown, toggleProjects, selectedTask, chooseHome, homeBusy, homeError],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
