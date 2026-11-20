import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import {
  BrainIcon as Brain,
  CubeIcon as Cube,
  GearSixIcon as GearSix,
  KanbanIcon as Kanban,
  SailboatIcon as Sailboat,
  SparkleIcon as Sparkle,
  type Icon,
} from "@phosphor-icons/react";
import type { FleetView } from "./bridge";
import { placeholder } from "./shell/Placeholder";

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
  /** Title bar crumb when no task is in focus. */
  crumb: string;
  icon: Icon;
  /** On the rail. Rail position n is bound to ⌘n. A view off the rail (Needs your call) is
   * reached from another view or the command menu. */
  rail: boolean;
  component: Lazy;
  /** Which frames show around the view. */
  layout: {
    /** The 232px project and worktree list (Fleet and Needs your call). */
    projects: boolean;
    /** The 292px inspector, toggled from the title bar or with ⌘\\. */
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
 * The views, in rail order (A2: Fleet, Board, Containers, Memory, Skills; Security and
 * Council arrive in Phases 2 and 3). Each area replaces its own line; a view not built yet
 * shows the shell's intentional placeholder.
 */
export const views: ViewDef[] = [
  { id: "fleet", label: "Fleet", crumb: "Fleet", icon: Sailboat, rail: true, component: lazy(() => import("./views/fleet")), layout: { projects: true, inspector: true }, badge: (v) => v?.rows.filter((r) => r.state === "needs-decision").length ?? 0 },
  { id: "board", label: "Board", crumb: "Board", icon: Kanban, rail: true, component: placeholder("Board", Kanban, "Columns for Queued, In flight, Waiting on you and Done, read from the backlog in the fleet snapshot."), layout: { projects: false, inspector: false } },
  { id: "containers", label: "Containers", crumb: "Containers", icon: Cube, rail: true, component: placeholder("Containers", Cube, "Containers, images, volumes and networks from the local Docker engine, grouped by worktree. Read-only."), layout: { projects: false, inspector: false } },
  { id: "memory", label: "Memory", crumb: "Memory and skills", icon: Brain, rail: true, component: placeholder("Memory", Brain, "Preferences, learnings, briefs and reports, readable and searchable. Firstmate keeps owning every edit."), layout: { projects: false, inspector: false } },
  { id: "skills", label: "Skills", crumb: "Skills", icon: Sparkle, rail: true, component: placeholder("Skills", Sparkle, "Every skill in the known skill folders, with where it comes from and where the same name appears twice."), layout: { projects: false, inspector: false } },
];

/** Settings, at the foot of the rail, bound to ⌘,. */
export const settingsView: ViewDef = { id: "settings", label: "Settings", crumb: "Settings", icon: GearSix, rail: false, component: lazy(() => import("./views/settings")), layout: { projects: false, inspector: false }, needsHome: false };

/** Inspector sections, one line per area, in `order`. */
export const inspectorSections: SectionDef[] = [
  { id: "task", order: 10, component: lazy(() => import("./views/fleet/TaskSection")) },
];

/** The project and worktree list inside the left frame. `null` shows the frame's placeholder. */
export const sidebar: Lazy | null = null;

export const railViews: ViewDef[] = views.filter((v) => v.rail);

export function viewById(id: string): ViewDef {
  return id === settingsView.id ? settingsView : (views.find((v) => v.id === id) ?? (views[0] as ViewDef));
}
