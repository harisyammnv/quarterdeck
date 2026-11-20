/**
 * The reports view's door to the core: a task's report and its status history. In the app
 * window it calls the generated commands; in a plain browser it serves invented preview data.
 */
import { inApp } from "../../bridge";
import { previewHistory, previewReport } from "./preview";

export interface ReportDoc {
  task_id: string;
  /** Untrusted Markdown; render it only through the shared Markdown component. */
  markdown: string;
  bytes: number;
  truncated: boolean;
  modified: number | null;
}

export interface StatusEvent {
  word: string;
  at: number | null;
  key: string | null;
  text: string;
}

export interface StatusHistory {
  task_id: string;
  /** Newest first. */
  events: StatusEvent[];
  truncated: boolean;
}

/** The task's report, or null when it has none yet. */
export function reportRead(taskId: string): Promise<ReportDoc | null> {
  if (inApp) return Promise.reject(new Error("Reports are not wired into this build yet."));
  return previewReport(taskId);
}

/** The task's status events, newest first. */
export function statusHistory(taskId: string): Promise<StatusHistory> {
  if (inApp) return Promise.reject(new Error("Status history is not wired into this build yet."));
  return previewHistory(taskId);
}
