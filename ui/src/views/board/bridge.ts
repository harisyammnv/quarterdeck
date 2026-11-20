/**
 * The board area's door to the core. In the app window it calls the typed board command and
 * listens to `board-updated`; in a plain browser it serves invented preview data so the
 * board and the decisions views can be laid out and screenshotted headlessly.
 */
import { commands, events } from "../../bindings";
import { inApp, taskPrOpen } from "../../bridge";
import { previewBoard } from "./preview";
import type { BoardView } from "./types";

export type { BoardCard, BoardLiveState, BoardView, Decision, HoldBucket } from "./types";

/** The current board, or null when no home is configured. */
export function boardView(): Promise<BoardView | null> {
  return inApp ? commands.boardView() : Promise.resolve(previewBoard());
}

/** Subscribes to board updates (sent only when the board changed); returns an unsubscribe function. */
export function onBoardUpdated(cb: (view: BoardView) => void): () => void {
  if (!inApp) return () => undefined;
  const pending = events.boardUpdated.listen((e) => {
    cb(e.payload);
  });
  return () => {
    void pending.then((unlisten) => {
      unlisten();
    });
  };
}

/** Opens a task's recorded pull request. The core looks the address up by id. */
export function openPullRequest(taskId: string): Promise<void> {
  return taskPrOpen(taskId);
}
