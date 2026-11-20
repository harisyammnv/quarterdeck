/**
 * Invented board data for `pnpm dev`. Nothing here is read from a Firstmate home.
 */
import type { BoardCard, BoardView, Decision } from "./types";

const now = Math.floor(Date.now() / 1000);
const MIN = 60;
const H = 3600;
const D = 86400;

function card(c: Partial<BoardCard> & Pick<BoardCard, "id" | "title" | "column">): BoardCard {
  return {
    project: null,
    kind: "ship",
    since: null,
    hold: null,
    until: null,
    blocked_by: [],
    live: null,
    pr_url: null,
    has_report: false,
    ...c,
  };
}

function decision(d: Partial<Decision> & Pick<Decision, "id" | "question">): Decision {
  return {
    detail: null,
    project: null,
    kind: "captain",
    bucket: "live",
    until: null,
    age_days: 0,
    blocked_by: [],
    raised_at: null,
    pr_url: null,
    has_report: false,
    links: [],
    ...d,
  };
}

const live: Decision[] = [
  decision({
    id: "deps-rollup",
    question: "Merge the dependency rollup?",
    detail:
      "Rollup PR #494 is green, 13 of 13 checks. It combines updates 469, 477 and 478 and supersedes 472.\n\nOptions:\n(1) merge now [recommended];\n(2) hold until the actions update is fixed too;\n(3) decide later.",
    project: "ledger-api",
    raised_at: now - 22 * MIN,
    pr_url: "https://forge.example.invalid/acme/ledger-api/pull/494",
    links: ["https://forge.example.invalid/acme/ledger-api/actions/runs/1"],
  }),
  decision({
    id: "rebase-or-fresh",
    question: "Rebase #286 or open a fresh PR?",
    detail: "The repo rejects merge commits. Options: (1) rebase the branch and force-push; (2) open a fresh PR from main.",
    project: "route-planner",
    kind: "ship",
    raised_at: now - (H + 40 * MIN),
    pr_url: "https://forge.example.invalid/acme/route-planner/pull/286",
  }),
  decision({
    id: "visual-pick",
    question: "Pick a visual direction for the desktop app",
    detail: "Two mock directions, light and dark. The report compares them side by side.",
    project: "firstmate",
    kind: "decision",
    raised_at: now - 4 * MIN,
    has_report: true,
  }),
  decision({
    id: "staging-creds",
    question: "Share staging credentials with the route map task?",
    detail: "The performance run needs read access to staging. Options: (1) grant read-only for a day; (2) run it on a copy of production data.",
    project: "portal-web",
    raised_at: now - 2 * H,
  }),
];

const deferred: Decision[] = [
  decision({
    id: "python-bump",
    question: "Bump the Python image to 3.15?",
    detail: "No wheels yet for two packages.",
    project: "portal-web",
    bucket: "dated",
    until: "2026-12-02",
    age_days: 1,
    raised_at: now - D,
  }),
  decision({
    id: "cache-plan",
    question: "Approve the caching plan from the performance report",
    detail: "Report ready, three options.",
    project: "route-planner",
    kind: "task",
    bucket: "aged",
    age_days: 21,
    raised_at: now - 21 * D,
    has_report: true,
  }),
  decision({
    id: "browser-pin",
    question: "Allow the new browser runner after the clock fix",
    detail: "13 of 15 end-to-end tests fail today.",
    project: "route-planner",
    bucket: "blocked",
    blocked_by: ["clock-fix"],
    age_days: 1,
    raised_at: now - D,
  }),
];

