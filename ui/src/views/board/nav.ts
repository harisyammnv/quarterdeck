/**
 * Moving between the board, the strip and the decisions view: which call to select when the
 * decisions view opens, and whether to focus its reply box. Taken once by the view.
 */
import { useCallback } from "react";
import { useApp } from "../../state/app";
import { useOpenTask } from "../terminal/open";

let pending: { id: string; reply: boolean } | null = null;

export function requestDecision(id: string | null, reply = false): void {
  pending = id ? { id, reply } : null;
}

export function takeDecisionRequest(): { id: string; reply: boolean } | null {
  const p = pending;
  pending = null;
  return p;
}

/**
 * "Read the report" opens the task view on its Report tab. That view shows tasks from the
 * fleet list, so the action is offered only when the task has a report and is in the list;
 * returns null otherwise.
 */
export function useReportOpener(): (taskId: string, hasReport: boolean) => (() => void) | null {
  const open = useOpenTask();
  const rows = useApp().live.view?.rows;
  return useCallback(
    (taskId: string, hasReport: boolean) =>
      hasReport && rows?.some((r) => r.id === taskId)
        ? () => {
            open(taskId, "report");
          }
        : null,
    [open, rows],
  );
}
