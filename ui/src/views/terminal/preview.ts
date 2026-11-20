/**
 * Invented terminal output for working on the view in a plain browser. Nothing here is read
 * from a Firstmate home.
 */
import type { Span, TerminalTail, Tone } from "./bridge";

const s = (text: string, tone: Tone = "default", bold = false): Span => ({ text, tone, bold });

const PROMPT = s("ledger-api/login-flake $ ", "dim");

const SCRIPT: Span[][] = [
  [PROMPT, s("uv run pytest tests/test_login.py -q --count 40")],
  [s("collected 18 items, repeated 40 times", "dim")],
  [s("tests/test_login.py "), s("........................................................", "green"), s(" [100%]", "dim")],
  [s("720 passed", "green"), s(" in 96.4s")],
  [],
  [s("●", "blue"), s(" Ran the flaky case 40 times: 40 passed.")],
  [s("●", "blue"), s(" Root cause: the session fixture reused a stale token across workers.")],
  [s('  tests/conftest.py:41   scope="session"  ->  scope="function"', "dim")],
  [s("●", "blue"), s(" Running the full suite before opening the pull request.")],
  [],
  [PROMPT, s("uv run pytest -q")],
  [s(".................................................................", "green"), s(" [ 38%]", "dim")],
  [s(".................................................................", "green"), s(" [ 76%]", "dim")],
  [s(".........................................", "green"), s("s", "yellow"), s("......", "green")],
  [s("107 passed", "green"), s(", "), s("1 skipped", "yellow"), s(" in 41.8s")],
  [],
  [s("●", "blue"), s(" Suite is green. Committing the fix.")],
  [PROMPT, s('git commit -m "fix(tests): give each login test its own session"')],
  [s("[fm/login-flake 3c1e9a2] 2 files changed, 9 insertions(+), 4 deletions(-)", "dim")],
  [s("●", "blue"), s(" Starting the delivery pipeline: review, tests, docs, then the pull request.")],
  [s("▌", "yellow")],
];

/** Preview behaviour by task id: one task is remote, one has no terminal, the rest stream. */
export function previewTail(taskId: string): Promise<TerminalTail> {
  return new Promise((resolve, reject) => {
    setTimeout(() => {
      if (taskId === "route-perf") {
        reject(new Error("This task runs on another host; its terminal is not read from here"));
        return;
      }
      if (taskId === "infra-sync") {
        reject(new Error("The terminal could not be read: backend target gone"));
        return;
      }
      if (taskId === "pin-update") {
        resolve({ task_id: taskId, lines: [], requested: 200, truncated: false, digest: "0", captured_at: Math.floor(Date.now() / 1000) });
        return;
      }
      resolve({
        task_id: taskId,
        lines: SCRIPT,
        requested: 200,
        truncated: false,
        digest: "preview",
        captured_at: Math.floor(Date.now() / 1000),
      });
    }, 120);
  });
}
