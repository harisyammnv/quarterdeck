import { useEffect, useState } from "react";
import { CircleDashedIcon as CircleDashed, SparkleIcon as Sparkle, WarningCircleIcon as WarningCircle } from "@phosphor-icons/react";
import { StatusPill, Tag, Zero } from "../../ui";
import { DocBody } from "../memory/DocBody";
import { skillRead, type SkillDoc } from "./bridge";
import { SCOPE_LABEL } from "./SkillRows";

type Load = { id: string; doc: SkillDoc | null; error: string | null };

/** One skill, read on demand: its header facts and its body. */
export function SkillDocPane({ id, find }: { id: string; find: string }) {
  const [load, setLoad] = useState<Load>({ id, doc: null, error: null });
  useEffect(() => {
    let live = true;
    skillRead(id)
      .then((doc) => {
        if (live) setLoad({ id, doc, error: null });
      })
      .catch((e: unknown) => {
        if (live) setLoad({ id, doc: null, error: e instanceof Error ? e.message : String(e) });
      });
    return () => {
      live = false;
    };
  }, [id]);
  const doc = load.id === id ? load.doc : null;
  const error = load.id === id ? load.error : null;
  const item = doc?.item;
  return (
    <>
      <div className="phead memory-phead">
        <Sparkle size={18} className="mut" aria-hidden="true" />
        <h1 className="ptitle skills-title trunc">{item?.name ?? "Skill"}</h1>
        {item && <Tag>{SCOPE_LABEL[item.scope]}</Tag>}
        {item && item.also_in > 0 && (
          <StatusPill tone="warning">
            Also in {item.also_in} {item.also_in === 1 ? "place" : "places"}
          </StatusPill>
        )}
        <span className="grow" />
      </div>
      {error ? (
        <Zero icon={WarningCircle} title="This skill could not be read" dashed={false}>
          {error}
        </Zero>
      ) : !doc ? (
        <Zero icon={CircleDashed} title="Reading the skill" dashed={false} />
      ) : (
        <>
          <dl className="skills-facts">
            <div>
              <dt>Trigger</dt>
              <dd>{doc.item.description || "No trigger description."}</dd>
            </div>
            {doc.item.source && (
              <div>
                <dt>Source</dt>
                <dd>{doc.item.source}</dd>
              </div>
            )}
            <div>
              <dt>Installed in</dt>
              <dd>{doc.item.locations.join(", ")}</dd>
            </div>
          </dl>
          <DocBody text={doc.text} truncated={doc.truncated} find={find} />
        </>
      )}
    </>
  );
}
