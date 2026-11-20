/**
 * Preview data for working on the UI in a plain browser. Names are invented; nothing here
 * is read from a Firstmate home. URL parameters: `theme=light|dark` pins the theme,
 * `nohome=1` shows the first-launch state, `area=<id>` opens an area.
 */
import type { AppInfo_Serialize, FleetView_Serialize } from "./bindings";

const now = Math.floor(Date.now() / 1000);
const params = new URLSearchParams(location.search);

export function previewParam(name: string): string | null {
  return params.get(name);
}

export function previewInfo(): AppInfo_Serialize {
  const theme = params.get("theme");
  const noHome = params.get("nohome") === "1";
  return {
    home: noHome ? null : "example/firstmate",
    home_source: noHome ? null : "chosen",
    home_error: noHome ? "no Firstmate home chosen yet" : null,
    theme: theme === "light" || theme === "dark" ? theme : null,
    version: "preview",
  };
}

export function previewView(): FleetView_Serialize | null {
  if (params.get("nohome") === "1") return null;
  const ev = (word: string, ago: number, text: string) => ({ word, at: now - ago, key: null, text });
  return {
    seq: 1,
    home: "example/firstmate",
    decisions: 3,
    snapshot: { generated_at: now - 14, running: false, error: null, cached: false },
    cause: null,
    rows: [
      { id: "login-flake", name: "Fix the login test flake", project: "ledger-api", kind: "ship", state: "working", state_word: "working", source: "event", doing: null, latest_event: ev("working", 40, "suite green, committing the fix") },
      { id: "deps-rollup", name: "Daily dependency rollup", project: "ledger-api", kind: "ship", state: "needs-decision", state_word: "needs-decision", source: "snapshot", doing: "PR ready, awaiting a merge word", latest_event: ev("needs-decision", 600, "merge the rollup?") },
      { id: "auth-audit", name: "Audit the auth middleware", project: "portal-web", kind: "scout", state: "working", state_word: "working", source: "snapshot", doing: "reading the session code", latest_event: ev("working", 900, "setup done") },
      { id: "route-perf", name: "Speed up the route map", project: "portal-web", kind: "ship", state: "blocked", state_word: "blocked", source: "snapshot", doing: "needs staging credentials", latest_event: ev("blocked", 1500, "needs staging credentials") },
      { id: "pin-update", name: "Platform pin to 4.14.4", project: "route-planner", kind: "ship", state: "done", state_word: "done", source: "snapshot", doing: "PR opened", latest_event: ev("done", 5400, "PR opened") },
      { id: "cache-scout", name: "Find the slow dashboard query", project: "route-planner", kind: "scout", state: "paused", state_word: "paused", source: "snapshot", doing: "waiting for CI", latest_event: ev("paused", 7200, "waiting for CI") },
      { id: "infra-sync", name: "Sync the deploy templates", project: "firstmate", kind: "ship", state: "unknown", state_word: "unknown", source: "snapshot", doing: "backend unreachable", latest_event: null },
    ],
  };
}
