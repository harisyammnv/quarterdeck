import { useMemo, useRef, useState } from "react";
import { MagnifyingGlassIcon as MagnifyingGlass, type Icon } from "@phosphor-icons/react";
import { Keycap, useHotkey } from "../ui";
import { inApp, refreshFleet } from "../bridge";
import { useApp } from "../state/app";
import { railViews, settingsView, views } from "../registry";

interface Cmd {
  id: string;
  label: string;
  hint?: string;
  icon?: Icon;
  run: () => void;
}

/**
 * "Search or run a command", bound to ⌘K. Lists the areas, the tasks in flight and a few
 * shell actions, filtered as you type. Every command only navigates or reads; none acts on
 * the fleet.
 */
export function CommandMenu() {
  const app = useApp();
  const { setArea, setTheme, chooseHome, setSelectedTask, live } = app;
  const [q, setQ] = useState("");
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const field = useRef<HTMLInputElement>(null);

  useHotkey("mod+k", () => {
    field.current?.focus();
    field.current?.select();
    setOpen(true);
  });

  const all = useMemo<Cmd[]>(() => {
    const areas: Cmd[] = [...views, settingsView].map((a) => ({
      id: `area:${a.id}`,
      label: a.label,
      hint: a.id === "settings" ? "⌘," : a.rail ? `⌘${railViews.indexOf(a) + 1}` : undefined,
      icon: a.icon,
      run: () => {
        setArea(a.id);
      },
    }));
    const tasks: Cmd[] = (live.view?.rows ?? []).map((r) => ({
      id: `task:${r.id}`,
      label: r.name,
      hint: r.project ?? r.id,
      run: () => {
        setArea("fleet");
        setSelectedTask(r.id);
      },
    }));
    const actions: Cmd[] = [
      { id: "refresh", label: "Refresh the fleet", run: () => void refreshFleet() },
      { id: "theme-light", label: "Use the light theme", run: () => { setTheme("light"); } },
      { id: "theme-dark", label: "Use the dark theme", run: () => { setTheme("dark"); } },
      { id: "theme-system", label: "Match the system theme", run: () => { setTheme("system"); } },
    ];
    if (inApp) actions.push({ id: "home", label: "Choose the Firstmate home", run: () => void chooseHome() });
    return [...areas, ...tasks, ...actions];
  }, [live.view, setArea, setSelectedTask, setTheme, chooseHome]);

  const shown = useMemo(() => {
    const t = q.trim().toLowerCase();
    const hits = t ? all.filter((c) => c.label.toLowerCase().includes(t) || (c.hint ?? "").toLowerCase().includes(t)) : all;
    return hits.slice(0, 9);
  }, [all, q]);

  const close = () => {
    setOpen(false);
    setQ("");
    setActive(0);
    field.current?.blur();
  };
  const run = (c: Cmd | undefined) => {
    if (!c) return;
    c.run();
    close();
  };

  return (
    <div className="cmdwrap">
      <label className="qd-cmd">
        <MagnifyingGlass size={15} />
        <span className="sr-only">Search or run a command</span>
        <input
          ref={field}
          value={q}
          placeholder="Search or run a command"
          role="combobox"
          aria-expanded={open}
          aria-controls="cmd-list"
          aria-activedescendant={open && shown[active] ? `cmd-${shown[active].id}` : undefined}
          onFocus={() => {
            setOpen(true);
          }}
          onBlur={() => {
            setOpen(false);
          }}
          onChange={(e) => {
            setQ(e.target.value);
            setActive(0);
            setOpen(true);
          }}
          onKeyDown={(e) => {
            if (e.key === "Escape") close();
            else if (e.key === "ArrowDown") {
              e.preventDefault();
              setActive((a) => Math.min(a + 1, shown.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setActive((a) => Math.max(a - 1, 0));
            } else if (e.key === "Enter") {
              e.preventDefault();
              run(shown[active]);
            }
          }}
        />
        <Keycap>⌘K</Keycap>
      </label>
      {open && (
        <ul className="cmdlist" id="cmd-list" role="listbox" aria-label="Commands">
          {shown.length === 0 && <li className="cmd-empty">Nothing matches.</li>}
          {shown.map((c, i) => {
            const I = c.icon;
            return (
              <li
                key={c.id}
                id={`cmd-${c.id}`}
                role="option"
                aria-selected={i === active}
                className={i === active ? "on" : undefined}
                onMouseDown={(e) => {
                  e.preventDefault();
                  run(c);
                }}
                onMouseEnter={() => {
                  setActive(i);
                }}
              >
                <span className="cmd-ic">{I && <I size={15} />}</span>
                <span className="trunc grow">{c.label}</span>
                {c.hint && <span className="mut trunc cmd-hint">{c.hint}</span>}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
