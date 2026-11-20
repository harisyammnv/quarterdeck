/**
 * The terminal view's door to the core. In the app window it calls the generated command;
 * in a plain browser it serves invented preview data.
 */
import { inApp } from "../../bridge";
import { previewTail } from "./preview";

export type Tone = "default" | "dim" | "red" | "green" | "yellow" | "blue" | "magenta" | "cyan";

export interface Span {
  text: string;
  tone: Tone;
  bold: boolean;
}

export interface TerminalTail {
  task_id: string;
  lines: Span[][];
  requested: number;
  truncated: boolean;
  digest: string;
  captured_at: number;
}

/** Reads the last lines of a task's terminal; rejects with a readable message. */
export function terminalTail(taskId: string): Promise<TerminalTail> {
  if (inApp) return Promise.reject(new Error("The terminal tail is not wired into this build yet."));
  return previewTail(taskId);
}
