import { lazy } from "react";
import { SparkleIcon as Sparkle } from "@phosphor-icons/react";
import type { AreaDef } from "../types";

export const skills: AreaDef = {
  id: "skills",
  label: "Skills",
  crumb: "Skills",
  icon: Sparkle,
  layout: { projects: false, inspector: false },
  View: lazy(() => import("./SkillsView")),
};
