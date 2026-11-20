import { monogram, tileFill } from "../states";

/**
 * A project tile. `src` is a project icon already resolved by the core (a `data:` URL, the
 * only image source the CSP allows); without one, a monogram with a colour derived from the
 * name, the last step of the design's icon resolution order.
 */
export function ProjectTile({ name, src, size = 20 }: { name: string; src?: string | null; size?: number }) {
  const style = { width: size, height: size };
  if (src) {
    return <img className="qd-tile ui-tile-img" src={src} alt="" aria-hidden="true" style={style} draggable={false} />;
  }
  return (
    <span className="qd-tile" aria-hidden="true" style={{ ...style, background: tileFill(name), fontSize: Math.round(size / 2) }}>
      {monogram(name)}
    </span>
  );
}
