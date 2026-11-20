import { BinocularsIcon as Binoculars, CircleDashedIcon as CircleDashed, SailboatIcon as Sailboat, UsersThreeIcon as UsersThree } from "@phosphor-icons/react";
import { Tag } from "./StatusChip";

/** The task kind as a quiet tag with its glyph: ship, scout or second mate. */
export function KindPill({ kind }: { kind: string | null | undefined }) {
  const k = kind ?? "task";
  const I = k === "ship" ? Sailboat : k === "scout" ? Binoculars : k === "secondmate" ? UsersThree : CircleDashed;
  return <Tag icon={I}>{k === "secondmate" ? "second mate" : k}</Tag>;
}
