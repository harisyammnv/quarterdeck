# Quarterdeck design system (vendored)

This folder is the Quarterdeck design system as the app uses it.
It is the visual source of truth; the app builds every element from these tokens and the `qd-` component classes.

| File | What |
|---|---|
| `tokens.json` | The source of truth for every value: colours for light and dark, type, spacing, radii, shadows and fixed sizes. |
| `tokens.css` | Generated from `tokens.json` by `scripts/gen-tokens.mjs`; never edit it by hand. `pnpm check` fails when it is stale. |
| `components.css` | The `qd-` component classes, loaded after `tokens.css`. |
| `fonts/` | DM Sans, variable, SIL Open Font License 1.1 (`fonts/OFL.txt`). |
| `icons/` | The 12 key Phosphor 2.1.1 icons, MIT (`icons/LICENSE`). The app imports Phosphor from `@phosphor-icons/react`; these files are the reference set. |
| `logos/` | The Raised deck mark, the lockup and the app icon, single ink, with embedded metadata removed. |

How the app uses it:

- `ui/src/main.tsx` loads `tokens.css`, then `components.css`, then the app's own styles.
- The app root carries `class="qd"`, and `<html>` always carries `data-theme` (light or dark; "match the system" resolves to one of them).
- React wrappers for the components live in `ui/src/ui/`: `Rail`, `Tooltip`, `Keycap`, `StatusChip` (with `Tag` and `Count`), `StateDot`, `IconButton`, `Button`, `SegmentedTabs`, `InspectorSection`, `ZeroState`, `NoteField` and `CommandBar`.
  Areas reuse them; something the system lacks is built from the tokens and called out in its pull request.

To change a value, edit `tokens.json` and run `node scripts/gen-tokens.mjs`.
