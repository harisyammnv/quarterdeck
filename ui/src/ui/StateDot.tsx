/** The five state dots of the design: a 7px dot with a soft halo in its status colour. */
export type DotKind = "working" | "waiting" | "blocked" | "invest" | "idle";

export function StateDot({ kind, label }: { kind: DotKind; label?: string }) {
  return <span className={`dot ${kind}`} role={label ? "img" : undefined} aria-label={label} aria-hidden={label ? undefined : true} />;
}
