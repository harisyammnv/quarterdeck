/** A keycap: DM Sans 11px with a heavier bottom edge. Inside a primary button it takes the label colour. */
export function Kbd({ children, label }: { children: string; label?: string }) {
  return (
    <span className="kbd" aria-label={label} aria-hidden={label ? undefined : true}>
      {children}
    </span>
  );
}
