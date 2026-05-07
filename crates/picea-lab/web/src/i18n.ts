import type {
  DiagnosticMarkerKind,
  DiagnosticSeverity,
  DiagnosticSource,
  MissingEvidenceKind,
  ScenarioDescriptor,
} from "./types";

export const supportedLocales = ["zh-CN", "en-US"] as const;

export type Locale = (typeof supportedLocales)[number];
export type SourceKind = "demo" | "artifact" | "live";
export type StatusKind = "idle" | "loading" | "playing" | "paused" | "failed" | "created" | "running" | "completed";
export type EntityKind = "body" | "collider" | "contact" | "joint";
export type BodyType = "static" | "dynamic" | "kinematic";
export type ScenarioGroupId =
  | "basics"
  | "stack"
  | "ccd"
  | "compound"
  | "lattice"
  | "diagnostics";
export type OverlayPresetId =
  | "stackStability"
  | "trajectoryFocus"
  | "perturbationReview"
  | "latticeGrid";
export type LayerKey =
  | "grid"
  | "rulers"
  | "shapes"
  | "aabbs"
  | "contacts"
  | "velocities"
  | "trace"
  | "broadphaseTree"
  | "islands"
  | "provenance"
  | "stackStability"
  | "lattice";

const storageKey = "picea-lab.locale";

