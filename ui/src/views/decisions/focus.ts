/**
 * A one-shot hand-off from the strip to the decisions view: which call to select, and
 * whether to focus the reply box. The view takes it on mount.
 */
let pending: { id: string; reply: boolean } | null = null;

export function requestDecision(id: string, reply = false): void {
  pending = { id, reply };
}

export function takeDecisionRequest(): { id: string; reply: boolean } | null {
  const p = pending;
  pending = null;
  return p;
}
