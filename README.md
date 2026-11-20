# Quarterdeck

Quarterdeck is a desktop window onto a Firstmate fleet.
It shows what every task is doing, live, and lets you leave Firstmate a note.
It is a companion, not a second supervisor: it never dispatches, steers, merges or cleans up work, and it never edits Firstmate's files.

"Quarterdeck" is a working name.

## Status

This is the Phase 0 spike from the design: one window, one fleet list, one inbox note.

- The fleet list comes from Firstmate's bearings snapshot and is kept live by watching the home's status records.
- A status change written by a worker reaches the screen in tens of milliseconds (measured: 36 ms from the file write to the painted row on this machine).
- The note goes through `bin/fm-inbox.sh note` with a request id, so a retried send can never create a second note.

Phase 1 (the companion: sidebar, board, decisions strip, terminal tail, reports, memory, skills) builds on this.

## Run it

You need Rust 1.90 or newer, Node 22 or newer and pnpm 10.

```sh
pnpm install
QD_FM_HOME=/path/to/firstmate/home pnpm app:dev
```

`pnpm app:build` builds a debug `Quarterdeck.app` under `target/debug/bundle/macos/`.
`pnpm dev` serves the UI alone in a browser with invented preview data, which is useful for layout work and headless screenshots.

| Variable | Meaning |
|---|---|
| `QD_FM_HOME` | The Firstmate home to watch. Falls back to `FM_HOME`. Without either, the window says so and shows nothing. |
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
 bin/fm-bearings-snapshot.sh ◀──gateway (argv)───────┘ every 10 s      │ rows
 bin/fm-inbox.sh note        ◀──gateway (argv)──── send_note           │
                                                                       │
                                                 app (Tauri 2) ──fleet-updated event──▶ ui (React)
```

- `core/` is a library with no window code.
  It finds and validates the home, runs the allow-listed Firstmate entry points through a gateway, parses the snapshot and the status and meta records, merges them into one fleet model, and keeps it live.
- `app/` is the Tauri 2 shell.
  It exposes five typed commands and one event through tauri-specta, and its capability grants only those commands, event listening and window dragging.
- `ui/` is React 19 and TypeScript, built with Vite, and imports the generated `ui/src/bindings.ts`.

How the two read speeds fit together:

- The bearings snapshot is the structured truth, but it takes several seconds.
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

## Design

The look is direction A, version 2 of the design mocks: a fog grey frame, floating white blocks, a black labelled icon rail, pill buttons and DM Sans.
Teal appears only on the brand disc in this build; in the full design it is also the room that holds the decisions strip.
Tokens live in `ui/src/styles/tokens.css` as CSS variables, with light and dark values.
Icons are Phosphor (regular, with duotone for the selected rail item).

## Security posture

- The window has no shell, file system or network permission; every command is individually allowed.
- The content security policy allows only the app's own scripts, styles, fonts and images.
- DM Sans and the icons are bundled; the app makes no network requests.
- The core reads the Firstmate home and writes nothing to it; the one write is a note through Firstmate's own intake script.

## Credits

- [PandaCode](https://github.com/aicodeking/pandacode) was the reference for the Tauri, xterm.js and portable-pty stack and its IPC contract.
  No PandaCode code is copied here.
- [Phosphor Icons](https://phosphoricons.com) (MIT).
- [DM Sans](https://github.com/googlefonts/dm-fonts) (SIL Open Font License), through Fontsource.
- [Tauri](https://tauri.app) and [tauri-specta](https://github.com/specta-rs/tauri-specta).
