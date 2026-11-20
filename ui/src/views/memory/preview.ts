/**
 * Invented memory for working on the view in a plain browser. Nothing here is read from a
 * Firstmate home.
 */
import type { MemoryDoc, MemoryItem, MemorySearch, MemoryView } from "./bridge";

const now = Math.floor(Date.now() / 1000);
const H = 3600;
const D = 24 * H;

const DOCS: Record<string, string> = {
  preferences: [
    "# Preferences",
    "",
    "- Keep pull requests small and say in the description what was left out.",
    "- Ask before any production change, and show the exact plan first.",
    "- Use the headless browser for UI checks; never leave a visible window open.",
    "- Every new project is added to the sidebar before work starts on it.",
  ].join("\n"),
  "shared-preferences": [
    "# Shared preferences",
    "",
    "- Workers start on the default branch unless a project says otherwise.",
    "- Reviews come from the delivery path; a separate review runs only when asked for.",
  ].join("\n"),
  learnings: [
    "### Dependency updates",
    "",
    "- Combine the day's updates into one pull request per project, and say which were left out and why.",
    "- ledger-api rejects merge commits, so a rollup must be rebased before it can land.",
    "- A browser test tool update is held back until the fake-clock tests pass again.",
    "",
    "### Local environments",
    "",
    "- Containers run on the local engine. Read the active Docker context instead of assuming a socket path.",
    "- Each worktree gets its own compose project, so ports must come from the worktree's `.env`.",
    "",
    "### Reviews",
    "",
    "- The delivery path owns its own review. A separate council run happens only when asked for.",
    "- A finding is confirmed by someone other than the one who found it.",
    "",
    "### Design",
    "",
    "- Reuse the product palettes: white canvas, black ink, flat surfaces, one accent.",
  ].join("\n"),
};

function item(id: string, kind: MemoryItem["kind"], title: string, label: string, age: number, owner: string | null = null): MemoryItem {
  return { id, kind, title, owner, label, modified: now - age };
}

const BRIEF_IDS = [
  "login-flake",
  "deps-rollup",
  "auth-audit",
  "route-perf",
  "pin-update",
  "cache-scout",
  "infra-sync",
  "export-csv",
  "search-index",
  "billing-retry",
  "theme-tokens",
  "audit-log",
];

export function previewMemory(): MemoryView {
  return {
    items: [
      item("preferences", "preferences", "Preferences", "data/captain.md", 2 * H),
      item("shared-preferences", "shared-preferences", "Shared preferences", "data/captain-shared.md", 3 * D),
      item("learnings", "learnings", "Learnings", "data/learnings.md", 5 * H),
      item("project:ledger-api", "project-guide", "ledger-api", "AGENTS.md", 2 * D, "ledger-api"),
      item("project:portal-web", "project-guide", "portal-web", "AGENTS.md", 6 * D, "portal-web"),
      item("project:route-planner", "project-guide", "route-planner", "CLAUDE.md", 12 * D, "route-planner"),
      ...BRIEF_IDS.map((id, i) => item(`brief:${id}`, "brief", id, `data/${id}/brief.md`, (i + 1) * 3 * H, id)),
    ],
  };
}

function briefText(id: string): string {
  return [
    "# Task",
    "",
    `## Intent`,
    "",
    `Fix ${id.replace(/-/g, " ")} in ledger-api and open a pull request.`,
    "",
    "## Rules",
    "",
    "1. Never push to the default branch.",
    "2. Keep proof and scratch output outside the worktree.",
    "3. Report status with one line per phase change.",
  ].join("\n");
}

export function previewDoc(id: string): MemoryDoc {
  const it = previewMemory().items.find((i) => i.id === id);
  if (!it) throw new Error(`no memory document has the id "${id}"`);
  const text = DOCS[id] ?? (it.kind === "brief" ? briefText(it.title) : `# ${it.title}\n\nRun the tests before opening a pull request.\n\n- Use \`pnpm check\`.\n`);
  const entries = text.split("\n").filter((l) => l.startsWith("- ") || l.startsWith("* ")).length;
  return { item: it, text, entries, truncated: false };
}

export function previewSearch(query: string): MemorySearch {
  const q = query.trim().toLowerCase();
  const out: MemorySearch = { query: query.trim(), hits: [], truncated: false, documents: 0 };
  if (q.length < 2) return out;
  for (const it of previewMemory().items) {
    const lines = previewDoc(it.id).text.split("\n");
    let found = false;
    lines.forEach((line, n) => {
      const at = line.toLowerCase().indexOf(q);
      if (at < 0) return;
      found = true;
      const lead = /^\s*(?:[-*>]\s+|#+\s*|\d+\.\s+)?/.exec(line)?.[0].length ?? 0;
      const snippet = line.slice(lead);
      out.hits.push({ id: it.id, kind: it.kind, title: it.title, line: n + 1, snippet, ranges: [[at - lead, at - lead + q.length]] });
    });
    if (found) out.documents += 1;
  }
  return out;
}
