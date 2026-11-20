/**
 * The memory view's door to the core: the generated commands inside the app window, the
 * invented preview data in a plain browser.
 */
import { inApp } from "../../bridge";
import {
  commands,
  type MemoryDoc_Serialize as MemoryDoc,
  type MemoryHit_Serialize as MemoryHit,
  type MemoryItem_Serialize as MemoryItem,
  type MemorySearch_Serialize as MemorySearch,
  type MemoryView_Serialize as MemoryView,
} from "../../bindings";
import { previewDoc, previewMemory, previewSearch } from "./preview";

export type { MemoryDoc, MemoryHit, MemoryItem, MemorySearch, MemoryView };
export type MemoryKind = MemoryItem["kind"];

/** Rescans the home's memory documents. */
export function memoryList(): Promise<MemoryView> {
  return inApp ? commands.memoryList() : Promise.resolve(previewMemory());
}

export function memoryRead(id: string): Promise<MemoryDoc> {
  if (inApp) return commands.memoryRead(id);
  try {
    return Promise.resolve(previewDoc(id));
  } catch (e) {
    return Promise.reject(e instanceof Error ? e : new Error(String(e)));
  }
}

export function memorySearch(query: string): Promise<MemorySearch> {
  return inApp ? commands.memorySearch(query) : Promise.resolve(previewSearch(query));
}