const enMessages = {
  "app.name": "picea-lab-web",
  "app.language": "Language",
  "app.session": "session",
  "app.runArtifact": "run artifact",
  "app.manifestArtifact": "manifest",
  "app.finalSnapshot": "final snapshot",
  "app.noSession": "no session",
  "app.noRunArtifact": "no run artifact",
  "scenario.select": "Select scenario",
  "scenario.group.basics": "Rigid body basics",
  "scenario.group.stack": "Stack stability",
  "scenario.group.ccd": "CCD",
  "scenario.group.compound": "Compound / provenance",
  "scenario.group.lattice": "Lattice proxy",
  "scenario.group.diagnostics": "Diagnostics",
  "tooltip.runScenario": "Run selected scenario",
  "tooltip.overlayPresets": "Overlay presets",
  "tooltip.pausePlayback": "Pause playback",
  "tooltip.playTimeline": "Play timeline",
  "tooltip.advanceFrame": "Advance one frame",
  "tooltip.resetTimeline": "Reset timeline",
  "tooltip.canvasLayers": "Canvas layers",
  "tooltip.canvasFit": "Fit canvas",
  "tooltip.canvasLockCore": "Lock onto core area",
  "tooltip.canvasUnlockCore": "Exit core lock",
  "tooltip.canvasReset": "Reset canvas view",
  "tooltip.canvasZoomIn": "Zoom in",
  "tooltip.canvasZoomOut": "Zoom out",
  "debug.copyContext": "Copy debug context",
  "debug.contextCopied": "Debug context copied.",
  "debug.contextCopyFailed": "Debug context copy failed: {message}",
  "debug.contextCopyFallback": "Debug context is shown below for manual copy.",
  "debug.contextPreview": "Debug context preview",
  "panel.sceneHierarchy": "Scene hierarchy",
  "panel.inspector": "Inspector",
  "panel.firstSliceFacts": "first slice facts",
  "panel.processFacts": "Process facts",
  "panel.trajectory": "Trajectory",
  "panel.stackStability": "Stack / Stability",
  "panel.velocityPerturbation": "Velocity perturbation",
  "panel.latticeProxy": "Lattice / Grid proxy",
  "tree.bodies": "Bodies",
  "tree.colliders": "Colliders",
  "tree.contacts": "Contacts",
  "tree.joints": "Joints",
  "tree.empty": "empty",
  "metric.bodies": "bodies",
  "metric.contacts": "contacts",
  "metric.dt": "dt",
  "metric.step": "step",
  "metric.simTime": "sim time",
  "metric.gravity": "gravity",
  "metric.manifolds": "manifolds",
  "inspector.measurementStatus": "Measurement status",
  "inspector.stageFacts": "Stage facts",
  "inspector.pendingMeasurements": "Pending measurements",
  "inspector.unmeasured": "unmeasured",
  "inspector.forces": "forces",
  "inspector.torques": "torques",
  "inspector.broadphaseCandidates": "broadphase candidates",
  "inspector.warmStart": "warm-start",
  "inspector.ccd": "CCD cand/hit/miss/clamp",
  "inspector.broadphaseTree": "broadphase tree",
  "inspector.treeShape": "tree shape",
  "inspector.treeCounters": "tree counters",
  "inspector.islandLifecycle": "island lifecycle",
  "inspector.compoundProvenance": "compound provenance",
  "inspector.latticeProxy": "lattice proxy",
  "inspector.authoredBody": "authored body",
  "inspector.generatedPieces": "generated pieces",
  "inspector.inheritance": "inheritance",
  "inspector.validationPath": "validation path",
  "inspector.pieceOrder": "piece order",
  "inspector.colliderHandle": "collider handle",
  "inspector.localPose": "local pose",
  "inspector.material": "material",
  "inspector.filter": "filter",
  "inspector.density": "density",
  "inspector.sensor": "sensor",
  "inspector.treeEmpty": "no exported tree nodes",
  "inspector.provenanceEmpty": "no authored compound provenance in this frame",
  "inspector.islandEmpty": "no island facts exported in this frame",
  "inspector.emptyTitle": "Nothing selected",
  "inspector.emptySelection": "Select a body, collider, contact, or joint in the hierarchy or canvas.",
  "inspector.emptyHintEsc": "clears the current selection",
  "fact.type": "type",
  "fact.position": "position",
  "fact.mass": "mass",
  "fact.inverseMass": "inverse mass",
  "fact.centerOfMass": "center of mass",
  "fact.inertia": "inertia",
  "fact.inverseInertia": "inverse inertia",
  "fact.linearVelocity": "linear velocity",
  "fact.angularVelocity": "angular velocity",
  "fact.sleeping": "sleeping",
  "fact.body": "body",
  "fact.shape": "shape",
  "fact.center": "center",
  "fact.friction": "friction",
  "fact.restitution": "restitution",
  "fact.sensor": "sensor",
  "fact.ownerVelocity": "owner velocity",
  "fact.point": "point",
  "fact.normal": "normal",
  "fact.depth": "depth",
  "fact.feature": "feature",
  "fact.reduction": "reduction",
  "fact.genericFallback": "generic fallback",
  "fact.gjk": "GJK",
  "fact.epa": "EPA",
  "fact.simplex": "simplex",
  "fact.warmStartNormal": "warm-start normal",
  "fact.warmStartTangent": "warm-start tangent",
  "fact.solverNormal": "solver normal",
  "fact.solverTangent": "solver tangent",
  "fact.normalClamped": "normal clamped",
  "fact.tangentClamped": "tangent clamped",
  "fact.ccdToi": "CCD TOI",
  "fact.ccdAdvancement": "CCD advancement",
  "fact.ccdClamp": "CCD clamp",
  "fact.ccdTargetKind": "CCD target kind",
  "fact.ccdTargetClamp": "CCD target clamp",
  "fact.ccdSlop": "CCD slop",
  "fact.ccdSweptStart": "swept start",
  "fact.ccdSweptEnd": "swept end",
  "fact.ccdTargetSweptStart": "target swept start",
  "fact.ccdTargetSweptEnd": "target swept end",
  "fact.ccdToiPoint": "TOI point",
  "fact.kind": "kind",
  "fact.anchors": "anchors",
  "common.unknown": "unknown",
  "common.true": "true",
  "common.false": "false",
  "contact.applied": "applied",
  "contact.suppressed": "suppressed",
  "timeline.timeline": "Timeline",
  "timeline.trajectory": "Trajectory",
  "timeline.stack": "Stack",
  "timeline.lattice": "Lattice",
  "timeline.logs": "Logs",
  "timeline.runSetup": "Run setup",
  "timeline.diagnostics": "Diagnostics",
  "timeline.evidence": "Evidence",
  "timeline.frameAt": "frame {frame}",
  "timeline.totalFrames": "{count} total",
  "timeline.sourceStatus": "{source} / {status}",
  "timeline.sessionStatus": "session {sessionId} / buffered {buffered} / current {current}",
  "timeline.liveCadence": "live {actual}/{target} fps · {stepMs}ms",
  "timeline.liveCadenceDegraded": "degraded {actual}/{target} fps · {stepMs}ms",
  "timeline.liveCadencePending": "live measuring",
  "diagnostics.empty": "No exported diagnostics for this frame.",
  "diagnostics.exported": "exported diagnostics",
  "diagnostics.performance": "performance",
  "diagnostics.stability": "stability",
  "diagnostics.markers": "markers",
  "diagnostics.missingEvidence": "missing evidence",
  "diagnostics.current": "current",
  "diagnostics.delta": "delta",
  "diagnostics.events": "events",
  "diagnostics.noEvents": "no exported events",
  "diagnostics.report": "step report",
  "diagnostics.stateHash": "state hash",
  "diagnostics.counterDelta": "counter delta",
  "diagnostics.penetration": "penetration",
  "diagnostics.contactChurn": "contact churn",
  "diagnostics.warmStart": "warm-start",
  "diagnostics.impulse": "impulse",
  "diagnostics.sleep": "sleep",
  "diagnostics.island": "island",
  "diagnostics.threshold": "threshold",
  "diagnostics.score": "score",
  "diagnostics.evidenceFields": "evidence fields",
  "diagnostics.source": "source",
  "diagnostics.source.rustAuthoritative": "Rust authoritative",
  "diagnostics.source.labDerived": "Lab derived",
  "diagnostics.source.webDerived": "Web derived",
  "diagnostics.source.missing": "Missing",
  "diagnostics.severity.info": "Info",
  "diagnostics.severity.warning": "Warning",
  "diagnostics.severity.severe": "Severe",
  "diagnostics.marker.performanceCounterSpike": "counter spike",
  "diagnostics.marker.solverRowSpike": "solver row spike",
  "diagnostics.marker.ccdSpike": "CCD spike",
  "diagnostics.marker.numericWarning": "numeric warning",
  "diagnostics.marker.penetrationSpike": "penetration spike",
  "diagnostics.marker.contactChurnSpike": "contact churn spike",
  "diagnostics.marker.warmStartDropSpike": "warm-start drop spike",
  "diagnostics.marker.sleepTransition": "sleep transition",
  "diagnostics.marker.sleepNeverConverged": "sleep never converged",
  "diagnostics.marker.angularDriftSpike": "angular drift spike",
  "diagnostics.marker.bodyDriftSpike": "body drift spike",
  "diagnostics.missing.previousFrame": "previous frame",
  "diagnostics.missing.contactFacts": "contact facts",
  "diagnostics.missing.contactIds": "contact ids",
  "diagnostics.missing.sleepEvents": "sleep events",
  "diagnostics.missing.islandFacts": "island facts",
  "evidence.source": "source",
  "evidence.scenario": "scenario",
  "evidence.stateHash": "state hash",
  "evidence.session": "session",
  "evidence.run": "run",
  "evidence.frame": "frame",
  "evidence.selection": "selection",
  "evidence.layers": "layers",
  "evidence.camera": "camera",
  "evidence.center": "center",
  "evidence.target": "target",
  "evidence.targetBounds": "bounds",
  "evidence.damping": "damping",
  "evidence.grid": "grid",
  "evidence.rulers": "rulers",
  "evidence.perfStatus": "perf",
  "evidence.perfSummary": "perf summary",
  "evidence.trajectory": "trajectory (Web-derived)",
  "evidence.stack": "stack (Web-derived)",
  "evidence.diagnostics": "diagnostics",
  "evidence.lattice": "lattice proxy (not soft-body)",
  "evidence.perturbation": "perturbation",
  "evidence.noArtifact": "No persistent perf artifact for this source.",
  "overlay.preset.stackStability": "Stack stability",
  "overlay.preset.trajectoryFocus": "Trajectory focus",
  "overlay.preset.perturbationReview": "Perturbation review",
  "overlay.preset.latticeGrid": "Lattice grid",
  "overlay.presetDesc.stackStability":
    "Stack overlay plus island-colored trajectories for stability review.",
  "overlay.presetDesc.trajectoryFocus":
    "Selected-body trajectory focus with dense sampling and reduced chrome.",
  "overlay.presetDesc.perturbationReview":
    "Velocity edit review with provenance, contacts, and recent traces.",
  "overlay.presetDesc.latticeGrid":
    "Rigid-body lattice proxy emphasis for node, edge, island, and stretch inspection.",
  "perf.missing": "perf.json missing",
  "perf.available": "perf.json available",
  "perf.liveUnavailable": "live source (no perf.json)",
  "perf.elapsed": "elapsed micros",
  "perf.finalHash": "final hash",
  "run.frameCount": "frame count",
  "run.mode": "run mode",
  "run.modeArtifact": "Rust artifact replay",
  "run.modeLive": "Rust live session",
  "run.gravityOverride": "gravity override",
  "run.sendOverride": "send override with next run",
  "run.gravityY": "gravity y",
  "perturbation.absoluteVelocityHelp":
    "Paused-only absolute velocity perturbation. This is not a continuous force, torque, or mouse joint control.",
  "perturbation.preview": "Preview",
  "perturbation.submit": "Submit",
  "perturbation.previewResult": "Preview result",
  "perturbation.commitResult": "Commit result",
  "perturbation.unavailable": "Unavailable",
  "perturbation.liveOnly": "Available only in a Rust live session.",
  "perturbation.selectDynamicBody":
    "Select a dynamic body (or a collider owned by one) to preview a paused velocity perturbation.",
  "perturbation.runningBlocked":
    "Pause the live session first. Submit is paused-only and does not apply continuous force or torque while running.",
  "perturbation.completedBlocked":
    "Completed live sessions cannot preview or submit velocity perturbations.",
  "perturbation.failedBlocked":
    "This live session already failed (session_failed), so preview and submit stay disabled.",
  "perturbation.missingFrame":
    "No authoritative latest frame is available yet. After create or reset, step once before preview or submit.",
  "perturbation.currentFrameOnly":
    "Preview and submit are allowed only on the current latest authoritative live frame. Scrub back to the live edge before editing velocity.",
  "perturbation.pausedOnly":
    "Preview and submit are paused-only for the current authoritative frame.",
  "perturbation.pausedOnlyBadge": "paused-only",
  "perturbation.liveOnlyBadge": "live-only",
  "perturbation.deltaX": "delta x",
  "perturbation.deltaY": "delta y",
  "perturbation.selectedBody": "target body",
  "perturbation.beforeVelocity": "before velocity",
  "perturbation.delta": "requested delta",
  "perturbation.targetVelocity": "target velocity",
  "perturbation.querySync": "query sync",
  "perturbation.rejectionReason": "rejection reason",
  "perturbation.sessionEpoch": "session epoch",
  "perturbation.worldRevision": "world revision",
  "perturbation.frameIndex": "frame index",
  "perturbation.actionId": "action id",
  "perturbation.wakeIntent": "wake intent",
  "perturbation.commitOutcome": "commit outcome",
  "perturbation.currentFrameProvenance": "current frame provenance",
  "perturbation.noProvenance":
    "No accepted velocity perturbation provenance is attached to the selected body in this frame.",
  "perturbation.requestError": "request error",
  "lattice.proxyLabel": "Rigid-body joint lattice / grid proxy",
  "lattice.notSoftBody": "This is not a true soft-body, cloth, or FEM solver.",
  "lattice.nodes": "nodes",
  "lattice.edges": "edges",
  "lattice.anchors": "anchors",
  "lattice.currentLength": "current length",
  "lattice.referenceLength": "reference length",
  "lattice.stretchRatio": "stretch ratio",
  "lattice.maxStretchRatio": "max stretch ratio",
  "trajectory.mode": "trajectory mode",
  "trajectory.historyLength": "history length",
  "trajectory.samplingStride": "sampling stride",
  "trajectory.fade": "fade",
  "trajectory.colorMode": "color mode",
  "trajectory.mode.selectedBody": "selected body",
  "trajectory.mode.allDynamic": "all dynamic",
  "trajectory.mode.selectedIsland": "selected island",
  "trajectory.mode.contacts": "contacts",
  "trajectory.mode.ccd": "CCD",
  "trajectory.colorMode.body": "body",
  "trajectory.colorMode.island": "island",
  "trajectory.empty.selectedBody": "Select a body or collider to render a body trajectory.",
  "trajectory.empty.allDynamic": "No dynamic bodies are available in the current frame.",
  "trajectory.empty.selectedIsland": "Select an entity with an exported island to render island trajectories.",
  "trajectory.empty.contacts": "No contact lineage is available in the current trail window.",
  "trajectory.empty.ccd": "No CCD sweep evidence is available in the current trail window.",
  "trajectory.empty.unsupportedSelection": "The current selection does not map to a trajectory target.",
  "trajectory.marker.bodyMotion": "body motion",
  "trajectory.marker.contactAppeared": "contact appeared",
  "trajectory.marker.contactDisappeared": "contact disappeared",
  "trajectory.marker.contactBurst": "contact burst",
  "trajectory.marker.ccdClamp": "CCD clamp",
  "trajectory.marker.ccdHit": "CCD hit",
  "trajectory.summary.derived": "summary",
  "trajectory.summary.distance": "distance traveled",
  "trajectory.summary.maxSpeed": "max speed",
  "trajectory.summary.awakeSleep": "awake / sleep",
  "trajectory.summary.contactCount": "contact count",
  "trajectory.summary.ccdEvidence": "CCD evidence",
  "trajectory.summary.window": "window",
  "trajectory.summary.mode": "mode",
  "stability.derived": "web-derived from exported frames",
  "stability.missing": "missing evidence",
  "stability.contribution": "stability contribution",
  "stability.overlay": "stack stability overlay",
  "stability.dynamicBodies": "dynamic",
  "stability.awakeSleep": "awake / sleep",
  "stability.contactSpike": "contact spike",
  "stability.impulses": "impulse N / T",
  "stability.solverRows": "solver rows",
  "stability.islands": "active / sleep islands",
  "stability.maxDrift": "max drift",
  "stability.maxAngularDrift": "max angular drift",
  "stability.jitter": "jitter proxy",
  "stability.quietWindow": "quiet window",
  "stability.window": "window",
  "stability.marker.contactSpike": "contact spike",
  "stability.marker.wakeTransition": "wake / sleep",
  "stability.marker.driftSpike": "drift",
  "stability.marker.angularDrift": "angular drift",
  "stability.marker.solverSpike": "solver rows",
  "stability.marker.quietWindow": "quiet window",
  "stability.contactFrames": "contact frames",
  "stability.awakeTransitions": "wake transitions",
  "stability.pairFrames": "pair frames",
  "stability.bodyIds": "body ids",
  "stability.islandId": "island",
  "stability.sleepingBodies": "sleeping bodies",
  "canvas.frame": "frame",
  "canvas.colliders": "colliders",
  "canvas.contacts": "contacts",
  "canvas.zoom": "zoom",
  "canvas.scale": "scale",
  "canvas.mode": "mode",
  "canvas.modeFree": "free",
  "canvas.modeLockedCore": "locked core",
  "canvas.targetActive": "active dynamic bodies / contacts",
  "canvas.targetAllColliders": "all collider bounds",
  "canvas.targetScene": "scene bounds fallback",
  "error.sessionWithoutRun": "session completed without run_id",
  "error.emptyFrames": "frames.jsonl was empty",
  "log.serverNotConfirmed": "Rust server not confirmed yet; showing built-in demo frames.",
  "log.connectedScenarios": "Connected to /api/scenarios.",
  "log.serverUnavailable": "Server unavailable: {message}. Demo fallback is active.",
  "log.loadedFrames": "Loaded {count} frames from run {runId}.",
  "log.sessionStatus": "Session {sessionId} status: {status}.",
  "log.serverRunFailed": "Server run failed; switched to demo fallback ({message}).",
  "log.generatedDemoFrames": "Generated {count} local demo frames for {scenarioId}.",
  "log.artifactsAvailable": "Replay artifacts: {manifest}, {finalSnapshot}.",
  "log.finalSnapshotLoaded": "Final snapshot loaded at step {step}.",
  "log.liveSessionReady": "Live session {sessionId} is ready; backend step will append frames on demand.",
  "log.liveFrameBuffered": "Live frame {frameIndex} buffered from Rust backend.",
  "log.sseFrame": "SSE frame {data}",
  "log.sseFailed": "SSE failed {data}",
  "log.sseIdle": "SSE idle {data}",
  "log.sseUnavailable": "SSE unavailable: {message}",
  "log.serverAccepted": "Server accepted {action}: {status}.",
  "log.serverControlFailed": "Server {action} failed: {message}.",
  "group.transform": "Transform",
  "group.massProperties": "Mass properties",
  "group.velocities": "Velocities",
  "group.material": "Material",
  "group.contactInfo": "Contact info",
  "group.warmStart": "Warm start",
  "group.solver": "Solver",
  "group.trace": "Trace",
  "group.ccdTrace": "CCD trace",
  "group.jointInfo": "Joint info",
} as const;

