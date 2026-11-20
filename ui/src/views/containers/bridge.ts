/**
 * The containers view's door to the core: typed wrappers over the generated bindings, with
 * the invented preview data in a plain browser, and one shared copy of the last listing so
 * the status bar item and the view read the same answer.
 */
import {
  commands,
  type ContainerDetail_Serialize,
  type EngineSummary_Serialize,
  type ContainerLogs_Serialize,
  type ContainerStats_Serialize,
  type ContainersView_Serialize,
} from "../../bindings";
import { inApp } from "../../bridge";
import { previewContainers, previewDetail, previewLogs, previewStats, previewSummary } from "./preview";

export type ContainersView = ContainersView_Serialize;
export type ContainerGroup = ContainersView["groups"][number];
export type ContainerRow = ContainerGroup["containers"][number];
export type ImageRow = ContainersView["images"][number];
export type VolumeRow = ContainersView["volumes"][number];
export type NetworkRow = ContainersView["networks"][number];
export type ContainerStats = ContainerStats_Serialize;
export type ContainerLogs = ContainerLogs_Serialize;
export type LogLine = ContainerLogs["lines"][number];
export type ContainerDetail = ContainerDetail_Serialize;
export type EngineSummary = EngineSummary_Serialize;

let last: ContainersView | null = null;
let pending: Promise<ContainersView> | null = null;
const listeners = new Set<() => void>();

/** The last listing, or null before the first one. */
export function lastContainers(): ContainersView | null {
  return last;
}

/** Subscribes to new listings; returns an unsubscribe function. */
export function onContainers(cb: () => void): () => void {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
}

/** Lists the engine, sharing one call in flight between every caller. */
export function loadContainers(): Promise<ContainersView> {
  if (pending) return pending;
  const call = inApp ? commands.containersList() : Promise.resolve(previewContainers());
  pending = call
    .then((v) => {
      last = v;
      for (const cb of listeners) cb();
      return v;
    })
    .finally(() => {
      pending = null;
    });
  return pending;
}

/** The Docker context and engine state, from discovery and `/version` only. */
export function containersStatus(): Promise<EngineSummary> {
  return inApp ? commands.containersStatus() : Promise.resolve(previewSummary());
}

/** Start and finish times of one listed container, inspected alone. */
export function containerInspect(id: string): Promise<ContainerDetail> {
  return inApp ? commands.containerInspect(id) : Promise.resolve(previewDetail(id));
}

export function containerLogs(id: string): Promise<ContainerLogs> {
  return inApp ? commands.containerLogs(id) : Promise.resolve(previewLogs(id));
}

export function containerStats(id: string): Promise<ContainerStats> {
  return inApp ? commands.containerStats(id) : Promise.resolve(previewStats(id));
}

/** Opens Kiri's own window. */
export async function kiriOpen(): Promise<void> {
  if (!inApp) throw new Error("Preview mode: Kiri opens from the app window.");
  await commands.kiriOpen();
}
