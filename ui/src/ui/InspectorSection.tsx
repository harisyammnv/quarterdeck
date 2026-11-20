import type { ReactNode } from "react";

/**
 * InspectorSection (design system): one section of the 292px inspector, a section-title
 * heading with an optional tag or link on the right, then content; sections are divided by
 * `line` rules. Key-value lists inside use `Facts` (`qd-kv`).
 */
export function InspectorSection({ title, aside, children }: { title: string; aside?: ReactNode; children: ReactNode }) {
  return (
    <section className="ui-insp-sect" aria-label={title}>
      <div className="qd-row">
        <h2 className="ui-section-title qd-trunc">{title}</h2>
        {aside && <span className="ui-insp-aside">{aside}</span>}
      </div>
      {children}
    </section>
  );
}
