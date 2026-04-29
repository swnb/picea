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

function assertLocalizedScenario(locale, scenario, expected) {
  assert.equal(JSON.stringify(localizeScenario(locale, scenario)), JSON.stringify(expected));
}

assert.deepEqual(Array.from(supportedLocales), ["zh-CN", "en-US"]);
assert.deepEqual(Object.keys(messages["zh-CN"]).sort(), Object.keys(messages["en-US"]).sort());
for (const locale of supportedLocales) {
  assert.equal(typeof messages[locale]["panel.sceneHierarchy"], "string");
  assert.equal(typeof messages[locale]["panel.processFacts"], "string");
  assert.equal(typeof messages[locale]["tooltip.canvasLayers"], "string");
  assert.equal(typeof messages[locale]["timeline.runSetup"], "string");
  assert.equal(typeof messages[locale]["canvas.contacts"], "string");
  assert.equal(typeof messages[locale]["inspector.compoundProvenance"], "string");
}
assert.equal(t("zh-CN", "timeline.frameAt", { frame: 8 }), "第 8 帧");
assert.equal(t("en-US", "timeline.frameAt", { frame: 8 }), "frame 8");
assert.equal(bodyTypeLabel("zh-CN", "dynamic"), "动态");
assert.equal(statusLabel("zh-CN", "playing"), "播放中");
assert.equal(sourceLabel("en-US", "server"), "Rust replay");
assert.equal(layerLabel("zh-CN", "contacts"), "接触点");
assert.equal(layerLabel("zh-CN", "provenance"), "来源");
assert.equal(entityLabel("zh-CN", "body", 2), "物体 2");
assert.equal(dynamicValueLabel("zh-CN", "generic_convex_fallback"), "通用凸形回退");
assert.equal(dynamicValueLabel("en-US", "stability_window"), "stability window");
assert.equal(dynamicValueLabel("en-US", "epa_failure_contained"), "EPA failure contained");
assertLocalizedScenario(
  "zh-CN",
  {
    id: "falling_box_contact",
    name: "Falling box contact",
    description: "Demo fallback",
  },
  {
    id: "falling_box_contact",
    name: "落箱接触",
    description: "动态箱体下落到静态地面接触，用于观察 AABB、轨迹和接触事实。",
  },
);
assertLocalizedScenario(
  "zh-CN",
  {
    id: "compound_provenance",
    name: "Compound provenance fixture",
    description: "Demo fallback",
  },
  {
    id: "compound_provenance",
    name: "复合体来源",
    description: "作者提供的复合体 fixture，展示 piece 顺序、继承语义、宽阶段树和岛事实。",
  },
);
assertLocalizedScenario(
  "zh-CN",
  {
    id: "ccd_dynamic_convex_pair",
    name: "CCD dynamic convex pair",
    description: "Two fast dynamic rectangles swept against each other.",
  },
  {
    id: "ccd_dynamic_convex_pair",
    name: "CCD 动态凸体对撞",
    description: "两个高速动态矩形彼此扫掠命中，用于观察动态目标 CCD 的 TOI、目标扫掠和目标钳制事实。",
  },
);
