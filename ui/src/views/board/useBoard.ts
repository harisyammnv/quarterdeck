import { useEffect, useState } from "react";
import { boardView, onBoardUpdated, type BoardView } from "./bridge";

export type BoardLoad =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; view: BoardView };

/** The live board: one fetch on mount, then the board-updated event. */
export function useBoard(): BoardLoad {
  const [load, setLoad] = useState<BoardLoad>({ status: "loading" });
  useEffect(() => {
    let alive = true;
    const off = onBoardUpdated((view) => {
      if (alive) setLoad({ status: "ready", view });
    });
    boardView().then(
      (view) => {
        if (alive) setLoad((cur) => (cur.status === "ready" ? cur : { status: "ready", view }));
      },
      (e: unknown) => {
        if (alive) setLoad({ status: "error", message: e instanceof Error ? e.message : String(e) });
      },
    );
    return () => {
      alive = false;
      off();
    };
  }, []);
  return load;
}

/** The current time in ms, refreshed every 30 s so relative ages stay honest. */
export function useNow(): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = setInterval(() => {
      setNow(Date.now());
    }, 30_000);
    return () => {
      clearInterval(t);
    };
  }, []);
  return now;
}
