import type { Key, ReactNode } from "react";

export interface Column<R> {
  key: string;
  header: ReactNode;
  /** CSS width of the column, such as "36%" or "96px". */
  width?: string;
  align?: "left" | "right";
  cell: (row: R) => ReactNode;
}

/**
 * The A2 table: 30px header, 36px rows, hairline separators, a sunken hover and a tonal
 * selected row with a 3px ink bar. Rows are buttons for the keyboard when `onSelect` is set.
 */
export function DataTable<R>({
  columns,
  rows,
  rowKey,
  selected,
  onSelect,
  rowClass,
  label,
}: {
  columns: Column<R>[];
  rows: R[];
  rowKey: (row: R) => Key;
  selected?: Key | null;
  onSelect?: (row: R) => void;
  rowClass?: (row: R) => string | undefined;
  label: string;
}) {
  return (
    <table className="dtable" aria-label={label}>
      <colgroup>
        {columns.map((c) => (
          <col key={c.key} style={c.width ? { width: c.width } : undefined} />
        ))}
      </colgroup>
      <thead>
        <tr>
          {columns.map((c) => (
            <th key={c.key} className={c.align === "right" ? "num" : undefined}>
              {c.header}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.map((r) => {
          const k = rowKey(r);
          const cls = [k === selected ? "sel" : "", onSelect ? "clickable" : "", rowClass?.(r) ?? ""].filter(Boolean).join(" ");
          return (
            <tr
              key={k}
              className={cls || undefined}
              aria-selected={onSelect ? k === selected : undefined}
              tabIndex={onSelect ? 0 : undefined}
              onClick={
                onSelect
                  ? () => {
                      onSelect(r);
                    }
                  : undefined
              }
              onKeyDown={
                onSelect
                  ? (e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        onSelect(r);
                      }
                    }
                  : undefined
              }
            >
              {columns.map((c) => (
                <td key={c.key} className={c.align === "right" ? "num" : undefined}>
                  {c.cell(r)}
                </td>
              ))}
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
