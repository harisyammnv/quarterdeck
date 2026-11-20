/**
 * The projects area's door to the core. Inside the app window it calls the generated bindings;
 * in a plain browser (`pnpm dev`) it serves the invented preview instead.
 *
 * One subscription is shared by the sidebar and the inspector sections, so the view is fetched
 * once and each core update reaches every reader.
 */
import { useSyncExternalStore } from "react";
import { commands, events, type ProjectsView_Serialize } from "../../bindings";
import { inApp } from "../../bridge";
import { previewIcon, previewProjects } from "./preview";

export type ProjectsView = ProjectsView_Serialize;
export type ProjectGroup = ProjectsView["projects"][number];
export type ProjectRow = ProjectGroup["tasks"][number];

interface Store {
  view: ProjectsView | null;
  error: string | null;
}

let store: Store = { view: null, error: null };
const listeners = new Set<() => void>();
let started = false;

function set(next: Store) {
  store = next;
  for (const l of listeners) l();
}

function start() {
  if (started) return;
  started = true;
  if (!inApp) {
    set({ view: previewProjects(), error: null });
    return;
  }
  void events.projectsUpdated.listen((e) => {
    set({ view: e.payload, error: null });
  });
  // A new home starts a new engine: forget the old projects until its first view arrives.
  void events.homeChanged.listen(() => {
    set({ view: null, error: null });
    icons.clear();
  });
  commands
    .projectsView()
    .then((v) => {
      // An update may have landed first; it is newer than this answer.
      if (!store.view) set({ view: v, error: null });
    })
    .catch((e: unknown) => {
      set({ view: store.view, error: e instanceof Error ? e.message : String(e) });
    });
}

function subscribe(l: () => void): () => void {
  start();
  listeners.add(l);
  return () => listeners.delete(l);
}

/** The live projects view (null until the core answers) and the error, if the call failed. */
export function useProjects(): Store {
  return useSyncExternalStore(subscribe, () => store);
}

/** A row and its project name, by task id. */
export function findRow(view: ProjectsView | null, id: string | null): { row: ProjectRow; project: string | null } | null {
  if (!view || !id) return null;
  for (const g of view.projects) {
    const row = g.tasks.find((t) => t.id === id);
    if (row) return { row, project: g.name };
  }
  return null;
}

/** A project's icon as a `data:` URL, or null for a monogram. Answers are kept per name. */
const icons = new Map<string, Promise<string | null>>();
export function projectIcon(name: string): Promise<string | null> {
  let p = icons.get(name);
  if (!p) {
    p = inApp ? commands.projectIcon(name).catch(() => null) : Promise.resolve(previewIcon(name));
    icons.set(name, p);
  }
  return p;
}

/** Brings the task's terminal (or worktree) forward in Orca. */
export async function focusInOrca(taskId: string): Promise<void> {
  if (!inApp) throw new Error(`Preview mode: ${taskId} opens in Orca from the app window.`);
  await commands.projectsFocusOrca(taskId);
}
