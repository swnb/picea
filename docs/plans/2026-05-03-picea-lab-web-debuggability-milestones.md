# Picea Lab-Web 可调试性深度优化里程碑计划

状态：已完成（live-first 实施）
计划文档：docs/plans/2026-05-03-picea-lab-web-debuggability-milestones.md
最后更新：2026-05-03
提交策略：不提交
执行策略：已按 M31-M34 连续执行

## 目标

把 `picea-lab-web` 从“观察一次产物”推进到“实时接入、可步进、可定位问题来源”的调试工作台。
完成后，用户应该能从同一个页面回答：

- 当前调试对象是 demo replay、Rust artifact replay，还是 Rust live session。
- live session 是否能可靠 step / play / pause / reset，并且每一步都来自 Rust 后端 authoritative world。
- 中间 panel 是否是完整可缩放 canvas，支持 pan / zoom / fit / reset，不再只是自适应观察窗口。
- 网格和标尺是否能独立 toggle，并且坐标、比例尺、命中选择都跟随 canvas camera。
- 某一帧里哪些 exported facts 变化了，哪些 `WorldEvent` / `StepReport` / counters 解释了变化。
- 浏览器中看到的 source、frame、selected entity、layers、camera、grid/ruler 和 provenance 是否足以复现一次调试观察。

## 约束

- 工作区：只在 `/Users/asyncrustacean/projects/picea` 当前主工作区规划。
- 计划阶段只写本计划文档，不动实现代码。
- 当前工作区有未跟踪 `.playwright-cli/`，视为既有本地验证产物；本计划不触碰、不清理、不纳入后续 diff。
- 所有验证命令使用 `rtk proxy`，遵守 `~/.codex/RTK.md`。
- 本轮改为 live-first：`Rust live session` 是主调试入口；artifact replay 保留为证据、回归和兼容路径，不再作为首要交互模型。
- `picea-lab-web` 不运行 physics，不重新计算 broadphase traversal、island grouping、decomposition、collision、mass/inertia 或 solver 结果；live step 必须来自 Rust 后端 session。
- Web 第一层优先消费已有 `FrameRecord`、`DebugSnapshot`、server live session facts、`debug_render.json`、`final_snapshot.json` 和 `perf.json`；只有证据表明 carrier 不足时，才做 additive Rust schema。
- 不实现 visual editor、running-world patch、paused transaction、handle invalidation 新语义、server-side autonomous tick loop、SSE/WebSocket streaming 或 live rewind。
- 不修改 solver row math、CCD 算法、narrowphase/contact manifold、broadphase heuristics 或 public core API，除非后续 M35 明确需要只读 debug carrier。
- Plan Gate 批准后、M31 实施前先把本计划登记进 `docs/ai/doc-catalog.yaml`；`docs/ai/index.md` / `docs/ai/repo-map.md` 在 M34 按最终调试 workflow 收口。

## 待确认问题

### 必须确认

- 这是一次方向重排：M31 不再优先做 `perf.json` artifact summary，而是先做 live 调试步进。
- live-first / zoomable canvas / grid-ruler 要求是在首轮 reviewer 之后加入的；最终 Plan Gate 前需要二次 reviewer 复审。

### 可带假设推进

- M31 先完善 `Rust live session` 的 step / play / pause / reset：早期验证是浏览器里从 Web 创建 live session，并连续 step，帧缓冲、状态和 source badge 一致。
- M32 做完整 zoomable canvas 和 grid/ruler toggle：早期验证是浏览器里 pan/zoom/fit/reset、点击选择、网格/标尺开关都工作。
- M33 只做 exported frame facts 的 drilldown，不新增物理事实：早期验证是 `FrameRecord` 中已有 `stats` / `events` / `report` 能被 Web typed 消费。
- M34 做 artifact evidence / `perf.json` summary / copy debug context closeout：早期验证是缺少 `perf.json` 时 UI 显示 missing evidence state，不崩溃、不误报为 0。
- 浏览器验收优先使用 `browser-use:browser`；若当前 Codex app 未暴露可用 in-app browser backend，允许降级到 Playwright CLI，并在结果中报告 `.playwright-cli/` 产物。
- 新增 trace carrier 不进入 M31-M34 默认执行；只有 M31-M34 完成后仍能证明 existing facts 不足，才另开 M35，并且默认最多落一个 trace family。

## 调研结论

### 已有能力