export type MessageKey = keyof typeof enMessages;

const zhMessages: Record<MessageKey, string> = {
  "app.name": "picea-lab-web",
  "app.language": "语言",
  "app.session": "会话",
  "app.runArtifact": "运行产物",
  "app.manifestArtifact": "manifest",
  "app.finalSnapshot": "final_snapshot",
  "app.noSession": "无会话",
  "app.noRunArtifact": "无运行产物",
  "scenario.select": "选择场景",
  "scenario.group.basics": "刚体基础",
  "scenario.group.stack": "堆叠稳定性",
  "scenario.group.ccd": "CCD",
  "scenario.group.compound": "复合体 / 来源",
  "scenario.group.lattice": "格点代理",
  "scenario.group.diagnostics": "诊断",
  "tooltip.runScenario": "运行当前场景",
  "tooltip.overlayPresets": "叠加预设",
  "tooltip.pausePlayback": "暂停播放",
  "tooltip.playTimeline": "播放时间线",
  "tooltip.advanceFrame": "前进一帧",
  "tooltip.resetTimeline": "重置时间线",
  "tooltip.canvasLayers": "画布图层",
  "tooltip.canvasFit": "适配画布",
  "tooltip.canvasLockCore": "锁定核心区域",
  "tooltip.canvasUnlockCore": "退出核心锁定",
  "tooltip.canvasReset": "重置画布视图",
  "tooltip.canvasZoomIn": "放大",
  "tooltip.canvasZoomOut": "缩小",
  "debug.copyContext": "复制调试上下文",
  "debug.contextCopied": "已复制调试上下文。",
  "debug.contextCopyFailed": "复制调试上下文失败：{message}",
  "debug.contextCopyFallback": "调试上下文已显示在下方，可手动复制。",
  "debug.contextPreview": "调试上下文预览",
  "panel.sceneHierarchy": "场景层级",
  "panel.inspector": "检查器",
  "panel.firstSliceFacts": "首个切片事实",
  "panel.processFacts": "过程事实",
  "panel.trajectory": "轨迹",
  "panel.stackStability": "堆叠 / 稳定性",
  "panel.velocityPerturbation": "速度扰动",
  "panel.latticeProxy": "格点 / 网格代理",
  "tree.bodies": "物体",
  "tree.colliders": "碰撞体",
  "tree.contacts": "接触点",
  "tree.joints": "关节",
  "tree.empty": "空",
  "metric.bodies": "物体",
  "metric.contacts": "接触点",
  "metric.dt": "dt",
  "metric.step": "步数",
  "metric.simTime": "模拟时间",
  "metric.gravity": "重力",
  "metric.manifolds": "流形",
  "inspector.measurementStatus": "测量状态",
  "inspector.stageFacts": "阶段事实",
  "inspector.pendingMeasurements": "待测量项",
  "inspector.unmeasured": "未测量",
  "inspector.forces": "力",
  "inspector.torques": "扭矩",
  "inspector.broadphaseCandidates": "宽阶段候选",
  "inspector.warmStart": "暖启动",
  "inspector.ccd": "CCD 候选/命中/错过/钳制",
  "inspector.broadphaseTree": "宽阶段树",
  "inspector.treeShape": "树形态",
  "inspector.treeCounters": "树计数器",
  "inspector.islandLifecycle": "岛生命周期",
  "inspector.compoundProvenance": "复合体来源",
  "inspector.latticeProxy": "格点代理",
  "inspector.authoredBody": "作者体",
  "inspector.generatedPieces": "生成片段",
  "inspector.inheritance": "继承",
  "inspector.validationPath": "校验路径",
  "inspector.pieceOrder": "片段顺序",
  "inspector.colliderHandle": "碰撞体句柄",
  "inspector.localPose": "局部位姿",
  "inspector.material": "材质",
  "inspector.filter": "过滤层",
  "inspector.density": "密度",
  "inspector.sensor": "传感器",
  "inspector.treeEmpty": "当前帧没有导出的树节点",
  "inspector.provenanceEmpty": "当前帧没有作者提供的复合体来源",
  "inspector.islandEmpty": "当前帧没有导出的岛事实",
  "inspector.emptyTitle": "未选中实体",
  "inspector.emptySelection": "在层级树或画布中选择物体、碰撞体、接触点或关节。",
  "inspector.emptyHintEsc": "清除当前选中",
  "fact.type": "类型",
  "fact.position": "位置",
  "fact.mass": "质量",
  "fact.inverseMass": "逆质量",
  "fact.centerOfMass": "质心",
  "fact.inertia": "转动惯量",
  "fact.inverseInertia": "逆转动惯量",
  "fact.linearVelocity": "线速度",
  "fact.angularVelocity": "角速度",
  "fact.sleeping": "休眠",
  "fact.body": "物体",
  "fact.shape": "形状",
  "fact.center": "中心",
  "fact.friction": "摩擦",
  "fact.restitution": "反弹",
  "fact.sensor": "传感器",
  "fact.ownerVelocity": "所属物体速度",
  "fact.point": "点",
  "fact.normal": "法线",
  "fact.depth": "深度",
  "fact.feature": "特征",
  "fact.reduction": "归约",
  "fact.genericFallback": "通用回退",
  "fact.gjk": "GJK",
  "fact.epa": "EPA",
  "fact.simplex": "单纯形",
  "fact.warmStartNormal": "暖启动法向",
  "fact.warmStartTangent": "暖启动切向",
  "fact.solverNormal": "求解器法向",
  "fact.solverTangent": "求解器切向",
  "fact.normalClamped": "法向被钳制",
  "fact.tangentClamped": "切向被钳制",
  "fact.ccdToi": "CCD TOI",
  "fact.ccdAdvancement": "CCD 推进",
  "fact.ccdClamp": "CCD 钳制",
  "fact.ccdTargetKind": "CCD 目标类型",
  "fact.ccdTargetClamp": "CCD 目标钳制",
  "fact.ccdSlop": "CCD slop",
  "fact.ccdSweptStart": "扫掠起点",
  "fact.ccdSweptEnd": "扫掠终点",
  "fact.ccdTargetSweptStart": "目标扫掠起点",
  "fact.ccdTargetSweptEnd": "目标扫掠终点",
  "fact.ccdToiPoint": "TOI 点",
  "fact.kind": "类型",
  "fact.anchors": "锚点",
  "common.unknown": "未知",
  "common.true": "是",
  "common.false": "否",
  "contact.applied": "已应用",
  "contact.suppressed": "已抑制",
  "timeline.timeline": "时间线",
  "timeline.trajectory": "轨迹",
  "timeline.stack": "堆叠",
  "timeline.lattice": "格点",
  "timeline.logs": "日志",
  "timeline.runSetup": "运行设置",
  "timeline.diagnostics": "诊断",
  "timeline.evidence": "证据",
  "timeline.frameAt": "第 {frame} 帧",
  "timeline.totalFrames": "共 {count} 帧",
  "timeline.sourceStatus": "{source} / {status}",
  "timeline.sessionStatus": "会话 {sessionId} / 已缓存 {buffered} / 当前 {current}",
  "timeline.liveCadence": "live {actual}/{target} fps · {stepMs}ms",
  "timeline.liveCadenceDegraded": "降级 {actual}/{target} fps · {stepMs}ms",
  "timeline.liveCadencePending": "live 测量中",
  "diagnostics.empty": "当前帧没有导出的诊断事实。",
  "diagnostics.exported": "导出诊断",
  "diagnostics.performance": "性能",
  "diagnostics.stability": "稳定性",
  "diagnostics.markers": "标记",
  "diagnostics.missingEvidence": "缺失证据",
  "diagnostics.current": "当前值",
  "diagnostics.delta": "增量",
  "diagnostics.events": "事件",
  "diagnostics.noEvents": "无导出事件",
  "diagnostics.report": "step report",
  "diagnostics.stateHash": "state hash",
  "diagnostics.counterDelta": "计数增量",
  "diagnostics.penetration": "穿透",
  "diagnostics.contactChurn": "接触 churn",
  "diagnostics.warmStart": "暖启动",
  "diagnostics.impulse": "冲量",
  "diagnostics.sleep": "休眠",
  "diagnostics.island": "岛",
  "diagnostics.threshold": "阈值",
  "diagnostics.score": "分值",
  "diagnostics.evidenceFields": "证据字段",
  "diagnostics.source": "来源",
  "diagnostics.source.rustAuthoritative": "Rust 权威事实",
  "diagnostics.source.labDerived": "Lab 派生",
  "diagnostics.source.webDerived": "Web 派生",
  "diagnostics.source.missing": "缺失",
  "diagnostics.severity.info": "信息",
  "diagnostics.severity.warning": "警告",
  "diagnostics.severity.severe": "严重",
  "diagnostics.marker.performanceCounterSpike": "计数尖峰",
  "diagnostics.marker.solverRowSpike": "求解行尖峰",
  "diagnostics.marker.ccdSpike": "CCD 尖峰",
  "diagnostics.marker.numericWarning": "数值警告",
  "diagnostics.marker.penetrationSpike": "穿透尖峰",
  "diagnostics.marker.contactChurnSpike": "接触 churn 尖峰",
  "diagnostics.marker.warmStartDropSpike": "暖启动丢弃尖峰",
  "diagnostics.marker.sleepTransition": "休眠切换",
  "diagnostics.marker.sleepNeverConverged": "休眠未收敛",
  "diagnostics.marker.angularDriftSpike": "角漂移尖峰",
  "diagnostics.marker.bodyDriftSpike": "位移漂移尖峰",
  "diagnostics.missing.previousFrame": "上一帧",
  "diagnostics.missing.contactFacts": "接触事实",
  "diagnostics.missing.contactIds": "接触 id",
  "diagnostics.missing.sleepEvents": "休眠事件",
  "diagnostics.missing.islandFacts": "岛事实",
  "evidence.source": "来源",
  "evidence.scenario": "场景",
  "evidence.stateHash": "状态哈希",
  "evidence.session": "会话",
  "evidence.run": "运行",
  "evidence.frame": "帧",
  "evidence.selection": "选择",
  "evidence.layers": "图层",
  "evidence.camera": "镜头",
  "evidence.center": "中心",
  "evidence.target": "目标",
  "evidence.targetBounds": "边界",
  "evidence.damping": "阻尼",
  "evidence.grid": "网格",
  "evidence.rulers": "标尺",
  "evidence.perfStatus": "perf",
  "evidence.perfSummary": "perf 摘要",
  "evidence.trajectory": "轨迹（Web 派生）",
  "evidence.stack": "堆叠（Web 派生）",
  "evidence.diagnostics": "诊断",
  "evidence.lattice": "格点代理（非软体）",
  "evidence.perturbation": "扰动",
  "evidence.noArtifact": "当前来源没有持久 perf 产物。",
  "overlay.preset.stackStability": "堆叠稳定性",
  "overlay.preset.trajectoryFocus": "轨迹聚焦",
  "overlay.preset.perturbationReview": "扰动复核",
  "overlay.preset.latticeGrid": "格点网格",
  "overlay.presetDesc.stackStability":
    "突出稳定性图层，并用按 island 着色的轨迹帮助复核堆叠。",
  "overlay.presetDesc.trajectoryFocus":
    "聚焦选中物体轨迹，采样更密，减少无关画布干扰。",
  "overlay.presetDesc.perturbationReview":
    "用于绝对速度扰动复核，强调 provenance、接触和近期轨迹。",
  "overlay.presetDesc.latticeGrid":
    "突出刚体格点代理的节点、边、island 和拉伸信息。",
  "perf.missing": "缺少 perf.json",
  "perf.available": "已读取 perf.json",
  "perf.liveUnavailable": "live 来源（无 perf.json）",
  "perf.elapsed": "耗时 micros",
  "perf.finalHash": "最终 hash",
  "run.frameCount": "帧数",
  "run.mode": "运行模式",
  "run.modeArtifact": "Rust 产物回放",
  "run.modeLive": "Rust 实时会话",
  "run.gravityOverride": "重力覆盖",
  "run.sendOverride": "下次运行发送覆盖",
  "run.gravityY": "重力 y",
  "perturbation.absoluteVelocityHelp":
    "这是 paused-only 的绝对速度扰动，不是连续力、扭矩或鼠标关节控制。",
  "perturbation.preview": "预览",
  "perturbation.submit": "提交",
  "perturbation.previewResult": "预览结果",
  "perturbation.commitResult": "提交结果",
  "perturbation.unavailable": "当前不可用",
  "perturbation.liveOnly": "仅 Rust 实时会话支持该操作。",
  "perturbation.selectDynamicBody":
    "请选择动态物体，或选择其所属的碰撞体，再预览 paused-only 速度扰动。",
  "perturbation.runningBlocked":
    "请先暂停实时会话。提交只允许 paused-only 当前帧，不会在运行中持续施加力或扭矩。",
  "perturbation.completedBlocked": "已完成的实时会话不能再预览或提交速度扰动。",
  "perturbation.failedBlocked":
    "这个实时会话已经失败（session_failed），所以预览和提交都会保持禁用。",
  "perturbation.missingFrame":
    "当前还没有 authoritative latest_frame。create/reset 之后请先 step 一次，再预览或提交。",
  "perturbation.currentFrameOnly":
    "预览和提交只允许当前最新的 authoritative live frame。请先回到实时缓冲区末尾，再编辑速度。",
  "perturbation.pausedOnly": "预览和提交都只针对当前 authoritative frame 的 paused-only 编辑。",
  "perturbation.pausedOnlyBadge": "仅暂停态",
  "perturbation.liveOnlyBadge": "仅 live",
  "perturbation.deltaX": "x 增量",
  "perturbation.deltaY": "y 增量",
  "perturbation.selectedBody": "目标物体",
  "perturbation.beforeVelocity": "编辑前速度",
  "perturbation.delta": "请求增量",
  "perturbation.targetVelocity": "目标速度",
  "perturbation.querySync": "查询同步",
  "perturbation.rejectionReason": "拒绝原因",
  "perturbation.sessionEpoch": "会话 epoch",
  "perturbation.worldRevision": "世界 revision",
  "perturbation.frameIndex": "帧索引",
  "perturbation.actionId": "动作 ID",
  "perturbation.wakeIntent": "唤醒意图",
  "perturbation.commitOutcome": "提交结果",
  "perturbation.currentFrameProvenance": "当前帧扰动来源",
  "perturbation.noProvenance": "当前帧里，选中物体还没有已接受的速度扰动 provenance。",
  "perturbation.requestError": "请求错误",
  "lattice.proxyLabel": "刚体关节格点 / 网格代理",
  "lattice.notSoftBody": "这不是 true soft-body、cloth 或 FEM 求解器。",
  "lattice.nodes": "节点",
  "lattice.edges": "边",
  "lattice.anchors": "锚点",
  "lattice.currentLength": "当前长度",
  "lattice.referenceLength": "参考长度",
  "lattice.stretchRatio": "拉伸比",
  "lattice.maxStretchRatio": "最大拉伸比",
  "trajectory.mode": "轨迹模式",
  "trajectory.historyLength": "历史长度",
  "trajectory.samplingStride": "采样步长",
  "trajectory.fade": "淡出",
  "trajectory.colorMode": "颜色模式",
  "trajectory.mode.selectedBody": "选中物体",
  "trajectory.mode.allDynamic": "全部动态体",
  "trajectory.mode.selectedIsland": "选中岛",
  "trajectory.mode.contacts": "接触点",
  "trajectory.mode.ccd": "CCD",
  "trajectory.colorMode.body": "按物体",
  "trajectory.colorMode.island": "按岛",
  "trajectory.empty.selectedBody": "请选择物体或碰撞体来显示物体轨迹。",
  "trajectory.empty.allDynamic": "当前帧没有可用的动态物体。",
  "trajectory.empty.selectedIsland": "请选择带导出岛信息的实体来显示岛轨迹。",
  "trajectory.empty.contacts": "当前轨迹窗口里没有可用的接触谱系。",
  "trajectory.empty.ccd": "当前轨迹窗口里没有可用的 CCD 扫掠证据。",
  "trajectory.empty.unsupportedSelection": "当前选择无法映射到轨迹目标。",
  "trajectory.marker.bodyMotion": "物体跃迁",
  "trajectory.marker.contactAppeared": "接触出现",
  "trajectory.marker.contactDisappeared": "接触消失",
  "trajectory.marker.contactBurst": "接触爆发",
  "trajectory.marker.ccdClamp": "CCD 钳制",
  "trajectory.marker.ccdHit": "CCD 命中",
  "trajectory.summary.derived": "摘要",
  "trajectory.summary.distance": "移动距离",
  "trajectory.summary.maxSpeed": "最大速度",
  "trajectory.summary.awakeSleep": "唤醒 / 休眠",
  "trajectory.summary.contactCount": "接触计数",
  "trajectory.summary.ccdEvidence": "CCD 证据",
  "trajectory.summary.window": "窗口",
  "trajectory.summary.mode": "模式",
  "stability.derived": "由导出帧聚合的 Web 派生显示",
  "stability.missing": "缺少证据",
  "stability.contribution": "稳定性贡献",
  "stability.overlay": "稳定性叠加",
  "stability.dynamicBodies": "动态体",
  "stability.awakeSleep": "唤醒 / 休眠",
  "stability.contactSpike": "接触峰值",
  "stability.impulses": "法 / 切冲量",
  "stability.solverRows": "求解行",
  "stability.islands": "活动 / 休眠岛",
  "stability.maxDrift": "最大漂移",
  "stability.maxAngularDrift": "最大角漂移",
  "stability.jitter": "抖动代理",
  "stability.quietWindow": "安静窗口",
  "stability.window": "窗口",
  "stability.marker.contactSpike": "接触峰值",
  "stability.marker.wakeTransition": "唤醒 / 休眠",
  "stability.marker.driftSpike": "漂移",
  "stability.marker.angularDrift": "角漂移",
  "stability.marker.solverSpike": "求解行",
  "stability.marker.quietWindow": "安静窗口",
  "stability.contactFrames": "接触帧数",
  "stability.awakeTransitions": "唤醒切换",
  "stability.pairFrames": "配对帧数",
  "stability.bodyIds": "物体 ID",
  "stability.islandId": "所属岛",
  "stability.sleepingBodies": "休眠物体",
  "canvas.frame": "帧",
  "canvas.colliders": "碰撞体",
  "canvas.contacts": "接触点",
  "canvas.zoom": "缩放",
  "canvas.scale": "比例尺",
  "canvas.mode": "模式",
  "canvas.modeFree": "自由",
  "canvas.modeLockedCore": "核心锁定",
  "canvas.targetActive": "活动动态物体 / 接触区域",
  "canvas.targetAllColliders": "全部碰撞体边界",
  "canvas.targetScene": "场景边界回退",
  "error.sessionWithoutRun": "会话完成但没有 run_id",
  "error.emptyFrames": "frames.jsonl 为空",
  "log.serverNotConfirmed": "尚未确认 Rust server；正在显示内置演示帧。",
  "log.connectedScenarios": "已连接 /api/scenarios。",
  "log.serverUnavailable": "Server 不可用：{message}。已启用演示回退。",
  "log.loadedFrames": "已从运行 {runId} 加载 {count} 帧。",
  "log.sessionStatus": "会话 {sessionId} 状态：{status}。",
  "log.serverRunFailed": "Server 运行失败；已切换到演示回退（{message}）。",
  "log.generatedDemoFrames": "已为 {scenarioId} 生成 {count} 个本地演示帧。",
  "log.artifactsAvailable": "回放产物：{manifest}、{finalSnapshot}。",
  "log.finalSnapshotLoaded": "已加载 final_snapshot，第 {step} 步。",
  "log.liveSessionReady": "实时会话 {sessionId} 已就绪；后端单步会按需追加帧。",
  "log.liveFrameBuffered": "已从 Rust 后端缓存实时帧 {frameIndex}。",
  "log.sseFrame": "SSE 帧 {data}",
  "log.sseFailed": "SSE 失败 {data}",
  "log.sseIdle": "SSE 空队列 {data}",
  "log.sseUnavailable": "SSE 不可用：{message}",
  "log.serverAccepted": "Server 已接受 {action}：{status}。",
  "log.serverControlFailed": "Server {action} 失败：{message}。",
  "group.transform": "变换",
  "group.massProperties": "质量属性",
  "group.velocities": "速度",
  "group.material": "材质",
  "group.contactInfo": "接触信息",
  "group.warmStart": "暖启动",
  "group.solver": "求解器",
  "group.trace": "轨迹",
  "group.ccdTrace": "CCD 轨迹",
  "group.jointInfo": "关节信息",
};

