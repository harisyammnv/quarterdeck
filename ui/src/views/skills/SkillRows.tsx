import { memo } from "react";
import { FolderSimpleIcon as FolderSimple } from "@phosphor-icons/react";
import { StatusChip, Tag } from "../../ui";
import type { SkillItem, SkillScope } from "./bridge";

export const SCOPE_LABEL: Record<SkillScope, string> = {
  firstmate: "Firstmate",
  user: "User",
  project: "Project",
};

export type ScopeFilter = "all" | SkillScope;

export function matchesSkill(s: SkillItem, scope: ScopeFilter, query: string): boolean {
  if (scope !== "all" && s.scope !== scope) return false;
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return s.name.toLowerCase().includes(q) || s.description.toLowerCase().includes(q) || (s.source ?? "").toLowerCase().includes(q);
}

/** One skill: name, duplicate warning and scope on the first line, then trigger and source. */
export const SkillRow = memo(function SkillRow({
  skill,
  selected,
  onOpen,
}: {
  skill: SkillItem;
  selected: boolean;
  onOpen: (id: string) => void;
}) {
  const where = skill.source ?? skill.locations[0] ?? "";
  return (
    <button
      type="button"
      className={`skills-row${selected ? " sel" : ""}`}
      aria-current={selected ? "true" : undefined}
      onClick={() => {
        onOpen(skill.id);
      }}
    >
      <span className="skills-line">
        <span className="skills-name trunc">{skill.name}</span>
        {skill.also_in > 0 && (
          <StatusChip tone="warning" title={`Installed in ${skill.also_in + 1} places: ${skill.locations.join(", ")}`}>
            Also in {skill.also_in} {skill.also_in === 1 ? "place" : "places"}
          </StatusChip>
        )}
        <span className="grow" />
        {skill.malformed && <StatusChip tone="danger">Unreadable header</StatusChip>}
        <Tag>{SCOPE_LABEL[skill.scope]}</Tag>
      </span>
      <span className="skills-desc trunc">{skill.description || "No trigger description."}</span>
      <span className="skills-src trunc" title={skill.locations.join(", ")}>
        <FolderSimple size={14} aria-hidden="true" />
        <span className="trunc">{where}</span>
      </span>
    </button>
  );
});