- Web 入口和状态编排在 `crates/picea-lab/web/src/App.tsx`，已经区分 `demo replay`、`Rust artifact replay`、`Rust live session`。
- `App.tsx` 已有 live session 状态：`runMode`、`source`、`status`、`liveStepInFlight`、`liveGenerationRef`、`liveRequestTokenRef`，并通过 `controlSession(sessionId, "step")` 从后端推进。
- `api.ts` 的 `controlSession` 已暴露 `play` / `run` / `reset` / `step` / `pause` action；`server.rs` 已有 `control_live_session`、`step_live_session`、`build_live_runtime`。
- `server_routes.rs` 已有 live session 行为锁：step 推进后端 world、reset 清空 buffer、play 当前是 status-only、override patch 会被拒绝且不 mutate live runtime。
- Workbench 布局在 `crates/picea-lab/web/src/components/workbench/WorkbenchLayout.tsx`，左侧 hierarchy、中间 canvas、底部 timeline/logs/run setup、右侧 inspector 的骨架已经稳定。
- Workbench 使用 `react-resizable-panels`，中间 panel 已经可以作为 canvas 主区域继续强化；当前缺口不是布局骨架，而是 canvas camera / controls。
- Canvas 在 `WorldCanvas.tsx` 中支持 HiDPI sizing、`ResizeObserver`、shapes、AABB、contacts、velocities、trace、broadphase tree、islands、provenance overlays，并有基本点击命中。
- `WorldCanvas.tsx` 当前以 `makeCamera(...)` 自动 fit 为主；未看到持久 pan/zoom state、fit/reset controls、wheel/drag camera 或 camera-aware ruler。
- `WorldCanvas.tsx` 当前无条件绘制 grid / axes；`LayerState` 还没有 `grid` / `rulers` 开关。`Toolbar.tsx` 已根据 `layers` 动态生成 layer menu，增加 layer 字段和 i18n 后可接入 toggle。
- Inspector 已经展示 entity facts、stage facts、process facts、contact GJK/EPA、CCD trace、warm-start/solver impulse 和 compound provenance。
- `types.ts` 已定义 `DebugBroadphaseTree`、`DebugIsland`、`CompoundProvenance`、CCD/contact fields 和部分 solver/island counters。
- Server 已支持 artifact replay 和 request-driven live session；live session 在 Rust 侧持有 authoritative `World + SimulationPipeline`，Web 只消费返回的 `FrameRecord`。
- Web contract 已有 `npm run test:ui-contract` 和 `npm run test:i18n`，能锁住关键 UI/i18n 入口。

### 已有 Rust / artifact facts

- `crates/picea/src/debug.rs` 已导出 `DebugSnapshot`、`DebugStats`、`DebugBroadphaseTree`、`DebugIsland` 等 read model。
- `crates/picea-lab/src/artifact.rs` 的 `FrameRecord` 写入 `report`、`stats`、`events`、`snapshot` 和 `compound_provenance`。
- `debug_render.json` 已保留 frame-level render facts，包括 broadphase tree、islands、contacts、manifolds、CCD counters、solver rows 和 compound provenance。
- `perf.json` 已有 `PerfCounterSummary`，包含 broadphase traversal/prune、tree depth、contact rows、joint rows、solver body slots、island counts、CCD counts 等运行级汇总。
- `crates/picea-lab/tests/artifact_run.rs::artifact_schema_keeps_final_observability_fact_set` 是当前结构化 artifact fact set 的核心锁点。

### 主要缺口

- Web 虽已有 live session，但 UI 仍像“选择一次 run/source 后看帧”而不是“进入调试会话后控制后端一步步运行”；需要把 live controls、in-flight 状态、错误恢复和 frame buffer 语义产品化。
- `play` 目前由前端 interval 连续调用后端 `step`，后端不做 autonomous tick loop；这是合理边界，但 UI 需要明确 pause/reset/stale response/in-flight 行为。
- 中间 canvas 目前是 auto-fit viewport，不是完整调试画布；缺少 pan/zoom/fit/reset、camera retention、camera-aware hit testing、网格 toggle、标尺 toggle 和比例尺表达。
- Web 目前主要读取 `frames.jsonl` 和 `final_snapshot.json`；`perf.json` 没有进入 Web API / UI 读链路，但这在 live-first 计划中降为 M34 evidence/closeout。
- Rust `FrameRecord` 有 `report` / `stats` / `events`，但 Web `FrameRecord` type 目前没有 typed 表达这些顶层字段，UI 也没有 frame event / counter delta drilldown。
- Solver body slots、contact rows、joint rows 等 counters 已在 schema 里，但 Web 只部分展示，不能形成调试面。
- `DebugIsland` 现在主要是结果态：id、bodies、sleeping、reason。若要解释 split / merge / sleep / wake 的过程，仍需要 event-like trace carrier。
- Broadphase tree overlay 能画 node AABB，但 traversal/prune 过程仍主要是聚合计数；若要解释“为什么这对被访问或剪枝”，需要 additive trace。
- Query 调试面目前缺失。Core `QueryPipeline` 与 query stats 已经存在，但 Web 没有独立 query read model 或 query path visualization；这不阻塞 live step / canvas / frame diagnostics。

### 已跑基线验证

- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`：通过。
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`：通过。
- `rtk proxy cargo test -p picea-lab --test artifact_run`：通过，16 tests。
- `rtk proxy cargo test -p picea-lab --test server_routes`：通过，4 tests。

## 优化方向

### 低风险高收益

