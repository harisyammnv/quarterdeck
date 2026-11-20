import { useEffect, useState } from "react";

export type Load<T> = { kind: "loading" } | { kind: "ready"; value: T } | { kind: "error"; message: string };

function errorText(e: unknown): string {
  const m = e instanceof Error ? e.message : String(e);
  return m.charAt(0).toUpperCase() + m.slice(1);
}

/**
 * Loads `fetch(taskId)` and loads it again whenever `reloadKey` changes, keeping the last
 * value on screen meanwhile. Mount the host with `key={taskId}` so another task starts from
 * the loading state.
 */
export function useLoad<T>(fetch: (taskId: string) => Promise<T>, taskId: string, reloadKey: string | null): Load<T> {
  const [load, setLoad] = useState<Load<T>>({ kind: "loading" });
  useEffect(() => {
    let live = true;
    fetch(taskId).then(
      (value) => {
        if (live) setLoad({ kind: "ready", value });
      },
      (e: unknown) => {
        if (live) setLoad({ kind: "error", message: errorText(e) });
      },
    );
    return () => {
      live = false;
    };
  }, [fetch, taskId, reloadKey]);
  return load;
}
