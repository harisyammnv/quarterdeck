/**
 * The skills view's door to the core: the generated commands inside the app window, the
 * invented preview data in a plain browser.
 */
import { inApp } from "../../bridge";
import {
  commands,
  type SkillDoc_Serialize as SkillDoc,
  type SkillItem_Serialize as SkillItem,
  type SkillsView_Serialize as SkillsView,
} from "../../bindings";
import { previewSkill, previewSkills } from "./preview";

export type { SkillDoc, SkillItem, SkillsView };
export type SkillScope = SkillItem["scope"];

/** Rescans the known skill folders. */
export function skillsList(): Promise<SkillsView> {
  return inApp ? commands.skillsList() : Promise.resolve(previewSkills());
}

export function skillRead(id: string): Promise<SkillDoc> {
  if (inApp) return commands.skillRead(id);
  try {
    return Promise.resolve(previewSkill(id));
  } catch (e) {
    return Promise.reject(e instanceof Error ? e : new Error(String(e)));
  }
}
