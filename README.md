# Quarterdeck

Quarterdeck is a desktop window onto a Firstmate fleet.
It shows what every task is doing, live, and lets you leave Firstmate a note.
It is a companion, not a second supervisor: it never dispatches, steers, merges or cleans up work, and it never edits Firstmate's files.

"Quarterdeck" is a working name.

## Status

Phase 0 (one window, one fleet list, one inbox note) is done, and Phase 1, the companion, is being built area by area on the foundation described below.

- The fleet list comes from Firstmate's fleet snapshot and is kept live by watching the home's status records.
- A status change written by a worker reaches the screen in tens of milliseconds (measured: 36 ms from the file write to the painted row on this machine).
- The note goes through `bin/fm-inbox.sh note` with a request id, so a retried send can never create a second note.

- The home is chosen once with the system folder picker and remembered in the app's own settings, so the app also works when opened from Finder.

## Run it

You need Rust 1.90 or newer, Node 22 or newer and pnpm 10.

```sh
pnpm install
pnpm app:dev
```

On first launch the window asks for the Firstmate home folder; Settings (Cmd-comma) changes it later.
`pnpm app:build` builds a debug `Quarterdeck.app` under `target/debug/bundle/macos/`.
`pnpm dev` serves the UI alone in a browser with invented preview data, which is useful for layout work and headless screenshots.
In that preview, `?theme=light` or `?theme=dark` pins the theme, `?area=<id>` opens a view, and `?nohome=1` shows the first-launch screen.

| Variable | Meaning |
|---|---|
| `QD_FM_HOME` | The Firstmate home to watch, overriding the one chosen in the window. Falls back to `FM_HOME`, then to the chosen home. |
| `QD_CONFIG_DIR` | Where the app keeps its own settings and snapshot cache. Defaults to `~/Library/Application Support/dev.quarterdeck.desktop`. |
| `QD_THEME` | `light` or `dark` pins the theme at start, for example for screenshots. Otherwise the theme follows the system or the switch in the title bar. |
| `QD_LATENCY_LOG` | A file to append one JSON line per status change: file write, core, emit and paint times. |

## Checks

```sh
pnpm check
```

This runs the UI typecheck and lint, `cargo fmt --check`, `cargo clippy -D warnings` and all Rust tests.
The core tests include an end-to-end run against a fake home with stub scripts, which asserts that a status change reaches the view within a second.
Set `QD_TEST_TMPDIR` to run that fake home on a specific volume.
The app crate has a test that fails when `ui/src/bindings.ts` no longer matches the Rust commands; regenerate it with `QD_WRITE_BINDINGS=1 cargo test -p quarterdeck bindings`.

## Architecture

```
 Firstmate home (read-only)                     Quarterdeck
 ─────────────────────────                      ───────────────────────────────────────────────
 state/<task>.status  ──file events (notify)──▶  core: engine ──▶ fleet model ──▶ FleetView
 state/<task>.meta    ──file events──────────▶       │                 ▲
 data/backlog.md      ──file events──────────▶       │ settle, at most │ snapshot
 bin/fm-fleet-snapshot.sh    ◀──gateway (argv)───────┘ every 10 s      │ Snapshot
 bin/fm-inbox.sh note        ◀──gateway (argv)──── send_note           │
                                                                       │
                                                 app (Tauri 2) ──fleet-updated event──▶ ui (React)
```

