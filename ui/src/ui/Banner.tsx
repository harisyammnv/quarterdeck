import type { ReactNode } from "react";
import { WarningCircleIcon as WarningCircle } from "@phosphor-icons/react";

/** A one-line warning inside a panel, for a degraded read (snapshot failed, list incomplete). */
export function Banner({ children, title }: { children: ReactNode; title?: string }) {
  return (
    <div className="banner" role="status">
      <WarningCircle size={16} />
      <span className="trunc grow" title={title}>
        {children}
      </span>
    </div>
  );
}
