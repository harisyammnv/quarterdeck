/** Small formatting helpers for the board and decisions views. */

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"] as const;

/** A duration since `thenS` (Unix seconds) the way the mocks print it: "now", "42 s", "22 min", "1 h 40 min", "3 d". */
export function age(thenS: number | null, nowMs: number): string {
  if (thenS === null) return "";
  const s = Math.max(0, Math.round(nowMs / 1000 - thenS));
  if (s < 5) return "now";
  if (s < 60) return `${String(s)} s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${String(m)} min`;
  const h = Math.floor(m / 60);
  if (h < 24) {
    const rest = m % 60;
    return rest ? `${String(h)} h ${String(rest)} min` : `${String(h)} h`;
  }
  return `${String(Math.floor(h / 24))} d`;
}

/** "2 Dec" from "2026-12-02"; the input as written when it is not a plain date. */
export function shortDate(ymd: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(ymd.trim());
  if (!m) return ymd;
  const month = MONTHS[Number(m[2]) - 1];
  return month ? `${String(Number(m[3]))} ${month}` : ymd;
}

/** "#1207" from a pull request URL ending in a number, else null. */
export function prNumber(url: string | null): string | null {
  if (!url) return null;
  const m = /\/(\d+)\/?$/.exec(url);
  return m ? `#${m[1] ?? ""}` : null;
}

/** The first line of a decision's detail, for one-line summaries. */
export function firstLine(text: string | null): string {
  if (!text) return "";
  return text.split(/\n/).find((l) => l.trim() !== "")?.trim() ?? "";
}
