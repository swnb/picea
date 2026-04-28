import assert from "node:assert/strict";
import fs from "node:fs";

const contractSources = [
  "../src/App.tsx",
  "../src/components/workbench/Toolbar.tsx",
  "../src/components/workbench/SceneHierarchy.tsx",
  "../src/components/workbench/Inspector.tsx",
].map((path) => fs.readFileSync(new URL(path, import.meta.url), "utf8"));
const appSource = contractSources.join("\n");

assert.doesNotMatch(
  appSource,
  /items=\{supportedLocales\.map/,
  "Toolbar language control should be a single toggle button, not a locale Select dropdown.",
);
assert.doesNotMatch(
  appSource,
  /function toggleLocale\(/,
  "Toolbar language control should open an option popup instead of directly toggling locales.",
);
assert.match(
  appSource,
  /function LanguageMenu\(/,
  "Toolbar should use the same DropdownMenu popup pattern for language options as other header popups.",
);
assert.match(
  appSource,
  /<Button[^>]*\bsize="icon"[^>]*\bvariant="outline"[^>]*\baria-label=\{t\(locale, "app\.language"\)\}/,
  "Toolbar language trigger should be icon-only while keeping an accessible label.",
);
assert.match(
  appSource,
  /onCloseAutoFocus=\{handleCloseAutoFocus\}/,
  "Language popup should not return focus to the trigger after a mouse selection closes the menu.",
);
assert.match(
  appSource,
  /triggerRef\.current\?\.blur\(\)/,
  "Language trigger should clear the active focus highlight after the popup closes.",
);
assert.ok(
  (appSource.match(/onCloseAutoFocus=\{handleCloseAutoFocus\}/g) ?? []).length >= 2,
  "Every header popup button should prevent Radix from returning focus to its trigger after close.",
);
assert.ok(
  (appSource.match(/triggerRef\.current\?\.blur\(\)/g) ?? []).length >= 2,
  "Every header popup button should clear its trigger focus highlight after close.",
);
assert.match(
  appSource,
  /onSelect=\{\(event\) => event\.preventDefault\(\)\}/,
  "Layer checkbox items should prevent Radix DropdownMenu's default select-close behavior.",
);
assert.match(
  appSource,
  /generic_convex_trace/,
  "Contact inspector should render M7 generic convex GJK/EPA trace facts.",
);
assert.match(
  appSource,
  /fact\.genericFallback/,
  "Contact inspector should label the generic convex fallback decision.",
);
assert.match(
  appSource,
  /fact\.gjk/,
  "Contact inspector should label GJK termination and iteration facts.",
);
assert.match(
  appSource,
  /fact\.epa/,
  "Contact inspector should label EPA termination and iteration facts.",
);
assert.match(
  appSource,
  /ccd_trace\.target_kind/,
  "Contact inspector should expose CCD target kind for dynamic target traces.",
);
assert.match(
  appSource,
  /ccd_trace\.target_swept_start/,
  "Contact inspector should expose CCD target swept start.",
);
assert.match(
  appSource,
  /ccd_trace\.target_swept_end/,
  "Contact inspector should expose CCD target swept end.",
);
assert.match(
  appSource,
  /ccd_trace\.target_clamp/,
  "Contact inspector should expose CCD target clamp distance.",
);
assert.match(
  appSource,
  /tree\.joints/,
  "Scene hierarchy should expose joints as selectable replay artifacts.",
);
assert.match(
  appSource,
  /frame\.snapshot\.joints\.map/,
  "Scene hierarchy should render joint rows from the debug snapshot.",
);
assert.match(
  appSource,
  /log\.sseIdle/,
  "Workbench logs should distinguish an empty SSE queue from a failed run.",
);
assert.match(
  appSource,
  /final_snapshot_artifact/,
  "Workbench should surface final_snapshot artifact provenance from the server session.",
);
