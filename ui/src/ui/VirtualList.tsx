import { useEffect, useRef, useState, type Key, type ReactNode } from "react";

/**
 * A windowed list for anything that can pass 100 rows: only the rows in view (plus
 * `overscan`) are mounted. Rows have one fixed height, which keeps it simple and exact.
 */
export function VirtualList<T>({
  items,
  rowHeight,
  rowKey,
  render,
  overscan = 6,
  label,
  className,
}: {
  items: readonly T[];
  rowHeight: number;
  rowKey: (item: T) => Key;
  render: (item: T, index: number) => ReactNode;
  overscan?: number;
  label: string;
  className?: string;
}) {
  const box = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(600);

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      setHeight(el.clientHeight);
    });
    ro.observe(el);
    return () => {
      ro.disconnect();
    };
  }, []);

  const first = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
  const last = Math.min(items.length, Math.ceil((scrollTop + height) / rowHeight) + overscan);
  const shown = items.slice(first, last);

  return (
    <div
      ref={box}
      className={`vlist${className ? ` ${className}` : ""}`}
      role="list"
      aria-label={label}
      onScroll={(e) => {
        setScrollTop(e.currentTarget.scrollTop);
      }}
    >
      <div style={{ height: items.length * rowHeight, position: "relative" }}>
        {shown.map((item, i) => (
          <div key={rowKey(item)} role="listitem" style={{ position: "absolute", top: (first + i) * rowHeight, left: 0, right: 0, height: rowHeight }}>
            {render(item, first + i)}
          </div>
        ))}
      </div>
    </div>
  );
}
