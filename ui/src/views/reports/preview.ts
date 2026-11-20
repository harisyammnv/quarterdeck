/**
 * Invented reports and status history for working on the views in a plain browser. Nothing
 * here is read from a Firstmate home.
 */
import type { ReportDoc, ReportHistory, ReportsList, StatusEvent } from "./bridge";

const now = Math.floor(Date.now() / 1000);

const REPORT = `# Fix the login test flake

## Outcome

The login suite is stable: the flaky case passed **40 of 40** runs after the fix, and the full suite is green (107 passed, 1 skipped).

## Root cause

The \`session\` fixture in \`tests/conftest.py\` was scoped to the whole session, so parallel workers shared one token.
When one worker logged out, the others failed with a stale token.

## Change

- Scope the fixture to each test function.
- Give each login test its own user.

| Run | Before | After |
|---|---|---|
| Flaky case, 40 times | 37 passed | 40 passed |
| Full suite | 106 passed | 107 passed |

## Follow-ups

1. The skipped test needs a staging key; it is tracked separately.
2. Consider a lint rule against session-scoped fixtures that hold credentials.
`;

function delay<T>(value: T): Promise<T> {
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve(value);
    }, 120);
  });
}

export function previewReport(taskId: string): Promise<ReportDoc | null> {
  if (taskId === "auth-audit" || taskId === "infra-sync") return delay(null);
  if (taskId === "route-perf") return Promise.reject(new Error("the report is a link, which is not followed"));
  return delay({ task_id: taskId, markdown: REPORT, truncated: false, modified: now - 300 });
}

const ev = (word: string, ago: number, text: string, key: string | null = null): StatusEvent => ({ word, at: now - ago, key, text });

export function previewHistory(taskId: string): Promise<ReportHistory> {
  if (taskId === "infra-sync") return delay({ task_id: taskId, events: [] });
  if (taskId !== "login-flake") {
    return delay({
      task_id: taskId,
      events: [ev("working", 900, "reading the code paths involved"), ev("working", 1800, "setup done")],
    });
  }
  return delay({
    task_id: taskId,
    events: [
      ev("working", 40, "Suite green, 107 passed"),
      ev("resolved", 220, "use a per-test user", "user-scope"),
      ev("needs-decision", 400, "one shared test user or one per test?", "user-scope"),
      ev("working", 660, "Root cause found in the session fixture"),
      ev("working", 1140, "Reproduced the flake, 3 of 40 runs failed"),
      ev("working", 1560, "Started from dev"),
      ev("working", 1600, "setup done"),
    ],
  });
}

export function previewList(): Promise<ReportsList> {
  return delay({
    truncated: false,
    items: [
      { task_id: "login-flake", title: "Fix the login test flake", kind: "ship", modified: now - 300 },
      { task_id: "cache-scout", title: "Find the slow dashboard query", kind: "scout", modified: now - 7200 },
      { task_id: "pin-update", title: "Platform pin to 4.14.4", kind: "ship", modified: now - 86_400 },
    ],
  });
}