export const messages: Record<Locale, Record<MessageKey, string>> = {
  "zh-CN": zhMessages,
  "en-US": enMessages,
};

export const localeLabels: Record<Locale, string> = {
  "zh-CN": "中文",
  "en-US": "English",
};

const scenarioMessages: Record<string, Record<Locale, Pick<ScenarioDescriptor, "name" | "description">>> = {
  falling_box_contact: {
    "zh-CN": { name: "落箱接触", description: "动态箱体下落到静态地面接触，用于观察 AABB、轨迹和接触事实。" },
    "en-US": { name: "Falling box contact", description: "A dynamic box falling into static floor contact." },
  },
  stack_4: {
    "zh-CN": { name: "四箱堆叠", description: "离线堆叠预览，用于没有 Rust server 时的烟测构建。" },
    "en-US": { name: "Four box stack", description: "Offline stack preview for smoke builds without the Rust server." },
  },
  stack_stability_tower: {
    "zh-CN": { name: "稳定性塔堆", description: "更复杂的确定性塔堆，用于观察接触峰值、岛休眠、漂移和稳定窗口。" },
    "en-US": { name: "Stack stability tower", description: "A denser deterministic tower for contact spikes, island sleep, drift, and quiet-window observations." },
  },
  matrix_stack: {
    "zh-CN": { name: "矩阵堆叠 8x6", description: "8x6 动态箱体矩阵堆叠，用于观察大规模静息接触、穿透、churn 和求解行压力。" },
    "en-US": { name: "Matrix stack 8x6", description: "An 8x6 dynamic box matrix stack for dense resting-contact, penetration, churn, and solver-row diagnostics." },
  },
  matrix_stack_aligned: {
    "zh-CN": { name: "对齐矩阵堆叠 4x3", description: "4x3 对齐动态箱体矩阵，用作稳定矩阵形态的行为锁。" },
    "en-US": { name: "Aligned matrix stack 4x3", description: "An aligned 4x3 dynamic box matrix for stable matrix-form behavior locks." },
  },
  joint_anchor: {
    "zh-CN": { name: "世界锚点关节", description: "带约束线的离线关节锚点预览。" },
    "en-US": { name: "World anchor joint", description: "Offline joint anchor preview with a constraint line." },
  },
  lattice_grid: {
    "zh-CN": {
      name: "刚体格点代理",
      description: "由刚体节点和关节网格组成的代理场景，不是 true soft-body 求解器。",
    },
    "en-US": {
      name: "Rigid-body lattice grid proxy",
      description: "Rigid-body joint lattice / grid proxy with node, edge, island, and stretch facts, not a true soft-body solver.",
    },
  },
  ccd_fast_circle_wall: {
    "zh-CN": { name: "CCD 快圆薄墙", description: "高速动态圆扫掠命中静态薄矩形墙，用于观察 TOI、钳制和接触事实。" },
    "en-US": { name: "CCD fast circle wall", description: "A fast dynamic circle swept against a static thin rectangle wall." },
  },
  ccd_fast_convex_walls: {
    "zh-CN": { name: "CCD 快凸体双墙", description: "高速动态矩形扫掠两个静态薄墙，用于观察最早命中和预算钳制事实。" },
    "en-US": { name: "CCD fast convex walls", description: "A fast dynamic rectangle swept against two static thin walls." },
  },
  ccd_dynamic_convex_pair: {
    "zh-CN": { name: "CCD 动态凸体对撞", description: "两个高速动态矩形彼此扫掠命中，用于观察动态目标 CCD 的 TOI、目标扫掠和目标钳制事实。" },
    "en-US": { name: "CCD dynamic convex pair", description: "Two fast dynamic rectangles swept against each other." },
  },
  compound_provenance: {
    "zh-CN": { name: "复合体来源", description: "作者提供的复合体 fixture，展示 piece 顺序、继承语义、宽阶段树和岛事实。" },
    "en-US": { name: "Compound provenance fixture", description: "An authored compound fixture exposing piece order, inherited collider semantics, broadphase tree, and island facts." },
  },
};

