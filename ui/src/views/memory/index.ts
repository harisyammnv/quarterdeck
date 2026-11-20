import { lazy } from "react";
import { BrainIcon as Brain } from "@phosphor-icons/react";
import type { AreaDef } from "../types";

export const memory: AreaDef = {
  id: "memory",
  label: "Memory",
  crumb: "Memory and skills",
  icon: Brain,
  layout: { projects: false, inspector: false },
  View: lazy(() => import("./MemoryView")),
};
