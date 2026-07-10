# Workbench 交互化与运行模型 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 落地 docs/design/2026-07-10-workbench-interactive-run-model-design.md:选中场景即运行真物理(默认 live)、demo 帧降级为带水印的显式离线模式、16 场景看点文案,以及 live 会话的刚体拖拽(spring 软弹簧 / direct 速度直驱双模式)。

**Architecture:** core(crates/picea)零改动。lab server 新增 grab 三路由,复用 velocity perturbation 的 session 锁与校验骨架,spring 模式在 world 内创建/patch/销毁 world_anchor joint,direct 模式在 `step_live_session` 开头注入 clamp 后的线速度。web 侧把 `changeScenario` 接到 `runScenario`,拖拽由 WorldCanvas pointer 状态机上抛 App 管生命周期。

**Tech Stack:** Rust(axum、tower oneshot 集成测试)、React + Canvas、自研 ui-contract/i18n-contract 静态断言脚本。

## Global Constraints

- 一切验证命令加 `rtk proxy` 前缀(仓库约定)。
- `crates/picea` core crate 一行不改;grab 只用 prelude 已导出的 `JointDesc/WorldAnchorJointDesc/JointPatch/WorldAnchorJointPatch/JointHandle/BodyPatch/Point/Vector/FloatNum`。
- 工作区已有一批 vNext 文档登记未提交(docs/ai/*、docs/design/README.md 等)与本设计/计划文档,属合法内容:不 revert、不覆盖;本计划各任务只 `git add` 自己明确列出的文件。
- 每个任务收尾:`rtk proxy cargo fmt` + `rtk proxy cargo clippy`(触碰 Rust 时)无警告后才 commit。
- commit 信息风格:`<area>: <summary>`(如 `web: ...` / `lab: ...`),结尾加 `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`。
- 竞态契约(设计已定,不要"修正"):grab 全程只校验 `session_epoch`,**不校验** `world_revision`,**不 bump** `session_epoch`。
- grab 弹簧默认:stiffness 40.0、damping 2.0、max_speed 40.0(浏览器验收阶段可调,量级参照 scene_lattice.rs 的 soft 6.5 / hard 420)。

---

### Task 0: 建立特性分支

**Files:** 无代码改动。

- [ ] **Step 1: 从 main 建分支**

```bash
git checkout -b feat/workbench-interactive-run-model
```

预期:`Switched to a new branch 'feat/workbench-interactive-run-model'`。工作区已有的未提交文档(设计文档、计划文档、doc 登记)随分支走,不要 stash/revert。

---

### Task 1: web — 选中即运行 + 默认 live_session

**Files:**
- Modify: `crates/picea-lab/web/src/App.tsx`(约 618-1147 行区域:state 默认值、`runScenario`、`changeScenario`、`fetchScenarios` effect)
- Test: `crates/picea-lab/web/scripts/ui-contract.mjs`

**Interfaces:**
- Produces: `runScenario(scenarioIdOverride?: string)` —— 后续任务(水印、拖拽门控)依赖"选中后 source 为 live/artifact 而非 demo"这一状态事实。

- [ ] **Step 1: 写失败的 ui-contract 断言**

在 `crates/picea-lab/web/scripts/ui-contract.mjs` 末尾(现有断言之后)追加:

```js
// --- Select-to-run contract: selecting a scenario runs real physics ---
assert.match(
  appSource,
  /const \[runMode, setRunMode\] = useState<RunMode>\(\s*"live_session",?\s*\)/,
  "Workbench should default to live_session so the first scenario selection is interactive real physics.",
);
const changeScenarioSource =
  appSource.match(/function changeScenario\(nextScenario: string\) \{[\s\S]*?\n  \}/)?.[0] ?? "";
assert.doesNotMatch(
  changeScenarioSource,
  /makeDemoFrames/,
  "Scenario selection must not seed the canvas with fabricated demo frames; demo is an explicit offline fallback only.",
);
assert.match(
  changeScenarioSource,
  /void runScenario\(nextScenario\)/,
  "Selecting a scenario must trigger a real backend run (select-to-run).",
);
assert.match(
  appSource,
  /async function runScenario\(scenarioIdOverride\?: string\)/,
  "runScenario must accept a scenario override so changeScenario can run the newly selected scenario without stale state.",
);
assert.match(
  appSource,
  /initialRunTriggeredRef\.current = true[\s\S]*?void runScenario\(\)/,
  "First successful scenario fetch should auto-run once so the initial canvas shows real physics, not demo frames.",
);
```

- [ ] **Step 2: 跑契约确认失败**

```bash
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
```

预期:FAIL,首条失败为 `Workbench should default to live_session ...`。同时记录现有断言中是否有与旧行为(选中灌 demo 帧)绑定的断言失败——若有,把该旧断言按新契约改写(保留其失败说明语气),这是行为锁的显式更新,属于本任务范围。

- [ ] **Step 3: 实现 App.tsx 改动**

3a. 默认 runMode(App.tsx:626):

```ts
const [runMode, setRunMode] = useState<RunMode>("live_session")
```

3b. `runScenario` 参数化(签名与内部读值;现 App.tsx:1019):

```ts
async function runScenario(scenarioIdOverride?: string) {
    const scenarioId = scenarioIdOverride ?? selectedScenario
    const scenarioDescriptor =
      scenarios.find((entry) => entry.id === scenarioId) ?? null
    const draftSceneParams =
      scenarioParamDrafts[scenarioId] ??
      defaultSceneParamsForScenario(scenarioDescriptor)
    setStatus("loading")
    ...
```

函数体内原有的 `selectedScenario` 全部替换为 `scenarioId`,原有的 `sceneParamDraft` 读取替换为 `draftSceneParams`(两处:`createSession(scenarioId, ...)` 的第一参与 `sceneParams` 组装;失败 fallback 的 `makeDemoFrames(scenarioId, frameCount)` 与日志插值)。其余逻辑不动。

3c. `changeScenario` 改为选中即运行(现 App.tsx:1522-1544),删除 demo 帧灌入与 `setSource("demo")`:

```ts
function changeScenario(nextScenario: string) {
    const nextDescriptor =
      scenarios.find((entry) => entry.id === nextScenario) ?? null
    const nextDefaults = defaultSceneParamsForScenario(nextDescriptor)
    invalidateLiveResponses()
    setScenarioParamDrafts((current) =>
      current[nextScenario]
        ? current
        : {
            ...current,
            [nextScenario]: nextDefaults,
          },
    )
    setSelectedScenario(nextScenario)
    setFrameIndex(0)
    setSelectedEntity(null)
    clearRunState()
    setAppliedGravityVector(DEFAULT_GRAVITY)
    setGravityUndoVector(null)
    setGravityPatchError(null)
    void runScenario(nextScenario)
  }
```

3d. 首次加载自动运行:在组件顶部 refs 区(App.tsx:685 附近)加 `const initialRunTriggeredRef = useRef(false)`,并在 `fetchScenarios` 成功回调(App.tsx:756-762)内、`pushLogs` 之后追加:

```ts
if (!initialRunTriggeredRef.current) {
  initialRunTriggeredRef.current = true
  void runScenario()
}
```

(该 effect 依赖 locale,切语言会重新 fetch;ref 防止重复自动运行。)

- [ ] **Step 4: 跑契约与构建确认通过**

```bash
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run build
```

预期:均 PASS/成功。

- [ ] **Step 5: Commit**

```bash
git add crates/picea-lab/web/src/App.tsx crates/picea-lab/web/scripts/ui-contract.mjs
git commit -m "web: run selected scenario immediately and default to live session

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: web — 离线 demo 水印

**Files:**
- Modify: `crates/picea-lab/web/src/components/workbench/WorldCanvas.tsx`(props 接口 + :513 信息条附近的覆盖层区)
- Modify: `crates/picea-lab/web/src/components/workbench/WorkbenchLayout.tsx`(透传 prop)
- Modify: `crates/picea-lab/web/src/App.tsx`(计算并下传水印文案)
- Modify: `crates/picea-lab/web/src/i18n.ts`(两 locale 各一条文案)
- Test: `crates/picea-lab/web/scripts/ui-contract.mjs`、`crates/picea-lab/web/scripts/i18n-contract.mjs`

**Interfaces:**
- Produces: WorldCanvas prop `offlineWatermark: string | null`(null = 不显示)。App 侧以 `source === "demo" ? t(locale, "canvas.offlineWatermark") : null` 计算,WorldCanvas 不感知 locale。

- [ ] **Step 1: 写失败断言**

`ui-contract.mjs` 追加:

```js
// --- Offline demo watermark contract ---
assert.match(
  appSource,
  /offlineWatermark=\{source === "demo" \? t\(locale, "canvas\.offlineWatermark"\) : null\}/,
  "Demo-sourced frames must surface an explicit offline watermark instead of silently posing as real simulation.",
);
assert.match(
  appSource,
  /offlineWatermark: string \| null/,
  "WorldCanvas should accept the watermark as a nullable string prop so it stays locale-agnostic.",
);
assert.match(
  appSource,
  /offlineWatermark \?[\s\S]*?pointer-events-none[\s\S]*?\{offlineWatermark\}/,
  "The watermark overlay must render as a non-interactive overlay showing the provided label.",
);
```

(`appSource` 已包含 WorldCanvas/WorkbenchLayout 源文件,ui-contract.mjs:4-23 的文件列表若缺其一,把它补进该列表。)

`i18n-contract.mjs` 在逐 key 断言区追加:

```js
assert.equal(typeof messages[locale]["canvas.offlineWatermark"], "string");
```

(该文件对每个 locale 循环断言的现有位置照抄一行。)

- [ ] **Step 2: 跑两个契约确认失败**

```bash
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
```

预期:均 FAIL(缺 prop / 缺 key)。

- [ ] **Step 3: 实现**

3a. `i18n.ts` 两个 locale 的 messages 表各加(放 `canvas.` 前缀键聚集处;若无聚集处,放 `scenario.` 键组之前):

```ts
"canvas.offlineWatermark": "离线演示数据 · 非真实模拟",
```

```ts
"canvas.offlineWatermark": "Offline demo data — not a real simulation",
```

3b. WorldCanvas props 接口加 `offlineWatermark: string | null`,并在画布容器内(现有 `:513` 左上信息条 `<div className="pointer-events-none absolute left-3 top-3 ...">` 同级)追加:

```tsx
{offlineWatermark ? (
  <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center">
    <div className="-rotate-12 rounded-md border border-lab-line bg-lab-panel/60 px-6 py-3 text-2xl font-bold tracking-widest text-lab-muted">
      {offlineWatermark}
    </div>
  </div>
) : null}
```

3c. WorkbenchLayout:props 接口加 `offlineWatermark: string | null`,原样透传给 `<WorldCanvas ... offlineWatermark={offlineWatermark} />`(WorkbenchLayout.tsx:246 附近的 WorldCanvas 调用点)。

3d. App.tsx 的 `<WorkbenchLayout ...>` 调用处加:

```tsx
offlineWatermark={source === "demo" ? t(locale, "canvas.offlineWatermark") : null}
```

- [ ] **Step 4: 跑契约与构建确认通过**

```bash
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
rtk proxy npm --prefix crates/picea-lab/web run build
```

预期:均通过。

- [ ] **Step 5: Commit**

```bash
git add crates/picea-lab/web/src/App.tsx crates/picea-lab/web/src/components/workbench/WorldCanvas.tsx crates/picea-lab/web/src/components/workbench/WorkbenchLayout.tsx crates/picea-lab/web/src/i18n.ts crates/picea-lab/web/scripts/ui-contract.mjs crates/picea-lab/web/scripts/i18n-contract.mjs
git commit -m "web: watermark offline demo frames as explicit non-simulation fallback

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: web — 16 场景看点(watch-for)文案

**Files:**
- Modify: `crates/picea-lab/web/src/i18n.ts`(`scenarioWatchForMessages` 表 + `scenarioWatchFor` 导出 + `scenario.watchForLabel` 键)
- Modify: `crates/picea-lab/web/src/components/workbench/Toolbar.tsx`(tooltip 拼接)
- Test: `crates/picea-lab/web/scripts/i18n-contract.mjs`

**Interfaces:**
- Produces: `export function scenarioWatchFor(locale: Locale, scenarioId: string): string | null`。刻意不并入 `scenarioMessages`/`localizeScenario`(那会把非 server 契约字段混进 `ScenarioDescriptor`)。

- [ ] **Step 1: 写失败断言**

`i18n-contract.mjs` 末尾追加(沙箱已导出 i18n.ts 的全部 export,`scenarioWatchFor` 可直接引用;若沙箱导出解构列表是显式的,把 `scenarioWatchFor` 加进去):

```js
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
```

- [ ] **Step 2: 跑契约确认失败**

```bash
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
```

预期:FAIL(`scenarioWatchFor` 未定义)。

- [ ] **Step 3: 实现 i18n.ts**

3a. 两 locale messages 表各加一键:zh `"scenario.watchForLabel": "看点"`;en `"scenario.watchForLabel": "Watch for"`。

3b. 在 `scenarioMessages` 表之后加表与导出(16 条全量,不得缩减):

```ts
const scenarioWatchForMessages: Record<string, Record<Locale, string>> = {
  falling_box_contact: {
    "zh-CN": "盒子落到地板后稳定压住:接触点出现、法向冲量收敛,不抖动不下陷。",
    "en-US": "The box lands and rests on the floor: contact points appear, normal impulses converge, no jitter or sinking.",
  },
  stack_4: {
    "zh-CN": "四盒整列落定并休眠;列不歪斜、层间无滑移,任何抖动都是回归。",
    "en-US": "Four boxes settle into a still, sleeping column with no lean or sliding — any jitter is a regression.",
  },
  stack_stability_tower: {
    "zh-CN": "窄支撑高塔分阶段落定;开稳定性叠加层看接触压力,确认没有首个坏帧标记。",
    "en-US": "The narrow-support tower settles in stages; open the stability overlay and confirm no first-bad-frame marker fires.",
  },
  matrix_stack: {
    "zh-CN": "8x6 交错矩阵压测边缘弹出:边角盒子不应被挤飞,整堆逐步安静。",
    "en-US": "The staggered 8x6 matrix stresses edge ejection: corner boxes must not pop out and the pile should quiet down.",
  },
  matrix_stack_aligned: {
    "zh-CN": "4x3 对齐矩阵是稳定行为锁:干净落定、快速休眠。",
    "en-US": "The aligned 4x3 matrix is a stability behavior lock: it settles cleanly and sleeps fast.",
  },
  newton_cradle: {
    "zh-CN": "单球摆入,另一端单球等高摆出;长窗口动能不衰减(恢复系数 1、零摩擦)。",
    "en-US": "One ball swings in, exactly one swings out to equal height; kinetic energy holds long-term (restitution 1, zero friction).",
  },
  joint_anchor: {
    "zh-CN": "刚体绕固定世界锚点受约束运动,约束距离不被拉长;开轨迹层看轨道收敛。",
    "en-US": "The body orbits a fixed world anchor without stretching the constraint; enable the trace layer to watch the orbit converge.",
  },
  lattice_grid: {
    "zh-CN": "刚体节点 + 距离/锚点关节的格点代理(非真软体):看节点下垂、边伸长率与岛休眠。",
    "en-US": "A rigid-body node + joint lattice proxy (not true soft-body): watch node sag, edge stretch ratios, and island sleep.",
  },
  broadphase_sparse: {
    "zh-CN": "五个静态盒子恰好产生一对宽阶段重叠:候选对计数恒为 1;开宽阶段树层看剪枝。",
    "en-US": "Five static boxes yield exactly one broadphase overlap: candidate count stays 1; open the broadphase tree layer to see pruning.",
  },
  sat_polygon: {
    "zh-CN": "矩形与凸多边形的 SAT 裁剪流形:应得到两个裁剪接触点,法向稳定不翻转。",
    "en-US": "SAT clipped manifold between a rectangle and a convex polygon: expect two clipped contact points with a stable, non-flipping normal.",
  },
  compound_provenance: {
    "zh-CN": "复合体三个 piece 的顺序与继承(材质/密度/过滤)固定可追溯:开来源层核对 piece 编号。",
    "en-US": "The compound body keeps stable piece order and inherited material/density/filter: open the provenance layer to verify piece indices.",
  },
  concave_decomposition: {
    "zh-CN": "静态凹多边形确定性分解为凸块:分解缝不产生虚假接触,块数量恒定。",
    "en-US": "The static concave polygon decomposes into deterministic convex pieces: no phantom contacts along seams, piece count constant.",
  },
  ccd_fast_circle_wall: {
    "zh-CN": "高速圆不得穿透薄墙:看 swept 路径、TOI 点与 clamp 后的贴墙停点;ccd_hit 计数为 1。",
    "en-US": "The fast circle must not tunnel through the thin wall: watch the swept path, TOI point, and clamped stop; ccd_hit count is 1.",
  },
  ccd_fast_convex_walls: {
    "zh-CN": "高速矩形面对两道薄墙:应在更早命中的一道停下,绝不到达第二道。",
    "en-US": "The fast rectangle facing two thin walls must clamp at the earlier hit and never reach the second wall.",
  },
  ccd_dynamic_convex_pair: {
    "zh-CN": "两个高速动态矩形对撞:CCD 在相遇点 clamp,双方互不穿透。",
    "en-US": "Two fast dynamic rectangles collide head-on: CCD clamps at the meeting point and neither tunnels through the other.",
  },
  ccd_dynamic_compound_wall: {
    "zh-CN": "高速复合体按最早命中的凸 piece 结算 TOI:整体停在墙前,piece 之间不散开。",
    "en-US": "The fast compound body resolves TOI by its earliest-hitting convex piece: it stops at the wall and pieces stay together.",
  },
}

export function scenarioWatchFor(locale: Locale, scenarioId: string): string | null {
  return scenarioWatchForMessages[scenarioId]?.[locale] ?? null
}
```

3c. Toolbar.tsx:import `scenarioWatchFor`,在 `groupedScenarios` 定义(Toolbar.tsx:80)附近加派生,并把 `:95` 的 `<Tooltip label={scenario.description}>` 与 `:97` 的 `aria-label` 换成拼接串:

```ts
const watchFor = scenarioWatchFor(locale, selectedScenario)
const scenarioTooltip = watchFor
  ? `${scenario.description} ${t(locale, "scenario.watchForLabel")}: ${watchFor}`
  : scenario.description
```

```tsx
<Tooltip label={scenarioTooltip}>
  <div aria-label={scenarioTooltip} ...>
```

- [ ] **Step 4: 跑契约与构建确认通过**

```bash
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run build
```

预期:均通过(ui-contract 不应受影响;若 Toolbar 旧断言锁了 `label={scenario.description}`,按新契约更新该断言)。

- [ ] **Step 5: Commit**

```bash
git add crates/picea-lab/web/src/i18n.ts crates/picea-lab/web/src/components/workbench/Toolbar.tsx crates/picea-lab/web/scripts/i18n-contract.mjs crates/picea-lab/web/scripts/ui-contract.mjs
git commit -m "web: add bilingual watch-for copy for all 16 scenarios

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: lab server — grab 类型 + POST(spring 模式)

**Files:**
- Modify: `crates/picea-lab/src/server.rs`(类型区 :99-266、路由 :268-293、handler 区、`LiveSessionState` :84-95、reset 分支 :653-672)
- Test: `crates/picea-lab/tests/server_routes.rs`

**Interfaces:**
- Produces(后续任务与 web 依赖的 wire 契约):
  - `POST /api/sessions/:id/grabs` body `{body_handle, grab_point: [x,y]|{x,y}, mode?: "spring"|"direct", stiffness?, damping?, max_speed?, session_epoch}` → `200 {"grab": ActiveGrabRecord, "session": SessionRecord}`
  - `ActiveGrabRecord`(serde JSON):`{id: string, body_handle, mode, local_anchor: Point, target: Point, joint_handle: number|null, stiffness, damping, max_speed}`
  - `SessionRecord.active_grab: Option<ActiveGrabRecord>`(默认 None;serde 正常序列化,不 skip)
  - Rust 内部:`fn release_active_grab(record: &mut SessionRecord, runtime: &mut LiveSessionState)`(Task 5/6 复用)

- [ ] **Step 1: 写失败的集成测试**

在 `crates/picea-lab/tests/server_routes.rs` 末尾追加(沿用文件头已有的 `app/LabServerState/ArtifactStore/Request/Method/header/Body/StatusCode/json!/json_body` 引入;若 `json_body`/helper 为文件内私有 fn 则直接可用):

```rust
#[tokio::test]
async fn live_grab_creates_spring_joint_and_rejects_invalid_targets() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let state = LabServerState::new(ArtifactStore::new(temp.path().join("runs")));
    let app = app(state);

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "scenario_id": "falling_box_contact",
                        "frame_count": 8,
                        "mode": "live_session"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = json_body(created).await;
    let session_id = created_body["session"]["id"].as_str().unwrap().to_owned();
    let session_epoch = created_body["session"]["session_epoch"].as_u64().unwrap();
    assert_eq!(created_body["session"]["active_grab"], Value::Null);

    // 先 step 一帧,拿到动态 body 的 handle 与位置。
    let stepped = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "action": "step", "detail": "full" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let stepped_body = json_body(stepped).await;
    let bodies = stepped_body["session"]["latest_frame"]["snapshot"]["bodies"]
        .as_array()
        .unwrap();
    let dynamic_body = bodies
        .iter()
        .find(|body| body["body_type"] == "dynamic")
        .expect("scenario should expose a dynamic body");
    let static_body = bodies
        .iter()
        .find(|body| body["body_type"] == "static")
        .expect("scenario should expose a static body");
    let dynamic_handle = dynamic_body["handle"].as_u64().unwrap();
    let grab_x = dynamic_body["transform"]["translation"]["x"].as_f64().unwrap();
    let grab_y = dynamic_body["transform"]["translation"]["y"].as_f64().unwrap();
    let joint_count_before = stepped_body["session"]["latest_frame"]["snapshot"]["joints"]
        .as_array()
        .unwrap()
        .len();

    // static body 拒绝。
    let static_grab = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/grabs"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "body_handle": static_body["handle"],
                        "grab_point": [0.0, 0.0],
                        "session_epoch": session_epoch
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(static_grab.status(), StatusCode::BAD_REQUEST);

    // 过期 epoch 拒绝。
    let stale_epoch = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/grabs"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "body_handle": dynamic_handle,
                        "grab_point": [grab_x, grab_y],
                        "session_epoch": session_epoch + 999
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stale_epoch.status(), StatusCode::BAD_REQUEST);

    // spring grab 成功:响应带 active_grab,默认 spring 模式,带 joint_handle。
    let grabbed = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/grabs"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "body_handle": dynamic_handle,
                        "grab_point": [grab_x, grab_y],
                        "session_epoch": session_epoch
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(grabbed.status(), StatusCode::OK);
    let grabbed_body = json_body(grabbed).await;
    assert_eq!(grabbed_body["grab"]["mode"], "spring");
    assert_eq!(grabbed_body["grab"]["body_handle"], dynamic_handle);
    assert!(grabbed_body["grab"]["joint_handle"].is_number());
    assert_eq!(
        grabbed_body["session"]["active_grab"]["id"],
        grabbed_body["grab"]["id"]
    );
    // grab 不 bump session_epoch。
    assert_eq!(
        grabbed_body["session"]["session_epoch"].as_u64().unwrap(),
        session_epoch
    );

    // 下一帧的 snapshot 中出现 grab 的 world_anchor joint。
    let after = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{session_id}/control"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "action": "step", "detail": "full" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let after_body = json_body(after).await;
    let joints_after = after_body["session"]["latest_frame"]["snapshot"]["joints"]
        .as_array()
        .unwrap();
    assert_eq!(joints_after.len(), joint_count_before + 1);
    assert!(joints_after
        .iter()
        .any(|joint| joint["kind"] == "world_anchor"));

    // artifact 会话拒绝 grab。
    let artifact_created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "scenario_id": "falling_box_contact",
                        "frame_count": 2,
                        "mode": "artifact_replay"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let artifact_body = json_body(artifact_created).await;
    let artifact_id = artifact_body["session"]["id"].as_str().unwrap().to_owned();
    let artifact_epoch = artifact_body["session"]["session_epoch"].as_u64().unwrap();
    let artifact_grab = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/sessions/{artifact_id}/grabs"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "body_handle": 1,
                        "grab_point": [0.0, 0.0],
                        "session_epoch": artifact_epoch
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(artifact_grab.status(), StatusCode::BAD_REQUEST);
}
```

注:`bodies[].transform.translation` 的 JSON 形状以 debug snapshot 实际序列化为准——若跑测试时该断言路径取不到值(`as_f64()` panic),打印一帧 JSON 修正取值路径,这是测试代码的机械修正,不是行为变更。

- [ ] **Step 2: 跑测试确认失败**

```bash
rtk proxy cargo test -p picea-lab --test server_routes live_grab_creates_spring_joint -- --exact live_grab_creates_spring_joint_and_rejects_invalid_targets
```

预期:FAIL(404,路由不存在)。

- [ ] **Step 3: 实现 server.rs**

3a. prelude import 扩充(server.rs:21-23):

```rust
use picea::prelude::{
    BodyHandle, BodyPatch, BodyType, FloatNum, JointDesc, JointHandle, JointPatch, Point,
    QueryPipeline, SimulationPipeline, Vector, WorldAnchorJointDesc, WorldAnchorJointPatch, World,
};
```

3b. 常量与类型(放在 `CachedVelocityPerturbationPreview` 之后):

```rust
const DEFAULT_GRAB_STIFFNESS: FloatNum = 40.0;
const DEFAULT_GRAB_DAMPING: FloatNum = 2.0;
const DEFAULT_GRAB_MAX_SPEED: FloatNum = 40.0;
/// Marks grab-owned joints in DebugSnapshot facts.
const GRAB_JOINT_USER_DATA: u64 = 0x505f_4752_4142; // "P_GRAB"

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrabMode {
    #[default]
    Spring,
    Direct,
}

/// Server-owned state for the single active pointer grab of a live session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveGrabRecord {
    pub id: String,
    pub body_handle: BodyHandle,
    pub mode: GrabMode,
    pub local_anchor: Point,
    pub target: Point,
    pub joint_handle: Option<JointHandle>,
    pub stiffness: FloatNum,
    pub damping: FloatNum,
    pub max_speed: FloatNum,
}

#[derive(Clone, Debug, Deserialize)]
struct CreateGrabRequest {
    body_handle: BodyHandle,
    grab_point: serde_json::Value,
    #[serde(default)]
    mode: GrabMode,
    #[serde(default)]
    stiffness: Option<FloatNum>,
    #[serde(default)]
    damping: Option<FloatNum>,
    #[serde(default)]
    max_speed: Option<FloatNum>,
    session_epoch: u64,
}
```

3c. 状态字段:`LiveSessionState` 加 `active_grab: Option<ActiveGrabRecord>` 与 `next_grab_id: u64`;`build_live_runtime` 的构造加 `active_grab: None, next_grab_id: 1,`。`SessionRecord` 加 `pub active_grab: Option<ActiveGrabRecord>`;`create_session` 里 record 初始化处加 `active_grab: None,`(找到 `SessionRecord { ... }` 字面构造点补字段,编译器会指出全部遗漏点)。

3d. 路由注册(app() 内,velocity-perturbations 路由之后):

```rust
.route("/api/sessions/:id/grabs", post(create_grab))
```

3e. handler 与释放 helper(放在 `commit_velocity_perturbation` handler 之后):

```rust
fn parse_world_point(value: &serde_json::Value) -> Option<Point> {
    parse_velocity_vector(value).map(Point::from)
}

/// Destroys the grab-owned joint (spring mode) and clears grab state on both
/// the runtime and the wire-visible record. Body velocity is intentionally
/// preserved so releases keep momentum ("fling").
fn release_active_grab(record: &mut SessionRecord, runtime: &mut LiveSessionState) {
    if let Some(grab) = runtime.active_grab.take() {
        if let Some(joint_handle) = grab.joint_handle {
            let _ = runtime.world.destroy_joint(joint_handle);
        }
        runtime.query.sync(&runtime.world);
    }
    record.active_grab = None;
}

async fn create_grab(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
    Json(request): Json<CreateGrabRequest>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = get_session_handle(&state, &id)?;
    let mut session = session.lock().expect("session mutex should not poison");
    if request.session_epoch != session.record.session_epoch {
        return Err(LabHttpError::bad_request(format!(
            "stale session epoch: expected {}, got {}",
            session.record.session_epoch, request.session_epoch
        )));
    }
    let Some(grab_point) = parse_world_point(&request.grab_point) else {
        return Err(LabHttpError::bad_request(
            "grab_point requires a finite [x, y] point",
        ));
    };
    // Reborrow the guard once so record/runtime take disjoint field borrows.
    let session = &mut *session;
    let record = &mut session.record;
    let SessionRuntime::Live(runtime) = &mut session.runtime else {
        return Err(LabHttpError::bad_request(
            "grabs are only available for live_session",
        ));
    };
    release_active_grab(record, runtime);
    let body = runtime
        .world
        .body(request.body_handle)
        .map_err(|_| LabHttpError::bad_request("invalid body handle"))?;
    if !matches!(body.body_type(), BodyType::Dynamic) {
        return Err(LabHttpError::bad_request(
            "only dynamic bodies can be grabbed",
        ));
    }
    let local_anchor = body.pose().inverse_transform_point(grab_point);
    let stiffness = request.stiffness.unwrap_or(DEFAULT_GRAB_STIFFNESS);
    let damping = request.damping.unwrap_or(DEFAULT_GRAB_DAMPING);
    let max_speed = request.max_speed.unwrap_or(DEFAULT_GRAB_MAX_SPEED);
    if !stiffness.is_finite() || !damping.is_finite() || !max_speed.is_finite() || max_speed <= 0.0
    {
        return Err(LabHttpError::bad_request(
            "grab stiffness/damping/max_speed must be finite (max_speed > 0)",
        ));
    }
    let joint_handle = match request.mode {
        GrabMode::Spring => Some(
            runtime
                .world
                .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
                    body: request.body_handle,
                    local_anchor,
                    world_anchor: grab_point,
                    stiffness,
                    damping,
                    user_data: GRAB_JOINT_USER_DATA,
                }))
                .map_err(|error| {
                    LabHttpError::bad_request(format!("grab joint rejected: {error:?}"))
                })?,
        ),
        GrabMode::Direct => None,
    };
    if runtime
        .world
        .apply_body_patch(
            request.body_handle,
            BodyPatch {
                wake: true,
                ..BodyPatch::default()
            },
        )
        .is_err()
    {
        release_active_grab(record, runtime);
        return Err(LabHttpError::bad_request("grab wake patch failed"));
    }
    runtime.query.sync(&runtime.world);
    let grab = ActiveGrabRecord {
        id: format!("grab-{}", runtime.next_grab_id),
        body_handle: request.body_handle,
        mode: request.mode,
        local_anchor,
        target: grab_point,
        joint_handle,
        stiffness,
        damping,
        max_speed,
    };
    runtime.next_grab_id = runtime.next_grab_id.saturating_add(1);
    runtime.active_grab = Some(grab.clone());
    record.active_grab = Some(grab.clone());
    Ok(Json(json!({ "grab": grab, "session": record.clone() })))
}
```

借用注意:先 `let record = &mut session.record;` 再解构 `session.runtime` 会双重可变借用 `session` —— 实际写法用已有惯例:`let session = &mut *session;` 先解引用 guard 再分别借字段(`let SessionState { record, runtime, .. } = session;`)。若现有代码没有此模式,直接写:

```rust
    let session = &mut *session;
    let record = &mut session.record;
    let SessionRuntime::Live(runtime) = &mut session.runtime else { ... };
```

3f. reset 清理:`control_live_session` 的 `"reset"` 分支(:653)在重建 runtime 后已天然丢弃旧 grab(整个 LiveSessionState 替换),补 `session.record.active_grab = None;`(与其他 record 字段清理并列)。

- [ ] **Step 4: 跑测试确认通过**

```bash
rtk proxy cargo test -p picea-lab --test server_routes live_grab_creates_spring_joint -- --exact live_grab_creates_spring_joint_and_rejects_invalid_targets
rtk proxy cargo test -p picea-lab --test server_routes
rtk proxy cargo fmt && rtk proxy cargo clippy
```

预期:新测试 PASS,全部既有 server_routes 测试 PASS,fmt/clippy 干净。

- [ ] **Step 5: Commit**

```bash
git add crates/picea-lab/src/server.rs crates/picea-lab/tests/server_routes.rs
git commit -m "lab: add live-session grab creation with spring world-anchor joint

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: lab server — PATCH/DELETE grab + 替换与 reset 语义

**Files:**
- Modify: `crates/picea-lab/src/server.rs`
- Test: `crates/picea-lab/tests/server_routes.rs`

**Interfaces:**
- Consumes: Task 4 的 `ActiveGrabRecord` / `release_active_grab` / `parse_world_point`。
- Produces:
  - `PATCH /api/sessions/:id/grabs/:grab_id` body `{target: [x,y], session_epoch}` → `200 {"grab", "session"}`;grab_id 不匹配 → 404。
  - `DELETE /api/sessions/:id/grabs/:grab_id` → `200 {"session"}`(重复 DELETE 幂等返回 200)。

- [ ] **Step 1: 写失败测试**

`server_routes.rs` 追加(session/grab 建立部分与 Task 4 测试相同写法——建 live 会话、step 一帧、找 dynamic body、POST grab;下面只列断言主干,建立代码照 Task 4 抄):

```rust
#[tokio::test]
async fn live_grab_update_moves_anchor_and_release_preserves_momentum() {
    // ... 同 Task 4:建 live 会话(frame_count 8) → step full → 找 dynamic body
    // → POST grab(spring,grab_point 取 body 当前位置),得 grab_id、session_epoch。

    // PATCH 目标点:把 world_anchor 挪到 body 上方远处。
    let updated = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/sessions/{session_id}/grabs/{grab_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "target": [grab_x + 2.0, grab_y - 2.0], "session_epoch": session_epoch })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated.status(), StatusCode::OK);
    let updated_body = json_body(updated).await;
    assert_eq!(
        updated_body["grab"]["target"]["x"].as_f64().unwrap(),
        grab_x + 2.0
    );

    // 错误 grab_id → 404。
    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/sessions/{session_id}/grabs/grab-999"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "target": [0.0, 0.0], "session_epoch": session_epoch }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    // 拖两帧,body 应朝新 target 方向获得速度。
    // step → 读 latest_frame.snapshot.bodies 里该 body 的 linear_velocity.x > 0。

    // DELETE 释放:session.active_grab 清空,joint 从下一帧 snapshot 消失,
    // 且 body 速度不被清零(保留甩飞动量,断言 linear_velocity 与释放前同号非零)。
    let released = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/sessions/{session_id}/grabs/{grab_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(released.status(), StatusCode::OK);
    let released_body = json_body(released).await;
    assert_eq!(released_body["session"]["active_grab"], Value::Null);

    // 重复 DELETE 幂等。
    // 再次 DELETE 相同 grab_id → 200。

    // 二次 POST grab 替换语义:连续两次 POST,第二次响应的 grab.id 不同,
    // session.active_grab 是第二个,且下一帧 snapshot 中 world_anchor joint 数量仍为 1(替换,不累加)。

    // reset 后 active_grab 为 Null(action=reset → session.active_grab == Value::Null)。
}
```

(执行者将注释处补成与 Task 4 相同风格的完整 oneshot 调用;每个注释就是一个断言点,全部要写,不许省略。)

- [ ] **Step 2: 跑测试确认失败**

```bash
rtk proxy cargo test -p picea-lab --test server_routes live_grab_update_moves_anchor -- --exact live_grab_update_moves_anchor_and_release_preserves_momentum
```

预期:FAIL(PATCH/DELETE 路由 404 或 405)。

- [ ] **Step 3: 实现**

3a. 请求类型:

```rust
#[derive(Clone, Debug, Deserialize)]
struct UpdateGrabRequest {
    target: serde_json::Value,
    session_epoch: u64,
}
```

3b. 路由(app() 内 grabs POST 之后):

```rust
.route(
    "/api/sessions/:id/grabs/:grab_id",
    patch(update_grab).delete(release_grab),
)
```

3c. handlers:

```rust
async fn update_grab(
    State(state): State<LabServerState>,
    Path((id, grab_id)): Path<(String, String)>,
    Json(request): Json<UpdateGrabRequest>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = get_session_handle(&state, &id)?;
    let mut session = session.lock().expect("session mutex should not poison");
    if request.session_epoch != session.record.session_epoch {
        return Err(LabHttpError::bad_request(format!(
            "stale session epoch: expected {}, got {}",
            session.record.session_epoch, request.session_epoch
        )));
    }
    let Some(target) = parse_world_point(&request.target) else {
        return Err(LabHttpError::bad_request(
            "target requires a finite [x, y] point",
        ));
    };
    let session = &mut *session;
    let record = &mut session.record;
    let SessionRuntime::Live(runtime) = &mut session.runtime else {
        return Err(LabHttpError::bad_request(
            "grabs are only available for live_session",
        ));
    };
    let Some(grab) = runtime.active_grab.as_mut().filter(|grab| grab.id == grab_id) else {
        return Err(LabHttpError::not_found(format!(
            "grab {grab_id} is not active for session {id}"
        )));
    };
    grab.target = target;
    if let Some(joint_handle) = grab.joint_handle {
        runtime
            .world
            .apply_joint_patch(
                joint_handle,
                JointPatch::WorldAnchor(WorldAnchorJointPatch {
                    world_anchor: Some(target),
                    ..WorldAnchorJointPatch::default()
                }),
            )
            .map_err(|error| {
                LabHttpError::bad_request(format!("grab joint patch rejected: {error:?}"))
            })?;
        runtime.query.sync(&runtime.world);
    }
    let grab = grab.clone();
    record.active_grab = Some(grab.clone());
    Ok(Json(json!({ "grab": grab, "session": record.clone() })))
}

async fn release_grab(
    State(state): State<LabServerState>,
    Path((id, grab_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = get_session_handle(&state, &id)?;
    let mut session = session.lock().expect("session mutex should not poison");
    let session = &mut *session;
    let record = &mut session.record;
    let SessionRuntime::Live(runtime) = &mut session.runtime else {
        return Err(LabHttpError::bad_request(
            "grabs are only available for live_session",
        ));
    };
    // Releasing an already-released grab is idempotent: pointerup can race a
    // reset/replace, and the client outcome (no grab) is identical.
    if runtime
        .active_grab
        .as_ref()
        .is_some_and(|grab| grab.id == grab_id)
    {
        release_active_grab(record, runtime);
    }
    Ok(Json(json!({ "session": record.clone() })))
}
```

(若既有代码 `LabHttpError::not_found` 签名不同,以 `get_live_frame` 里的用法为准照抄。)

- [ ] **Step 4: 跑测试与门禁确认通过**

```bash
rtk proxy cargo test -p picea-lab --test server_routes
rtk proxy cargo fmt && rtk proxy cargo clippy
```

预期:全部 PASS,无警告。

- [ ] **Step 5: Commit**

```bash
git add crates/picea-lab/src/server.rs crates/picea-lab/tests/server_routes.rs
git commit -m "lab: support grab target updates, idempotent release, and replace semantics

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 6: lab server — direct 模式速度直驱

**Files:**
- Modify: `crates/picea-lab/src/server.rs`(`step_live_session` :1261 开头注入)
- Test: `crates/picea-lab/tests/server_routes.rs`

**Interfaces:**
- Consumes: Task 4 的 `GrabMode::Direct`(POST 时 `joint_handle: None`)、`ActiveGrabRecord.target/local_anchor/max_speed`。
- Produces: direct grab 的行为契约——每帧 step 前 body 线速度被设为 `clamp((target - anchor_world) / dt, max_speed)`。

- [ ] **Step 1: 写失败测试**

```rust
#[tokio::test]
async fn live_direct_grab_drives_body_toward_target_with_speed_clamp() {
    // 建立部分同 Task 4:live 会话(scenario falling_box_contact, frame_count 16)
    // → step full 一帧 → 找 dynamic body handle 与位置 (grab_x, grab_y)。

    // POST direct grab,max_speed 压到 3.0 以便断言 clamp:
    // body: { "body_handle": dynamic_handle, "grab_point": [grab_x, grab_y],
    //         "mode": "direct", "max_speed": 3.0, "session_epoch": session_epoch }
    // 断言:200、grab.mode == "direct"、grab.joint_handle == Value::Null。

    // PATCH target 到远处 [grab_x + 100.0, grab_y]。

    // step full 一帧:
    // 1) body 的 linear_velocity.x 应接近 +3.0(clamp 生效;容差 |v - 3.0| < 0.5,
    //    因为该 step 内解算器可能有接触/重力修正),且明显 > 0。
    // 2) snapshot.joints 数量与 grab 前相同(direct 不建 joint)。

    // DELETE 释放后再 step 一帧:速度不被注入(相邻两帧 x 速度差由物理决定,
    // 断言释放后下一帧 linear_velocity.x 不再被拉回 3.0 —— 取消注入的直接证据是
    // 连续两次 step 的 x 速度呈自由衰减/恒定而非重置;最少断言:释放后
    // linear_velocity.x 仍 > 0,保留动量)。
}
```

(建立代码照 Task 4/5 风格补全;每个注释点都是必写断言。)

- [ ] **Step 2: 跑测试确认失败**

```bash
rtk proxy cargo test -p picea-lab --test server_routes live_direct_grab_drives_body -- --exact live_direct_grab_drives_body_toward_target_with_speed_clamp
```

预期:FAIL(direct 模式无速度注入,linear_velocity.x ≈ 0)。

- [ ] **Step 3: 实现注入**

`step_live_session`(:1261)在 `let frame_index = ...` 之前插入:

```rust
    apply_direct_grab_drive(&session.record, runtime);
```

并新增函数(放在 `step_live_session` 之前):

```rust
/// Direct-mode grabs steer the body with a clamped velocity toward the pointer
/// target once per frame, before the physics step, so collision response still
/// owns the rest of the frame.
fn apply_direct_grab_drive(record: &SessionRecord, runtime: &mut LiveSessionState) {
    let Some(grab) = runtime.active_grab.as_ref() else {
        return;
    };
    if !matches!(grab.mode, GrabMode::Direct) {
        return;
    }
    let Ok(body) = runtime.world.body(grab.body_handle) else {
        return;
    };
    let anchor_world = body.pose().transform_point(grab.local_anchor);
    let dt = record.effective_runtime_config.step.dt.max(1.0e-6);
    let mut velocity: Vector = (grab.target - anchor_world) / dt;
    let speed = velocity.length();
    if speed > grab.max_speed {
        velocity = velocity * (grab.max_speed / speed);
    }
    let _ = runtime.world.apply_body_patch(
        grab.body_handle,
        BodyPatch {
            linear_velocity: Some(velocity),
            wake: true,
            ..BodyPatch::default()
        },
    );
}
```

借用注意:`step_live_session` 里 runtime 已被 `&mut` 解构;record 是 `&session.record` 不可变借用与 runtime 可变借用并存于同一 `session` —— 沿用 Task 4 的 `let session = &mut *session;` 字段分借;若 `step_live_session` 现有结构是 `let SessionRuntime::Live(runtime) = &mut session.runtime`,把调用写成先取 `let dt = session.record.effective_runtime_config.step.dt;` 再传值:`apply_direct_grab_drive(dt, runtime)`(签名相应改为 `fn apply_direct_grab_drive(dt: FloatNum, runtime: &mut LiveSessionState)`)。两种写法都可,以无借用冲突为准。
(`Point - Point` 的输出类型若非 `Vector` 而是别的,编译器给出精确类型后就地调整 `let velocity` 行——这是机械修正。)

- [ ] **Step 4: 跑测试与门禁确认通过**

```bash
rtk proxy cargo test -p picea-lab --test server_routes
rtk proxy cargo test -p picea-lab
rtk proxy cargo fmt && rtk proxy cargo clippy
```

预期:全部 PASS,无警告。

- [ ] **Step 5: Commit**

```bash
git add crates/picea-lab/src/server.rs crates/picea-lab/tests/server_routes.rs
git commit -m "lab: drive direct-mode grabs with clamped per-step velocity steering

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 7: web — grab 客户端 + 画布拖拽状态机 + 指示线

**Files:**
- Modify: `crates/picea-lab/web/src/api.ts`(三个请求函数)
- Modify: `crates/picea-lab/web/src/types.ts`(`ActiveGrabRecord`、`GrabMode`、`SessionRecord.active_grab`)
- Modify: `crates/picea-lab/web/src/components/workbench/WorldCanvas.tsx`(pointer 状态机 + 指示线绘制 + 新 props)
- Modify: `crates/picea-lab/web/src/components/workbench/WorkbenchLayout.tsx`(透传)
- Modify: `crates/picea-lab/web/src/App.tsx`(grab 生命周期编排)
- Test: `crates/picea-lab/web/scripts/ui-contract.mjs`

**Interfaces:**
- Consumes: Task 4/5/6 的 wire 契约。
- Produces(组件间契约):
  - types.ts:`export type GrabMode = "spring" | "direct"`;`export type ActiveGrabRecord = { id: string; body_handle: number; mode: GrabMode; local_anchor: Vec2; target: Vec2; joint_handle: number | null; stiffness: number; damping: number; max_speed: number }`;`SessionRecord` 加 `active_grab?: ActiveGrabRecord | null`。
  - api.ts:`createGrab(sessionId, request) / updateGrab(sessionId, grabId, request) / releaseGrab(sessionId, grabId)`。
  - WorldCanvas 新 props:`grabEnabled: boolean`、`activeGrab: ActiveGrabRecord | null`、`onGrabStart: (bodyHandle: number, point: Vec2) => void`、`onGrabMove: (point: Vec2) => void`、`onGrabEnd: () => void`。

- [ ] **Step 1: 写失败断言**

`ui-contract.mjs` 追加:

```js
// --- Live grab interaction contract ---
const canvasSource = read("src/components/workbench/WorldCanvas.tsx");
assert.match(
  canvasSource,
  /function handlePointerDown[\s\S]*?grabEnabled[\s\S]*?hitTest/,
  "Pointer-down must attempt a body grab before falling back to camera panning when grabbing is enabled.",
);
assert.match(
  canvasSource,
  /body_type === "dynamic"/,
  "Only dynamic bodies are grabbable from the canvas; static hits fall through to panning.",
);
assert.match(
  canvasSource,
  /onGrabMove\(/,
  "Pointer moves while grabbing must forward the world-space target upward.",
);
assert.match(
  canvasSource,
  /onGrabEnd\(\)/,
  "Pointer up/cancel while grabbing must release the grab.",
);
assert.match(
  appSource,
  /grabEnabled=\{source === "live" && sessionId != null\}/,
  "Grabbing is gated to live sessions only; artifact/demo sources stay pan-only.",
);
assert.match(
  appSource,
  /async function handleGrabStart[\s\S]*?if \(status === "paused"\)[\s\S]*?handleControl\("play"\)/,
  "Grabbing a paused live session must auto-resume playing so the drag has physical feedback.",
);
assert.match(
  appSource,
  /grabMoveInFlight[\s\S]*?pendingGrabTarget/,
  "Grab target updates must coalesce to the latest pointer position with a single in-flight request.",
);
```

(`read(...)`:ui-contract.mjs 若无单文件读取 helper,沿用其现有的文件读取方式——文件列表 join 出 `appSource` 的同时,对 WorldCanvas 单独 `readFileSync` 一次;照现有 `liveControlSource` 抠子串的惯例组织即可,断言语义不变。)

- [ ] **Step 2: 跑契约确认失败**

```bash
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
```

预期:FAIL。

- [ ] **Step 3: 实现**

3a. types.ts(SessionRecord 定义 :590 附近):

```ts
export type GrabMode = "spring" | "direct";

export type ActiveGrabRecord = {
  id: string;
  body_handle: number;
  mode: GrabMode;
  local_anchor: Vec2;
  target: Vec2;
  joint_handle: number | null;
  stiffness: number;
  damping: number;
  max_speed: number;
};
```

`SessionRecord` 内加一行 `active_grab?: ActiveGrabRecord | null;`。

3b. api.ts:

```ts
export async function createGrab(
  sessionId: string,
  request: {
    body_handle: number;
    grab_point: [number, number];
    mode: GrabMode;
    stiffness?: number;
    damping?: number;
    max_speed?: number;
    session_epoch: number;
  },
): Promise<{ grab: ActiveGrabRecord; session: SessionRecord }> {
  return requestJson(`/api/sessions/${sessionId}/grabs`, {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export async function updateGrab(
  sessionId: string,
  grabId: string,
  request: { target: [number, number]; session_epoch: number },
): Promise<{ grab: ActiveGrabRecord; session: SessionRecord }> {
  return requestJson(`/api/sessions/${sessionId}/grabs/${grabId}`, {
    method: "PATCH",
    body: JSON.stringify(request),
  });
}

export async function releaseGrab(
  sessionId: string,
  grabId: string,
): Promise<{ session: SessionRecord }> {
  return requestJson(`/api/sessions/${sessionId}/grabs/${grabId}`, {
    method: "DELETE",
  });
}
```

(types import 行补 `ActiveGrabRecord, GrabMode`。)

3c. WorldCanvas.tsx——props 接口加五项(见 Interfaces),`handlePointerDown`(:378)改为先试抓取:

```ts
const grabPointerId = useRef<number | null>(null)

function handlePointerDown(event: PointerEvent<HTMLCanvasElement>) {
    if (grabEnabled && event.button === 0) {
      const rect = event.currentTarget.getBoundingClientRect()
      const point = screenToWorld(
        { x: event.clientX - rect.left, y: event.clientY - rect.top },
        camera,
      )
      const hit = hitTest(frame.snapshot.colliders, [], [], [], point)
      if (hit?.kind === "collider") {
        const collider = frame.snapshot.colliders.find(
          (entry) => entry.handle === hit.id,
        )
        const body = collider
          ? frame.snapshot.bodies.find((entry) => entry.handle === collider.body)
          : undefined
        if (body && body.body_type === "dynamic") {
          grabPointerId.current = event.pointerId
          event.currentTarget.setPointerCapture(event.pointerId)
          onGrabStart(body.handle, point)
          return
        }
      }
    }
    // 未命中可抓取物体:回落到既有 pan 手势(保留原函数体)。
    ...原 handlePointerDown 逻辑...
}
```

`handlePointerMove`(:391)开头加:

```ts
if (grabPointerId.current === event.pointerId) {
  const rect = event.currentTarget.getBoundingClientRect()
  const point = screenToWorld(
    { x: event.clientX - rect.left, y: event.clientY - rect.top },
    camera,
  )
  onGrabMove(point)
  return
}
```

`handlePointerUp`(:416)与 `handlePointerCancel`(:437)开头加:

```ts
if (grabPointerId.current === event.pointerId) {
  grabPointerId.current = null
  onGrabEnd()
  return
}
```

指示线绘制:`drawWorld`(:565-665)里 lattice 分支之后加 `if (activeGrab) drawGrabOverlay(ctx, activeGrab, frame, camera)`,新函数(风格照 `drawLatticeProxy`):

```ts
function drawGrabOverlay(
  ctx: CanvasRenderingContext2D,
  grab: ActiveGrabRecord,
  frame: FrameRecord,
  camera: Camera,
) {
  // spring 模式画 world 里的真实 joint(锚点来自 snapshot 事实);
  // direct 模式画显示层指示线(body 锚点 → 指针目标)。
  const grabJoint =
    grab.joint_handle != null
      ? frame.snapshot.joints.find((joint) => joint.handle === grab.joint_handle)
      : undefined
  const body = frame.snapshot.bodies.find(
    (entry) => entry.handle === grab.body_handle,
  )
  const anchorWorld = grabJoint
    ? grabJoint.anchors[0]
    : body
      ? {
          x:
            body.transform.translation.x +
            grab.local_anchor.x * Math.cos(body.transform.rotation) -
            grab.local_anchor.y * Math.sin(body.transform.rotation),
          y:
            body.transform.translation.y +
            grab.local_anchor.x * Math.sin(body.transform.rotation) +
            grab.local_anchor.y * Math.cos(body.transform.rotation),
        }
      : null
  const targetWorld = grabJoint ? grabJoint.anchors[1] : grab.target
  if (!anchorWorld) {
    return
  }
  const start = worldToScreen(anchorWorld, camera)
  const end = worldToScreen(targetWorld, camera)
  ctx.strokeStyle = grab.mode === "spring" ? "#8ec07c" : "#83a598"
  ctx.lineWidth = 2.2
  ctx.setLineDash(grab.mode === "spring" ? [] : [6, 4])
  ctx.beginPath()
  ctx.moveTo(start.x, start.y)
  ctx.lineTo(end.x, end.y)
  ctx.stroke()
  ctx.setLineDash([])
  ctx.fillStyle = "#8ec07c"
  ctx.beginPath()
  ctx.arc(end.x, end.y, 4, 0, Math.PI * 2)
  ctx.fill()
}
```

3d. App.tsx grab 编排(放在 velocity perturbation 编排附近;`grabSettings` 状态在 Task 8 落地前先用常量 `const grabSettings = { mode: "spring" as GrabMode, stiffness: 40 }`,Task 8 替换为 state):

```ts
const [activeGrab, setActiveGrab] = useState<ActiveGrabRecord | null>(null)
const grabMoveInFlight = useRef(false)
const pendingGrabTarget = useRef<Vec2 | null>(null)

async function handleGrabStart(bodyHandle: number, point: Vec2) {
    if (source !== "live" || !sessionId) {
      return
    }
    if (status === "paused") {
      void handleControl("play")
    }
    try {
      const result = await createGrab(sessionId, {
        body_handle: bodyHandle,
        grab_point: [point.x, point.y],
        mode: grabSettings.mode,
        stiffness: grabSettings.stiffness,
        session_epoch: sessionEpoch,
      })
      setActiveGrab(result.grab)
    } catch (error) {
      pushLogs(
        log("warn", t(locale, "log.grabFailed", { message: messageOf(error) })),
      )
    }
}

function handleGrabMove(point: Vec2) {
    pendingGrabTarget.current = point
    void flushGrabTarget()
}

async function flushGrabTarget() {
    if (grabMoveInFlight.current) {
      return
    }
    const grab = activeGrabRef.current
    const target = pendingGrabTarget.current
    if (!grab || !target || !sessionId) {
      return
    }
    pendingGrabTarget.current = null
    grabMoveInFlight.current = true
    try {
      const result = await updateGrab(sessionId, grab.id, {
        target: [target.x, target.y],
        session_epoch: sessionEpoch,
      })
      setActiveGrab(result.grab)
    } catch {
      // 拖拽期间的瞬时失配交给下一次 move 重试;释放路径兜底清理。
    } finally {
      grabMoveInFlight.current = false
      if (pendingGrabTarget.current) {
        void flushGrabTarget()
      }
    }
}

async function handleGrabEnd() {
    const grab = activeGrabRef.current
    setActiveGrab(null)
    pendingGrabTarget.current = null
    if (!grab || !sessionId) {
      return
    }
    try {
      await releaseGrab(sessionId, grab.id)
    } catch (error) {
      pushLogs(
        log("warn", t(locale, "log.grabReleaseFailed", { message: messageOf(error) })),
      )
    }
}
```

`activeGrabRef`:`const activeGrabRef = useRef<ActiveGrabRecord | null>(null)`,配 `useEffect(() => { activeGrabRef.current = activeGrab }, [activeGrab])`(异步回调不吃过期闭包)。会话失效时清理:在 `invalidateLiveResponses()` 里追加 `setActiveGrab(null); pendingGrabTarget.current = null; grabMoveInFlight.current = false;`(该函数是普通函数不是 hook,直接加)。i18n 加四键:`log.grabFailed` zh "抓取失败:{message}" / en "Grab failed: {message}",`log.grabReleaseFailed` zh "释放抓取失败:{message}" / en "Grab release failed: {message}"(i18n-contract 逐 key 断言各补一行)。

3e. WorkbenchLayout 透传五个新 props 到 WorldCanvas;App 的 `<WorkbenchLayout>` 传:

```tsx
grabEnabled={source === "live" && sessionId != null}
activeGrab={activeGrab}
onGrabStart={handleGrabStart}
onGrabMove={handleGrabMove}
onGrabEnd={handleGrabEnd}
```

- [ ] **Step 4: 跑契约与构建确认通过**

```bash
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
rtk proxy npm --prefix crates/picea-lab/web run build
```

预期:均通过。

- [ ] **Step 5: Commit**

```bash
git add crates/picea-lab/web/src/api.ts crates/picea-lab/web/src/types.ts crates/picea-lab/web/src/App.tsx crates/picea-lab/web/src/components/workbench/WorldCanvas.tsx crates/picea-lab/web/src/components/workbench/WorkbenchLayout.tsx crates/picea-lab/web/src/i18n.ts crates/picea-lab/web/scripts/ui-contract.mjs crates/picea-lab/web/scripts/i18n-contract.mjs
git commit -m "web: grab-and-drag rigid bodies in live sessions with canvas overlay

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 8: web — 拖拽设置 UI(模式切换 + 强度滑条)

**Files:**
- Modify: `crates/picea-lab/web/src/App.tsx`(`grabSettings` 常量 → state)
- Modify: `crates/picea-lab/web/src/components/workbench/RunSettingsPanel.tsx`(设置控件)
- Modify: `crates/picea-lab/web/src/i18n.ts`
- Test: `crates/picea-lab/web/scripts/ui-contract.mjs`、`crates/picea-lab/web/scripts/i18n-contract.mjs`

**Interfaces:**
- Consumes: Task 7 的 `grabSettings` 用点(`handleGrabStart` 的 createGrab 参数)。
- Produces: `type GrabSettings = { mode: GrabMode; stiffness: number }`(App state);RunSettingsPanel props 加 `grabSettings: GrabSettings`、`setGrabSettings: (value: GrabSettings) => void`。

- [ ] **Step 1: 写失败断言**

`ui-contract.mjs`:

```js
assert.match(
  appSource,
  /const \[grabSettings, setGrabSettings\] = useState<GrabSettings>/,
  "Grab mode and spring strength must be user-adjustable state, not hardcoded constants.",
);
assert.match(
  appSource,
  /"run\.grabMode"/,
  "Run settings should expose the grab mode selector with localized labels.",
);
assert.match(
  appSource,
  /min=\{2\}[\s\S]{0,220}max=\{420\}/,
  "Grab spring strength slider should span the scene-calibrated stiffness range (2..420).",
);
```

`i18n-contract.mjs` 逐 key 断言补:`run.grabMode`、`run.grabModeSpring`、`run.grabModeDirect`、`run.grabStiffness`(每 locale 各四行 `assert.equal(typeof messages[locale][...], "string")`)。

- [ ] **Step 2: 跑契约确认失败**

```bash
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
```

预期:FAIL。

- [ ] **Step 3: 实现**

3a. i18n 两 locale 各加:

```ts
"run.grabMode": "拖拽模式",
"run.grabModeSpring": "软弹簧",
"run.grabModeDirect": "硬跟手",
"run.grabStiffness": "拖拽强度",
```

```ts
"run.grabMode": "Grab mode",
"run.grabModeSpring": "Spring",
"run.grabModeDirect": "Direct",
"run.grabStiffness": "Grab strength",
```

3b. App.tsx:Task 7 的常量替换为

```ts
type GrabSettings = { mode: GrabMode; stiffness: number }
const [grabSettings, setGrabSettings] = useState<GrabSettings>({
  mode: "spring",
  stiffness: 40,
})
```

`handleGrabStart` 的 createGrab 参数改为 `mode: grabSettings.mode, stiffness: grabSettings.stiffness, damping: Math.max(0.5, grabSettings.stiffness * 0.05)`(阻尼随强度联动)。

3c. RunSettingsPanel:props 加 `grabSettings/setGrabSettings`,在 runMode Select(:105-119)之后加一组控件(照现有 Select/Input 用法):

```tsx
<Select
  value={grabSettings.mode}
  onValueChange={(value) =>
    setGrabSettings({ ...grabSettings, mode: value as GrabMode })
  }
  ariaLabel={t(locale, "run.grabMode")}
  items={[
    { value: "spring", label: t(locale, "run.grabModeSpring") },
    { value: "direct", label: t(locale, "run.grabModeDirect") },
  ]}
/>
<Input
  type="number"
  min={2}
  max={420}
  step={1}
  value={grabSettings.stiffness}
  onChange={(event) =>
    setGrabSettings({
      ...grabSettings,
      stiffness: Number(event.target.value) || 40,
    })
  }
  aria-label={t(locale, "run.grabStiffness")}
/>
```

(控件外层的 label/布局 div 照该面板 frameCount 字段的现有包装抄。)App 的 `<WorkbenchLayout>`→RunSettingsPanel 链路透传两个新 props。

- [ ] **Step 4: 跑契约与构建确认通过**

```bash
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
rtk proxy npm --prefix crates/picea-lab/web run build
```

- [ ] **Step 5: Commit**

```bash
git add crates/picea-lab/web/src/App.tsx crates/picea-lab/web/src/components/workbench/RunSettingsPanel.tsx crates/picea-lab/web/src/components/workbench/WorkbenchLayout.tsx crates/picea-lab/web/src/i18n.ts crates/picea-lab/web/scripts/ui-contract.mjs crates/picea-lab/web/scripts/i18n-contract.mjs
git commit -m "web: expose grab mode and spring strength in run settings

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 9: 全量门禁 + 浏览器三面验收 + 文档收尾

**Files:**
- Modify: `docs/design/2026-07-10-workbench-interactive-run-model-design.md`(状态:待评审 → 已确认)
- Modify: `docs/ai/doc-catalog.yaml`(登记本计划文档)

- [ ] **Step 1: 全量门禁**

```bash
rtk proxy cargo test -p picea-lab
rtk proxy cargo test -p picea --lib
rtk proxy cargo test --workspace --all-targets --no-run
rtk proxy cargo fmt && rtk proxy cargo clippy
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
rtk proxy npm --prefix crates/picea-lab/web run build
```

预期:全部通过、无警告(macro-tools 的既有 trybuild 漂移不在本计划范围,不因它 fail 本计划)。

- [ ] **Step 2: 浏览器三面验收(真实 dev URL)**

```bash
just web-start && just status
```

打开 `http://127.0.0.1:5173/`,依次验收并截图:

1. **live 面(默认)**:首屏加载后无需点击运行即出现真实模拟(source badge = 实时会话);切换到 `newton_cradle`,选中即自动运行;鼠标按住任一摆球拖动——spring 模式看到实线弹簧线、球被碰撞阻挡、甩手可抛;切 direct 模式重复,虚线指示线、球贴指针但不穿透地板。工具栏场景 tooltip 出现看点文案(切中英文各看一次)。
2. **artifact 面**:runMode 切到"生成产物并回放",选中 `matrix_stack` 自动跑一次 artifact 并回放;画布不可拖拽(pan 正常)。
3. **离线面**:`just web-stop` 停后端(或临时停 API server),刷新页面——画布出现"离线演示数据 · 非真实模拟"水印,source badge = 演示回放。

验收后:

```bash
just web-stop
```

任何一面不符 → 回到对应任务修复,重跑该任务测试,再回本步。

- [ ] **Step 3: 文档收尾**

设计文档头部"状态:待评审"改"状态:已确认";`docs/ai/doc-catalog.yaml` 末尾登记(格式照 2026-06-17 计划条目):

```yaml
- path: docs/plans/2026-07-10-workbench-interactive-run-model-plan.md
  title: Workbench Interactive Run Model Plan
  kind: plan
  authority: medium
  status: active
  owner: supervisor
  applies_to: [picea-lab, picea-lab-web, live-session, interaction, run-model, milestone]
  keywords: [select-to-run, watermark, watch-for, grab, drag, spring, direct, server-routes, ui-contract]
  last_reviewed: 2026-07-10
```

```bash
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
```

- [ ] **Step 4: Commit(文档)**

```bash
git add docs/design/2026-07-10-workbench-interactive-run-model-design.md docs/plans/2026-07-10-workbench-interactive-run-model-plan.md docs/ai/doc-catalog.yaml docs/design/README.md
git commit -m "docs: land workbench interactive run-model design and plan

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

(注意:doc-catalog.yaml / README.md 里已存在的 vNext 未提交登记行属上一批工作,如果此时仍未被单独提交,先向用户确认是否随本 commit 一并入库,不要擅自决定。)

- [ ] **Step 5: 汇报**

向用户汇报:全部门禁输出、三面验收截图、遗留事项(spring 手感参数最终值、④ revolute 立项待启动),并请示合并方式(direct merge / PR)。
