/**
 * Invented skills for working on the view in a plain browser. Nothing here is read from a
 * real skill folder.
 */
import type { SkillDoc, SkillItem, SkillsView } from "./bridge";

function skill(
  id: string,
  name: string,
  scope: SkillItem["scope"],
  description: string,
  source: string | null,
  locations: string[],
  alsoIn = 0,
  internal = false,
): SkillItem {
  return { id, name, description, scope, source, locations, also_in: alsoIn, internal, malformed: false };
}

const ITEMS: SkillItem[] = [
  skill("user:agents:security-audit", "security-audit", "user", "Security questions, focused reviews, audits and pen tests.", "example-org/security-audit-skill", ["~/.agents/skills"]),
  skill("user:agents:tdd", "tdd", "user", "Build features or fix bugs test-first.", "example-org/skills", ["~/.agents/skills", "~/.claude/skills"], 2),
  skill("user:agents:diagnosing-bugs", "diagnosing-bugs", "user", "A diagnosis loop for hard bugs and regressions.", "example-org/skills", ["~/.agents/skills"]),
  skill("user:agents:code-review", "code-review", "user", "Review a change for bugs and upkeep.", "example-org/skills", ["~/.agents/skills"], 2),
  skill("user:agents:domain-modeling", "domain-modeling", "user", "Pin down domain terms and record decisions.", "example-org/skills", ["~/.agents/skills"]),
  skill("user:claude:code-review", "code-review", "user", "Another copy of the review skill.", null, ["~/.claude/skills"], 2),
  skill("user:claude:release-notes", "release-notes", "user", "Draft release notes from merged pull requests.", null, ["~/.claude/skills"]),
  skill("firstmate:ship-landing", "ship-landing", "firstmate", "When a ship reports a pull request or a ready branch.", "firstmate", ["Firstmate .agents/skills"], 0, true),
  skill("firstmate:coding-guidelines", "coding-guidelines", "firstmate", "Before editing Firstmate's shared files.", "firstmate", ["Firstmate .agents/skills"], 0, true),
  skill("firstmate:stow", "stow", "firstmate", "Curate preferences and learnings into tiers instead of appending forever.", "firstmate", ["Firstmate .agents/skills"]),
  skill("project:ledger-api:claude:tdd", "tdd", "project", "Test-first rules for this repository's ledger code.", "ledger-api", ["ledger-api/.claude/skills"], 2),
  skill("project:portal-web:agents:form-callbacks", "form-callbacks", "project", "Rules for safe form callbacks in this repository.", "portal-web", ["portal-web/.agents/skills"]),
];

export function previewSkills(): SkillsView {
  const pad = Array.from({ length: 34 }, (_, i) =>
    skill(`user:agents:extra-${i}`, `helper-${String(i + 1).padStart(2, "0")}`, "user", "An invented helper skill used to fill the preview list.", "example-org/skills", ["~/.agents/skills"]),
  );
  return { items: [...ITEMS, ...pad], capped: false };
}

export function previewSkill(id: string): SkillDoc {
  const item = previewSkills().items.find((s) => s.id === id);
  if (!item) throw new Error(`no skill has the id "${id}"`);
  const text = [
    `# ${item.name}`,
    "",
    item.description,
    "",
    "## When to use it",
    "",
    "- A change touches behaviour that tests should pin down.",
    "- A bug report comes with steps that can be scripted.",
    "",
    "## Steps",
    "",
    "1. Write the failing test.",
    "2. Make it pass with the smallest change.",
    "3. Refactor with the suite green.",
    "",
    "\u0060\u0060\u0060sh",
    "pnpm test -- --watch",
    "\u0060\u0060\u0060",
  ].join("\n");
  return { item, text, truncated: false };
}