- 把 `Rust live session` 提到主入口，完善 step / play / pause / reset / source badge / status / stale response / in-flight feedback。
- 保持 request-driven live step：Web 负责控制节奏，Rust 后端负责 authoritative step；不引入 server-side autoplay loop。
- 中间 panel 做完整 canvas：pan / zoom / fit / reset、wheel/trackpad/drag、HiDPI、camera retention、camera-aware hit test。
- 网格和标尺进入 layer controls：grid 可隐藏，ruler 可独立显示坐标刻度，比例尺跟随 zoom。
- 接入 `perf.json` 时做 run-level counter summary，但放到 live/canvas/diagnostics 之后，不再抢首个 milestone。
- 旧 artifact 如果缺少 `perf.json`，Web 应显示“缺失证据”状态，不崩溃、不把缺失解释成 0 成本。

### 不新增物理事实的 Web 深化

- typed 消费 live frame 和 artifact frame 的 `FrameRecord.report`、`FrameRecord.stats`、`FrameRecord.events`。
- 做 frame diagnostics tab：counter delta、event list、selected entity 关联 facts、当前帧与上一帧的 exported-fact 变化。
- 让 timeline 可以筛选“候选激增、island sleep/wake、CCD clamp、contact row 变化”等已有事实，不计算物理，只比较已导出字段。

### 需要 Rust additive schema（后续候选）

- Broadphase traversal / prune trace：用 enum-like facts 表达访问、剪枝、候选输出原因，避免 Web 猜测 tree 过程。
- Island lifecycle trace：表达 split / merge / sleep / wake / skipped hot rows 的原因和相关 body/contact/joint handles。
- Query debug carrier：表达 query input、candidate/drop/hit counters、filters、shape/ray/AABB hit summary，仍不暴露 internal proxy ids。

### 需要真实浏览器验收

- Live step/play/pause/reset、canvas overlay、layer menu、pan/zoom、grid/ruler、click hit-testing、selection-to-inspector correlation、source badge、timeline diagnostics 都必须用真实页面验收。
- Web build 和 contract scripts 只能证明类型/文本/入口存在，不能证明调试工作流真的可用。

### 后续大工程

- 多 run 对比、benchmark explorer、live session rewind、paused transaction preview、streaming live tick 和 visual editor 都属于后续路线，不进入本轮 M31-M34。

## 规划依据

- explorer：已运行。结论：当前 Web 已有清晰 workbench 骨架，最大短板是 live 调试体验仍不够产品化、canvas 仍是 auto-fit 观察窗口、`perf.json` 没进读链路，以及过程 facts 仍多为聚合计数。
- 关键证据：
  - `crates/picea-lab/web/src/App.tsx`：已有 live session 编排和快捷键，但 live-first 调试体验还不完整。
  - `crates/picea-lab/web/src/api.ts`：已有 `controlSession`，但没有 `fetchPerf` / `fetchDebugRender`。
  - `crates/picea-lab/web/src/components/workbench/WorldCanvas.tsx`：当前自动 fit、无条件绘制 grid/axes，还没有可缩放 canvas camera 和 grid/ruler layer toggle。
  - `crates/picea-lab/web/src/types.ts`：Web `FrameRecord` 缺少 Rust `report` / `stats` / `events` 顶层字段，`LayerState` 也缺 `grid` / `rulers`。
  - `crates/picea-lab/src/artifact.rs`：Rust `FrameRecord`、`DebugRenderArtifact`、`PerfArtifact` 已有结构化 facts。
  - `crates/picea-lab/tests/artifact_run.rs`：artifact schema / serde-default / final observability fact set 已有测试基础。
  - `crates/picea-lab/tests/server_routes.rs`：live session step/reset/play/patch 行为已有锁点。
  - `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`：M24/M25 已完成，后续不能重开为 visual editor 或 live patch。
  - `docs/design/picea-lab-observability-architecture.md`：lab 负责 capture/render/compare/summarize facts，core 负责 expose facts。
- 主要未知：
  - canvas camera 优先自写轻量 state 还是引入组件；本计划的选择标准是：能保留现有 canvas draw pipeline、HiDPI、hit-testing 和 overlay，不引入重型 scene graph。若现有组件无法低风险贴合，则自写 camera state。
  - `WorldEvent` 在 Web 中是否需要完整 typed union，还是第一版先显示结构化 JSON 摘要。
  - Query 调试应该先做统计面板还是路径可视化；本计划把 query path 放到 M35 之后的候选 trace slice，不阻塞 M31-M34。
  - browser-use backend 在当前 Codex app 中可能不可用，需要保留 Playwright CLI 降级验收路径。

## 计划验收

