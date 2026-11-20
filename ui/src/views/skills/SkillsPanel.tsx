import { useMemo, useState, type ReactNode } from "react";
import { InfoIcon as Info, MagnifyingGlassIcon as MagnifyingGlass, SparkleIcon as Sparkle, WarningCircleIcon as WarningCircle } from "@phosphor-icons/react";
import { Banner, EmptyState, QuietPill } from "../../components";
import { Windowed } from "../memory/Windowed";
import type { Loaded } from "../memory/useRefreshing";
import type { SkillsView } from "./bridge";
import { SkillRow, matchesSkill, type ScopeFilter } from "./SkillRows";

const FILTERS: [ScopeFilter, string][] = [
  ["all", "All"],
  ["user", "User"],
  ["project", "Project"],
];

/** Rows shown before the "more skills" note in the compact panel. */
const COMPACT_ROWS = 8;
const ROW_HEIGHT = 75;

/**
 * The skills list as a panel: a scope filter and the rows. `compact` is the column on the
 * memory screen, which shows the first rows and a note pointing to the full Skills view;
 * otherwise the whole list is shown, windowed, and filtered by `query`.
 */
export function SkillsPanel({
  skills,
  selected,
  onOpen,
  compact,
  query = "",
  onMore,
  search,
}: {
  skills: Loaded<SkillsView>;
  selected: string | null;
  onOpen: (id: string) => void;
  compact: boolean;
  query?: string;
  onMore?: () => void;
  /** A find field shown under the header (the full view). */
  search?: ReactNode;
}) {
  const [scope, setScope] = useState<ScopeFilter>("all");
  const all = skills.data?.items;
  const rows = useMemo(() => (all ?? []).filter((s) => matchesSkill(s, scope, query)), [all, scope, query]);
  const shown = compact ? rows.slice(0, COMPACT_ROWS) : rows;
  const more = rows.length - shown.length;

  return (
    <section className={`panel skills-panel${compact ? " compact" : ""}`} aria-label="Skills">
      <div className="skills-head">
        <h2 className="skills-h2">Skills</h2>
        {all && <QuietPill>{all.length} installed</QuietPill>}
        <span className="grow" />
        <span className="seg" role="group" aria-label="Show skills from">
          {FILTERS.map(([id, label]) => (
            <button
              key={id}
              type="button"
              aria-pressed={scope === id}
              onClick={() => {
                setScope(id);
              }}
            >
              {label}
            </button>
          ))}
        </span>
      </div>
      {search}
      {skills.error && <Banner title={skills.error}>Skills could not be listed: {skills.error}</Banner>}
      {skills.data?.capped && <Banner>Some skill folders hold more skills than are listed.</Banner>}
      {!all ? (
        skills.error ? (
          <EmptyState icon={WarningCircle} title="No skills to show" dashed={false}>
            The skill folders could not be read.
          </EmptyState>
        ) : (
          <EmptyState icon={Sparkle} title="Finding skills" dashed={false}>
            Reading the skill folders.
          </EmptyState>
        )
      ) : rows.length === 0 ? (
        <EmptyState icon={MagnifyingGlass} title="No skills match" dashed={false}>
          {query ? "Try another word, or show all scopes." : "Nothing is installed in this scope."}
        </EmptyState>
      ) : compact ? (
        <div className="skills-list" role="list" aria-label="Installed skills">
          {shown.map((s) => (
            <div role="listitem" key={s.id}>
              <SkillRow skill={s} selected={s.id === selected} onOpen={onOpen} />
            </div>
          ))}
          {more > 0 && (
            <button type="button" className="endnote skills-more" onClick={onMore} disabled={!onMore}>
              <span className="zi">
                <MagnifyingGlass size={18} />
              </span>
              <span>
                <b>{more} more skills</b>
                <span className="t12">Open Skills (⌘7) to search them, or filter by User or Project.</span>
              </span>
            </button>
          )}
        </div>
      ) : (
        <Windowed
          className="skills-list"
          label="Installed skills"
          items={rows}
          rowHeight={ROW_HEIGHT}
          render={(s) => <SkillRow skill={s} selected={s.id === selected} onOpen={onOpen} />}
        />
      )}
      {compact && <span className="grow" />}
      <div className="memory-foot">
        <Info size={16} aria-hidden="true" />
        <span>Read-only. Skills are installed and switched outside the app.</span>
      </div>
    </section>
  );
}
