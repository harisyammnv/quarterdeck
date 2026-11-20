import type { ReactNode } from "react";
import type { Icon } from "@phosphor-icons/react";

/** Status tones. The word carries the meaning; the colour only repeats it. */
export type Tone = "success" | "warning" | "danger" | "info";

/** StatusChip (design system): soft fill, a 6px dot and a word, `qd-chip--<tone>`. States only. */
export function StatusChip({ tone, title, children }: { tone: Tone; title?: string; children: ReactNode }) {
  return (
    <span className={`qd-chip qd-chip--${tone}`} title={title}>
      {children}
    </span>
  );
}

/** Tag (design system): what kind of thing it is (PR, Report, ship), outlined and quiet, `qd-tag`. */
export function Tag({ icon: I, title, children }: { icon?: Icon; title?: string; children: ReactNode }) {
  return (
    <span className={`qd-tag${I ? " ui-tag-icon" : ""}`} title={title}>
      {I && <I size={12} />}
      {children}
    </span>
  );
}

/** Count (design system): a sunken pill with a tabular number, `qd-count`. */
export function Count({ n, label }: { n: number; label?: string }) {
  return (
    <span className="qd-count" aria-label={label}>
      {n}
    </span>
  );
}
