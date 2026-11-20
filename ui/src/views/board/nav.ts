/**
 * Moving between the board, the strip and the decisions view: which call to select when the
 * decisions view opens, and whether to focus its reply box. Taken once by the view.
 */
import { views } from "../../registry";

let pending: { id: string; reply: boolean } | null = null;

export function requestDecision(id: string | null, reply = false): void {
  pending = id ? { id, reply } : null;
}

export function takeDecisionRequest(): { id: string; reply: boolean } | null {
  const p = pending;
  pending = null;
  return p;
}

/** The reports view, when one is registered; "Read the report" opens it on the task. */
export const reportsArea: string | null = views.find((v) => v.id === "reports")?.id ?? null;
