import { useCallback, useEffect, useState } from "react";

export type Loaded<T> = { data: T | null; error: string | null; loading: boolean };

/**
 * Loads `fetch` on mount, again whenever the window regains focus, `reload` is called or
 * `key` changes (the home was switched, or its first snapshot arrived). Memory and skills change rarely, so they refresh on focus instead of being
 * watched. A slower response never overwrites a newer one.
 */
export function useRefreshing<T>(fetch: () => Promise<T>, key = ""): [Loaded<T>, () => void] {
  const [state, setState] = useState<Loaded<T>>({ data: null, error: null, loading: true });
  const [tick, setTick] = useState(0);
  const reload = useCallback(() => {
    setState((s) => ({ ...s, loading: true }));
    setTick((t) => t + 1);
  }, []);

  useEffect(() => {
    let live = true;
    fetch()
      .then((data) => {
        if (live) setState({ data, error: null, loading: false });
      })
      .catch((e: unknown) => {
        if (live) setState((s) => ({ data: s.data, error: e instanceof Error ? e.message : String(e), loading: false }));
      });
    return () => {
      live = false;
    };
  }, [fetch, tick, key]);

  useEffect(() => {
    window.addEventListener("focus", reload);
    return () => {
      window.removeEventListener("focus", reload);
    };
  }, [reload]);

  return [state, reload];
}
