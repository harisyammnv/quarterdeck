/**
 * The UI's only door to the core. Inside the app window it calls the generated bindings;
 * in a plain browser (`pnpm dev` without Tauri) it serves the preview data instead, so the
 * layout can be worked on and screenshotted headlessly. The preview is labelled in the UI.
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

export function fleetView(): Promise<FleetView | null> {
  return inApp ? commands.fleetView() : Promise.resolve(previewView());
}

export function refreshFleet(): Promise<void> {
  return inApp ? commands.refreshFleet().then(() => undefined) : Promise.resolve();
}

export function sendNote(text: string, requestId: string): Promise<NoteReceipt> {
  if (inApp) return commands.sendNote(text, requestId);
  return Promise.reject(new Error("Preview mode: notes are only sent from the app window."));
}

export function viewRendered(seq: number, renderedMs: number): void {
  if (inApp) void commands.viewRendered(seq, renderedMs);
}

/** Subscribes to fleet updates; returns an unsubscribe function. */
export function onFleetUpdated(cb: (view: FleetView) => void): () => void {
  if (!inApp) return () => undefined;
  const pending = events.fleetUpdated.listen((e) => {
    cb(e.payload);
  });
  return () => {
    void pending.then((unlisten) => {
      unlisten();
    });
  };
}
