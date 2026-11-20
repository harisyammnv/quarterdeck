import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import {
  BrainIcon as Brain,
  CubeIcon as Cube,
  GearSixIcon as GearSix,
  HandPalmIcon as HandPalm,
  KanbanIcon as Kanban,
  SailboatIcon as Sailboat,
  ScalesIcon as Scales,
  ShieldCheckIcon as ShieldCheck,
  SparkleIcon as Sparkle,
  TerminalWindowIcon as TerminalWindow,
  type Icon,
} from "@phosphor-icons/react";
import type { FleetView } from "./bridge";
import { placeholder } from "./shell/Placeholder";
import ContainersStatus from "./views/containers/StatusItem";

type Lazy = ComponentType | LazyExoticComponent<ComponentType>;

/**
 * One view of the app. Each lives in `ui/src/views/<id>/` with `index.tsx` as its default
 * export; the shell draws the frame around it (title bar, rail, project list, inspector,
 * status bar) and the view fills the middle.
 */
export interface ViewDef {
  id: string;
  /** Rail label and command name, such as "Fleet". */
  label: string;
  /** One line on what is there, for the rail tooltip. */
  description: string;
  /** Title bar crumb when no task is in focus. */
  crumb: string;
  icon: Icon;
  /** A shortcut shown in the tooltip and command menu; rail views default to ⌘n by position. */
  shortcut?: string;
  /** On the rail. Rail position n is bound to ⌘n. A view off the rail (Needs your call) is
   * reached from another view or the command menu. */
  rail: boolean;
  component: Lazy;
  /** Which frames show around the view. */
  layout: {
    /** The 232px project and worktree list, shown by default (Fleet and Needs your call).
     * Elsewhere ⌘\\ or the title-bar toggle brings it back. */
    projects: boolean;
    /** The 292px inspector (Fleet). */
    inspector: boolean;
  };
  /** A count for the rail badge, from the live fleet view. */
  badge?: (view: FleetView | null) => number;
  /** A cheap, always-mounted item for the status bar, such as the Docker context. */
  statusItem?: ComponentType;
  /** False for views that read nothing from the home (Settings). Defaults to true. */
  needsHome?: boolean;
}

/** One section of the inspector, stacked by `order` under the task in focus. */
export interface SectionDef {
  id: string;
  order: number;
  component: Lazy;
}

/**
 * The views, in rail order (design system Rail: Fleet, Board, Security, Council, Containers,
 * Memory, Skills). Each area replaces its own line; a view not built yet shows the shell's
 * intentional placeholder. Security and Council arrive in Phases 2 and 3.
 */
export const views: ViewDef[] = [
  { id: "fleet", label: "Fleet", description: "Every task in flight, live", crumb: "Fleet", icon: Sailboat, rail: true, component: lazy(() => import("./views/fleet")), layout: { projects: true, inspector: true }, badge: (v) => v?.decisions ?? 0 },
  { id: "board", label: "Board", description: "Queued, in flight, waiting on you, done", crumb: "Board", icon: Kanban, rail: true, component: lazy(() => import("./views/board")), layout: { projects: false, inspector: false } },
  { id: "decisions", label: "Needs your call", description: "Every open decision, with its reason", crumb: "Needs your call", icon: HandPalm, rail: false, component: lazy(() => import("./views/decisions")), layout: { projects: true, inspector: false } },
  { id: "security", label: "Security", description: "Findings and dependency rollups", crumb: "Security", icon: ShieldCheck, rail: true, component: placeholder("Security", ShieldCheck, "Findings, dependency rollups and scanner jobs for every project, gathered in one place.", "Phase 2"), layout: { projects: false, inspector: false } },
  { id: "council", label: "Council", description: "Independent reviews of a pull request", crumb: "Council", icon: Scales, rail: true, component: placeholder("Council", Scales, "Independent reviewers read a pull request through separate lenses and agree on a verdict.", "Phase 3"), layout: { projects: false, inspector: false } },
  { id: "containers", label: "Containers", description: "Docker containers by worktree", crumb: "Containers", icon: Cube, rail: true, component: lazy(() => import("./views/containers")), layout: { projects: false, inspector: false }, statusItem: ContainersStatus },
  { id: "memory", label: "Memory", description: "Preferences, learnings and reports", crumb: "Memory and skills", icon: Brain, rail: true, component: placeholder("Memory", Brain, "Preferences, learnings, briefs and reports, readable and searchable. Firstmate keeps owning every edit."), layout: { projects: false, inspector: false } },
  { id: "skills", label: "Skills", description: "Every installed skill and where it comes from", crumb: "Skills", icon: Sparkle, rail: true, component: placeholder("Skills", Sparkle, "Every skill in the known skill folders, with where it comes from and where the same name appears twice."), layout: { projects: false, inspector: false } },
  { id: "terminal", label: "Terminal and report", description: "A task's terminal tail and report", crumb: "Task", icon: TerminalWindow, rail: false, component: lazy(() => import("./views/terminal")), layout: { projects: true, inspector: true } },
];

/** Settings, at the foot of the rail, bound to ⌘,. */
export const settingsView: ViewDef = { id: "settings", label: "Settings", description: "Firstmate home and appearance", crumb: "Settings", icon: GearSix, rail: false, component: lazy(() => import("./views/settings")), layout: { projects: false, inspector: false }, needsHome: false };

/** Inspector sections, one line per area, in `order`. */
export const inspectorSections: SectionDef[] = [
  { id: "containers", order: 40, component: lazy(() => import("./views/containers/Section")) },
  { id: "task", order: 10, component: lazy(() => import("./views/fleet/TaskSection")) },
  { id: "projects-worktree", order: 15, component: lazy(() => import("./views/projects/WorktreeSection")) },
  { id: "projects-security", order: 30, component: lazy(() => import("./views/projects/SecuritySection")) },
  { id: "projects-council", order: 50, component: lazy(() => import("./views/projects/CouncilSection")) },
  { id: "terminal", order: 20, component: lazy(() => import("./views/terminal/OpenSection")) },
  { id: "reports", order: 30, component: lazy(() => import("./views/reports/HistorySection")) },
];

/** Above the Fleet panel: the "Needs your call" room (the one teal room, Fleet only). */
export const fleetTop: Lazy | null = lazy(() => import("./views/decisions/Strip"));

/** The project and worktree list inside the left frame. `null` shows the frame's placeholder. */
export const sidebar: Lazy | null = lazy(() => import("./views/projects"));

export const railViews: ViewDef[] = views.filter((v) => v.rail);

export function viewById(id: string): ViewDef {
  return id === settingsView.id ? settingsView : (views.find((v) => v.id === id) ?? (views[0] as ViewDef));
}
