import { useMemo, useState, type ReactNode } from "react";
import { ArrowsClockwiseIcon as ArrowsClockwise, InfoIcon as Info, MagnifyingGlassIcon as MagnifyingGlass, SparkleIcon as Sparkle, WarningCircleIcon as WarningCircle } from "@phosphor-icons/react";
import { Banner, IconButton, SegmentedTabs, VirtualList, ZeroState, type Segment } from "../../ui";
import type { Loaded } from "../memory/useRefreshing";
import type { SkillsView } from "./bridge";
import { SkillRow, matchesSkill, type ScopeFilter } from "./SkillRows";

/** The memory screen's column keeps the mock's three filters; the full view adds Firstmate. */
const FILTERS: Segment<ScopeFilter>[] = [
  { id: "all", label: "All" },
  { id: "user", label: "User" },
  { id: "project", label: "Project" },
];
const FULL_FILTERS: Segment<ScopeFilter>[] = [
  { id: "all", label: "All" },
  { id: "user", label: "User" },
  { id: "firstmate", label: "Firstmate" },
  { id: "project", label: "Project" },
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
  onRefresh,
}: {
  skills: Loaded<SkillsView>;
  selected: string | null;
  onOpen: (id: string) => void;
  compact: boolean;
  query?: string;
  onMore?: () => void;
  /** A find field shown under the header (the full view). */
  search?: ReactNode;
  /** Shows a refresh button in the header (the full view). */
  onRefresh?: () => void;
}) {
  const [scope, setScope] = useState<ScopeFilter>("all");
  const all = skills.data?.items;
  const rows = useMemo(() => (all ?? []).filter((s) => matchesSkill(s, scope, query)), [all, scope, query]);
  const shown = compact ? rows.slice(0, COMPACT_ROWS) : rows;
  const more = rows.length - shown.length;

  return (
    <section className={`qd-panel ui-panel skills-panel${compact ? " compact" : ""}`} aria-label="Skills">
      <div className="skills-head">
        <h2 className="skills-h2">Skills</h2>
        {all && <span className="qd-count">{all.length} installed</span>}
        <span className="grow" />
        {compact && <SegmentedTabs items={FILTERS} value={scope} onChange={setScope} label="Show skills from" />}
        {onRefresh && <IconButton icon={ArrowsClockwise} label="Read the skill folders again" onClick={onRefresh} disabled={skills.loading} />}
      </div>
      {search}
      {!compact && (
        <div className="skills-filters">
          <SegmentedTabs items={FULL_FILTERS} value={scope} onChange={setScope} label="Show skills from" />
        </div>
      )}
      {skills.error && <Banner title={skills.error}>Skills could not be listed: {skills.error}</Banner>}
      {skills.data?.capped && <Banner>Some skill folders hold more skills than are listed.</Banner>}
      {!all ? (
        skills.error ? (
          <ZeroState icon={WarningCircle} title="No skills to show">
            The skill folders could not be read.
          </ZeroState>
        ) : (
          <ZeroState icon={Sparkle} title="Finding skills">
            Reading the skill folders.
          </ZeroState>
        )
      ) : rows.length === 0 ? (
        <ZeroState icon={MagnifyingGlass} title="No skills match">
          {query ? "Try another word, or show all scopes." : "Nothing is installed in this scope."}
        </ZeroState>
      ) : compact ? (
        <div className="skills-list" role="list" aria-label="Installed skills">
          {shown.map((s) => (
            <div role="listitem" key={s.id}>
              <SkillRow skill={s} selected={s.id === selected} onOpen={onOpen} />
            </div>
          ))}
          {more > 0 && (
            <button type="button" className="qd-endnote ui-endnote skills-more" onClick={onMore} disabled={!onMore}>
              <span className="ui-endnote__i">
                <MagnifyingGlass size={18} />
              </span>
              <span>
                <b>{more} more skills</b>
                <span className="t12">Press ⌘7 to search them, or filter by User or Project.</span>
              </span>
            </button>
          )}
        </div>
      ) : (
        <VirtualList
          className="skills-list"
          label="Installed skills"
          items={rows}
          rowHeight={ROW_HEIGHT}
          rowKey={(s) => s.id}
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
