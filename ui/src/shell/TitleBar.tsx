import { DesktopIcon as Desktop, MoonIcon as Moon, SidebarSimpleIcon as SidebarSimple, SunIcon as Sun, type Icon } from "@phosphor-icons/react";
import { IconButton } from "../ui";
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
    <span className="seg" role="group" aria-label="Theme">
      {opts.map(([t, I, label]) => (
        <button
          key={t}
          type="button"
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

/** The 40px title bar: name, crumb, command search, theme and the inspector toggle. */
export function TitleBar() {
  const { area, info, live, selectedTask, inspectorOpen, toggleInspector } = useApp();
  const def = viewById(area);
  const task = def.layout.projects ? live.view?.rows.find((r) => r.id === selectedTask) : undefined;
  const crumb = task ? `${task.project ?? "unknown project"} / ${task.id}` : def.crumb;
  return (
    <header className="titlebar" data-tauri-drag-region>
      <span className="wtitle" data-tauri-drag-region>
        Quarterdeck
      </span>
      <span className="crumb trunc" data-tauri-drag-region title={info?.home ?? undefined}>
        {crumb}
      </span>
      <span className="tb-mid" data-tauri-drag-region>
        <CommandMenu />
      </span>
      <span className="tb-right">
        <ThemeSwitch />
        <IconButton
          icon={SidebarSimple}
          label={inspectorOpen ? "Hide the inspector (⌘\\)" : "Show the inspector (⌘\\)"}
          pressed={def.layout.inspector && inspectorOpen}
          disabled={!def.layout.inspector}
          onClick={toggleInspector}
        />
      </span>
    </header>
  );
}
