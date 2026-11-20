import type { Icon } from "@phosphor-icons/react";

export interface Segment<T extends string> {
  id: T;
  label: string;
  icon?: Icon;
  count?: number;
}

/**
 * SegmentedTabs (design system): a sunken track with the selected segment lifted and bold,
 * `qd-seg`. For panel tabs, filters and scope switches; never solid black.
 */
export function SegmentedTabs<T extends string>({ items, value, onChange, label }: { items: Segment<T>[]; value: T; onChange: (id: T) => void; label: string }) {
  return (
    <div className="qd-seg" role="tablist" aria-label={label}>
      {items.map((s) => {
        const on = s.id === value;
        const I = s.icon;
        return (
          <button
            key={s.id}
            type="button"
            role="tab"
            aria-selected={on}
            className="qd-seg__tab"
            onClick={() => {
              onChange(s.id);
            }}
          >
            {I && <I size={14} weight={on ? "bold" : "regular"} />}
            {s.label}
            {s.count !== undefined && <span className="qd-seg__n">{s.count}</span>}
          </button>
        );
      })}
    </div>
  );
}
