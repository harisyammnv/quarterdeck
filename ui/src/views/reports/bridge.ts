/**
 * The reports area's door to the core: a task's report and its status history. In the app
 * window it calls the generated commands; in a plain browser it serves invented preview data.
 */
import { inApp } from "../../bridge";
import { commands, type ReportDoc_Serialize, type ReportHistory_Serialize } from "../../bindings";
import { previewHistory, previewReport } from "./preview";

export type ReportDoc = ReportDoc_Serialize;
export type ReportHistory = ReportHistory_Serialize;
export type StatusEvent = ReportHistory["events"][number];

/** The task's report, or null when it has none yet. */
export function reportRead(taskId: string): Promise<ReportDoc | null> {
  return inApp ? commands.reportRead(taskId) : previewReport(taskId);
}

/** The task's status events, newest first. */
export function reportHistory(taskId: string): Promise<ReportHistory> {
  return inApp ? commands.reportHistory(taskId) : previewHistory(taskId);
}
