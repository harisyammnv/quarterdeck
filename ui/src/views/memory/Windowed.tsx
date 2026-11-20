import { useLayoutEffect, useRef, useState, type ReactNode } from "react";

/**
 * A fixed-row-height windowed list: only the rows in view (plus a few either side) are
 * mounted, so a list of thousands of briefs or skills costs a screenful of nodes.
 */
export function Windowed<T>({
  items,
  rowHeight,
  render,
  className,
  label,
  overscan = 6,
}: {
  items: readonly T[];
  rowHeight: number;
  render: (item: T, index: number) => ReactNode;
  className?: string;
  label: string;
  overscan?: number;
}) {
  const box = useRef<HTMLDivElement>(null);
  const [scroll, setScroll] = useState(0);
  const [height, setHeight] = useState(600);

  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    setHeight(el.clientHeight);
    const ro = new ResizeObserver(() => {
      setHeight(el.clientHeight);
    });
    ro.observe(el);
    return () => {
      ro.disconnect();
    };
  }, []);

  const first = Math.max(0, Math.floor(scroll / rowHeight) - overscan);
  const last = Math.min(items.length, Math.ceil((scroll + height) / rowHeight) + overscan);
  return (
    <div
      ref={box}
      className={`memory-windowed${className ? ` ${className}` : ""}`}
      role="list"
      aria-label={label}
      onScroll={(e) => {
        setScroll(e.currentTarget.scrollTop);
      }}
    >
      <div style={{ height: items.length * rowHeight, position: "relative" }}>
        {items.slice(first, last).map((item, i) => (
          <div key={first + i} role="listitem" style={{ position: "absolute", top: (first + i) * rowHeight, left: 0, right: 0, height: rowHeight }}>
            {render(item, first + i)}
          </div>
        ))}
      </div>
    </div>
  );
}
