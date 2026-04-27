import assert from "node:assert/strict";
import fs from "node:fs";

const appSource = fs.readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

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
