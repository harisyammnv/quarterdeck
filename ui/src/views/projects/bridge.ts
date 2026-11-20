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
  void (async () => {
    // Attach both listeners before the first fetch, so no update can fall between them.
    await events.projectsUpdated.listen((e) => {
      clearRetry();
      set({ view: e.payload, error: null });
    });
    // A new home starts a new engine: drop the old projects and ask the new engine directly,
    // in case its first update arrived before this notice.
    await events.homeChanged.listen(() => {
      set({ view: null, error: null });
      icons.clear();
      fetchView();
    });
    fetchView();
  })();
}

/** A failed read is retried a few times with a growing wait; any update ends the retries. */
const RETRY_MS = [2_000, 5_000, 15_000, 30_000];
let retry: { timer: ReturnType<typeof setTimeout>; n: number } | null = null;

function clearRetry() {
  if (retry) clearTimeout(retry.timer);
  retry = null;
}

function fetchView() {
  commands
    .projectsView()
    .then((v) => {
      clearRetry();
      // An update may have landed first; it is newer than this answer.
      if (!store.view) set({ view: v, error: null });
    })
    .catch((e: unknown) => {
      set({ view: store.view, error: e instanceof Error ? e.message : String(e) });
      const n = retry ? retry.n + 1 : 0;
      const wait = RETRY_MS[n];
      if (wait === undefined) return;
      retry = { n, timer: setTimeout(fetchView, wait) };
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

/**
 * A project's icon as a `data:` URL, or null for a monogram. A found icon is kept per name;
 * a miss or a failure is asked again after a few minutes, as the core does, so an icon added
 * to a repository appears without a restart.
 */
const MISS_MS = 5 * 60_000;
const icons = new Map<string, { at: number; p: Promise<string | null> }>();
export function projectIcon(name: string): Promise<string | null> {
  const hit = icons.get(name);
  if (hit && Date.now() - hit.at < MISS_MS) return hit.p;
  const at = Date.now();
  const p = inApp ? commands.projectIcon(name).catch(() => null) : Promise.resolve(previewIcon(name));
  icons.set(name, { at, p });
  void p.then((src) => {
    // Found icons are kept; the core re-checks the file itself when it changes.
    if (src !== null && icons.get(name)?.p === p) icons.set(name, { at: Number.POSITIVE_INFINITY, p });
  });
  return p;
}

/** Brings the task's terminal forward in Orca. */
export async function focusInOrca(taskId: string): Promise<void> {
  if (!inApp) throw new Error(`Preview mode: ${taskId} opens in Orca from the app window.`);
  await commands.orcaFocus(taskId);
}
