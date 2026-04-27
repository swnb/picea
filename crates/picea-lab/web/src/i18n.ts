import type { ScenarioDescriptor } from "./types";

export const supportedLocales = ["zh-CN", "en-US"] as const;

export type Locale = (typeof supportedLocales)[number];
export type SourceKind = "server" | "demo";
export type StatusKind = "idle" | "loading" | "playing" | "paused" | "failed" | "created" | "running" | "completed";
export type EntityKind = "body" | "collider" | "contact" | "joint";
export type BodyType = "static" | "dynamic" | "kinematic";
export type LayerKey = "shapes" | "aabbs" | "contacts" | "velocities" | "trace";

const storageKey = "picea-lab.locale";

const enMessages = {
  "app.name": "picea-lab-web",
  "app.language": "Language",
  "app.noSession": "no session",
  "app.noRunArtifact": "no run artifact",
  "tooltip.runScenario": "Run selected scenario",
  "tooltip.pausePlayback": "Pause playback",
  "tooltip.playTimeline": "Play local timeline",
  "tooltip.advanceFrame": "Advance one frame",
  "tooltip.resetTimeline": "Reset timeline",
  "tooltip.canvasLayers": "Canvas layers",
  "panel.sceneHierarchy": "Scene hierarchy",
  "panel.inspector": "Inspector",
  "panel.firstSliceFacts": "first slice facts",
  "tree.bodies": "Bodies",
  "tree.colliders": "Colliders",
  "tree.contacts": "Contacts",
  "tree.empty": "empty",
  "metric.bodies": "bodies",
  "metric.contacts": "contacts",
  "metric.dt": "dt",
  "metric.step": "step",
  "metric.simTime": "sim time",
  "metric.gravity": "gravity",
  "metric.manifolds": "manifolds",
  "inspector.measurementStatus": "Measurement status",
  "inspector.unmeasured": "unmeasured",
  "inspector.forces": "forces",
  "inspector.torques": "torques",
  "inspector.broadphaseCandidates": "broadphase candidates",
  "inspector.warmStart": "warm-start",
  "inspector.ccd": "CCD cand/hit/miss/clamp",
  "inspector.emptySelection": "Select a body, collider, contact, or joint in the hierarchy or canvas.",
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
  "fact.ccdSlop": "CCD slop",
  "fact.ccdSweptStart": "swept start",
  "fact.ccdSweptEnd": "swept end",
  "fact.ccdToiPoint": "TOI point",
  "fact.kind": "kind",
  "fact.anchors": "anchors",
  "common.unknown": "unknown",
  "common.true": "true",
  "common.false": "false",
  "contact.applied": "applied",
  "contact.suppressed": "suppressed",
  "timeline.timeline": "Timeline",
  "timeline.logs": "Logs",
  "timeline.runSetup": "Run setup",
  "timeline.frameAt": "frame {frame}",
  "timeline.totalFrames": "{count} total",
  "run.frameCount": "frame count",
  "run.gravityOverride": "gravity override",
  "run.sendOverride": "send override with next run",
  "run.gravityY": "gravity y",
  "canvas.frame": "frame",
  "canvas.colliders": "colliders",
  "canvas.contacts": "contacts",
  "error.sessionWithoutRun": "session completed without run_id",
  "error.emptyFrames": "frames.jsonl was empty",
  "log.serverNotConfirmed": "Rust server not confirmed yet; showing built-in demo frames.",
  "log.connectedScenarios": "Connected to /api/scenarios.",
  "log.serverUnavailable": "Server unavailable: {message}. Demo fallback is active.",
  "log.loadedFrames": "Loaded {count} frames from run {runId}.",
  "log.sessionStatus": "Session {sessionId} status: {status}.",
  "log.serverRunFailed": "Server run failed; switched to demo fallback ({message}).",
  "log.generatedDemoFrames": "Generated {count} local demo frames for {scenarioId}.",
  "log.sseFrame": "SSE frame {data}",
  "log.sseFailed": "SSE failed {data}",
  "log.sseUnavailable": "SSE unavailable: {message}",
  "log.serverAccepted": "Server accepted {action}: {status}.",
  "log.serverControlFailed": "Server {action} failed: {message}.",
} as const;

export type MessageKey = keyof typeof enMessages;

