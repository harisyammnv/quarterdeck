import type { ReactNode } from "react";
import type { Icon } from "@phosphor-icons/react";

/**
 * ZeroState (design system): every list and panel draws its empty state (`qd-zero`: a 32px
 * disc with the area's icon, what is true, and when it was checked) and its end-of-list note
 * (`end`: a dashed `qd-endnote` box saying what is deliberately left out). Facts, not cheer.
 */
export function ZeroState({ icon: I, title, children, end = false, action }: { icon: Icon; title: string; children?: ReactNode; end?: boolean; action?: ReactNode }) {
  return (
    <div className={end ? "qd-endnote ui-endnote" : "qd-zero"}>
      <span className={end ? "ui-endnote__i" : "qd-zero__i"}>
        <I size={end ? 18 : 16} />
      </span>
      <span className="ui-zero__text">
        <b>{title}</b>
        {children && <span>{children}</span>}
      </span>
      {action}
    </div>
  );
}