- 状态：已通过
- reviewer：已运行首轮；live-first / canvas 新需求由用户确认后进入实施。本轮以 contract / build / Rust tests / browser-use 真实页面验收作为收口门。
- 审查结论：首轮发现已采纳：原 M33 收窄为后续单一 trace family + carrier 归属规则，旧 `perf.json` 缺失兼容契约保留到 M34，浏览器验收改为可复跑命令；新增 live-first / zoomable canvas / grid-ruler 要求已重排进 M31-M32。
- 用户确认：已确认执行；要求最终通过 `browser-use:browser` 验收。

## 里程碑

## M31：Live Debug Session Controls

状态：已完成

目标：
把 `Rust live session` 变成 `picea-lab-web` 的主调试入口，完善 step / play / pause / reset / status / source provenance，让用户不是观察一个产物，而是在页面里控制 Rust 后端世界一步步前进。

为什么现在做：
用户现在最关心调试步进和实时接入。仓库已经有 request-driven live session、server route 行为锁和 Web 基础状态，第一步应把这些能力打磨成稳定的调试工作流，而不是继续围绕 artifact observation 做 UI。

范围：
- 如果 Plan Gate 批准后尚未登记，本里程碑先把本计划登记进 `docs/ai/doc-catalog.yaml`，并用 YAML 校验确认路由可读。
- 将 `Rust live session` 提升为明确的调试 mode：source badge、session id、scenario、frame buffer length、current frame、backend status 可见。
- 完善 step：单步按钮和 `ArrowRight` 在 live session 下只发一次后端 `step`，in-flight 时禁用或显示 busy，不允许 stale response 覆盖新 session。
- 完善 play/pause：前端可以用 interval 连续发后端 `step`，但 UI 必须清楚表达这是 Web-driven stepping；pause 能停止后续请求，不引入 server-side tick loop。
- 完善 reset：reset 后 frame buffer、selection、timeline、status、error state 与后端 world 一致；旧 generation 的响应不能污染新 buffer。
- 完善错误与空态：后端 session missing、step failure、reset failure、scenario rebuild failure 都进入可恢复 UI 状态。
- 保留 artifact replay / demo replay 入口，但视觉主心智不再是“观察一次产物”。
- 更新 `ui-contract` 和 `i18n-contract`，锁住 live controls、source/status、错误/空态文案。

不做：
- 不做 SSE/WebSocket streaming。
- 不做 server-side autonomous autoplay loop。
- 不做 live patch、visual editor、paused transaction 或 live rewind。
- 不新增 physics facts、query debug、frame diagnostics 或 `perf.json` summary。

所有权：
- 可能触及：`docs/ai/doc-catalog.yaml`、`crates/picea-lab/web/src/App.tsx`、`crates/picea-lab/web/src/api.ts`、`crates/picea-lab/web/src/types.ts`、`crates/picea-lab/web/src/components/workbench/Toolbar.tsx`、`crates/picea-lab/web/src/components/workbench/Timeline.tsx`、`crates/picea-lab/web/src/components/workbench/RunSetup.tsx`、`crates/picea-lab/web/src/i18n.ts`、contract scripts。
- 可能少量触及：`crates/picea-lab/src/server.rs`、`crates/picea-lab/tests/server_routes.rs`，仅限补齐 live status/reset/step 行为锁或返回字段。
- 不应触及：`crates/picea/src/*`、physics algorithms、artifact schema、running-world patch semantics。

验收标准：
- 用户能从 Web 创建 live session，并使用 step / play / pause / reset 调试后端 world。
- 每个 live frame 都来自后端返回的 `FrameRecord`；Web 不计算下一帧 physics。
- 快速连续 step、reset 后旧请求返回、切换 source 后旧 session 返回，都不会污染当前 frame buffer。
- source badge/status 能清楚区分 demo replay、Rust artifact replay、Rust live session。
- Keyboard shortcuts 与按钮行为一致；in-flight / error / empty states 清晰。
- Artifact replay 和 demo replay 不被 live-first UI 破坏。

验证方式：
- `rtk proxy cargo test -p picea-lab --test server_routes`
- 如触及 server：`rtk proxy cargo test -p picea-lab`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `rtk proxy git diff --check`
- 如更新 `docs/ai/doc-catalog.yaml`：`rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- 浏览器验收：
  - 后端：`rtk proxy cargo run -p picea-lab -- serve --bind 127.0.0.1:18080`
  - Web：`cd crates/picea-lab/web && rtk proxy npm run dev -- --host 127.0.0.1 --port 5174`
  - 优先用 `browser-use:browser` 打开 `http://127.0.0.1:5174/`，创建 `Rust live session`，执行 step、连续 play、pause、reset，再切换 artifact replay，确认 source/status/frame buffer 不混淆。
  - 如果 `browser-use:browser` 不可用，降级 Playwright CLI；verifier 必须报告是否产生 `.playwright-cli/` 及其路径。

Subagent 执行计划：
- explorer：不需要单独再跑；worker 实施前快速复核 live route / Web state 现状。分派时必须明确写入：“你是叶子 agent，不允许再启动 subagent。”
- worker：叶子 agent；优先 `gpt-5.4`。只做 live controls、状态、contract 和必要 server behavior locks，不改 physics。
- reviewer：叶子 agent；审查 stale response、reset consistency、source provenance、是否引入 server-side autoplay。
- verifier：叶子 agent；运行上述 Rust/Web/Browser gates，报告是否产生 `.playwright-cli/`。

