/**
 * The age shown at the end of a memory row: "Today" (or "N h" with `hours`), "N d", or a
 * date past two months.
 */
export function shortAge(seconds: number | null, now: number, hours = false): string {
  if (seconds === null) return "";
  const then = new Date(seconds * 1000);
  const today = new Date(now);
  const startOf = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((startOf(today) - startOf(then)) / 86_400_000);
  if (days <= 0) {
    const h = Math.floor((now - seconds * 1000) / 3_600_000);
    return hours && h >= 1 ? `${h} h` : "Today";
  }
  if (days < 60) return `${days} d`;
  return then.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

export function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}
