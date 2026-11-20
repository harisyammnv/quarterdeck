import { Suspense } from "react";
import { FolderSimpleIcon as FolderSimple } from "@phosphor-icons/react";
import { sidebar as Sidebar } from "../registry";
import { StateDot } from "../ui";

/**
 * The 232px project and worktree list frame, shown on areas with `layout.projects`.
 * The sidebar slice fills the tree; the frame (header, scroll body, legend foot) is the shell's.
 */
export function ProjectsPane() {
  return (
    <aside className="side panel" aria-label="Projects">
      <div className="side-h">
        <span>Projects</span>
      </div>
      <div className="tree scroll">
        {Sidebar ? (
          <Suspense fallback={null}>
            <Sidebar />
          </Suspense>
        ) : (
          <div className="side-empty">
            <FolderSimple size={18} />
            <span>Projects and their worktrees appear here, with icons and state dots.</span>
          </div>
        )}
      </div>
      <div className="side-foot">
        <div className="legend">
          <span className="row">
            <StateDot kind="working" />
            Working
          </span>
          <span className="row">
            <StateDot kind="waiting" />
            Needs you
          </span>
          <span className="row">
            <StateDot kind="blocked" />
            Blocked
          </span>
          <span className="row">
            <StateDot kind="invest" />
            Investigating
          </span>
        </div>
      </div>
    </aside>
  );
}