提交策略：
- auto-commit：否。
- message hint：`Polish lab-web live debug controls`。

风险 / 后续：
- 前端 interval 连续 step 是可接受的 V1；如果后续需要真正 streaming，可另立 milestone，避免本轮扩大协议。

## M32：Zoomable Full Canvas And Grid Rulers

状态：已完成

目标：
把中间 panel 做成完整调试 canvas：支持 pan / zoom / fit / reset，保留 HiDPI 和 overlay，网格与标尺可以 toggle，并且命中选择、坐标显示、比例尺都跟随 camera。

为什么现在做：
live step 能跑起来后，用户马上需要在同一个 canvas 上放大看局部、拖动观察世界、开关网格和标尺。当前 `WorldCanvas` 已有绘制与 overlay 基础，但还是 auto-fit 观察窗口，不够像调试画布。

范围：
- 增加 canvas camera state：center/offset、zoom、fit-to-world、reset view，resize 后不丢失用户视角。
- 支持 wheel/trackpad zoom、drag pan、fit/reset controls；按钮使用图标并有 tooltip。
- 保留 `WorldCanvas` 现有 HiDPI sizing、ResizeObserver、overlay draw pipeline 和 selection hit-testing。
- hit-testing、hover/selection、trace highlight、broadphase tree/island overlay 都使用同一 camera transform。
- `LayerState` 增加 `grid` / `rulers`，接入 `Toolbar` layer menu 和 i18n。
- grid toggle 控制背景网格；ruler toggle 控制顶部/左侧或 canvas 内坐标刻度与比例尺。ruler 关闭时不影响 axes/shape 本身。
- camera / grid / ruler 状态进入 copy-debug-context 的候选字段，但真正复制可留到 M34。
- 组件选择规则：优先沿用现有 canvas 并自写小 camera helper；只有当第三方组件能低风险保留 draw pipeline、HiDPI、hit-testing 和 overlay 时才引入。
- 更新 UI/i18n contract，锁住 canvas controls、grid/ruler layer labels 和 empty/loading 文案。

不做：
- 不改 physics、server session 或 artifact schema。
- 不做 visual editor、拖动物体、live patch、selection mutation。
- 不做 mini-map、多 viewport、measure tool 或 benchmark explorer。
- 不引入重型 scene graph，除非实施前明确证明收益大于风险并修订计划。

所有权：
- 可能触及：`crates/picea-lab/web/src/types.ts`、`crates/picea-lab/web/src/App.tsx`、`crates/picea-lab/web/src/components/workbench/WorldCanvas.tsx`、`crates/picea-lab/web/src/components/workbench/Toolbar.tsx`、`crates/picea-lab/web/src/components/workbench/WorkbenchLayout.tsx`、`crates/picea-lab/web/src/components/workbench/selection.ts`、`crates/picea-lab/web/src/i18n.ts`、contract scripts。
- 可新增小文件：`crates/picea-lab/web/src/components/workbench/canvasCamera.ts` 或等价 helper。
- 不应触及：Rust crates、server routes、artifact schema。

验收标准：
- 中间 panel 是完整 canvas 调试区域，缩放/拖动不会让布局跳动或文本遮挡。
- 用户可以 zoom in/out、pan、fit-to-world、reset view；resize 后视角行为可解释。
- grid 和 ruler 可独立 toggle，layer menu 文案完整。
- ruler/scale 随 zoom 更新，且不会在小 viewport 与 toolbar/timeline 重叠。
- 选择 body/collider/contact/joint 在缩放/拖动后仍命中正确。
- live session 和 artifact replay 的 canvas 行为一致。

验证方式：
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `rtk proxy git diff --check`
- 浏览器验收：
  - 后端：`rtk proxy cargo run -p picea-lab -- serve --bind 127.0.0.1:18080`
  - Web：`cd crates/picea-lab/web && rtk proxy npm run dev -- --host 127.0.0.1 --port 5174`
  - 优先用 `browser-use:browser` 打开 `http://127.0.0.1:5174/`，在 live session 和 artifact replay 中分别执行 zoom、pan、fit、reset、grid toggle、ruler toggle、点击选择；检查 canvas 非空、无 console error、无明显 overlap。
  - 如果 `browser-use:browser` 不可用，降级 Playwright CLI；verifier 必须报告是否产生 `.playwright-cli/` 及其路径。

Subagent 执行计划：
- explorer：worker 实施前只读确认 `WorldCanvas` camera/draw/hit-test 结构和是否已有可复用 helper。分派时必须明确写入：“你是叶子 agent，不允许再启动 subagent。”
- worker：叶子 agent；优先 `gpt-5.4`。限定 Web canvas/type/UI/contract，不改 Rust。
- reviewer：叶子 agent；审查 camera transform、HiDPI、hit-testing、grid/ruler toggle、layout overlap 和第三方组件风险。
- verifier：叶子 agent；运行 Web gates 和浏览器 canvas 验收。

