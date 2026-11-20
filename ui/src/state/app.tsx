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
const INSPECTOR_KEY = "quarterdeck.inspector";

function readThemePref(): ThemePref {
  const v = localStorage.getItem(THEME_KEY);
  return v === "light" || v === "dark" ? v : "system";
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
  inspectorOpen: boolean;
  toggleInspector: () => void;
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
  const [inspectorOpen, setInspectorOpen] = useState(() => localStorage.getItem(INSPECTOR_KEY) !== "closed");
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

  // Theme: pinned light or dark sets data-theme; system leaves it to prefers-color-scheme.
  useEffect(() => {
    const root = document.documentElement;
    if (theme === "system") root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", theme);
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

  const toggleInspector = useCallback(() => {
    setInspectorOpen((o) => {
      localStorage.setItem(INSPECTOR_KEY, o ? "closed" : "open");
      return !o;
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
      inspectorOpen,
      toggleInspector,
      selectedTask,
      setSelectedTask,
      chooseHome,
      homeBusy,
      homeError,
    }),
    [info, live, ipcError, theme, setTheme, area, inspectorOpen, toggleInspector, selectedTask, chooseHome, homeBusy, homeError],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
