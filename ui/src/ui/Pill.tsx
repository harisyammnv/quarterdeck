import type { ReactNode } from "react";
import type { Icon } from "@phosphor-icons/react";

/** Status tones. Each status chip is one form: a soft fill, a dot (or icon) and a word. */
export type Tone = "success" | "warning" | "danger" | "info";

/**
 * A status chip: soft fill, a 6px dot and a word, or an icon in place of the dot.
 * Use it for states (Working, Blocked, Checks green), never for categories.
 */
export function StatusPill({ tone, icon: I, title, children }: { tone: Tone; icon?: Icon; title?: string; children: ReactNode }) {
  return (
    <span className={`pill ${tone}${I ? " has-icon" : ""}`} title={title}>
      {I && <I size={12} weight="bold" />}
      {children}
    </span>
  );
}

/** A category tag (PR, Report, User, ship): outlined and quiet, so it reads below status. */
export function Tag({ icon: I, title, children }: { icon?: Icon; title?: string; children: ReactNode }) {
  return (
    <span className="pill neutral" title={title}>
      {I && <I size={12} />}
      {children}
    </span>
  );
}

/** A quiet filled chip for a settled state such as Done. */
export function QuietPill({ icon: I, title, children }: { icon?: Icon; title?: string; children: ReactNode }) {
  return (
    <span className="pill accent" title={title}>
      {I && <I size={12} weight="bold" />}
      {children}
    </span>
  );
}

/** A small count badge, as in the project list and the lifted tabs. */
export function Count({ n, label }: { n: number; label?: string }) {
  return (
    <span className="count tn" aria-label={label}>
      {n}
    </span>
  );
}