- `core/` is a library with no window code.
  It finds and validates the home, runs the allow-listed Firstmate entry points through a gateway, parses the snapshot and the status and meta records, merges them into one fleet model, and keeps it live.
  Every change publishes a `HomeState` (the parsed snapshot, the latest event per task and the snapshot's freshness) that each area projects into its own compact view.
- `app/` is the Tauri 2 shell.
  It exposes typed commands and events through tauri-specta, and its capability grants only those commands, event listening and window dragging.
  The folder picker for the home runs from Rust, so the window never supplies a path.
- `ui/` is React 19 and TypeScript, built with Vite, and imports the generated `ui/src/bindings.ts`.

How the two read speeds fit together:

- The fleet snapshot (`bin/fm-fleet-snapshot.sh --json`) is the structured truth, but it takes several seconds.
  The last good one is cached in the app's own data directory (mode 0600), so the window opens with content and reconciles in the background.
  It runs at start, then once changes have been quiet for 1.5 s, and at most every 10 s while they continue.
  Only one run is ever in flight.
- A change to a status record is re-read and published at once, with no debounce.
  A status event replaces a row's snapshot state when it is newer than the snapshot, or when the snapshot could not tell (`unknown`).
  Such a row is labelled as coming from the latest event, because Firstmate documents a status line as a wake event and not as current truth.
- Before the first snapshot arrives, rows come from the meta and status records, so the window is never empty for long.

The write path is one narrow door.
The gateway runs a fixed list of scripts with an argument vector, never through a shell, with `FM_HOME` set, stdin closed and a timeout.
Scripts get a short allow-list of environment variables and the `PATH` of the user's login shell, plus Homebrew and Orca's bundled CLI.
The note body always follows `--`, so text that looks like a flag is still body text.

## Adding an area

Each area of the app is one directory module in `core/src/<area>/`, one file of commands in `app/src/commands/<area>.rs`, and one view folder in `ui/src/views/<area>/`.
Shared files take one line per item, in alphabetical order by area:

- `core/src/lib.rs`: `pub mod <area>;`.
- `app/src/commands/mod.rs`: `pub mod <area>;`.
- `app/src/lib.rs`: each command in `collect_commands![]`, each event in `collect_events![]`, and for a view driven by the snapshot one `commands::projection(...)` line in `projections()`.
- `app/build.rs`: each command name; `app/capabilities/default.json`: `allow-<command>` for each.
  A test fails when these lists disagree.
- `ui/src/registry.ts`: the view's entry (replacing its placeholder), and any inspector section.

A view driven by the snapshot is a pure `view(&HomeState)` function; the app emits its event only when the result changes.
Files are read through `core::safe_read`, which refuses anything outside its root; other programs start only through `core::external`.
Untrusted Markdown renders through `ui/src/ui/Markdown.tsx` (sanitized, raw HTML dropped, links confirmed before they open).
Shared primitives live in `ui/src/ui/`; areas compose them rather than restyling them.

## Containers

The Containers view reads the local Docker engine over the Unix socket of the active Docker context (`DOCKER_HOST` when it is `unix://`, else `currentContext` in `~/.docker/config.json`); a TCP or SSH context is reported as not supported and never dialled.
It lists containers, images, volumes and networks, groups containers under the task worktree that holds their compose folder or a bind-mounted path, and reads a bounded log tail and CPU and memory for the selected container only.
It is read-only: nothing is started, stopped, removed or exec'd, and container environment variables never reach the window.
When the engine is Kiri, Open Kiri runs `/usr/bin/open -b dev.kiri.app`.
`pnpm dev` previews the engine states with `?area=containers&containers=down|tcp|empty|many`.

## Terminal and reports

A task's view (from the inspector's Worker section) shows the tail of its terminal and its report.
The tail comes from Firstmate's `bin/fm-peek.sh <task-id> <lines>`, the gateway's one read-only terminal entry: at most 200 lines and 64 KiB, read on open, when the task reports a status event and on Refresh, never on a timer.
The core keeps only colour from the captured escape sequences and sends plain text spans, so nothing in agent output can drive the webview; remote tasks are not read.
Reports are `data/<task>/report.md`, read through `safe_read` with links refused and rendered through the shared sanitizing Markdown; the inspector also lists the task's status history.

## Design

The look is direction A, version 2 of the design mocks: a fog grey frame, floating white blocks, a black labelled icon rail, pill buttons and DM Sans.
Teal appears only on the brand disc so far; in the full design it is also the room that holds the decisions strip.
Tokens live in `ui/src/styles/tokens.css` as CSS variables, with light and dark values.
Icons are Phosphor (regular, with duotone for the selected rail item).

## Security posture

- The window has no shell, file system, dialog or network permission; every command is individually allowed.
- Links from documents open only after a confirmation that shows the full address, and only `https` links are handed to the browser.
- The content security policy allows only the app's own scripts, styles, fonts and images.
- DM Sans and the icons are bundled; the app makes no network requests.
- The core reads the Firstmate home and writes nothing to it; the one write is a note through Firstmate's own intake script.

## Credits

- [PandaCode](https://github.com/aicodeking/pandacode) was the reference for the Tauri, xterm.js and portable-pty stack and its IPC contract.
  No PandaCode code is copied here.
- [Phosphor Icons](https://phosphoricons.com) (MIT).
- [DM Sans](https://github.com/googlefonts/dm-fonts) (SIL Open Font License), through Fontsource.
- [Tauri](https://tauri.app) and [tauri-specta](https://github.com/specta-rs/tauri-specta).