const queuedTitles: [string, string, string][] = [
  ["clock-fix", "Fix the fake clock in the test harness", "route-planner"],
  ["storage-move", "Move uploads to own storage", "portal-web"],
  ["audit-log", "Add an audit log for admin actions", "ledger-api"],
  ["dark-charts", "Dark mode for the report charts", "portal-web"],
  ["retry-jobs", "Retry failed export jobs with backoff", "ledger-api"],
  ["docs-refresh", "Refresh the onboarding docs", "firstmate"],
  ["tz-bug", "Wrong time zone on weekly digests", "route-planner"],
  ["csv-export", "CSV export for the route list", "route-planner"],
  ["rate-limit", "Rate-limit the public search endpoint", "ledger-api"],
];

export function previewBoard(): BoardView {
  const queued: BoardCard[] = [
    card({ id: "python-bump", title: "Bump the Python image to 3.15?", column: "queued", project: "portal-web", kind: "captain", hold: "dated", until: "2026-12-02", since: now - D }),
    card({ id: "cache-plan", title: "Approve the caching plan from the performance report", column: "queued", project: "route-planner", kind: "task", hold: "aged", since: now - 21 * D, has_report: true }),
    card({ id: "browser-pin", title: "Allow the new browser runner after the clock fix", column: "queued", project: "route-planner", kind: "captain", hold: "blocked", blocked_by: ["clock-fix"], since: now - D }),
    ...queuedTitles.map(([id, title, project], i) =>
      card({
        id,
        title,
        column: "queued",
        project,
        since: now - (i + 2) * D,
        blocked_by: id === "storage-move" ? ["api-cutover"] : [],
      }),
    ),
  ];
  const ev = (word: string, ago: number, latest: string, state: LiveStateWord, source: "snapshot" | "event" = "event") => ({
    state,
    word,
    source,
    latest,
    latest_at: now - ago,
  });
  const in_flight: BoardCard[] = [
    card({ id: "login-flake", title: "Fix the login test flake", column: "in-flight", project: "ledger-api", since: now - 18 * MIN, live: ev("working", 40, "suite green, committing the fix", "working") }),
    card({ id: "auth-audit", title: "Audit the auth middleware", column: "in-flight", project: "portal-web", kind: "scout", since: now - 3 * H, has_report: true, live: ev("working", 900, "reading the session code", "working", "snapshot") }),
    card({ id: "route-perf", title: "Speed up the route map", column: "in-flight", project: "portal-web", since: now - 5 * H, live: ev("blocked", 1500, "needs staging credentials", "blocked") }),
    card({ id: "release-notes", title: "Release notes generator", column: "in-flight", project: "route-planner", since: now - 2 * H, pr_url: "https://forge.example.invalid/acme/route-planner/pull/286", live: ev("needs-decision", 6000, "which changelog format: A or B?", "needs-decision") }),
    card({ id: "cache-scout", title: "Find the slow dashboard query", column: "in-flight", project: "route-planner", kind: "scout", since: now - D, live: ev("paused", 7200, "waiting for CI", "paused", "snapshot") }),
    card({ id: "infra-sync", title: "Sync the deploy templates", column: "in-flight", project: "firstmate", since: now - 2 * D, live: { state: "unknown", word: "unknown", source: "snapshot", latest: null, latest_at: null } }),
  ];
  const done: BoardCard[] = [
    card({ id: "pin-update", title: "Platform pin to 4.14.4", column: "done", project: "route-planner", since: now - D, pr_url: "https://forge.example.invalid/acme/route-planner/pull/37" }),
    card({ id: "theme-switch", title: "Theme switch in the title bar", column: "done", project: "firstmate", since: now - 2 * D, has_report: true }),
    card({ id: "deps-1117", title: "Daily dependency rollup", column: "done", project: "ledger-api", since: now - 3 * D, pr_url: "https://forge.example.invalid/acme/ledger-api/pull/488" }),
  ];
  return {
    ready: true,
    generated_at: now - 14,
    queued,
    queued_total: queued.length,
    in_flight,
    done,
    done_total: 27,
    decisions: { live, deferred, deferred_total: deferred.length },
    other: [],
  };
}

type LiveStateWord = NonNullable<BoardCard["live"]>["state"];
