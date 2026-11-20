import type { ReactNode } from "react";

/**
 * CommandBar (design system): the 40px title bar, `qd-titlebar`. The app name, the current
 * area as a crumb, the search and command field centred in the window, and icon buttons on
 * the right. The bar is a window drag region; its controls are not.
 */
export function CommandBar({ crumb, crumbTitle, search, actions }: { crumb: string; crumbTitle?: string; search: ReactNode; actions: ReactNode }) {
  return (
    <header className="qd-titlebar ui-titlebar" data-tauri-drag-region>
      <span className="ui-wtitle" data-tauri-drag-region>
        Quarterdeck
      </span>
      <span className="ui-crumb qd-trunc" data-tauri-drag-region title={crumbTitle}>
        {crumb}
      </span>
      <span className="ui-tb-spacer" data-tauri-drag-region />
      <span className="ui-tb-mid">{search}</span>
      <span className="ui-tb-right">{actions}</span>
    </header>
  );
}
