#!/usr/bin/env node
// Generates ui/src/design-system/tokens.css from tokens.json, the design system's source of
// truth. `node scripts/gen-tokens.mjs` writes the file; `--check` fails when it is stale,
// which `pnpm check` runs.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const dir = join(dirname(fileURLToPath(import.meta.url)), "..", "ui", "src", "design-system");
const tokens = JSON.parse(readFileSync(join(dir, "tokens.json"), "utf8"));

function themed(theme) {
  const vars = [...tokens.color.tokens, ...tokens.shadow.tokens].map((t) => `--${t.name}:${t.value[theme]};`);
  return vars.join("");
}

const scalars = [...tokens.spacing.tokens, ...tokens.radius.tokens, ...tokens.size.tokens].map((t) => `--${t.name}:${t.value};`);
const families = Object.entries(tokens.type.families).map(([k, v]) => `--font-${k}:${v};`);
const fonts = tokens.type.fonts.map(
  (f) => `@font-face{font-family:"${f.family}";src:url("${f.file}") format("woff2");font-weight:${f.weight};font-style:${f.style}}`,
);

const css = [
  `/* ${tokens.name} - generated from tokens.json by scripts/gen-tokens.mjs; do not edit */`,
  `:root, [data-theme="light"]{${themed("light")}}`,
  `[data-theme="dark"]{${themed("dark")}}`,
  `:root{${scalars.join("")}${families.join("")}}`,
  ...fonts,
].join("\n");

const out = join(dir, "tokens.css");
if (process.argv.includes("--check")) {
  let have = "";
  try {
    have = readFileSync(out, "utf8");
  } catch {
    // A missing file is stale too.
  }
  if (have !== css) {
    console.error("ui/src/design-system/tokens.css is stale: run node scripts/gen-tokens.mjs");
    process.exit(1);
  }
} else {
  writeFileSync(out, css);
}