const bodyTypeLabels: Record<Locale, Record<BodyType, string>> = {
  "zh-CN": { static: "静态", dynamic: "动态", kinematic: "运动学" },
  "en-US": { static: "static", dynamic: "dynamic", kinematic: "kinematic" },
};

const entityKindLabels: Record<Locale, Record<EntityKind, string>> = {
  "zh-CN": { body: "物体", collider: "碰撞体", contact: "接触点", joint: "关节" },
  "en-US": { body: "Body", collider: "Collider", contact: "Contact", joint: "Joint" },
};

const layerLabels: Record<Locale, Record<LayerKey, string>> = {
  "zh-CN": { grid: "网格", rulers: "标尺", shapes: "形状", aabbs: "AABB", contacts: "接触点", velocities: "速度", trace: "轨迹", broadphaseTree: "宽阶段树", islands: "岛", provenance: "来源", stackStability: "稳定性", lattice: "格点代理" },
  "en-US": { grid: "Grid", rulers: "Rulers", shapes: "Shapes", aabbs: "AABBs", contacts: "Contacts", velocities: "Velocities", trace: "Trace", broadphaseTree: "Broadphase tree", islands: "Islands", provenance: "Provenance", stackStability: "Stack stability", lattice: "Lattice proxy" },
};

const sourceLabels: Record<Locale, Record<SourceKind, string>> = {
  "zh-CN": {
    demo: "演示回放",
    artifact: "Rust 产物回放",
    live: "Rust 实时会话",
  },
  "en-US": {
    demo: "demo replay",
    artifact: "Rust artifact replay",
    live: "Rust live session",
  },
};

