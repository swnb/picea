import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const source = fs.readFileSync(new URL("../src/i18n.ts", import.meta.url), "utf8");
const typesSource = fs.readFileSync(new URL("../src/types.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText;
const instrumented = `${compiled}
module.exports.__dynamicValueLabels = dynamicValueLabels;
`;

const module = { exports: {} };
vm.runInNewContext(instrumented, {
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
  overlayPresetDescription,
  overlayPresetLabel,
  parameterSourceLabel,
  scenarioParameterHelp,
  scenarioParameterLabel,
  scenarioGroupForId,
  scenarioGroupLabel,
  scenarioWatchFor,
  sourceLabel,
  statusLabel,
  supportedLocales,
  t,
  __dynamicValueLabels,
} = module.exports;

const expectedPerturbationRejectionReasons = [
  "not_live_session",
  "session_created",
  "session_running",
  "session_completed",
  "session_failed",
  "missing_frame_snapshot",
  "stale_world_revision",
  "stale_session_epoch",
  "stale_frame",
  "invalid_body_handle",
  "static_body",
  "kinematic_body",
  "invalid_velocity_delta",
  "missing_preview_action",
  "stale_preview_action",
  "reused_action",
  "body_patch_failed",
];
const perturbationReasonUnionSource =
  typesSource.match(/export type VelocityPerturbationRejectionReason =([\s\S]*?);/)?.[1] ?? "";
const perturbationReasonUnion = Array.from(
  perturbationReasonUnionSource.matchAll(/\|\s*"([^"]+)"/g),
  (match) => match[1],
).sort();

function assertLocalizedScenario(locale, scenario, expected) {
  assert.equal(JSON.stringify(localizeScenario(locale, scenario)), JSON.stringify(expected));
}

assert.deepEqual(Array.from(supportedLocales), ["zh-CN", "en-US"]);
assert.deepEqual(Object.keys(messages["zh-CN"]).sort(), Object.keys(messages["en-US"]).sort());
for (const locale of supportedLocales) {
  assert.equal(typeof messages[locale]["panel.sceneHierarchy"], "string");
  assert.equal(typeof messages[locale]["panel.processFacts"], "string");
  assert.equal(typeof messages[locale]["timeline.stack"], "string");
  assert.equal(typeof messages[locale]["timeline.trajectory"], "string");
  assert.equal(typeof messages[locale]["panel.stackStability"], "string");
  assert.equal(typeof messages[locale]["panel.trajectory"], "string");
  assert.equal(typeof messages[locale]["stability.derived"], "string");
  assert.equal(typeof messages[locale]["stability.missing"], "string");
  assert.equal(typeof messages[locale]["stability.marker.contactSpike"], "string");
  assert.equal(typeof messages[locale]["stability.marker.angularDrift"], "string");
  assert.equal(typeof messages[locale]["stability.marker.quietWindow"], "string");
  assert.equal(typeof messages[locale]["stability.contribution"], "string");
  assert.equal(typeof messages[locale]["stability.overlay"], "string");
  assert.equal(typeof messages[locale]["tooltip.canvasLayers"], "string");
  assert.equal(typeof messages[locale]["tooltip.canvasFit"], "string");
  assert.equal(typeof messages[locale]["tooltip.canvasLockCore"], "string");
  assert.equal(typeof messages[locale]["tooltip.canvasUnlockCore"], "string");
  assert.equal(typeof messages[locale]["tooltip.canvasReset"], "string");
  assert.equal(typeof messages[locale]["tooltip.canvasZoomIn"], "string");
  assert.equal(typeof messages[locale]["tooltip.canvasZoomOut"], "string");
  assert.equal(typeof messages[locale]["timeline.runSetup"], "string");
  assert.equal(typeof messages[locale]["timeline.diagnostics"], "string");
  assert.equal(typeof messages[locale]["timeline.evidence"], "string");
  assert.equal(typeof messages[locale]["diagnostics.exported"], "string");
  assert.equal(typeof messages[locale]["diagnostics.performance"], "string");
  assert.equal(typeof messages[locale]["diagnostics.stability"], "string");
  assert.equal(typeof messages[locale]["diagnostics.markers"], "string");
  assert.equal(typeof messages[locale]["diagnostics.missingEvidence"], "string");
  assert.equal(typeof messages[locale]["diagnostics.source.rustAuthoritative"], "string");
  assert.equal(typeof messages[locale]["diagnostics.source.labDerived"], "string");
  assert.equal(typeof messages[locale]["diagnostics.source.webDerived"], "string");
  assert.equal(typeof messages[locale]["diagnostics.source.missing"], "string");
  assert.equal(typeof messages[locale]["diagnostics.marker.solverRowSpike"], "string");
  assert.equal(typeof messages[locale]["diagnostics.marker.penetrationSpike"], "string");
  assert.equal(typeof messages[locale]["canvas.contacts"], "string");
  assert.equal(typeof messages[locale]["canvas.zoom"], "string");
  assert.equal(typeof messages[locale]["canvas.scale"], "string");
  assert.equal(typeof messages[locale]["canvas.mode"], "string");
  assert.equal(typeof messages[locale]["canvas.modeFree"], "string");
  assert.equal(typeof messages[locale]["canvas.modeLockedCore"], "string");
  assert.equal(typeof messages[locale]["canvas.targetActive"], "string");
  assert.equal(typeof messages[locale]["canvas.targetAllColliders"], "string");
  assert.equal(typeof messages[locale]["canvas.targetScene"], "string");
  assert.equal(typeof messages[locale]["canvas.offlineWatermark"], "string");
  assert.equal(typeof messages[locale]["scenario.group.basics"], "string");
  assert.equal(typeof messages[locale]["scenario.group.stack"], "string");
  assert.equal(typeof messages[locale]["scenario.group.ccd"], "string");
  assert.equal(typeof messages[locale]["scenario.group.compound"], "string");
  assert.equal(typeof messages[locale]["scenario.group.lattice"], "string");
  assert.equal(typeof messages[locale]["scenario.group.diagnostics"], "string");
  assert.equal(typeof messages[locale]["tooltip.startRunScenario"], "string");
  assert.equal(typeof messages[locale]["tooltip.rerunScenario"], "string");
  assert.equal(typeof messages[locale]["tooltip.overlayPresets"], "string");
  assert.equal(typeof messages[locale]["tooltip.pausePlayback"], "string");
  assert.equal(typeof messages[locale]["tooltip.playTimeline"], "string");
  assert.equal(typeof messages[locale]["tooltip.replayTimeline"], "string");
  assert.equal(typeof messages[locale]["evidence.source"], "string");
  assert.equal(typeof messages[locale]["evidence.scenario"], "string");
  assert.equal(typeof messages[locale]["evidence.stateHash"], "string");
  assert.equal(typeof messages[locale]["evidence.session"], "string");
  assert.equal(typeof messages[locale]["evidence.run"], "string");
  assert.equal(typeof messages[locale]["evidence.frame"], "string");
  assert.equal(typeof messages[locale]["evidence.selection"], "string");
  assert.equal(typeof messages[locale]["evidence.layers"], "string");
  assert.equal(typeof messages[locale]["evidence.camera"], "string");
  assert.equal(typeof messages[locale]["evidence.center"], "string");
  assert.equal(typeof messages[locale]["evidence.target"], "string");
  assert.equal(typeof messages[locale]["evidence.targetBounds"], "string");
  assert.equal(typeof messages[locale]["evidence.damping"], "string");
  assert.equal(typeof messages[locale]["evidence.grid"], "string");
  assert.equal(typeof messages[locale]["evidence.rulers"], "string");
  assert.equal(typeof messages[locale]["evidence.perfStatus"], "string");
  assert.equal(typeof messages[locale]["evidence.noArtifact"], "string");
  assert.equal(typeof messages[locale]["evidence.trajectory"], "string");
  assert.equal(typeof messages[locale]["evidence.stack"], "string");
  assert.equal(typeof messages[locale]["evidence.diagnostics"], "string");
  assert.equal(typeof messages[locale]["evidence.lattice"], "string");
  assert.equal(typeof messages[locale]["evidence.perturbation"], "string");
  assert.equal(typeof messages[locale]["debug.copyContext"], "string");
  assert.equal(typeof messages[locale]["debug.contextCopied"], "string");
  assert.equal(typeof messages[locale]["debug.contextCopyFailed"], "string");
  assert.equal(typeof messages[locale]["debug.contextCopyFallback"], "string");
  assert.equal(typeof messages[locale]["debug.contextPreview"], "string");
  assert.equal(typeof messages[locale]["common.true"], "string");
  assert.equal(typeof messages[locale]["common.false"], "string");
  assert.equal(typeof messages[locale]["perf.missing"], "string");
  assert.equal(typeof messages[locale]["perf.available"], "string");
  assert.equal(typeof messages[locale]["perf.liveUnavailable"], "string");
  assert.equal(typeof messages[locale]["inspector.compoundProvenance"], "string");
  assert.equal(typeof messages[locale]["trajectory.mode"], "string");
  assert.equal(typeof messages[locale]["trajectory.historyLength"], "string");
  assert.equal(typeof messages[locale]["trajectory.samplingStride"], "string");
  assert.equal(typeof messages[locale]["trajectory.fade"], "string");
  assert.equal(typeof messages[locale]["trajectory.colorMode"], "string");
  assert.equal(typeof messages[locale]["trajectory.mode.selectedBody"], "string");
  assert.equal(typeof messages[locale]["trajectory.mode.allDynamic"], "string");
  assert.equal(typeof messages[locale]["trajectory.mode.selectedIsland"], "string");
  assert.equal(typeof messages[locale]["trajectory.mode.contacts"], "string");
  assert.equal(typeof messages[locale]["trajectory.mode.ccd"], "string");
  assert.equal(typeof messages[locale]["trajectory.colorMode.body"], "string");
  assert.equal(typeof messages[locale]["trajectory.colorMode.island"], "string");
  assert.equal(typeof messages[locale]["trajectory.empty.selectedBody"], "string");
  assert.equal(typeof messages[locale]["trajectory.empty.allDynamic"], "string");
  assert.equal(typeof messages[locale]["trajectory.empty.selectedIsland"], "string");
  assert.equal(typeof messages[locale]["trajectory.empty.contacts"], "string");
  assert.equal(typeof messages[locale]["trajectory.empty.ccd"], "string");
  assert.equal(typeof messages[locale]["trajectory.empty.unsupportedSelection"], "string");
  assert.equal(typeof messages[locale]["trajectory.marker.bodyMotion"], "string");
  assert.equal(typeof messages[locale]["trajectory.marker.contactAppeared"], "string");
  assert.equal(typeof messages[locale]["trajectory.marker.contactDisappeared"], "string");
  assert.equal(typeof messages[locale]["trajectory.marker.contactBurst"], "string");
  assert.equal(typeof messages[locale]["trajectory.marker.ccdClamp"], "string");
  assert.equal(typeof messages[locale]["trajectory.marker.ccdHit"], "string");
  assert.equal(typeof messages[locale]["trajectory.summary.derived"], "string");
  assert.equal(typeof messages[locale]["trajectory.summary.distance"], "string");
  assert.equal(typeof messages[locale]["trajectory.summary.maxSpeed"], "string");
  assert.equal(typeof messages[locale]["trajectory.summary.awakeSleep"], "string");
  assert.equal(typeof messages[locale]["trajectory.summary.contactCount"], "string");
  assert.equal(typeof messages[locale]["trajectory.summary.ccdEvidence"], "string");
  assert.equal(typeof messages[locale]["trajectory.summary.window"], "string");
  assert.equal(typeof messages[locale]["trajectory.summary.mode"], "string");
  assert.equal(typeof messages[locale]["overlay.preset.stackStability"], "string");
  assert.equal(typeof messages[locale]["overlay.preset.trajectoryFocus"], "string");
  assert.equal(typeof messages[locale]["overlay.preset.perturbationReview"], "string");
  assert.equal(typeof messages[locale]["overlay.preset.latticeGrid"], "string");
  assert.equal(typeof messages[locale]["overlay.presetDesc.stackStability"], "string");
  assert.equal(typeof messages[locale]["overlay.presetDesc.trajectoryFocus"], "string");
  assert.equal(typeof messages[locale]["overlay.presetDesc.perturbationReview"], "string");
  assert.equal(typeof messages[locale]["overlay.presetDesc.latticeGrid"], "string");
  assert.equal(typeof messages[locale]["run.gravityVector"], "string");
  assert.equal(typeof messages[locale]["run.gravityDial"], "string");
  assert.equal(typeof messages[locale]["run.gravityX"], "string");
  assert.equal(typeof messages[locale]["run.gravityY"], "string");
  assert.equal(typeof messages[locale]["run.gravityMagnitude"], "string");
  assert.equal(typeof messages[locale]["run.gravityApply"], "string");
  assert.equal(typeof messages[locale]["run.gravityApplying"], "string");
  assert.equal(typeof messages[locale]["run.gravityUndo"], "string");
  assert.equal(typeof messages[locale]["run.gravityReset"], "string");
  assert.equal(typeof messages[locale]["run.gravityPendingIdle"], "string");
  assert.equal(typeof messages[locale]["run.gravityPendingActive"], "string");
  assert.equal(typeof messages[locale]["run.gravityPendingDisabled"], "string");
  assert.equal(typeof messages[locale]["run.gravityDirtyLive"], "string");
  assert.equal(typeof messages[locale]["run.gravityAppliedLive"], "string");
  assert.equal(typeof messages[locale]["run.gravityAppliedValue"], "string");
  assert.equal(typeof messages[locale]["run.gravityApplyError"], "string");
  assert.equal(typeof messages[locale]["run.sceneParameters"], "string");
  assert.equal(typeof messages[locale]["run.parameterDefault"], "string");
  assert.equal(typeof messages[locale]["run.parameterCurrent"], "string");
  assert.equal(typeof messages[locale]["run.parameterEffective"], "string");
  assert.equal(typeof messages[locale]["run.parameterDirty"], "string");
  assert.equal(typeof messages[locale]["run.parameterResetDefaults"], "string");
  assert.equal(typeof messages[locale]["run.parameterRevertRunning"], "string");
  assert.equal(typeof messages[locale]["run.parameterNoSchema"], "string");
  assert.equal(typeof messages[locale]["run.param.ball_count.label"], "string");
  assert.equal(typeof messages[locale]["run.param.ball_count.help"], "string");
  assert.equal(typeof messages[locale]["run.param.radius.label"], "string");
  assert.equal(typeof messages[locale]["run.param.radius.help"], "string");
  assert.equal(typeof messages[locale]["run.param.string_length.label"], "string");
  assert.equal(typeof messages[locale]["run.param.string_length.help"], "string");
  assert.equal(typeof messages[locale]["run.param.release_offset.label"], "string");
  assert.equal(typeof messages[locale]["run.param.release_offset.help"], "string");
  assert.equal(typeof messages[locale]["run.param.restitution.label"], "string");
  assert.equal(typeof messages[locale]["run.param.restitution.help"], "string");
  assert.equal(typeof messages[locale]["run.param.friction.label"], "string");
  assert.equal(typeof messages[locale]["run.param.friction.help"], "string");
  assert.equal(typeof messages[locale]["run.param.velocity_iterations.label"], "string");
  assert.equal(typeof messages[locale]["run.param.velocity_iterations.help"], "string");
  assert.equal(typeof messages[locale]["run.param.position_iterations.label"], "string");
  assert.equal(typeof messages[locale]["run.param.position_iterations.help"], "string");
  assert.equal(typeof messages[locale]["run.param.substeps_per_frame.label"], "string");
  assert.equal(typeof messages[locale]["run.param.substeps_per_frame.help"], "string");
  assert.equal(typeof messages[locale]["run.param.contact_position_correction.label"], "string");
  assert.equal(typeof messages[locale]["run.param.contact_position_correction.help"], "string");
  assert.equal(typeof messages[locale]["run.param.joint_velocity_projection.label"], "string");
  assert.equal(typeof messages[locale]["run.param.joint_velocity_projection.help"], "string");
  for (const key of [
    "columns",
    "rows",
    "box_width",
    "box_height",
    "gap_x",
    "gap_y",
    "layout",
    "material",
    "density",
    "spacing_x",
    "spacing_y",
    "node_radius",
    "constraint_profile",
  ]) {
    assert.equal(typeof messages[locale][`run.param.${key}.label`], "string");
    assert.equal(typeof messages[locale][`run.param.${key}.help`], "string");
  }
  assert.equal(typeof messages[locale]["run.option.disabled"], "string");
  assert.equal(typeof messages[locale]["run.option.baumgarte"], "string");
  assert.equal(typeof messages[locale]["run.option.ngs"], "string");
  assert.equal(typeof messages[locale]["panel.velocityPerturbation"], "string");
  assert.equal(typeof messages[locale]["panel.latticeProxy"], "string");
  assert.equal(typeof messages[locale]["timeline.lattice"], "string");
  assert.equal(typeof messages[locale]["inspector.latticeProxy"], "string");
  assert.equal(typeof messages[locale]["lattice.proxyLabel"], "string");
  assert.equal(typeof messages[locale]["lattice.notSoftBody"], "string");
  assert.equal(typeof messages[locale]["lattice.nodes"], "string");
  assert.equal(typeof messages[locale]["lattice.edges"], "string");
  assert.equal(typeof messages[locale]["lattice.anchors"], "string");
  assert.equal(typeof messages[locale]["lattice.currentLength"], "string");
  assert.equal(typeof messages[locale]["lattice.referenceLength"], "string");
  assert.equal(typeof messages[locale]["lattice.stretchRatio"], "string");
  assert.equal(typeof messages[locale]["lattice.maxStretchRatio"], "string");
  assert.equal(typeof messages[locale]["perturbation.absoluteVelocityHelp"], "string");
  assert.equal(typeof messages[locale]["perturbation.deltaX"], "string");
  assert.equal(typeof messages[locale]["perturbation.deltaY"], "string");
  assert.equal(typeof messages[locale]["perturbation.preview"], "string");
  assert.equal(typeof messages[locale]["perturbation.submit"], "string");
  assert.equal(typeof messages[locale]["perturbation.previewResult"], "string");
  assert.equal(typeof messages[locale]["perturbation.commitResult"], "string");
  assert.equal(typeof messages[locale]["perturbation.unavailable"], "string");
  assert.equal(typeof messages[locale]["perturbation.runningBlocked"], "string");
  assert.equal(typeof messages[locale]["perturbation.missingFrame"], "string");
  assert.equal(typeof messages[locale]["perturbation.currentFrameOnly"], "string");
  assert.equal(typeof messages[locale]["perturbation.selectedBody"], "string");
  assert.equal(typeof messages[locale]["perturbation.targetVelocity"], "string");
  assert.equal(typeof messages[locale]["perturbation.querySync"], "string");
  assert.equal(typeof messages[locale]["perturbation.rejectionReason"], "string");
  assert.equal(typeof messages[locale]["perturbation.sessionEpoch"], "string");
  assert.equal(typeof messages[locale]["perturbation.worldRevision"], "string");
  assert.equal(typeof messages[locale]["perturbation.currentFrameProvenance"], "string");
  assert.equal(typeof messages[locale]["perturbation.noProvenance"], "string");
}
assert.equal(t("zh-CN", "timeline.frameAt", { frame: 8 }), "第 8 帧");
assert.equal(t("en-US", "timeline.frameAt", { frame: 8 }), "frame 8");
assert.equal(messages["en-US"]["tooltip.startRunScenario"], "Start a new run");
assert.equal(messages["en-US"]["tooltip.rerunScenario"], "Rerun scenario");
assert.equal(messages["en-US"]["tooltip.playTimeline"], "Resume current session");
assert.equal(messages["en-US"]["tooltip.replayTimeline"], "Reset timeline to replay");
assert.equal(messages["en-US"]["run.generatingArtifact"], "Generating artifact...");
assert.equal(messages["en-US"]["run.startingLiveSession"], "Starting live session...");
assert.equal(scenarioParameterLabel("en-US", "ball_count"), "ball count");
assert.equal(scenarioParameterLabel("zh-CN", "ball_count"), "球数量");
assert.equal(scenarioParameterLabel("zh-CN", "columns"), "列数");
assert.equal(scenarioParameterLabel("zh-CN", "constraint_profile"), "约束档位");
assert.equal(scenarioParameterHelp("en-US", "contact_position_correction"), "Contact position correction strategy.");
assert.equal(scenarioParameterHelp("zh-CN", "contact_position_correction"), "接触位置修正策略。");
assert.match(scenarioParameterHelp("en-US", "constraint_profile"), /Soft, balanced, or hard/);
assert.equal(parameterSourceLabel("en-US", "effective"), "effective");
assert.equal(parameterSourceLabel("zh-CN", "effective"), "生效值");
assert.equal(messages["en-US"]["run.modeArtifact"], "Generate artifact + replay");
assert.equal(messages["en-US"]["run.modeLive"], "Live session");
assert.equal(messages["zh-CN"]["tooltip.startRunScenario"], "启动新运行");
assert.equal(messages["zh-CN"]["tooltip.rerunScenario"], "重新运行场景");
assert.equal(messages["zh-CN"]["tooltip.playTimeline"], "继续当前会话");
assert.equal(messages["zh-CN"]["tooltip.replayTimeline"], "回到起点后重新播放");
assert.equal(messages["zh-CN"]["run.generatingArtifact"], "正在生成产物...");
assert.equal(messages["zh-CN"]["run.startingLiveSession"], "正在创建实时会话...");
assert.equal(messages["zh-CN"]["run.modeArtifact"], "生成产物并回放");
assert.equal(messages["zh-CN"]["run.modeLive"], "实时会话");
assert.equal(bodyTypeLabel("zh-CN", "dynamic"), "动态");
assert.equal(statusLabel("zh-CN", "playing"), "播放中");
assert.equal(sourceLabel("en-US", "artifact"), "generate artifact + replay");
assert.equal(sourceLabel("en-US", "live"), "live session");
assert.equal(sourceLabel("zh-CN", "artifact"), "生成产物并回放");
assert.equal(sourceLabel("zh-CN", "live"), "实时会话");
assert.equal(layerLabel("zh-CN", "contacts"), "接触点");
assert.equal(layerLabel("zh-CN", "provenance"), "来源");
assert.equal(layerLabel("zh-CN", "stackStability"), "稳定性叠加");
assert.equal(layerLabel("zh-CN", "lattice"), "刚体格点代理");
assert.equal(layerLabel("zh-CN", "grid"), "网格");
assert.equal(layerLabel("zh-CN", "rulers"), "标尺");
assert.equal(messages["en-US"]["evidence.trajectory"], "trajectory (Web-derived)");
assert.equal(messages["en-US"]["evidence.stack"], "stack (Web-derived)");
assert.equal(messages["en-US"]["evidence.lattice"], "lattice proxy (not soft-body)");
assert.equal(messages["zh-CN"]["evidence.trajectory"], "轨迹（Web 派生）");
assert.equal(messages["zh-CN"]["evidence.stack"], "堆叠（Web 派生）");
assert.equal(messages["zh-CN"]["evidence.lattice"], "格点代理（非软体）");
assert.equal(scenarioGroupForId("falling_box_contact"), "basics");
assert.equal(scenarioGroupForId("stack_stability_tower"), "stack");
assert.equal(scenarioGroupForId("ccd_fast_convex_walls"), "ccd");
assert.equal(scenarioGroupForId("compound_provenance"), "compound");
assert.equal(scenarioGroupForId("lattice_grid"), "lattice");
assert.equal(scenarioGroupForId("broadphase_tree_probe"), "diagnostics");
assert.equal(scenarioGroupLabel("zh-CN", "lattice"), "格点代理");
assert.equal(overlayPresetLabel("en-US", "perturbationReview"), "Perturbation review");
assert.match(overlayPresetDescription("en-US", "latticeGrid"), /rigid-body lattice/i);
assert.equal(entityLabel("zh-CN", "body", 2), "物体 2");
assert.equal(dynamicValueLabel("zh-CN", "generic_convex_fallback"), "通用凸形回退");
assert.equal(dynamicValueLabel("en-US", "stability_window"), "stability window");
assert.equal(dynamicValueLabel("en-US", "epa_failure_contained"), "EPA failure contained");
assert.equal(dynamicValueLabel("zh-CN", "not_live_session"), "不是 live 会话");
assert.equal(dynamicValueLabel("en-US", "not_live_session"), "not live session");
assert.equal(dynamicValueLabel("zh-CN", "session_failed"), "会话已失败");
assert.equal(dynamicValueLabel("en-US", "session_failed"), "session failed");
assert.equal(dynamicValueLabel("zh-CN", "body_patch_failed"), "物体补丁应用失败");
assert.equal(dynamicValueLabel("en-US", "body_patch_failed"), "body patch failed");
assert.match(
  messages["zh-CN"]["perturbation.absoluteVelocityHelp"],
  /绝对速度扰动.*不是.*力.*扭矩.*鼠标关节/u,
);
assert.match(
  messages["en-US"]["perturbation.absoluteVelocityHelp"],
  /absolute velocity perturbation.*not .*force.*torque.*mouse joint/i,
);
assert.match(
  messages["zh-CN"]["perturbation.runningBlocked"],
  /先暂停.*paused-only/u,
);
assert.match(
  messages["en-US"]["perturbation.runningBlocked"],
  /pause the live session first.*paused-only/i,
);
assert.match(
  messages["zh-CN"]["perturbation.missingFrame"],
  /latest_frame.*create\/reset.*step 一次/u,
);
assert.match(
  messages["en-US"]["perturbation.missingFrame"],
  /latest frame.*create.*reset.*step once/i,
);
assert.match(
  messages["zh-CN"]["perturbation.currentFrameOnly"],
  /当前.*最新.*authoritative live frame/u,
);
assert.match(
  messages["en-US"]["perturbation.currentFrameOnly"],
  /current latest authoritative live frame/i,
);
assert.deepEqual(
  perturbationReasonUnion,
  [...expectedPerturbationRejectionReasons].sort(),
  "VelocityPerturbationRejectionReason should stay in exact parity with the backend rejection-reason contract.",
);
for (const locale of supportedLocales) {
  const dynamicReasonKeys = Object.keys(__dynamicValueLabels[locale]);
  for (const reason of expectedPerturbationRejectionReasons) {
    assert.ok(
      dynamicReasonKeys.includes(reason),
      `${locale} dynamicValueLabels should explicitly map ${reason}.`,
    );
    assert.notEqual(
      dynamicValueLabel(locale, reason),
      reason,
      `${locale} dynamicValueLabel should localize ${reason} instead of falling back to the raw key.`,
    );
  }
}
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
    id: "stack_stability_tower",
    name: "Stack stability tower",
    description: "Deterministic tower",
  },
  {
    id: "stack_stability_tower",
    name: "稳定性塔堆",
    description: "更复杂的确定性塔堆，用于观察接触峰值、岛休眠、漂移和稳定窗口。",
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
    id: "lattice_grid",
    name: "Rigid-body lattice grid proxy",
    description: "Rigid-body joint lattice grid proxy, not a true soft-body solver.",
  },
  {
    id: "lattice_grid",
    name: "刚体格点代理",
    description: "用许多刚体节点和距离关节近似网格形变，只是调试代理，不是 true soft-body 求解器。",
  },
);
assertLocalizedScenario(
  "zh-CN",
  {
    id: "newton_cradle",
    name: "Newton cradle",
    description: "Five suspended bouncy balls.",
  },
  {
    id: "newton_cradle",
    name: "牛顿摆",
    description: "五球悬挂碰撞场景，用于观察摆绳约束、接触传递和长时间动能包络。",
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

// --- Scenario watch-for copy: every backend scenario explains what to watch ---
const watchForScenarioIds = [
  "falling_box_contact", "stack_4", "stack_stability_tower", "matrix_stack",
  "matrix_stack_aligned", "newton_cradle", "joint_anchor", "lattice_grid",
  "broadphase_sparse", "sat_polygon", "compound_provenance", "concave_decomposition",
  "ccd_fast_circle_wall", "ccd_fast_convex_walls", "ccd_dynamic_convex_pair",
  "ccd_dynamic_compound_wall",
];
for (const scenarioId of watchForScenarioIds) {
  for (const locale of ["zh-CN", "en-US"]) {
    assert.equal(
      typeof scenarioWatchFor(locale, scenarioId),
      "string",
      `scenario ${scenarioId} must ship ${locale} watch-for copy`,
    );
  }
}
assert.equal(scenarioWatchFor("zh-CN", "unknown_scenario"), null);
for (const locale of ["zh-CN", "en-US"]) {
  assert.equal(typeof messages[locale]["scenario.watchForLabel"], "string");
}
