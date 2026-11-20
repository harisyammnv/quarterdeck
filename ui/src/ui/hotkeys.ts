import { useEffect, useRef } from "react";

/** Keys that are combos with ⌘ (Meta), such as "mod+k" or "mod+1"; plain keys such as "j" or "/". */
export type Combo = string;

function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}

function matches(e: KeyboardEvent, combo: Combo): boolean {
  const parts = combo.toLowerCase().split("+");
  const key = parts.pop() ?? "";
  const mod = parts.includes("mod");
  const shift = parts.includes("shift");
  const alt = parts.includes("alt");
  if (mod !== e.metaKey || shift !== e.shiftKey || alt !== e.altKey || e.ctrlKey) return false;
  return e.key.toLowerCase() === key || e.code.toLowerCase() === `key${key}` || e.code.toLowerCase() === `digit${key}`;
}

/**
 * Binds one shortcut while `enabled`. Plain keys (no ⌘) are ignored while the user types in a
 * field, so J, K and / never steal text; ⌘ combos always fire.
 */
export function useHotkey(combo: Combo, handler: (e: KeyboardEvent) => void, enabled = true): void {
  const ref = useRef(handler);
  useEffect(() => {
    ref.current = handler;
  });
  useEffect(() => {
    if (!enabled) return;
    const plain = !combo.toLowerCase().includes("mod+");
    const onKey = (e: KeyboardEvent) => {
      if (plain && isTyping(e.target)) return;
      if (!matches(e, combo)) return;
      e.preventDefault();
      ref.current(e);
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [combo, enabled]);
}
