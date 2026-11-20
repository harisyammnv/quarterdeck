import { useSyncExternalStore } from "react";

/** The two read-only panes of a task. */
export type TaskTab = "terminal" | "report";

let current: TaskTab = "terminal";
const listeners = new Set<() => void>();

function subscribe(cb: () => void): () => void {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
}

/** Chooses which pane the task view shows. */
export function setTaskTab(tab: TaskTab): void {
  if (tab === current) return;
  current = tab;
  for (const l of listeners) l();
}

/** The pane the task view shows, kept across visits while the window is open. */
export function useTaskTab(): TaskTab {
  return useSyncExternalStore(subscribe, () => current);
}

/** The area id of the task view in the registry. */
export const TASK_AREA = "terminal";
