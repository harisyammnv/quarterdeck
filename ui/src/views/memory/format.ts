/** "Today", "Yesterday" or "N d", as in the memory list; a date past two months. */
export function shortAge(seconds: number | null, now: number): string {
  if (seconds === null) return "";
  const then = new Date(seconds * 1000);
  const today = new Date(now);
  const startOf = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((startOf(today) - startOf(then)) / 86_400_000);
  if (days <= 0) {
    const hours = Math.floor((now - seconds * 1000) / 3_600_000);
    return hours >= 1 ? `${hours} h` : "Today";
  }
  if (days < 60) return `${days} d`;
  return then.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

export function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}
