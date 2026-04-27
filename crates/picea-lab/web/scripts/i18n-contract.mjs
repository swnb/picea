import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const source = fs.readFileSync(new URL("../src/i18n.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText;

const module = { exports: {} };
vm.runInNewContext(compiled, {
  exports: module.exports,
  module,
  require(id) {
    throw new Error(`Unexpected require from i18n contract: ${id}`);
  },
});

const {
  bodyTypeLabel,
  dynamicValueLabel,
  entityLabel,
  layerLabel,
  localizeScenario,
  messages,
  sourceLabel,
  statusLabel,
  supportedLocales,
  t,
} = module.exports;

assert.deepEqual(Array.from(supportedLocales), ["zh-CN", "en-US"]);
assert.deepEqual(Object.keys(messages["zh-CN"]).sort(), Object.keys(messages["en-US"]).sort());
for (const locale of supportedLocales) {
  assert.equal(typeof messages[locale]["panel.sceneHierarchy"], "string");
  assert.equal(typeof messages[locale]["tooltip.canvasLayers"], "string");
  assert.equal(typeof messages[locale]["timeline.runSetup"], "string");
  assert.equal(typeof messages[locale]["canvas.contacts"], "string");
}
assert.equal(t("zh-CN", "timeline.frameAt", { frame: 8 }), "第 8 帧");
assert.equal(t("en-US", "timeline.frameAt", { frame: 8 }), "frame 8");
assert.equal(bodyTypeLabel("zh-CN", "dynamic"), "动态");
assert.equal(statusLabel("zh-CN", "playing"), "播放中");
assert.equal(sourceLabel("en-US", "server"), "Rust replay");
assert.equal(layerLabel("zh-CN", "contacts"), "接触点");
assert.equal(entityLabel("zh-CN", "body", 2), "物体 2");
assert.equal(dynamicValueLabel("zh-CN", "generic_convex_fallback"), "通用凸形回退");
assert.equal(dynamicValueLabel("en-US", "epa_failure_contained"), "EPA failure contained");
assert.equal(
  JSON.stringify(localizeScenario("zh-CN", {
    id: "falling_box_contact",
    name: "Falling box contact",
    description: "Demo fallback",
  })),
  JSON.stringify({
    id: "falling_box_contact",
    name: "落箱接触",
    description: "动态箱体下落到静态地面接触，用于观察 AABB、轨迹和接触事实。",
  }),
);