提交策略：
- auto-commit：否。
- message hint：`Add lab-web zoomable debug canvas`。

风险 / 后续：
- canvas camera 很容易和 hit-testing 分叉；必须让绘制和命中共享同一 world/screen transform helper。

## M33：Frame Diagnostics Drilldown

状态：已完成

目标：
让 Web 能按帧阅读 live / artifact frame 中 Rust 已导出的 `FrameRecord.report`、`FrameRecord.stats`、`FrameRecord.events`，并把 counter delta / event correlation 做成可筛选的调试面。

为什么现在做：
M31 解决“我能控制后端一步步跑”。M32 解决“我能在 canvas 上看清楚”。M33 再回答“这一帧为什么变了”。这仍然消费已有 exported facts，不需要先新增 core debug carrier。

范围：
- 扩展 Web `FrameRecord` type，覆盖 Rust 顶层 `stats`、`events`、`report` 的第一版只读结构。
- 增加 frame diagnostics tab 或 panel，显示当前帧 exported stats、上一帧 delta、events、state hash、StepReport 状态。
- `WorldEvent` 第一版使用最小 envelope：`type` / `phase` / `body_handles` / `collider_handles` / `joint_handles` / `contact_id` / `payload_json`；未知事件保留 JSON details，但 UI 只把它表达为 exported event facts。
- 增加常用 filter：broadphase candidate/traversal/prune 变化、island sleep/wake、solver row 变化、CCD hit/clamp、contact/manifold count 变化。
- 选中 body/collider/contact/joint 时，诊断面尽量高亮相关 exported facts；只做 handle/event/filter 关联，不推导物理过程。
- 更新 demo data 或 fallback 使缺少 `report/events/stats` 的 frame 有清楚 empty state。
- 让 diagnostics 同时适用于 live session buffer 和 artifact replay frames。
- 更新 UI/i18n contract，锁住 diagnostics 入口和关键文案。

不做：
- 不新增 Rust core facts。
- 不做 broadphase traversal trace、query path visualization 或 island split/merge trace。
- 不做 run compare、benchmark explorer、live rewind。
- 不把 counter delta 表达成物理因果，只表达 exported facts 的变化。

所有权：
- 可能触及：`crates/picea-lab/web/src/types.ts`、`crates/picea-lab/web/src/App.tsx`、`crates/picea-lab/web/src/demo.ts`、`crates/picea-lab/web/src/components/workbench/Timeline.tsx`、`crates/picea-lab/web/src/components/workbench/Inspector.tsx`、`crates/picea-lab/web/src/components/workbench/Facts.tsx`、`crates/picea-lab/web/src/components/workbench/selection.ts`、`crates/picea-lab/web/src/i18n.ts`、contract scripts。
- 不应触及：Rust core algorithms、`crates/picea-lab/src/artifact.rs` schema、server live session semantics。

验收标准：
- 用户能在 Web 选择某一帧，看到当前帧 stats、上一帧 delta、events 或明确 empty state。
- `FrameRecord.report/stats/events` 的缺失不会让旧 demo 或旧 artifact 崩溃。
- `WorldEvent` 未知 variant 仍能作为 generic exported event 显示，不要求第一版 TypeScript union 覆盖全部 payload。
- 过滤入口能快速定位 broadphase/island/solver/CCD/contact 变化。
- 选中实体时，诊断面不会显示未经 exported facts 支撑的物理推断。
- live buffer 中新到的 frame 会更新 diagnostics，不要求 live rewind。

验证方式：
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy cargo test -p picea-lab`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `rtk proxy git diff --check`
- 浏览器验收：
  - 后端：`rtk proxy cargo run -p picea-lab -- serve --bind 127.0.0.1:18080`
  - Web：`cd crates/picea-lab/web && rtk proxy npm run dev -- --host 127.0.0.1 --port 5174`
  - 优先用 `browser-use:browser` 打开 `http://127.0.0.1:5174/`，在 live session 和真实 artifact replay 中检查 diagnostics tab；逐帧移动 timeline，确认 delta、events、source badge、selection 不错位。
  - 如果 `browser-use:browser` 不可用，降级 Playwright CLI；verifier 必须报告是否产生 `.playwright-cli/` 及其路径。

Subagent 执行计划：
- explorer：worker 实施前只读确认 Rust `FrameRecord` JSON shape、live frame response shape 和 Web fallback shape。分派时必须明确写入：“你是叶子 agent，不允许再启动 subagent。”
- worker：叶子 agent；优先 `gpt-5.4`。限定 Web type/UI/contract，必要时只加小型 fixture/fallback 数据。
- reviewer：叶子 agent；审查 UI 是否把 delta 误说成因果、是否对旧 artifact 缺字段稳健。
- verifier：叶子 agent；运行 Web/Rust gates 和浏览器逐帧验收。

