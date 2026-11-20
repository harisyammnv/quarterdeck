import { useRef, useState } from "react";
import { SparkleIcon as Sparkle } from "@phosphor-icons/react";
import { Zero, useHotkey } from "../../ui";
import { FindField } from "../memory/FindField";
import { useHomeKey } from "../memory/useHomeKey";
import { useRefreshing } from "../memory/useRefreshing";
import { skillsList } from "./bridge";
import { SkillDocPane } from "./SkillDocPane";
import { SkillsPanel } from "./SkillsPanel";
import "../memory/memory.css";
import "./skills.css";

/** Every installed skill, searchable, with the selected one read on the right. */
export default function SkillsView() {
  const [skills, reload] = useRefreshing(skillsList, useHomeKey());
  const [selected, setSelected] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const find = useRef<HTMLInputElement>(null);
  useHotkey("/", () => find.current?.focus());

  const open = selected ?? skills.data?.items[0]?.id ?? null;
  return (
    <div className="memory-cols">
      <SkillsPanel
        skills={skills}
        selected={open}
        onOpen={setSelected}
        compact={false}
        onRefresh={reload}
        query={query}
        search={<FindField ref={find} value={query} onChange={setQuery} placeholder="Find a skill" className="skills-find" />}
      />
      <section className="panel memory-main" aria-label="Skill">
        {open ? (
          <SkillDocPane id={open} find={query} />
        ) : (
          <Zero icon={Sparkle} title="No skill selected" dashed={false}>
            Choose a skill to read what it does.
          </Zero>
        )}
      </section>
    </div>
  );
}