const statusLabels: Record<Locale, Record<StatusKind, string>> = {
  "zh-CN": { idle: "空闲", loading: "加载中", playing: "播放中", paused: "已暂停", failed: "失败", created: "已创建", running: "运行中", completed: "已完成" },
  "en-US": { idle: "idle", loading: "loading", playing: "playing", paused: "paused", failed: "failed", created: "created", running: "running", completed: "completed" },
};

const actionLabels: Record<Locale, Record<"play" | "pause" | "step" | "reset", string>> = {
  "zh-CN": { play: "播放", pause: "暂停", step: "单步", reset: "重置" },
  "en-US": { play: "play", pause: "pause", step: "step", reset: "reset" },
};

const dynamicValueLabels: Record<Locale, Record<string, string>> = {
  "zh-CN": {
    circle: "圆形",
    static: "静态",
    dynamic: "动态",
    polygon: "多边形",
    segment: "线段",
    default: "默认",
    ice: "低摩擦",
    rough: "粗糙",
    bouncy: "弹性",
    sticky: "高摩擦",
    static_geometry: "静态几何",
    dynamic_body: "动态物体",
    sensor: "传感器",
    query_only: "仅查询",
    distance: "距离关节",
    world_anchor: "世界锚点",
    single_point: "单点",
    clipped: "裁剪",
    duplicate_reduced: "去重归约",
    non_m2_fallback: "非 M2 回退",
    generic_convex_fallback: "通用凸形回退",
    none: "无",
    epa_failure_contained: "EPA 失败已收容",
    unknown: "未知",
    separated: "分离",
    touching: "贴合",
    intersect: "相交",
    degenerate_direction: "退化方向",
    max_iterations: "达到迭代上限",
    invalid_support: "无效支撑点",
    converged: "已收敛",
    gjk_did_not_intersect: "GJK 未相交",
    degenerate_edge: "退化边",
    stability_window: "稳定窗口",
    impact: "碰撞唤醒",
    contact_impulse: "接触冲量",
    joint_correction: "关节修正",
    transform_edit: "位姿编辑",
    velocity_edit: "速度编辑",
    user_patch: "用户补丁",
    sleep_disabled: "禁用休眠",
    hit: "命中",
    miss_no_previous: "无历史",
    miss_feature_id: "特征不匹配",
    miss_previous_sensor: "历史传感器",
    skipped_sensor: "跳过传感器",
    dropped_normal_mismatch: "法线不匹配丢弃",
    dropped_point_drift: "点漂移丢弃",
    dropped_invalid_impulse: "无效冲量丢弃",
    accepted: "已接受",
    synced: "已同步",
    stale: "陈旧",
    not_live_session: "不是 live 会话",
    session_created: "会话仍处于 created",
    session_running: "会话正在运行",
    session_completed: "会话已完成",
    session_failed: "会话已失败",
    missing_frame_snapshot: "缺少当前帧快照",
    stale_world_revision: "world revision 已过期",
    stale_session_epoch: "session epoch 已过期",
    stale_frame: "帧索引已过期",
    invalid_body_handle: "物体句柄无效",
    static_body: "静态物体不可编辑",
    kinematic_body: "运动学物体不可编辑",
    invalid_velocity_delta: "速度增量无效",
    missing_preview_action: "缺少预览动作",
    stale_preview_action: "预览结果已过期",
    body_patch_failed: "物体补丁应用失败",
    reused_action: "动作已被重复使用",
  },
  "en-US": {
    circle: "circle",
    static: "static",
    dynamic: "dynamic",
    polygon: "polygon",
    segment: "segment",
    distance: "distance joint",
    world_anchor: "world anchor",
    single_point: "single point",
    clipped: "clipped",
    duplicate_reduced: "duplicate reduced",
    non_m2_fallback: "non-M2 fallback",
    default: "default",
    ice: "ice",
    rough: "rough",
    bouncy: "bouncy",
    sticky: "sticky",
    static_geometry: "static geometry",
    dynamic_body: "dynamic body",
    sensor: "sensor",
    query_only: "query only",
    generic_convex_fallback: "generic convex fallback",
    none: "none",
    epa_failure_contained: "EPA failure contained",
    unknown: "unknown",
    separated: "separated",
    touching: "touching",
    intersect: "intersect",
    degenerate_direction: "degenerate direction",
    max_iterations: "max iterations",
    invalid_support: "invalid support",
    converged: "converged",
    gjk_did_not_intersect: "GJK did not intersect",
    degenerate_edge: "degenerate edge",
    stability_window: "stability window",
    impact: "impact",
    contact_impulse: "contact impulse",
    joint_correction: "joint correction",
    transform_edit: "transform edit",
    velocity_edit: "velocity edit",
    user_patch: "user patch",
    sleep_disabled: "sleep disabled",
    hit: "hit",
    miss_no_previous: "no previous",
    miss_feature_id: "feature mismatch",
    miss_previous_sensor: "previous sensor",
    skipped_sensor: "sensor skipped",
    dropped_normal_mismatch: "normal mismatch dropped",
    dropped_point_drift: "point drift dropped",
    dropped_invalid_impulse: "invalid impulse dropped",
    accepted: "accepted",
    synced: "synced",
    stale: "stale",
    not_live_session: "not live session",
    session_created: "session created",
    session_running: "session running",
    session_completed: "session completed",
    session_failed: "session failed",
    missing_frame_snapshot: "missing frame snapshot",
    stale_world_revision: "stale world revision",
    stale_session_epoch: "stale session epoch",
    stale_frame: "stale frame",
    invalid_body_handle: "invalid body handle",
    static_body: "static body",
    kinematic_body: "kinematic body",
    invalid_velocity_delta: "invalid velocity delta",
    missing_preview_action: "missing preview action",
    stale_preview_action: "stale preview action",
    body_patch_failed: "body patch failed",
    reused_action: "reused action",
  },
};