提交策略：
- auto-commit：否。
- message hint：`Add lab-web frame diagnostics drilldown`。

风险 / 后续：
- `WorldEvent` 可能很宽；第一版可以显示 typed envelope + JSON details，避免一次性手写过大的 TypeScript union。

## M34：Evidence Summary And Repro Closeout

状态：已完成

目标：
把 live 调试、canvas camera、frame diagnostics 和 artifact evidence 收束成可复现上下文：用户能复制当前 source / scenario / run/session / frame / selection / layers / camera / grid/ruler / counters 摘要，并在 artifact replay 中看到 `perf.json` 证据状态。

为什么现在做：
调试工作台不只是看图，还要能把“我看到了什么”交给下一次修复或 review。`perf.json` summary 和 copy context 放在最后，避免重新把产品心智拉回“产物观察器”。

范围：
- 新增 Web `PerfArtifact` / `PerfCounterSummary` types。
- 新增 `fetchPerf(runId)`，从 `/api/runs/:id/artifacts/perf.json` 读取真实 artifact。
- artifact replay 完成后加载 `perf.json`；live session 下清楚显示“无持久 run perf artifact”或等价状态。
- 旧 Rust artifact replay 缺少 `perf.json` 时显示明确 missing evidence state；不崩溃、不 fallback 成 demo、不把缺失解释为 0 成本。
- 在 `Timeline` 或 `Inspector` 中新增 run-level evidence summary，展示 broadphase、island、solver row、CCD、final state hash 和 elapsed micros。
- 将 solver body slot / contact row / joint row counters 纳入可见 UI。
- 增加 `copy debug context` 或等价入口，复制当前 scenario、source、session/run id、frame index、selected entity、enabled layers、camera、grid/ruler、state hash、关键 counters。
- 如果 M33 已落地，将 diagnostics filter / selected event 纳入 context。
- 增加 browser smoke checklist 或文档段落，固定 artifact replay 与 live session 的调试验收路径。
- 更新 UI/i18n contract，锁住 perf summary 和 debug context 入口。
- Plan Gate 批准后先登记 `docs/ai/doc-catalog.yaml`；M34 closeout 再按最终 workflow 同步 `docs/ai/index.md` / `docs/ai/repo-map.md`，必要时更新 `docs/public-beta.md`。

不做：
- 不做多 run diff、artifact export bundle、benchmark explorer、live rewind、live patch。
- 不把复制的 context 当成可执行 replay 脚本；它是调试证据摘要。
- 不清理当前未跟踪 `.playwright-cli/`，除非用户另行要求。

所有权：
- 可能触及：`crates/picea-lab/web/src/api.ts`、`crates/picea-lab/web/src/types.ts`、`crates/picea-lab/web/src/App.tsx`、`crates/picea-lab/web/src/components/workbench/Toolbar.tsx`、`crates/picea-lab/web/src/components/workbench/Timeline.tsx`、`crates/picea-lab/web/src/components/workbench/Inspector.tsx`、`crates/picea-lab/web/src/i18n.ts`、contract scripts、`docs/public-beta.md`、`docs/ai/index.md`、`docs/ai/repo-map.md`、`docs/ai/doc-catalog.yaml`。
- 不应触及：Rust physics core、artifact schema、server live session semantics。

验收标准：
- Rust artifact replay 后 Web 显示 `perf.json` 的 run-level counter summary。
- Summary 能看到至少：total broadphase candidates/traversals/prunes、max tree depth、total island/active/sleeping skip、solver body slots、contact rows、joint rows、CCD candidates/hits。
- Live session 不误导用户说已有 run-level perf artifact；source provenance 保持清楚。
- 旧 Rust artifact 若缺少 `perf.json`，Web 显示缺失证据状态；不能崩溃、不能误报为 0、不能静默切到 demo。
- 旧 demo fallback 仍可加载，不因缺少 `perf.json` 崩溃。
- 用户能复制一段足够复现观察的 debug context。
- Context 清楚区分 demo replay、Rust artifact replay、Rust live session。
- Browser 验收覆盖 artifact replay 和 live session step/reset，不再只靠 build。
- 文档指向真实工作流，不暴露 `target/...` 作为主要用户心智模型。

验证方式：
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy cargo test -p picea-lab --test server_routes`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- 如更新 AI docs：`rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- `rtk proxy git diff --check`
- 浏览器验收：
  - 后端：`rtk proxy cargo run -p picea-lab -- serve --bind 127.0.0.1:18080`
  - Web：`cd crates/picea-lab/web && rtk proxy npm run dev -- --host 127.0.0.1 --port 5174`
  - 优先用 `browser-use:browser` 打开 `http://127.0.0.1:5174/`，artifact replay 和 Rust live session 各复制一次 debug context，确认内容、source badge、frame/selection/layers/camera/grid/ruler 与页面一致；artifact replay 确认 perf summary 可见，live session 确认 UI 不声称已有 `perf.json`。
  - 如果 `browser-use:browser` 不可用，降级 Playwright CLI；verifier 必须报告是否产生 `.playwright-cli/` 及其路径。

