import type { ReactNode } from "react";

/** A floating panel (design system `qd-panel`): white or dark panel fill, 16px radius, hairline border. */
export function Panel({ children, className, label, as = "section" }: { children: ReactNode; className?: string; label?: string; as?: "section" | "main" | "aside" }) {
  const Tag = as;
  return (
    <Tag className={`qd-panel ui-panel${className ? ` ${className}` : ""}`} aria-label={label}>
      {children}
    </Tag>
  );
}

/** A 48px pane header: a 16px bold title, optional meta, and actions pushed to the right. */
export function PanelHead({ title, meta, children }: { title: ReactNode; meta?: ReactNode; children?: ReactNode }) {
  return (
    <div className="phead">
      <h1 className="ptitle">{title}</h1>
      {meta !== undefined && <span className="meta tn">{meta}</span>}
      <span className="grow" />
      {children}
    </div>
  );
}

/** A labelled key and value list, as in the inspector's Task section. */
export function Facts({ items }: { items: [string, ReactNode][] }) {
  return (
    <dl className="qd-kv">
      {items.map(([k, v]) => (
        <div key={k} className="ui-kvrow">
          <dt>{k}</dt>
          <dd className="qd-trunc">{v}</dd>
        </div>
      ))}
    </dl>
  );
}
