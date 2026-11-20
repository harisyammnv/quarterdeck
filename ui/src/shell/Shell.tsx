import { Suspense } from "react";
import { Rail, useHotkey, type RailEntry } from "../ui";
import { useApp } from "../state/app";
import { inspectorSections, railViews, settingsView, viewById, type ViewDef } from "../registry";
import { NoHome } from "./NoHome";
import { ProjectsPane } from "./ProjectsPane";
import { StatusBar } from "./StatusBar";
import { TitleBar } from "./TitleBar";

function Loading() {
  return <div className="qd-panel ui-panel fill" aria-busy="true" />;
}

/** One rail shortcut, ⌘n for rail position n. */
function RailKey({ n }: { n: number }) {
  const { setArea } = useApp();
  const view = railViews[n - 1];
  useHotkey(
    `mod+${String(n)}`,
    () => {
      if (view) setArea(view.id);
    },
    view !== undefined,
  );
  return null;
}

/** The global shortcuts: ⌘1 to ⌘9 for the rail, ⌘, for Settings, ⌘\\ for the project list. */
function Shortcuts({ view }: { view: ViewDef }) {
  const { setArea, toggleProjects } = useApp();
  useHotkey("mod+,", () => {
    setArea(settingsView.id);
  });
  useHotkey("mod+\\", () => {
    toggleProjects(view.id, view.layout.projects);
  });
  return (
    <>
      {[1, 2, 3, 4, 5, 6, 7, 8, 9].map((n) => (
        <RailKey key={n} n={n} />
      ))}
    </>
  );
}

const sections = [...inspectorSections].sort((a, b) => a.order - b.order);

function entry(v: ViewDef, shortcut: string, badge: number): RailEntry {
  return { id: v.id, label: v.label, icon: v.icon, shortcut: v.shortcut ?? shortcut, description: v.description, badge };
}

/**
 * The window frame. Each view decides, through its registry entry, whether the project list
 * and the inspector show around it; the shell draws them and the areas fill them.
 */
export function Shell() {
  const { area, setArea, info, live, projectsShown } = useApp();
  const def = viewById(area);
  const View = def.component;
  const homeless = info !== null && info.home === null;
  const needsHome = def.needsHome ?? true;
  const showFrames = !(homeless && needsHome);
  const items = railViews.map((v, i) => entry(v, `⌘${String(i + 1)}`, v.badge?.(live.view) ?? 0));
  return (
    <div className="qd app">
      <Shortcuts view={def} />
      <TitleBar />
      <div className="shell">
        <Rail items={items} footer={[entry(settingsView, "⌘,", 0)]} current={area} onSelect={setArea} />
        {showFrames && projectsShown(def.id, def.layout.projects) && <ProjectsPane />}
        <div className="main">
          {homeless && needsHome ? (
            <NoHome />
          ) : (
            <Suspense fallback={<Loading />}>
              <View />
            </Suspense>
          )}
        </div>
        {showFrames && def.layout.inspector && (
          <aside className="qd-panel ui-panel insp" aria-label="Inspector">
            <Suspense fallback={null}>
              {sections.map((s) => {
                const S = s.component;
                return <S key={s.id} />;
              })}
            </Suspense>
          </aside>
        )}
      </div>
      <StatusBar />
    </div>
  );
}