Subagent 执行计划：
- explorer：通常不需要；若要更新 docs routing，先确认当前 routing 入口和 `perf.json` route。分派时必须明确写入：“你是叶子 agent，不允许再启动 subagent。”
- worker：叶子 agent；优先 `gpt-5.4`。只做 Web evidence/debug-context/docs closeout。
- reviewer：叶子 agent；审查 context 是否误导、缺 artifact fallback、live session perf 文案和 docs 是否漂移。
- verifier：叶子 agent；运行 Rust/Web/docs gates 和浏览器验收。

提交策略：
- auto-commit：否。
- message hint：`Add lab-web debug evidence context`。

风险 / 后续：
- `elapsed_micros` 是本机运行时间上下文，不应被 UI 表达成 correctness oracle；文案必须强调 counters 是 deterministic evidence，timing 只是参考。
- 复制 context 容易被误解为完整 artifact export。本轮只做 human-readable context；真正 bundle/export、run compare 和 benchmark explorer 留给后续。

## 后续候选 M35：Process Trace Carrier

状态：候选，不进入本轮默认执行

触发条件：
M31-M34 完成后，若 live controls、canvas、frame diagnostics 和 existing evidence 仍无法解释一个明确调试问题，再进入本候选 milestone。

默认规则：
- 默认优先 lab-owned additive artifact/live carrier（例如 `FrameProcessTrace`），因为目标是 lab 可调试性。
- 只有当 live session、`DebugSnapshot` 消费者和多个非 lab 读模型都需要同一事实时，才提升到 core `DebugSnapshot` / `DebugStats` read model。
- 每次最多落一个 trace family。默认候选顺序：`broadphase` traversal/prune -> `island` lifecycle；`query debug` 与 solver-row trace 必须拆到后续 slice，除非先修订计划。
- 新 trace fields 必须 additive / serde default，旧 artifact 兼容；Web 只消费 trace facts，不重新计算 physics。

不做：
- 不修改 contact solver math、CCD TOI 算法、GJK/EPA、narrowphase、broadphase heuristics。
- 不暴露 internal proxy ids、leaf ids、store slots 或 cache internals。
- 不做 query mutator、live patch、editor、run compare 或 always-on hot-path tracing。

## 进度记录

### 2026-05-03 - Plan Draft

- 状态：计划中
- Commit：none
- 验证：`npm run test:ui-contract`、`npm run test:i18n`、`cargo test -p picea-lab --test artifact_run`、`cargo test -p picea-lab --test server_routes` 作为调研基线通过。
- 风险 / 后续：reviewer 首轮发现 M33 过大、M31 缺旧 `perf.json` 缺失兼容契约、浏览器验收不够具体；本版已收窄 M33、补 M31 兼容、写明浏览器验收命令和降级路径。用户确认前不执行 M31。

### 2026-05-03 - Live-First Requirement Amendment

- 状态：计划中
- Commit：none
- 变更：根据新增要求，把计划从 artifact evidence first 重排为 live-first：M31 live 调试步进，M32 zoomable full canvas + grid/ruler toggle，M33 frame diagnostics，M34 evidence summary + repro context，process trace carrier 降为后续候选 M35。
- 风险 / 后续：这是 reviewer 首轮之后的实质性计划变更；最终 Plan Gate 前需要二次 reviewer 复审 live-first / canvas 范围。

### 2026-05-03 - M31-M34 Implementation Closeout

- 状态：已完成
- Commit：none
- 变更：完成 live-first 调试入口、request-driven step/play/pause/reset、canvas pan/zoom/fit/reset、grid/ruler layer toggle、frame diagnostics、artifact `perf.json` evidence、debug context 预览和 clipboard fallback。
- 验证：`rtk proxy npm run test:ui-contract`、`rtk proxy npm run test:i18n`、`rtk proxy npm run build`、`rtk proxy cargo test -p picea-lab`、`rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`、`rtk proxy git diff --check` 均通过。
- 浏览器验收：`browser-use:browser` 打开 `http://127.0.0.1:5176/`，通过 fresh backend `127.0.0.1:18081` 创建 `Rust live session`，step 后缓存后端帧，play/pause 后缓存 75 帧并保持 `Rust 实时会话 / 已暂停`；canvas zoom/pan/fit/reset、grid/ruler toggle、canvas hit selection、diagnostics tab、live debug context 预览、artifact replay `perf.json` summary 均通过；页面 console error 为空。
- 风险 / 后续：in-app browser 拦截页面 clipboard write，因此 debug context 按“先显示预览、再尽力复制”的策略处理；如后续需要过程级 traversal/island trace，进入候选 M35。
