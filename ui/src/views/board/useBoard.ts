import { useEffect, useState } from "react";
import { boardView, onBoardUpdated, type BoardView } from "./bridge";

export type BoardLoad =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "no-home" }
  | { status: "ready"; view: BoardView };

/** The live board: one fetch on mount, then `board-updated`, which wins over a late fetch. */
export function useBoard(): BoardLoad {
  const [load, setLoad] = useState<BoardLoad>({ status: "loading" });
  useEffect(() => {
    let alive = true;
    const off = onBoardUpdated((view) => {
      if (alive) setLoad({ status: "ready", view });
    });
    boardView().then(
      (view) => {
        if (!alive) return;
        setLoad((cur) => (cur.status === "ready" ? cur : view ? { status: "ready", view } : { status: "no-home" }));
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