const zhMessages: Record<MessageKey, string> = {
  "app.name": "picea-lab-web",
  "app.language": "语言",
  "app.noSession": "无会话",
  "app.noRunArtifact": "无运行产物",
  "tooltip.runScenario": "运行当前场景",
  "tooltip.pausePlayback": "暂停播放",
  "tooltip.playTimeline": "播放本地时间线",
  "tooltip.advanceFrame": "前进一帧",
  "tooltip.resetTimeline": "重置时间线",
  "tooltip.canvasLayers": "画布图层",
  "panel.sceneHierarchy": "场景层级",
  "panel.inspector": "检查器",
  "panel.firstSliceFacts": "首个切片事实",
  "tree.bodies": "物体",
  "tree.colliders": "碰撞体",
  "tree.contacts": "接触点",
  "tree.empty": "空",
  "metric.bodies": "物体",
  "metric.contacts": "接触点",
  "metric.dt": "dt",
  "metric.step": "步数",
  "metric.simTime": "模拟时间",
  "metric.gravity": "重力",
  "metric.manifolds": "流形",
  "inspector.measurementStatus": "测量状态",
  "inspector.unmeasured": "未测量",
  "inspector.forces": "力",
  "inspector.torques": "扭矩",
  "inspector.broadphaseCandidates": "宽阶段候选",
  "inspector.warmStart": "暖启动",
  "inspector.ccd": "CCD 候选/命中/错过/钳制",
  "inspector.emptySelection": "在层级树或画布中选择物体、碰撞体、接触点或关节。",
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
  "fact.ccdSlop": "CCD slop",
  "fact.ccdSweptStart": "扫掠起点",
  "fact.ccdSweptEnd": "扫掠终点",
  "fact.ccdToiPoint": "TOI 点",
  "fact.kind": "类型",
  "fact.anchors": "锚点",
  "common.unknown": "未知",
  "common.true": "是",
  "common.false": "否",
  "contact.applied": "已应用",
  "contact.suppressed": "已抑制",
  "timeline.timeline": "时间线",
  "timeline.logs": "日志",
  "timeline.runSetup": "运行设置",
  "timeline.frameAt": "第 {frame} 帧",
  "timeline.totalFrames": "共 {count} 帧",
  "run.frameCount": "帧数",
  "run.gravityOverride": "重力覆盖",
  "run.sendOverride": "下次运行发送覆盖",
  "run.gravityY": "重力 y",
  "canvas.frame": "帧",
  "canvas.colliders": "碰撞体",
  "canvas.contacts": "接触点",
  "error.sessionWithoutRun": "会话完成但没有 run_id",
  "error.emptyFrames": "frames.jsonl 为空",
  "log.serverNotConfirmed": "尚未确认 Rust server；正在显示内置演示帧。",
  "log.connectedScenarios": "已连接 /api/scenarios。",
  "log.serverUnavailable": "Server 不可用：{message}。已启用演示回退。",
  "log.loadedFrames": "已从运行 {runId} 加载 {count} 帧。",
  "log.sessionStatus": "会话 {sessionId} 状态：{status}。",
  "log.serverRunFailed": "Server 运行失败；已切换到演示回退（{message}）。",
  "log.generatedDemoFrames": "已为 {scenarioId} 生成 {count} 个本地演示帧。",
  "log.sseFrame": "SSE 帧 {data}",
  "log.sseFailed": "SSE 失败 {data}",
  "log.sseUnavailable": "SSE 不可用：{message}",
  "log.serverAccepted": "Server 已接受 {action}：{status}。",
  "log.serverControlFailed": "Server {action} 失败：{message}。",
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
  joint_anchor: {
    "zh-CN": { name: "世界锚点关节", description: "带约束线的离线关节锚点预览。" },
    "en-US": { name: "World anchor joint", description: "Offline joint anchor preview with a constraint line." },
  },
  ccd_fast_circle_wall: {
    "zh-CN": { name: "CCD 快圆薄墙", description: "高速动态圆扫掠命中静态薄矩形墙，用于观察 TOI、钳制和接触事实。" },
    "en-US": { name: "CCD fast circle wall", description: "A fast dynamic circle swept against a static thin rectangle wall." },
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
  "zh-CN": { shapes: "形状", aabbs: "AABB", contacts: "接触点", velocities: "速度", trace: "轨迹" },
  "en-US": { shapes: "Shapes", aabbs: "AABBs", contacts: "Contacts", velocities: "Velocities", trace: "Trace" },
};

const sourceLabels: Record<Locale, Record<SourceKind, string>> = {
  "zh-CN": { server: "server", demo: "演示" },
  "en-US": { server: "server", demo: "demo" },
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
    polygon: "多边形",
    segment: "线段",
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
    hit: "命中",
    miss_no_previous: "无历史",
    miss_feature_id: "特征不匹配",
    miss_previous_sensor: "历史传感器",
    skipped_sensor: "跳过传感器",
    dropped_normal_mismatch: "法线不匹配丢弃",
    dropped_point_drift: "点漂移丢弃",
    dropped_invalid_impulse: "无效冲量丢弃",
  },
  "en-US": {
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
