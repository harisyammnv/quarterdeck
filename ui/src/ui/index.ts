// Shared UI primitives: thin React wrappers over the design system's qd- classes
// (ui/src/design-system). Areas compose these; they do not restyle or fork them.
export { StatusChip, StatusChip as Pill, Tag, Count, type Tone } from "./StatusChip";
export { StateDot, type DotKind } from "./StateDot";
export { StatePill, stateDot } from "./StatePill";
export { KindPill } from "./KindPill";
export { Keycap, Keycap as Kbd } from "./Keycap";
export { Tooltip } from "./Tooltip";
export { Button, IconButton } from "./Button";
export { SegmentedTabs, SegmentedTabs as Tabs, type Segment } from "./SegmentedTabs";
export { DataTable, type Column } from "./DataTable";
export { ZeroState, ZeroState as Zero } from "./ZeroState";
export { Banner } from "./Banner";
export { Panel, PanelHead, Facts } from "./Panel";
export { InspectorSection } from "./InspectorSection";
export { ProjectTile } from "./ProjectTile";
export { BrandMark } from "./BrandMark";
export { Rail, type RailEntry } from "./Rail";
export { CommandBar } from "./CommandBar";
export { NoteField } from "./NoteField";
export { Composer } from "./Composer";
export { VirtualList } from "./VirtualList";
export { Markdown } from "./Markdown";
export { useHotkey, type Combo } from "./hotkeys";
export { useNow } from "./time";
export { STATE_LOOK, ago, monogram, tileFill } from "../states";
