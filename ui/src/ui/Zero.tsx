import type { ReactNode } from "react";
import type { Icon } from "@phosphor-icons/react";

/**
 * An intentional empty or end-of-list state: an icon disc, a title and one line of text.
 * `dashed` draws the end-of-list card ("That is everything for now").
 */
export function Zero({ icon: I, title, children, dashed = true, action }: { icon: Icon; title: string; children?: ReactNode; dashed?: boolean; action?: ReactNode }) {
  return (
    <div className={dashed ? "endnote" : "zero"}>
      <span className="zi">
        <I size={18} />
      </span>
      <span className="grow">
        <b>{title}</b>
        {children && <span className="mut">{children}</span>}
      </span>
      {action}
    </div>
  );
}
