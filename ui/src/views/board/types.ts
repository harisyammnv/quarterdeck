/**
 * Wire types of the board projection (core/src/board). They mirror the Rust structs until the
 * generated bindings carry them; then this file re-exports from bindings.
 */
import type { TaskState } from "../../bindings";

export type Column = "queued" | "in-flight" | "done";
export type HoldBucket = "live" | "dated" | "blocked" | "aged";
export type StateSource = "snapshot" | "event" | "bootstrap";

export interface LiveState {
  state: TaskState;
  word: string;
  source: StateSource;
  latest: string | null;
  latest_at: number | null;
}

export interface BoardCard {
  id: string;
  title: string;
  column: Column;
  project: string | null;
  kind: string | null;
  since: number | null;
  hold: HoldBucket | null;
  until: string | null;
  blocked_by: string[];
  live: LiveState | null;
  pr_url: string | null;
  has_report: boolean;
}

export interface Decision {
  id: string;
  question: string;
  detail: string | null;
  project: string | null;
  kind: string | null;
  bucket: HoldBucket;
  until: string | null;
  age_days: number | null;
  blocked_by: string[];
  raised_at: number | null;
  pr_url: string | null;
  has_report: boolean;
  links: string[];
}

export interface Decisions {
  live: Decision[];
  deferred: Decision[];
}

export interface Unplaced {
  id: string;
  why: string;
}

export interface BoardView {
  ready: boolean;
  generated_at: number | null;
  queued: BoardCard[];
  in_flight: BoardCard[];
  done: BoardCard[];
  done_total: number;
  decisions: Decisions;
  other: Unplaced[];
}
