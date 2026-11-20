import { DesktopIcon as Desktop, MoonIcon as Moon, SidebarSimpleIcon as SidebarSimple, SunIcon as Sun, type Icon } from "@phosphor-icons/react";
import { CommandBar, IconButton } from "../ui";
import { useApp, type ThemePref } from "../state/app";
import { viewById } from "../registry";
import { CommandMenu } from "./CommandMenu";

function ThemeSwitch() {
  const { theme, setTheme } = useApp();
  const opts: [ThemePref, Icon, string][] = [
    ["light", Sun, "Light"],
    ["dark", Moon, "Dark"],
    ["system", Desktop, "Match the system"],
  ];
  return (
    <span className="qd-seg ui-theme" role="group" aria-label="Theme">
      {opts.map(([t, I, label]) => (
        <button
          key={t}
          type="button"
          className="qd-seg__tab ui-theme__tab"
          aria-pressed={theme === t}
          aria-label={label}
          title={label}
          onClick={() => {
            setTheme(t);
          }}
        >
          <I size={14} />
        </button>
      ))}
    </span>
  );
}

/** The title bar: crumb, command search, theme and the project-list toggle (⌘\\). */
export function TitleBar() {
  const { area, info, live, selectedTask, projectsShown, toggleProjects } = useApp();
  const def = viewById(area);
  const homeless = info !== null && info.home === null && (def.needsHome ?? true);
  const shown = !homeless && projectsShown(def.id, def.layout.projects);
  const task = shown ? live.view?.rows.find((r) => r.id === selectedTask) : undefined;
  const crumb = task ? `${task.project ?? "unknown project"} / ${task.id}` : def.crumb;
  return (
    <CommandBar
      crumb={crumb}
      crumbTitle={info?.home ?? undefined}
      search={<CommandMenu />}
      actions={
        <>
          <IconButton
            icon={SidebarSimple}
            label={shown ? "Hide the project list" : "Show the project list"}
            shortcut="⌘\\"
            pressed={shown}
            disabled={homeless}
            onClick={() => {
              toggleProjects(def.id, def.layout.projects);
            }}
          />
          <ThemeSwitch />
        </>
      }
    />
  );
}