export function t(locale: Locale, key: MessageKey, params?: Record<string, string | number>): string {
  let text = messages[locale][key];
  for (const [name, value] of Object.entries(params ?? {})) {
    text = text.split(`{${name}}`).join(String(value));
  }
  return text;
}

export function normalizeLocale(value: string | null | undefined): Locale | null {
  if (!value) {
    return null;
  }
  const normalized = value.toLowerCase();
  if (normalized === "zh" || normalized.startsWith("zh-")) {
    return "zh-CN";
  }
  if (normalized === "en" || normalized.startsWith("en-")) {
    return "en-US";
  }
  return null;
}

export function detectInitialLocale(): Locale {
  const stored = safeLocalStorage()?.getItem(storageKey);
  const storedLocale = normalizeLocale(stored);
  if (storedLocale) {
    return storedLocale;
  }
  const languages = typeof navigator === "undefined" ? [] : navigator.languages ?? [navigator.language];
  for (const language of languages) {
    const locale = normalizeLocale(language);
    if (locale) {
      return locale;
    }
  }
  return "zh-CN";
}

export function storeLocale(locale: Locale): void {
  safeLocalStorage()?.setItem(storageKey, locale);
}

export function localizeScenario(locale: Locale, scenario: ScenarioDescriptor): ScenarioDescriptor {
  const copy = scenarioMessages[scenario.id]?.[locale];
  return copy ? { ...scenario, ...copy } : scenario;
}

