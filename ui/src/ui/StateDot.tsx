/** The five state dots of the design system: a 7px dot with a soft halo, `qd-dot--<kind>`. */
export type DotKind = "working" | "waiting" | "blocked" | "investigating" | "idle";

/** Always pair a dot with a word somewhere on screen (row text, legend or tooltip). */
export function StateDot({ kind, label }: { kind: DotKind; label?: string }) {
  return <span className={`qd-dot qd-dot--${kind}`} role={label ? "img" : undefined} aria-label={label} aria-hidden={label ? undefined : true} />;
}
