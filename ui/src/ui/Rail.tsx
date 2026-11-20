import type { Icon } from "@phosphor-icons/react";
import { BrandMark } from "./BrandMark";
import { Tooltip } from "./Tooltip";

export interface RailEntry {
  id: string;
  label: string;
  icon: Icon;
  shortcut: string;
  /** One line on what is there, for the tooltip. */
  description?: string;
  badge?: number;
}

function RailItem({ entry, current, onSelect }: { entry: RailEntry; current: boolean; onSelect: (id: string) => void }) {
  const I = entry.icon;
  const badge = entry.badge ?? 0;
  return (
    <Tooltip label={entry.label} shortcut={entry.shortcut} description={entry.description} side="right">
      <button
        type="button"
        className="qd-rail__item ui-rail__item"
        aria-current={current ? "page" : undefined}
        aria-label={`${entry.label}${badge ? `, ${String(badge)} need you` : ""}, ${entry.shortcut}`}
        onClick={() => {
          onSelect(entry.id);
        }}
      >
        <span className="qd-rail__icon">
          <I size={18} weight={current ? "duotone" : "regular"} />
          {badge > 0 && <span className="qd-rail__badge">{badge}</span>}
        </span>
        <span className="qd-rail__label">{entry.label}</span>
      </button>
    </Tooltip>
  );
}

/**
 * Rail (design system): the black 68px rail, `qd-rail`. The white Raised deck tile on top,
 * the areas in order with their names under the icons, and `footer` items (Settings) pinned
 * to the bottom. The selected item uses the duotone glyph.
 */
export function Rail({ items, footer, current, onSelect }: { items: RailEntry[]; footer: RailEntry[]; current: string; onSelect: (id: string) => void }) {
  return (
    <nav className="qd-rail ui-rail" aria-label="Areas">
      <span className="qd-rail__brand" aria-hidden="true">
        <BrandMark size={26} />
      </span>
      {items.map((e) => (
        <RailItem key={e.id} entry={e} current={e.id === current} onSelect={onSelect} />
      ))}
      <span className="qd-rail__sp" />
      {footer.map((e) => (
        <RailItem key={e.id} entry={e} current={e.id === current} onSelect={onSelect} />
      ))}
    </nav>
  );
}
