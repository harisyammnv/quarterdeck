import type { Icon } from "@phosphor-icons/react";

export interface Segment<T extends string> {
  id: T;
  label: string;
  icon?: Icon;
  count?: number;
}

/**
 * The A2 lifted segment: a sunken track with the selected segment raised and bold.
 * Used for tabs (Terminal, Changes, Report) and filters (All, User, Project).
 */
export function Tabs<T extends string>({ items, value, onChange, label }: { items: Segment<T>[]; value: T; onChange: (id: T) => void; label: string }) {
  return (
    <div className="segtabs" role="tablist" aria-label={label}>
      {items.map((s) => {
        const on = s.id === value;
        const I = s.icon;
        return (
          <button
            key={s.id}
            type="button"
            role="tab"
            aria-selected={on}
            className={`tab${on ? " on" : ""}`}
            onClick={() => {
              onChange(s.id);
            }}
          >
            {I && <I size={15} weight={on ? "bold" : "regular"} />}
            {s.label}
            {s.count !== undefined && <span className="count tn">{s.count}</span>}
          </button>
        );
      })}
    </div>
  );
}
