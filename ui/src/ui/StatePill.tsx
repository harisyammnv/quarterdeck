import { CheckIcon as Check, PauseIcon as Pause, QuestionIcon as Question } from "@phosphor-icons/react";
import type { TaskState } from "../bindings";
import { STATE_LOOK } from "../states";
import type { DotKind } from "./StateDot";
import { StatusChip, Tag } from "./StatusChip";

/** The state dot for a task state, or null for settled states. */
export function stateDot(state: TaskState): DotKind | null {
  return STATE_LOOK[state].dot;
}

/**
 * The chip for a task state: a status chip for live states, a quiet tag with a glyph for
 * settled or unknown ones. `word` overrides the label so an unknown word stays honest.
 */
export function StatePill({ state, word, title }: { state: TaskState; word?: string; title?: string }) {
  const look = STATE_LOOK[state];
  const label = word ?? look.word;
  if (look.tone === "quiet" || look.tone === "neutral") {
    const I = state === "done" ? Check : state === "paused" ? Pause : Question;
    return (
      <Tag icon={I} title={title}>
        {label}
      </Tag>
    );
  }
  return (
    <StatusChip tone={look.tone} title={title}>
      {label}
    </StatusChip>
  );
}
