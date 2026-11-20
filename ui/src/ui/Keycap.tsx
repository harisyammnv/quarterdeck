/**
 * Keycap (design system): a shortcut next to the thing it triggers, `qd-kbd`. One shortcut
 * per keycap, with the Mac glyphs. Inside a primary button or the room it takes their colours.
 */
export function Keycap({ children, label }: { children: string; label?: string }) {
  return (
    <span className="qd-kbd" aria-label={label} aria-hidden={label ? undefined : true}>
      {children}
    </span>
  );
}
