/**
 * The board area's door to the core. In the app window it calls the typed commands; in a
 * plain browser it serves invented preview data so the view can be laid out headlessly.
 */
import { inApp, sendNote, type NoteReceipt } from "../../bridge";
import { previewBoard } from "./preview";
import type { BoardView } from "./types";

export type { BoardCard, BoardView, Decision, HoldBucket } from "./types";

/** The current board. Until the board command is registered, the app shows preview data too. */
export function boardView(): Promise<BoardView> {
  return Promise.resolve(previewBoard());
}

/** Subscribes to board updates; returns an unsubscribe function. */
export function onBoardUpdated(cb: (view: BoardView) => void): () => void {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
}

const listeners = new Set<(view: BoardView) => void>();

/** A fresh request id, so a retried send can never create a second note. */
function requestId(): string {
  return `qd-reply-${crypto.randomUUID()}`;
}

/** Sends a reply about a decision to Firstmate as an inbox note carrying the task id. */
export function replyToDecision(taskId: string, text: string): Promise<NoteReceipt> {
  const body = `Re ${taskId}: ${text.trim()}`;
  return sendNote(body, requestId());
}

/**
 * Opens a pull request or link in the browser through the core's checked opener. The UI never
 * opens a URL itself. Wired to the shell's `url_open` at integration.
 */
export function openLink(url: string): Promise<void> {
  return Promise.reject(new Error(`Opening links is not wired yet: ${url}`));
}

/** Opens a task's report in the reports view, by task id. Wired at integration. */
export function openReport(taskId: string): void {
  lastReportRequest = taskId;
}

export let lastReportRequest: string | null = null;

export { inApp };
