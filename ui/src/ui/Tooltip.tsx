import type { ReactNode } from "react";
import { Keycap } from "./Keycap";

/**
 * Tooltip (design system): black in both themes, a bold name with its keycap and an optional
 * one-line description, `qd-tip`. Shown after a 500 ms hover, or at once on keyboard focus.
 * It never holds the only copy of important information, so it is hidden from screen readers;
 * the control it wraps carries its own label.
 */
export function Tooltip({
  label,
  shortcut,
  description,
  side = "bottom",
  children,
}: {
  label: string;
  shortcut?: string;
  description?: string;
  side?: "right" | "bottom";
  children: ReactNode;
}) {
  return (
    <span className={`ui-tipwrap ui-tipwrap--${side}`}>
      {children}
      <span className="qd-tip ui-tip" aria-hidden="true">
        <b>
          {label}
          {shortcut && <Keycap>{shortcut}</Keycap>}
        </b>
        {description && <span>{description}</span>}
      </span>
    </span>
  );
}
