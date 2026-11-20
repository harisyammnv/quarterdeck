/**
 * The UI's only door to the core. Inside the app window it calls the generated bindings;
 * in a plain browser (`pnpm dev` without Tauri) it serves the preview data instead, so the
 * layout can be worked on and screenshotted headlessly. The preview is labelled in the UI.
 *
 * Areas add their own calls here, next to the command they wrap, and a preview fallback.
 */
import { isTauri } from "@tauri-apps/api/core";
import {
  commands,
  events,
  type AppInfo_Serialize,
  type FleetView_Serialize,
  type NoteReceipt,
} from "./bindings";
import { previewInfo, previewView } from "./preview";

export type AppInfo = AppInfo_Serialize;
export type FleetView = FleetView_Serialize;
export type FleetRow = FleetView["rows"][number];
export type { NoteReceipt };

export const inApp = isTauri();

export function appInfo(): Promise<AppInfo> {
  return inApp ? commands.appInfo() : Promise.resolve(previewInfo());
}

/**
 * Opens the system folder picker in the core and switches to the picked home. Resolves with
 * the new facts, or the unchanged ones when the picker was cancelled.
 */
export async function homePick(): Promise<AppInfo> {
  if (!inApp) throw new Error("Preview mode: the home is chosen from the app window.");
  return commands.homePick();
}

/** Opens an https link in the default browser. Call only after the person confirmed the address. */
export async function urlOpen(url: string): Promise<void> {
  if (!inApp) throw new Error("Preview mode: links open from the app window.");
  await commands.urlOpen(url);
}

/** Opens a task's recorded pull request; the core takes the address from the snapshot. */
export async function taskPrOpen(taskId: string): Promise<void> {
  if (!inApp) throw new Error("Preview mode: pull requests open from the app window.");
  await commands.taskPrOpen(taskId);
}

export function fleetView(): Promise<FleetView | null> {
  return inApp ? commands.fleetView() : Promise.resolve(previewView());
}

export function refreshFleet(): Promise<void> {
  return inApp ? commands.refreshFleet().then(() => undefined) : Promise.resolve();
}

export async function sendNote(text: string, requestId: string): Promise<NoteReceipt> {
  if (!inApp) throw new Error("Preview mode: notes are only sent from the app window.");
  return commands.sendNote(text, requestId);
}

export function viewRendered(seq: number, renderedMs: number): void {
  if (inApp) void commands.viewRendered(seq, renderedMs);
}

function listen<T>(event: { listen: (cb: (e: { payload: T }) => void) => Promise<() => void> }, cb: (payload: T) => void): () => void {
  if (!inApp) return () => undefined;
  const pending = event.listen((e) => {
    cb(e.payload);
  });
  return () => {
    void pending.then((unlisten) => {
      unlisten();
    });
  };
}

/** Subscribes to fleet updates; returns an unsubscribe function. */
export function onFleetUpdated(cb: (view: FleetView) => void): () => void {
  return listen(events.fleetUpdated, cb);
}

/** Subscribes to home changes (a new home chosen); returns an unsubscribe function. */
export function onHomeChanged(cb: (info: AppInfo) => void): () => void {
  return listen(events.homeChanged, cb);
}