export function scenarioGroupForId(scenarioId: string): ScenarioGroupId {
  if (scenarioId.includes("stack")) {
    return "stack";
  }
  if (scenarioId.includes("ccd")) {
    return "ccd";
  }
  if (
    scenarioId.includes("compound") ||
    scenarioId.includes("concave") ||
    scenarioId.includes("provenance")
  ) {
    return "compound";
  }
  if (scenarioId.includes("lattice") || scenarioId.includes("grid")) {
    return "lattice";
  }
  if (
    scenarioId.includes("broadphase") ||
    scenarioId.includes("sat") ||
    scenarioId.includes("diagnostic")
  ) {
    return "diagnostics";
  }
  return "basics";
}

export function scenarioGroupLabel(locale: Locale, group: ScenarioGroupId): string {
  return t(locale, `scenario.group.${group}` as MessageKey);
}

export function overlayPresetLabel(locale: Locale, preset: OverlayPresetId): string {
  return t(locale, `overlay.preset.${preset}` as MessageKey);
}

export function overlayPresetDescription(locale: Locale, preset: OverlayPresetId): string {
  return t(locale, `overlay.presetDesc.${preset}` as MessageKey);
}

export function diagnosticSourceLabel(
  locale: Locale,
  source: DiagnosticSource | null | undefined,
): string {
  switch (source ?? "missing") {
    case "rust_authoritative":
      return t(locale, "diagnostics.source.rustAuthoritative");
    case "lab_derived":
      return t(locale, "diagnostics.source.labDerived");
    case "web_derived":
      return t(locale, "diagnostics.source.webDerived");
    case "missing":
      return t(locale, "diagnostics.source.missing");
  }
}

export function diagnosticSeverityLabel(
  locale: Locale,
  severity: DiagnosticSeverity | null | undefined,
): string {
  switch (severity ?? "info") {
    case "info":
      return t(locale, "diagnostics.severity.info");
    case "warning":
      return t(locale, "diagnostics.severity.warning");
    case "severe":
      return t(locale, "diagnostics.severity.severe");
  }
}

export function diagnosticMarkerLabel(
  locale: Locale,
  kind: DiagnosticMarkerKind | null | undefined,
): string {
  switch (kind) {
    case "performance_counter_spike":
      return t(locale, "diagnostics.marker.performanceCounterSpike");
    case "solver_row_spike":
      return t(locale, "diagnostics.marker.solverRowSpike");
    case "ccd_spike":
      return t(locale, "diagnostics.marker.ccdSpike");
    case "numeric_warning":
      return t(locale, "diagnostics.marker.numericWarning");
    case "penetration_spike":
      return t(locale, "diagnostics.marker.penetrationSpike");
    case "contact_churn_spike":
      return t(locale, "diagnostics.marker.contactChurnSpike");
    case "warm_start_drop_spike":
      return t(locale, "diagnostics.marker.warmStartDropSpike");
    case "sleep_transition":
      return t(locale, "diagnostics.marker.sleepTransition");
    case "sleep_never_converged":
      return t(locale, "diagnostics.marker.sleepNeverConverged");
    case "angular_drift_spike":
      return t(locale, "diagnostics.marker.angularDriftSpike");
    case "body_drift_spike":
      return t(locale, "diagnostics.marker.bodyDriftSpike");
    default:
      return kind ?? t(locale, "diagnostics.markers");
  }
}

export function missingEvidenceLabel(
  locale: Locale,
  kind: MissingEvidenceKind | null | undefined,
): string {
  switch (kind) {
    case "previous_frame":
      return t(locale, "diagnostics.missing.previousFrame");
    case "contact_facts":
      return t(locale, "diagnostics.missing.contactFacts");
    case "contact_ids":
      return t(locale, "diagnostics.missing.contactIds");
    case "sleep_events":
      return t(locale, "diagnostics.missing.sleepEvents");
    case "island_facts":
      return t(locale, "diagnostics.missing.islandFacts");
    default:
      return kind ?? t(locale, "diagnostics.source.missing");
  }
}

export function bodyTypeLabel(locale: Locale, value: BodyType): string {
  return bodyTypeLabels[locale][value] ?? value;
}

export function entityKindLabel(locale: Locale, kind: EntityKind): string {
  return entityKindLabels[locale][kind] ?? kind;
}

export function entityLabel(locale: Locale, kind: EntityKind, id: number): string {
  return `${entityKindLabel(locale, kind)} ${id}`;
}

export function layerLabel(locale: Locale, key: LayerKey): string {
  return layerLabels[locale][key] ?? key;
}

export function sourceLabel(locale: Locale, source: SourceKind): string {
  return sourceLabels[locale][source] ?? source;
}

export function statusLabel(locale: Locale, status: StatusKind): string {
  return statusLabels[locale][status] ?? status;
}

export function actionLabel(locale: Locale, action: keyof (typeof actionLabels)[Locale]): string {
  return actionLabels[locale][action] ?? action;
}

export function booleanLabel(locale: Locale, value: boolean): string {
  return t(locale, value ? "common.true" : "common.false");
}

export function dynamicValueLabel(locale: Locale, value: string | null | undefined): string {
  if (!value) {
    return t(locale, "common.unknown");
  }
  return dynamicValueLabels[locale][value] ?? value;
}

function safeLocalStorage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}
