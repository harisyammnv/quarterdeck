/**
 * Invented preview data for the projects sidebar and inspector. Nothing here is read from a
 * Firstmate home; names follow the ones already used on main.
 */
import type { ProjectsView_Serialize as ProjectsView } from "../../bindings";

const now = Math.floor(Date.now() / 1000);
const ev = (word: string, ago: number, text: string) => ({ word, at: now - ago, key: null, text });

type Row = ProjectsView["projects"][number]["tasks"][number];

function row(id: string, kind: string, state: Row["state"], ago: number, text: string, extra: Partial<Row> = {}): Row {
  return {
    id,
    title: null,
    kind,
    state,
    state_word: state,
    source: "snapshot",
    harness: "claude",
    branch: `fm/${id}`,
    mode: "direct-PR",
    started_at: now - ago - 240,
    latest_event: ev(state, ago, text),
    in_orca: true,
    ...extra,
  };
}

export function previewProjects(): ProjectsView {
  // The same tasks as the shell's fleet preview, so selection stays in step across areas.
  return {
    ready: true,
    projects: [
      {
        name: "ledger-api",
        tasks: [
          row("deps-rollup", "ship", "needs-decision", 600, "Merge the rollup?", { title: "Daily dependency rollup", mode: "no-mistakes" }),
          row("login-flake", "ship", "working", 40, "Suite green, committing the fix", { title: "Fix the login test flake", source: "event" }),
        ],
      },
      {
        name: "portal-web",
        tasks: [
          row("auth-audit", "scout", "working", 900, "Reading the session middleware", { title: "Audit the auth middleware" }),
          row("route-perf", "ship", "blocked", 1500, "Needs staging credentials", { title: "Speed up the route map", harness: "codex" }),
        ],
      },
      {
        name: "route-planner",
        tasks: [
          row("cache-scout", "scout", "paused", 7200, "Waiting for CI", { title: "Find the slow dashboard query", in_orca: false }),
          row("pin-update", "ship", "done", 5400, "PR opened", { title: "Platform pin to 4.14.4", mode: "local-only" }),
        ],
      },
      {
        name: "firstmate",
        tasks: [row("infra-sync", "ship", "unknown", 0, "", { title: "Sync the deploy templates", latest_event: null, started_at: null })],
      },
    ],
  };
}

/** Invented icons: a ledger mark and a compass, drawn for the preview. */
const ICONS: Record<string, string> = {
  "ledger-api":
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="#5d49d9"/><path d="M18 16h20l8 8v24H18z" fill="#fff"/><path d="M24 30h16M24 37h16M24 44h10" stroke="#5d49d9" stroke-width="3" stroke-linecap="round"/></svg>',
  "portal-web":
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="#fff"/><circle cx="32" cy="32" r="20" fill="none" stroke="#1d5fbf" stroke-width="5"/><path d="M32 16l6 16-6 16-6-16z" fill="#1d5fbf"/></svg>',
};

export function previewIcon(name: string): string | null {
  const svg = ICONS[name];
  return svg ? `data:image/svg+xml;base64,${btoa(svg)}` : null;
}
