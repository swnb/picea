# Picea Lab-Web 工业级调试体验里程碑计划

状态：执行中
计划文档：docs/plans/2026-05-04-picea-lab-web-industrial-ux-milestones.md
最后更新：2026-05-05
提交策略：不提交
执行策略：按 milestone gate 顺序推进；每个 milestone 先补行为锁或失败测试，再写最小实现，通过对应验证门后，才决定是否进入下一层。不得一次性连续执行 M35-M40。

## 目标

把 `picea-lab-web` 从“能看见 live / artifact / canvas / process facts”继续推进到更接近工业物理引擎调试器的工作台。完成后，用户应该能在浏览器里更自然地回答：

- 复杂多物体堆叠为什么稳定、什么时候失稳、哪些接触 / island / solver rows 在起作用。
- 多个物体、接触点、CCD、用户扰动的轨迹如何随时间演化。
- 用户如何在受控语义下对物体施加扰动、调整参数，并清楚知道这是 reset-time override、paused preview/commit、absolute velocity perturbation，还是未来才可能扩展的 force / impulse API。
- “网格模拟”在当前 Picea 里先落成刚体 lattice / joint-grid 调试代理，而不是假装已经有 true soft-body / FEM solver。
- 如果未来要做真实 soft-body / particle / deformable mesh，需要哪些 core 数据模型、artifact schema、UI 和验证前置条件。

## 约束

- 工作区：`/Users/asyncrustacean/projects/picea` 当前主工作区。
- 当前工作区已有未提交 `picea-lab-web` / docs 改动，本计划基于这些事实，不覆盖、不 revert、不格式化用户已有改动。
- 计划阶段只新增本计划文档并登记 `docs/ai/doc-catalog.yaml`；不改实现代码。
- 所有验证命令使用 `rtk proxy`。
- `picea-lab-web` 不运行 physics，不重新计算 broadphase、island、collision、solver、decomposition 或 soft-body 结果。
- Web 可以对已导出的 frames 做 UI 层聚合、筛选、轨迹抽样和差分展示，但不能把这些展示包装成 authoritative physics fact。
- live world 的权威状态仍在 Rust server；任何运行中修改必须通过明确 session contract，不能绕过 M25-B 对 running-world patch 的红线。
- 第一版手动扰动优先使用 paused-only / transaction-like 语义和现有 `BodyPatch` 能表达的速度/位姿边界；真正连续外力、扭矩 accumulator 或 force integration 是后续 core API 议题。
- 第一版“网格模拟”优先做 rigid-body lattice / joint-grid authoring 和调试视图；true soft-body、particle、FEM、cloth tearing、runtime mesh topology mutation 不进入本计划的实现范围。
- 浏览器验收优先使用 `browser-use:browser`。默认 `just picea-lab-web` 使用 backend `127.0.0.1:8080`、Web `127.0.0.1:5173`；如端口占用，可用 `PICEA_LAB_BIND` / `PICEA_LAB_WEB_PORT` 覆盖，并在验证记录里写明实际端口。
- 若当前 Codex in-app browser backend 仍 discovery 失败，允许降级到 Playwright CLI，但 verifier 必须报告失败原因；当前工作区已有未跟踪 `.playwright-cli/` 时，必须区分新旧产物。

## 待确认问题

### 必须确认

- 无。当前草案用保守解释推进：先做 lab 调试体验和 rigid-body lattice 代理，不把 true soft-body 当成本轮 UI 工作。

### 可带假设推进

- “多物体堆叠稳定”默认解释为可观察、可对比、可定位原因，不承诺本计划修改 solver 行为；早期验证：复杂堆叠场景能展示 sleep/wake、contact impulse、island、drift/jitter summary。
- “轨迹显示”默认包括 selected / all dynamic bodies、contact / CCD / perturbation trails，带 history length 和降噪；早期验证：在 `stack_4`、falling box、CCD 场景中可切换轨迹模式。
- “用户手动施力”默认从 paused-only velocity perturbation preview/commit V1 开始；早期验证：选中动态 body 后拖拽生成扰动向量，经后端受控 step 后产生可追溯 frame。真正 force / impulse API 不在本轮偷加。
- “网格模拟”默认先做 joint-grid / lattice proxy；早期验证：用户能看到 nodes / edges / joints / strain-like display，但 UI 明确标注不是 soft-body solver。
- true soft-body / particle / deformable mesh 只做 RFC / boundary，不在本轮实现；影响：减少第一轮 blast radius，避免把底层物理模型和 UI 改动混成不可验收大 diff。

## 规划依据

- explorer：已运行。结论：当前 UI 已有 live session、canvas camera、grid/ruler、trace layer、process facts、diagnostics 和 evidence；主要缺口是 forces/torques 仍为未测量占位、轨迹偏单体历史路径、复杂堆叠缺少稳定性工作台、mesh/soft-body/particle/cloth 没有 core 入口。
- browser-use：2026-05-05 已成功使用 `browser-use:browser` 连接 Codex in-app browser 并分析 `http://127.0.0.1:5174/`。首次 IAB discovery 失败，重置 Node REPL 后 `setupAtlasRuntime({ backend: "iab" })` 成功，后续页面检查均通过 browser-use 完成；计划仍要求 verifier 记录 browser-use 成功或失败原因。
- 本地 UI 观察：通过 `just picea-lab-web` 以临时端口 `PICEA_LAB_BIND=127.0.0.1:18082` / `PICEA_LAB_WEB_PORT=5178` 启动，Playwright CLI 降级观察到当前页面有 `Rust 实时会话`、step/play/pause/reset、camera lock、grid/ruler、layer menu、`轨迹` layer、diagnostics/evidence、run setup 中的 frame count 和 gravity override；Inspector 中 `力` / `扭矩` 仍显示 `未测量`。
- browser-use 基线观察（2026-05-05，`http://127.0.0.1:5174/`）：
  - 页面标题为 `Picea Lab Workbench`，console warn/error 为空。
  - 顶部运行模式只有 `Rust 产物回放` 和 `Rust 实时会话`；没有手动编辑、扰动、force/torque 或 lattice/soft-body 模式。
  - 场景列表包含 `落箱接触`、`四箱堆叠`、`世界锚点关节`、`Sparse broadphase`、`SAT polygon manifold`、`复合体来源`、`Concave decomposition fixture` 和多个 CCD 场景；没有 lattice/grid、particle、soft-body、cloth、mesh 场景。
  - 图层菜单包含 `网格`、`标尺`、`形状`、`AABB`、`接触点`、`速度`、`轨迹`、`宽阶段树`、`岛`、`来源`；`轨迹` 当前是 layer toggle，不是 selected/all/island/contact/CCD 的可配置 trajectory workbench。
  - `运行设置` 只有 frame count、run mode、gravity override、gravity y；没有物体级力、扭矩、velocity perturbation、paused transaction 或 commit/provenance controls。
  - `落箱接触` Rust 产物回放可跑到 120 帧，末帧有 2 个接触点；接触 Inspector 能显示 depth、normal、feature id、reduction、warm-start normal/tangent、solver normal/tangent impulse。
  - `四箱堆叠` Rust 产物回放末帧有 8 个接触点；Rust 实时会话可创建 `session-*` 并缓存 live frames，实测帧 98 有 3 个接触点、broadphase tree、island lifecycle 和 CCD counters。当前页面能显示“发生了什么”，但不能解释“为什么稳定 / 为什么失稳”。
- 工业参考：
  - Box2D samples 把 pan/zoom camera、dynamic body mouse dragging、sample tree、parameter tuning、debug drawing options、pause/single-step 和 performance data 作为 testbed 能力。
  - MuJoCo visualizer 明确区分 free/tracking camera、selection、interactive perturbation、force/torque application 和 trajectory rendering。
  - PhysX Visual Debugger 提供 object frames、contact points、bounds、velocities、center of mass、force fields、object drag、physical properties、transparency、camera、stats/profiling。
  - PhysX debug visualization 把 debug facts 作为 points/lines/triangles/text primitives，并提醒 debug visualization 有性能成本、需要 culling 和 release 关闭策略。
  - PhysX soft body 文档显示 true soft-body 需要 simulation/collision/render meshes、cooking/preprocess、solver/material tuning，且当前 GPU soft bodies 是单独能力面；这支持 Picea 先把 soft-body 当未来 core surface，而不是 UI 小修。
- 关键仓库证据：
  - `crates/picea-lab/web/src/App.tsx`：已有 source/run mode/live session 状态和 backend step 缓冲。
  - `crates/picea-lab/web/src/components/workbench/WorldCanvas.tsx`：已有 shapes/AABB/contacts/velocities/trace/broadphase tree/islands/provenance 绘制和 camera interaction。
  - `crates/picea-lab/web/src/components/workbench/Timeline.tsx`：run setup 当前只有 frame count、run mode、gravity override / gravity y。
  - `crates/picea-lab/web/src/components/workbench/Inspector.tsx`：forces/torques 仍是 pending measurement。
  - `crates/picea-lab/web/src/types.ts`：`FrameRecord` 已 typed 表达 `report`、`stats`、`events`、`DebugSnapshot`、`DebugIsland`、`CompoundProvenance`。
  - `crates/picea-lab/src/scenario.rs`：已有 `stack_4`、broadphase、compound/concave、CCD 场景，但没有 mesh/soft-body/particle/cloth 类型。
  - `docs/design/picea-lab-live-session-semantics.md`：M25-B 明确 live session 当前拒绝 running-world patch，未来 paused patch 需要 transaction / handle invalidation / query sync / provenance。
  - `crates/picea/src/body.rs` / `crates/picea/src/world/api.rs`：`BodyPatch` 可表达 pose、linear/angular velocity、sleep 等 patch，但没有通用 force accumulator public API。
- 主要未知：
  - 手动施力应长期建模为 force accumulator、one-step impulse、drag constraint，还是 absolute velocity patch；本计划先用 absolute velocity perturbation V1 验证 UX。
  - stack stability summary 中哪些指标应成为 artifact schema，哪些只是 Web 对 frames 的可视化聚合；M35 先避免新增 public physics contract。
  - rigid-body lattice 是否足以满足用户短期“网格模拟”预期；true soft-body 需要另过 core architecture gate。
  - browser-use 基线已经证明当前 UI 缺的是解释/控制层，而不是基础可视化完全不可用；因此 M35-M38 仍保持增量推进，不重写 workbench。

## 计划验收

- 状态：reviewer 已通过，用户已确认执行
- reviewer：已运行多轮只读审查；首轮和复审阻塞项已修订，2026-05-05 browser-use 基线更新复核 verdict 为 `pass-with-minor-edits`，无 blocking finding，minor edits 已处理。
- 审查结论：首轮阻塞项包括执行策略过宽、M37 preview/commit 混杂、velocity patch / impulse 语义混杂、M37 缺 query gate、M38 缺 server route gate；复审指出 M37 还必须绑定 `WorldRevision` / session epoch 并在 commit mismatch 时拒绝。本版本已按这些项修订。2026-05-05 复核确认 browser-use 基线写回没有破坏 M37-A/B 边界，也没有把 UI-derived display 误写成 authoritative physics fact。
- 用户确认：已确认（2026-05-05，用户要求使用 subagent-current-workspace 依次完成剩余里程碑，并完成 browser-use 验收）

## 里程碑

## M35：Stack Stability Observatory

状态：已完成（2026-05-05）

目标：
把复杂堆叠从“有一个 `stack_4` 案例和若干 counters”推进到“能观察稳定性、定位失稳原因、对比扰动前后”的调试视图。完成后，用户能看见多物体堆叠中的接触、island、solver impulse、sleep/wake、drift/jitter 和稳定窗口。

为什么现在做：
这是用户明确指出的第一类复杂场景。2026-05-05 browser-use 基线证明 `四箱堆叠` 已能在 artifact/live 中显示接触点、宽阶段树、island lifecycle 和 CCD counters，但页面只能说明“发生了什么”，还不能解释“为什么稳定 / 为什么倒了”。先把这些 facts 组织成稳定性工作台，比直接调 solver 或做 visual editor 风险小、验收清楚。

范围：
- 增加或整理复杂堆叠场景入口，例如 higher stack、side-load stack、mixed material stack、narrow support stack。
- 在 Web 中增加 stack stability panel，展示每帧或时间窗的 body drift、angular drift、sleep readiness、awake/sleep island、contact count、normal/tangent impulse、solver row count。
- 在 timeline 中标记 stability window、wake/sleep transition、contact count spike、large angular drift、solver row spike。
- Canvas 叠加可选的 contact impulse heat、island tint、sleep/awake badges、center-of-mass trail。
- 为后续 baseline/run 对比保留 summary 形状；首版只做单次 run 的稳定性 summary、markers 和 overlay。
- Web 聚合只消费 exported frames / stats / events；如发现 carrier 不足，新增 additive lab artifact field 前必须补 serde default tests。
- 更新 UI/i18n contract，锁住 stability panel、timeline markers 和 layer labels。

不做：
- 不修改 contact solver、sleep algorithm、island solver ordering 或 friction math。
- 不设置新的性能 hard threshold。
- 不做跨 run baseline diff view；该能力在 M35 之后另行过门。
- 不做 live patch 或手动施力；扰动进入 M37。
- 不做 soft-body / lattice；进入 M38。

所有权：
- 可能触及：`crates/picea-lab/src/scenario.rs`、`crates/picea-lab/src/artifact.rs`、`crates/picea-lab/tests/artifact_run.rs`、`crates/picea-lab/web/src/*`、`crates/picea-lab/web/scripts/*`、`crates/picea-lab/web/src/i18n.ts`。
- 可能少量触及：`crates/picea/src/debug.rs`，仅限 additive / serde-default debug carrier。
- 不应触及：`crates/picea/src/solver/contact.rs`、`crates/picea/src/pipeline/island.rs`、`crates/picea/src/pipeline/sleep.rs` 的行为实现。

验收标准：
- 至少一个复杂堆叠场景可以从 scenario selector 进入。
- 用户能在页面里看到 stack stability summary、timeline markers、contact/island/impulse 关联信息。
- `四箱堆叠` 的 browser-use 基线缺口被补齐：例如本次观测中的 artifact 末帧 8 个接触点、live 帧 98 附近 3 个接触点这类事实，能被组织成 drift/jitter/contact/island 的稳定性解释，而不只是原始 counters。
- 选中一个 body / contact 后，Inspector 能解释它在堆叠稳定性中的贡献。
- Artifact replay 和 Rust live session 均可展示已有 facts；缺失 facts 时显示 missing evidence，不误报为 0。
- 旧 `stack_4` 场景和 demo fallback 不被破坏。

验证方式：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep`
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy cargo test -p picea-lab`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `rtk proxy git diff --check`
- 浏览器验收：`rtk proxy just picea-lab-web`，优先 `browser-use:browser` 打开默认 `http://127.0.0.1:5173/` 或实际覆盖端口（如 `http://127.0.0.1:5174/`），运行 `四箱堆叠` 的 artifact replay 与 Rust live session，检查 stability panel、timeline markers、overlay、selection-to-inspector、console error。

Subagent 执行计划：
- explorer：叶子 agent；确认现有 stack / sleep / contact / solver facts 足够哪些 stability summary，哪些指标只能作为 Web-derived display。
- worker：叶子 agent；优先 `gpt-5.4`。只做 scenario/artifact additive facts 和 Web stability view，不改 solver。
- reviewer：叶子 agent；审查是否把 Web-derived summary 误写成 core fact、是否触碰 solver、UI 是否可验收。
- verifier：叶子 agent；运行上述 Rust/Web/browser gates，报告 `.playwright-cli/` 产物。

提交策略：
- auto-commit：否。
- message hint：`Add lab-web stack stability observatory`

风险 / 后续：
- 如果 stability 指标暴露出真实 solver bug，应停止并另开 core behavior milestone，不在 UI 里掩盖。

## M36：Multi-Body Trajectory And Event Trails

状态：已完成（2026-05-05）

目标：
把当前偏窄的 `trace` layer 升级成可控的多物体轨迹系统。完成后，用户能按 selected / all / island / contact / CCD 维度查看历史路径，并控制 history length、降噪、颜色和透明度。

为什么现在做：
轨迹显示是用户明确提到的缺口，也是手动扰动和复杂堆叠观察的共同基础。browser-use 基线看到当前 `轨迹` 只是画布图层里的一个 checkbox，没有 trajectory mode、history length、selected/all/island/contact/CCD 控制；它主要消费已有 frame history，风险低于先做 live patch。

范围：
- 将 trajectory controls 从单一 layer 扩展为模式：selected body、all dynamic bodies、selected island、contacts、CCD sweep。
- 增加 history length / sampling stride / fade / color-by-body-or-island 控制，避免复杂场景视觉拥挤。
- Canvas 轨迹与 camera、grid/ruler、selection 共用同一 transform。
- Timeline 支持按轨迹事件跳转：body moved most、contact appeared/disappeared、CCD clamp、sleep/wake。
- Inspector 增加 selected entity 的 trajectory summary：distance traveled、max speed、sleep/awake spans、contact count over trail window。
- 轨迹数据优先从 `frames` / `FrameRecord` 派生；只有需要复现 artifact 时才考虑 additive artifact metadata。
- 更新 UI/i18n contract。

不做：
- 不新增 physics trajectory integrator。
- 不做 offline video export。
- 不做 force/perturbation trail；M37-B 在有真实 provenance 后再接入。
- 不做 arbitrary query path visualization；可作为后续 trace family。

所有权：
- 可能触及：`WorldCanvas.tsx`、`Timeline.tsx`、`Inspector.tsx`、`components/workbench/types.ts`、`crates/picea-lab/web/src/types.ts`、`i18n.ts`、contract scripts。
- 不应触及：Rust core、server route、artifact schema，除非 reviewer 证明需要 additive metadata。

验收标准：
- `falling_box_contact`、`stack_4`、至少一个 CCD 场景中能看到多物体轨迹。
- 原本只有 `轨迹` layer toggle 的 UI 升级为可配置 trajectory workbench，至少能在 selected/all dynamic bodies/contact/CCD 模式之间切换。
- 轨迹模式和 history length 能在 UI 中切换，复杂场景不会遮挡主体。
- 选中 body/contact 后，轨迹与 Inspector summary 对应。
- 轨迹与 camera pan/zoom/lock、grid/ruler、layer menu 不冲突。

验证方式：
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `rtk proxy cargo test -p picea-lab --test artifact_run`（如触及 artifact/demo schema）
- `rtk proxy git diff --check`
- 浏览器验收：运行 live session 与 artifact replay，切换 trajectory modes、history length、selected body/island/contact/CCD，确认它不再只是 `轨迹` checkbox；检查 canvas 非空、无明显 overlap、console error 为空。

Subagent 执行计划：
- explorer：叶子 agent；确认当前 `WorldCanvas` trace 实现和 frame buffer 形状，列出最小改动点。
- worker：叶子 agent；优先 `gpt-5.4`。只实现 Web trajectory controls / rendering / summary。
- reviewer：叶子 agent；审查性能、视觉拥挤、camera transform、是否误算 physics。
- verifier：叶子 agent；运行 Web gates 和浏览器验收。

提交策略：
- auto-commit：否。
- message hint：`Add lab-web multi-body trajectory trails`

风险 / 后续：
- 大量 body 的历史轨迹可能带来渲染压力；第一版必须有采样和 history cap。

## M37-A：Paused Perturbation Contract Preview Gate

状态：已完成（2026-05-05）

目标：
先把手动扰动的 session contract、失败回滚、handle/query 同步和 provenance 边界锁住。完成后，系统能在 `created/paused` session 中计算一个不修改 world 的扰动 preview，并在 `running/completed`、stale handle、`WorldRevision` / session epoch mismatch 场景下稳定拒绝。

为什么现在做：
工业级调试器常见 body drag、force/torque perturbation、参数调节；但 browser-use 基线显示当前 `运行设置` 只有 frame count、run mode、gravity override、gravity y，Inspector 中 `力` / `扭矩` 仍是 `未测量`，页面也没有物体级扰动入口。Picea 当前没有通用 force accumulator public API，live session 也拒绝 running-world patch。M37-A 先做 preview / contract，不进入 commit / UI controls，避免把 UI 需求偷渡成新的 runtime contract。

范围：
- 梳理并固定 V1 语义：`created/paused` session 只允许受限 preview transaction；`running/completed` 显式拒绝。
- 明确命名为 velocity perturbation preview，而不是 physics impulse。现有 `BodyPatch` 是 absolute linear/angular velocity replacement；若 UI 输入是 delta，server 必须从当前权威 state 计算 target velocity，并在 preview 里同时记录 delta 与 target。
- Preview 结果必须绑定 `WorldRevision` / session epoch，不能只依赖 frame index。`WorldRevision` / epoch 需要在 reset、accepted commit、任何会让 handles/query/debug snapshot 失效的 world mutation 后递增。
- Preview 结果包含 action id、session id、`WorldRevision` / epoch、body handle、frame index、before velocity、requested delta、computed target velocity、wake intent、rejection reason。
- Preview 不修改 live world、frame buffer、latest frame、selection、query pipeline 或 artifact。
- 行为锁覆盖 stale session、stale preview、revision/epoch mismatch、stale handle、static/kinematic body、sleeping body wake intent、invalid magnitude、failed validation。
- 文档更新 `docs/design/picea-lab-live-session-semantics.md`，把 preview-first gate、query sync 风险和 future commit gate 写清楚。
- 如发现需要真正 force accumulator、drag constraint 或 additive impulse API，停止并写入后续 core API milestone，不在 M37-A 临时扩 solver。

不做：
- 不提交扰动，不修改 world。
- 不做 canvas drag controls 或 Inspector commit controls。
- 不做 arbitrary body/collider/joint editor。
- 不做 continuous force integration、torque accumulator、drag constraint solver、mouse joint。
- 不做 running-world arbitrary patch。

所有权：
- 可能触及：`docs/design/picea-lab-live-session-semantics.md`、`crates/picea-lab/src/server.rs`、`crates/picea-lab/tests/server_routes.rs`、`crates/picea-lab/src/scenario.rs`、`crates/picea-lab/web/src/api.ts`、`crates/picea-lab/web/src/types.ts`。
- 可能触及：`crates/picea/src/body.rs` / `world/api.rs` tests only if existing absolute velocity patch semantics need characterization。
- 不应触及：solver row math、contact/narrowphase/CCD algorithms、query internals 行为实现。

验收标准：
- `created/paused` session 能返回 velocity perturbation preview，且 preview 清楚区分 requested delta 与 computed absolute target velocity，并携带 `WorldRevision` / session epoch。
- `running/completed` session、stale preview、revision/epoch mismatch、stale handle、invalid target、unsupported body type 均被明确拒绝。
- Preview/reject 都不改变 live world、frame buffer、latest frame、selection 或 query state。
- Browser 中仍不出现可提交的 drag/force controls；M37-A 只允许暴露 preview/contract evidence，不让用户误以为已经可以施加连续力或提交编辑。
- 文档明确 M37-B 才允许 commit 和 UI drag controls。

验证方式：
- `rtk proxy cargo test -p picea-lab --test server_routes`
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy cargo test -p picea --test core_model_world`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo test -p picea --test query_debug_contract`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：叶子 agent；确认 M25-B server contract、`BodyPatch` absolute velocity 表达能力、stale handle/query/sleep 风险。
- worker：叶子 agent；架构风险较高，优先 `gpt-5.5` 或明确 `gpt-5.4` 加强 reviewer。只做 preview / contract / tests / docs。
- reviewer：叶子 agent；重点审查 live patch 红线、preview 是否无副作用、absolute velocity 语义、handle invalidation、query sync。
- verifier：叶子 agent；运行 Rust/docs gates。

提交策略：
- auto-commit：否。
- message hint：`Define paused perturbation preview contract`

风险 / 后续：
- 如果 preview 语义仍不足以表达用户想要的“施力”，必须先开 force accumulator / impulse API 设计 milestone，再继续 M37-B。

## M37-B：Paused Perturbation Commit And Controls V1

状态：已完成（2026-05-05）

依赖：
M37-A 已通过全部验证门，且 reviewer 确认 preview contract 没有违反 M25-B 红线。

目标：
在 M37-A 的 contract 稳定后，为用户提供第一版手动扰动提交和 UI controls。完成后，用户能选中动态 body，拖拽生成 velocity perturbation preview，在 paused session 中提交受限 absolute velocity patch，并看到每次动作的 provenance 和后续轨迹。

为什么现在做：
只有 contract / preview 先稳定，commit 和 UI drag 才有安全边界。M37-B 负责把 browser-use 基线中的“无物体级扰动入口 / force torque 未测量”推进到 paused-only 的可追溯 perturbation V1；它仍然不是 continuous force / torque，也不是 running-world editor。

范围：
- 在 `created/paused` session 中接受 M37-A preview 生成的受限 commit；commit request 必须带回 preview 的 action id 与 `WorldRevision` / session epoch，server 仅在当前 revision/epoch 完全匹配时计算最终 absolute velocity patch。
- UI 上以 canvas drag / inspector controls 表达扰动向量，显示 magnitude、direction、target body、requested delta、computed target velocity、pending/accepted/rejected state。
- 每个 accepted perturbation 进入 session provenance：action id、session id、accepted `WorldRevision` / epoch、body handle、before/after velocity、frame index、query sync status、reason。
- rejected transaction，尤其是 stale preview / revision mismatch，不改变 live world、frame buffer、selection、latest frame 或 query state。
- commit 后必须处理 query pipeline / debug snapshot 同步，避免 stale query cache。
- 与 M36 轨迹联动：有真实 provenance 后再显示 perturbation vector / trail，后续帧能追踪效果。
- 对 gravity、frame_count、solver/debug display 这类 reset-time / UI-only 参数做清晰分组，避免用户误以为它们是 running-world patch。

不做：
- 不做 arbitrary body/collider/joint editor。
- 不做 continuous force integration、torque accumulator、drag constraint solver、mouse joint。
- 不做 running-world arbitrary patch。
- 不做 collaborative editing、undo/redo timeline 或 rewind。

所有权：
- 可能触及：`crates/picea-lab/src/server.rs`、`crates/picea-lab/tests/server_routes.rs`、`crates/picea-lab/src/scenario.rs`、`crates/picea-lab/web/src/App.tsx`、`api.ts`、`types.ts`、`WorldCanvas.tsx`、`Inspector.tsx`、`Timeline.tsx`、`i18n.ts`、contract scripts。
- 可能触及：`docs/design/picea-lab-live-session-semantics.md`，仅限记录 M37-B 已实现的 commit gate。
- 不应触及：solver row math、contact/narrowphase/CCD algorithms、query internals 行为实现。

验收标准：
- 选中动态 body 后，用户能生成扰动 preview，并在 paused session 中提交受限 absolute velocity patch。
- accepted/rejected provenance 清晰可见；stale preview / revision mismatch 会被明确拒绝，且 rejected transaction 不改变 live world、frame buffer、selection、latest frame 或 query state。
- `running` session patch 被明确拒绝，错误文案解释原因。
- 后续 frame 的轨迹/速度变化可以从 UI 追溯到 perturbation action。
- reset-time gravity override 与 paused perturbation 在 UI 中分组清晰。
- 原本 `运行设置` 只有 frame count / run mode / gravity override 的状态被保留为 reset-time controls；新的 paused perturbation controls 必须在视觉和文案上单独分组。

验证方式：
- `rtk proxy cargo test -p picea-lab --test server_routes`
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy cargo test -p picea --test core_model_world`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo test -p picea --test query_debug_contract`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `rtk proxy git diff --check`
- 浏览器验收：live session 创建、pause、选 body、drag perturbation、preview、submit、step、查看 provenance 和 trajectory；running 状态下提交必须清晰拒绝。

Subagent 执行计划：
- explorer：叶子 agent；复核 M37-A contract 是否仍匹配当前 server/Web 类型。
- worker：叶子 agent；架构风险较高，优先 `gpt-5.5` 或明确 `gpt-5.4` 加强 reviewer。按 server behavior lock -> Web preview -> commit UI -> provenance 顺序推进。
- reviewer：叶子 agent；重点审查 live patch 红线、transaction 原子性、handle invalidation、query sync、UI 是否误导为 continuous force。
- verifier：叶子 agent；运行 Rust/Web/browser gates。

提交策略：
- auto-commit：否。
- message hint：`Add paused perturbation controls`

风险 / 后续：
- 如果用户真正想要 continuous force / torque，而不是 absolute velocity perturbation，需要先扩 core API；M37-B 只验证安全扰动 UX。

## M38：Rigid-Body Lattice / Grid Simulation Proxy

状态：已完成（2026-05-05）

目标：
把“网格模拟”先落成 Picea 当前能力能支持的 rigid-body lattice / joint-grid 调试代理。完成后，用户能运行由刚体节点和 distance/world-anchor joints 组成的网格场景，看到 nodes、edges、joint stretch、contact、island 和 trajectory，而 UI 明确说明这不是 true soft-body solver。

为什么现在做：
用户提到网格模拟，但 browser-use 基线中的 scenario picker 没有 lattice/grid、particle、soft-body、cloth、mesh 场景，仓库也没有对应 core 入口。参考 PhysX soft-body 的 simulation mesh / collision mesh / cooking / material tuning 成本，Picea 不应把 true soft-body 混进 UI milestone。rigid-body lattice 能先验证 authoring、可视化和调试 UX。

范围：
- 开工前再确认用户当前期待仍是 rigid-body lattice proxy；如果用户明确要 true deformable physics，则停止 M38，先执行 M39。
- 增加 `lattice_grid` 或类似 deterministic lab 场景：dynamic node bodies、distance joints、static anchors、可选碰撞体。
- Web scenario selector 将它归类为 lattice / grid proxy。
- Canvas 增加 lattice overlay：nodes、edges、anchors、selected joint stretch、broken/overstressed placeholder state（如无真实 break，不显示为物理事实）。
- Inspector 展示 joint length/rest distance、node velocity、island membership、solver rows。
- Timeline 展示 grid deformation display：bounding box、max edge stretch ratio、active/sleep islands、contact count。
- 文案明确：这是 rigid-body joint lattice proxy，不是 FEM/cloth/soft-body。
- 如现有 joint facts 不足，可加 additive debug carrier；保持 serde default。

不做：
- 不实现 soft-body、particle system、cloth tearing、FEM、XPBD/PBD solver。
- 不改变 distance joint solver 行为。
- 不做 runtime topology edit。
- 不做 mesh cooking、surface/collision/simulation mesh 分层。

所有权：
- 可能触及：`crates/picea-lab/src/scenario.rs`、`crates/picea-lab/src/artifact.rs`、`crates/picea-lab/tests/artifact_run.rs`、`crates/picea-lab/web/src/*`、`i18n.ts`、contract scripts。
- 可能少量触及：`crates/picea/src/debug.rs` 或 `crates/picea/src/pipeline/joints.rs` 的 additive debug facts。
- 不应触及：joint solver behavior、contact solver、CCD、broadphase heuristics。

验收标准：
- `lattice_grid` 场景可通过 artifact replay 和 live session 运行。
- Scenario picker 中出现明确的 lattice/grid proxy 场景，且不与 soft-body/cloth/FEM 混名。
- 用户能在 UI 中看见 lattice nodes/edges/anchors、selected joint facts、trajectory 和 island/solver facts。
- UI 清楚区分 rigid-body lattice proxy 与 true soft-body。
- 旧 joint / stack / CCD / compound 场景不回归。

验证方式：
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy cargo test -p picea-lab --test server_routes`
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy cargo test -p picea --test core_model_world`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `rtk proxy git diff --check`
- 浏览器验收：用 `browser-use:browser` 打开实际 dev URL，确认当前基线中缺失的 lattice/grid 场景已出现；打开 lattice/grid 场景，切换 lattice overlay、trajectory、island、joint selection，检查文案边界和 console error。

Subagent 执行计划：
- explorer：叶子 agent；确认 joint debug facts 和 scenario authoring helper 是否足够表达 lattice。
- worker：叶子 agent；优先 `gpt-5.4`。只做 lab scenario / additive facts / Web lattice view，不改 solver。
- reviewer：叶子 agent；审查是否暗示 true soft-body、joint facts 是否准确、schema 是否兼容。
- verifier：叶子 agent；运行 Rust/Web/browser gates。

提交策略：
- auto-commit：否。
- message hint：`Add rigid-body lattice lab scenario`

风险 / 后续：
- Lattice proxy 可能暴露 joint solver 稳定性问题；如果出现，另开 solver/joint milestone。

## M39：Soft-Body / Particle / Deformable Mesh RFC Gate

状态：已完成（2026-05-05）

目标：
为真实 mesh / soft-body / particle / cloth 类能力写清楚工程边界和路线。完成后，仓库有一个设计文档回答：Picea 是否要支持 deformable bodies、选择 PBD/XPBD/FEM/rigid-lattice 哪条路线、需要哪些 core types、artifact facts、viewer controls 和性能门。

为什么现在做：
工业引擎里的 soft-body 不是 UI skin。PhysX soft-body 需要 simulation/collision/render mesh、cooking、material tuning、solver iterations 和性能策略。Picea 当前是 2D rigid-body engine，必须先立边界，避免 M35-M38 的 UI 工作被误解成已经支持 true soft-body。

范围：
- 新增设计文档，例如 `docs/design/deformable-body-roadmap.md`。
- 梳理候选路线：继续 rigid-body lattice、PBD/XPBD particle-grid、FEM/finite elements、external plugin/interop。
- 明确 V1 非目标：tearing/cutting、3D tet mesh、GPU soft-body、runtime topology mutation。
- 定义最小 public surface 候选：authoring schema、body/particle handles、debug snapshot、artifact schema、lab viewer controls。
- 定义 future query / selection boundary：`QueryPipeline` 是否命中 whole deformable、particle、element / constraint 或 collision proxy，以及 revision-aware handle 映射。
- 定义验证矩阵：determinism、energy/stability、collision coupling、performance counters、browser visualization、benchmark scenes。
- 更新 docs routing，让未来 soft-body 讨论不会误落到 `picea-lab-web` UI 文件里。

不做：
- 不实现 core solver。
- 不实现 Web mesh editor。
- 不引入外部 heavy dependency。
- 不承诺路线一定进入 public beta。

所有权：
- 可能触及：`docs/design/deformable-body-roadmap.md`、`docs/design/README.md`、`docs/ai/doc-catalog.yaml`、`docs/ai/index.md`、`docs/ai/repo-map.md`。
- 不应触及：`crates/picea/src/*`、`crates/picea-lab/src/*`、`crates/picea-lab/web/src/*`。

验收标准：
- 文档清楚区分 rigid-body lattice proxy、particle/PBD、true FEM soft-body。
- 文档列出第一条可执行 core milestone 的输入、输出、验证和非目标。
- 文档说明 lab-web 需要哪些 exported facts 才能可视化 deformable state。
- 文档明确 future deformable state 如何进入 query / selection，或明确 V1 不进入 public query surface。
- AI routing 能把后续“网格/软体”问题路由到设计文档，而不是误改 UI。

验证方式：
- `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- `rtk proxy scripts/validate-ai-context.sh`（若当前仓库脚本可用；不可用时记录原因并至少人工复核 `docs/ai/index.md` / `repo-map.md` / `doc-catalog.yaml` 路由）
- `rtk proxy git diff --check`
- 文档审查：reviewer 确认没有把未实现 soft-body 写成已支持能力。

Subagent 执行计划：
- explorer：叶子 agent；对比当前 core/lab schema 与 deformable body 需求，列出 gap。
- worker：叶子 agent；优先 `gpt-5.4`。只写设计/路由文档，不改实现。
- reviewer：叶子 agent；审查路线边界、术语、过度承诺、与 M38 的关系。
- verifier：叶子 agent；运行 YAML / diff check。

提交策略：
- auto-commit：否。
- message hint：`Document deformable body roadmap`

风险 / 后续：
- 如果用户确认当前最想要的是 true soft-body 而非 rigid-body lattice，M38 应改为 design-only，先执行 M39。

## M40：Industrial Debugger UX Closeout

状态：进行中（browser-use 验收待恢复）

目标：
把 M35-M39 的功能收束成一个可持续的工业调试体验，其中 M37-A/B 分别代表 perturbation contract 和 UI commit 两道门。完成后，用户能从页面一眼区分 stack stability、trajectory、perturbation、lattice proxy、artifact/live source，并能把一次观察复制成可复现 debug context。

为什么现在做：
工业级体验不是功能堆叠，而是清晰的信息架构。前面 milestones 会加入新面板、轨迹和扰动，如果最后不收口，workbench 会变成散乱按钮集合。

范围：
- 统一 scenario 分组：rigid body basics、stack stability、CCD、compound/provenance、lattice proxy、diagnostics。
- 增加 overlay presets：stack stability、trajectory focus、perturbation review、lattice grid。
- Debug context 纳入 active scenario、source、frame、selected entity、layers、camera、trajectory settings、perturbation provenance、stability summary。
- 更新 README / docs/ai 路由，说明如何启动 `just picea-lab-web` 和如何验收 browser workflow。
- 复查 UI 文案：避免把 derived display、proxy、true physics facts 混在一起。
- 保留 keyboard / icon-button / tooltip 的紧凑工作台风格。

不做：
- 不新增新的 physics capability。
- 不做 landing page 或营销式 hero。
- 不做大规模视觉重写。

所有权：
- 可能触及：`README.md`、`docs/ai/doc-catalog.yaml`、`docs/ai/index.md`、`docs/ai/repo-map.md`、`crates/picea-lab/web/src/*`、contract scripts。
- 不应触及：Rust physics core、server live patch semantics、artifact schema，除非只是文档引用。

验收标准：
- 页面中各调试模式入口清晰，overlay presets 可用。
- Copy debug context 足以复现一次复杂堆叠 / 轨迹 / 扰动 / lattice 观察。
- 文档能指导下一位 agent 启动、验证、定位代码边界。
- browser acceptance 覆盖 demo、artifact、live 三条路径，且记录 `browser-use` 成功或降级原因。
- 本次 `http://127.0.0.1:5174/` browser-use 基线中的可用能力和缺口都有对应 closeout 检查；未来验收使用实际 dev URL。已有 artifact/live、contact inspector、diagnostics/evidence 被保留；新增 stack/trajectory/perturbation/lattice 能力能被 copy debug context 描述。

验证方式：
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `rtk proxy cargo test -p picea-lab --test server_routes`
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- `rtk proxy git diff --check`
- 浏览器验收：用 `just picea-lab-web` 启动，优先 `browser-use:browser`，完整走 demo / artifact / live、stack / trajectory / perturbation / lattice presets、copy debug context、console check。

Subagent 执行计划：
- explorer：叶子 agent；检查 IA / docs / contract 当前缺口。
- worker：叶子 agent；优先 `gpt-5.4`。只做收口 UI、文档、contract，不改 physics。
- reviewer：叶子 agent；审查信息架构、文案边界、是否过度扩 scope。
- verifier：叶子 agent；运行 full Web/Rust/docs/browser gates。

提交策略：
- auto-commit：否。
- message hint：`Polish lab-web industrial debugger UX`

风险 / 后续：
- 如果 M35-M38 任一功能或 M37-A/B 任一 gate 未完成，M40 只能收口已完成能力，不能在 closeout 中补实现。

## 进度记录

### 2026-05-04 - Plan Gate 草案与审查

- 状态：reviewer pass，等待用户确认
- Commit：none
- 验证：
  - `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
  - `rtk proxy git diff --check`
  - `rtk proxy git diff --check -- docs/plans/2026-05-04-picea-lab-web-industrial-ux-milestones.md docs/ai/doc-catalog.yaml`
- 验证限制：`rtk proxy scripts/validate-ai-context.sh` 当前失败，原因是仓库没有该脚本路径；本计划只把它作为 M39 未来可选 gate，并要求不可用时记录原因并人工复核路由。
- 风险 / 后续：等待用户 Plan Gate 确认；确认后从 M35 开始逐 milestone gate 执行，不一次性连续推进。

### 2026-05-05 - browser-use 基线更新

- 状态：reviewer pass-with-minor-edits 已处理，等待用户确认
- Commit：none
- 验证：
  - `browser-use:browser` 成功打开 `http://127.0.0.1:5174/`
  - 页面 console warn/error：0
  - `rtk proxy git status --short`
- 观察：
  - 当前 workbench 已能通过 artifact replay / Rust live session 展示 `四箱堆叠`、接触点、宽阶段树、island lifecycle、CCD counters、contact solver/warm-start facts。
  - 当前缺口集中在解释和交互层：没有 stack stability summary、没有可配置 trajectory workbench、没有物体级 perturbation controls、没有 lattice/grid/soft-body 场景。
- 风险 / 后续：本次只把 browser-use 证据和验收口径写回计划，不改实现；reviewer 已复核通过，新的 Plan Gate 基线等待用户确认。
- Reviewer：2026-05-05 复核结论为 `pass-with-minor-edits`，无 blocking finding；已接受并修正状态口径与“本次观测样本”限定。

### 2026-05-05 - 执行启动

- 状态：Plan Gate 已确认，开始从 M35 顺序执行剩余里程碑。
- Commit：none
- 执行约束：使用 `subagent-current-workspace` 闭环；当前工作区不创建 worktree；默认最多一个写入 worker；每个 subagent 都是叶子 agent，不允许继续启动 subagent。
- 验收约束：每个 milestone 先通过自身验证门，再进入下一 milestone；最终浏览器验收优先使用 `browser-use:browser`。

### 2026-05-05 - M35 Stack Stability Observatory 验收

- 状态：通过；M35 已完成，允许进入 M36。
- Commit：none
- 实现摘要：
  - 新增 `stack_stability_tower` 场景入口，artifact/live 均可进入。
  - Web 新增 stack stability summary、timeline marker rail、稳定性图层、contact impulse / missing evidence 显示和 Inspector 稳定性贡献。
  - 稳定性 summary 明确标注为 Web 对导出帧的派生显示，不新增 authoritative core physics fact，不触碰 solver/sleep/island 行为。
- Subagent 闭环：
  - explorer 已确认现有 `FrameRecord` / `DebugSnapshot` / perf/debug facts 足够支撑 M35 首版 Web 聚合。
  - worker 完成 M35 实现，reviewer 发现 3 个 P2：missing evidence 误作 0、contact contribution 过宽、marker rail 缺少 timeline/角漂移覆盖。
  - fix worker 已修复 P2；verifier 复跑全部 M35 gates 通过。
- 验证：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack`：PASS，1 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep`：PASS，7 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run`：PASS，17 passed。
  - `rtk proxy cargo test -p picea-lab --test server_routes`：PASS，4 passed。
  - `rtk proxy cargo test -p picea-lab`：PASS，lib 25 passed，artifact_run 17 passed，server_routes 4 passed。
  - `cd crates/picea-lab/web && rtk proxy npm run build`：PASS。
  - `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`：PASS。
  - `cd crates/picea-lab/web && rtk proxy npm run test:i18n`：PASS。
  - `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`：PASS，`yaml ok`。
  - `rtk proxy git diff --check`：PASS。
- browser-use 验收：
  - 使用 `browser-use:browser` / `iab` 成功打开 `http://127.0.0.1:5173/`；页面标题 `Picea Lab Workbench`。
  - `stack_stability_tower` 的 `Rust 产物回放` 可运行并显示 `堆叠 / 稳定性` panel、timeline markers、稳定性图层与 contact peak / solver row / drift / angular drift markers。
  - 在 artifact replay 第 69 帧选择 `接触点 305`，Inspector 显示稳定性贡献：物体 ID、接触帧数、法 / 切冲量、活动 / 休眠岛、休眠物体。
  - `Rust 实时会话` 可运行、暂停并继续显示 stack summary / markers；暂停 live session 后选择 `接触点 532`，Inspector 同样显示稳定性贡献。
  - 浏览器 console warn/error：0。
- 风险 / 后续：
  - 稳定性图层 badge 密度在高接触帧仍可能偏拥挤；M40 收口时可用 overlay preset / debug context 做进一步整理。

### 2026-05-05 - M36 Multi-Body Trajectory And Event Trails 验收

- 状态：通过；M36 已完成，允许进入 M37-A。
- Commit：none
- 实现摘要：
  - 保留 `layers.trace` 作为轨迹总开关，新增独立 `TrajectorySettings`，由 App 持有并传给 Canvas / Timeline / Inspector。
  - 底部 workbench 新增 `轨迹` tab，提供 `selectedBody`、`allDynamic`、`selectedIsland`、`contacts`、`ccd` 五种模式，以及 history length、sampling stride、fade、color mode 控件。
  - Canvas 轨迹从原先“第一个 dynamic body”升级为 Web-derived 多物体 / 接触 / CCD trail；所有绘制复用同一 world-to-screen camera transform。
  - Timeline 新增 trajectory event markers：body motion、contact appeared/disappeared/burst、CCD clamp/hit，并用平衡选取避免后半段重要事件被早期 marker 挤掉。
  - Inspector 新增 selected body/collider/contact 的 trajectory summary，明确标注为 Web 对导出帧聚合的派生显示。
  - M36 未修改 Rust core、server route 或 artifact schema。
- Reviewer / fix：
  - reviewer 未发现 P1；P2 包括 allDynamic 空态语义错误、contact stale-selection / lineage 不稳、main marker rail 时间截断、contract 过浅。
  - fix worker 已修复：新增 `trajectory.empty.allDynamic`；contact summary 从窗口内最近仍携带瞬时 id 的帧恢复稳定 lineage，并用 collider pair + feature id + approximate normal bucket 保守匹配；marker rail 改成 source-balanced / score-prioritized 后按时间展示；contract 加锁这些行为。
- 验证：
  - `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`：PASS。
  - `cd crates/picea-lab/web && rtk proxy npm run test:i18n`：PASS。
  - `cd crates/picea-lab/web && rtk proxy npm run build`：PASS，`1676 modules transformed`。
  - `rtk proxy git diff --check`：PASS。
  - `rtk proxy cargo test -p picea-lab --test artifact_run`：未作为 M36 gate 运行；M36 为 Web-only，没有触碰 Rust artifact/schema/serialization。
- browser-use 验收：
  - 重启 `just picea-lab-web` 后，`browser-use:browser` 打开 `http://127.0.0.1:5173/`，页面出现新的 `轨迹` tab。
  - 在 `四箱堆叠` 的 `Rust 产物回放` 中切换 `全部动态体`、`选中物体`、`选中岛`、`接触点` 模式，并调整历史长度 `64`、采样步长 `1`、淡出 `55`。
  - `四箱堆叠` frame 49 可见多物体轨迹和 trajectory markers；选择 `物体 2` 后 Inspector 显示移动距离、最大速度、唤醒 / 休眠、接触计数、CCD 证据；选择 `接触点 128` 后 Inspector 显示 contact trajectory summary。
  - `CCD 快凸体双墙` 场景的 CCD 模式可见 sweep / TOI 轨迹；选择 `接触点 0` 后 Inspector 显示 `CCD 轨迹`、TOI、推进、钳制、扫掠起止和 TOI 点。
  - `Rust 实时会话` 跑到 frame 119 后暂停，`全部动态体` 轨迹 workbench 仍可显示 live frame window。
  - 浏览器 console warn/error：0。
- 风险 / 后续：
  - 第一版 trajectory markers 仍是 Web heuristic，不是新的 physics event contract；若后续要稳定复现实验报告里的事件语义，可在独立 artifact/event schema milestone 中增加 authoritative event carrier。

### 2026-05-05 - M37-A Paused Perturbation Contract Preview Gate 验收

- 状态：通过；M37-A 已完成，允许进入 M37-B。
- Commit：none
- 实现摘要：
  - 新增 `POST /api/sessions/:id/velocity-perturbations/preview`，只做 server-side read-only velocity perturbation preview。
  - Live session response 新增 `session_epoch`：创建为 `0`，`reset` 递增，preview 不递增。
  - Preview 绑定 `WorldRevision`、`session_epoch`、`frame_index` 和当前 `latest_frame` authoritative handle source；没有当前 frame snapshot 的 `created` / reset 后状态返回 `missing_frame_snapshot`，不再允许旧 handle 跨 epoch 复用。
  - Preview response 返回 action id、session id、revision、epoch、body handle、frame index、before velocity、requested delta、computed target velocity、wake intent 和 rejection reason。
  - `running`、`completed`、stale revision、stale epoch、stale frame、invalid / foreign handle、static body、kinematic body、zero / invalid / non-finite delta 均明确拒绝；rejected preview 不改变 live world、frame buffer、latest frame、run id、status、events 或 epoch。
  - 文档更新 `docs/design/picea-lab-live-session-semantics.md`，明确 M37-A 不是 commit endpoint，也不暴露 UI drag / force controls。
- Reviewer / fix：
  - 首轮 reviewer 发现 P1：`created` preview 可用旧 handle + 新 revision/epoch 复用 reset 后世界；P2：rejection path 未锁 no-side-effect；P3：kinematic 分支未测。
  - fix worker 将 V1 preview 收窄为必须有当前 frame snapshot 的 authoritative handle source，补 rejected preview 前后 session / event / 下一步 step 行为锁，并补 helper-level kinematic rejection test。
  - 二轮 reviewer 无 blocker / major finding；残余风险是未来如果 create/reset 预填 snapshot，created preview 分支需要重新 review。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test server_routes`：PASS，`7 passed`。
  - `rtk proxy cargo test -p picea-lab`：PASS，lib、`artifact_run`、`server_routes`、doc-tests 均通过。
  - `rtk proxy cargo test -p picea --test core_model_world`：PASS，`18 passed`。
  - `rtk proxy cargo test -p picea --test world_step_review_regressions`：PASS，`11 passed`。
  - `rtk proxy cargo test -p picea --test query_debug_contract`：PASS，`26 passed`。
  - `rtk proxy git diff --check`：PASS。
- browser-use 验收：
  - M37-A 不新增可提交的 browser drag / force controls；浏览器验收留到 M37-B UI commit gate。
- 风险 / 后续：
  - M37-B 必须在 accepted commit 中补齐 revision / epoch bump、query resync、provenance 和 UI 可见性；M37-A 只提供 preview contract，不改变 world。

### 2026-05-05 - M37-B Paused Perturbation Commit And Controls V1 验收

- 状态：通过；M37-B 已完成，允许进入 M38。
- Commit：none
- 实现摘要：
  - Server 新增 `POST /api/sessions/:id/velocity-perturbations/commit`，只允许 paused live session 消费当前 preview action，并在 revision / session epoch / frame / body / delta 全部匹配时提交 absolute linear velocity patch。
  - Accepted commit 递增 world revision 与 `session_epoch`，同步 query pipeline，刷新同一 `frame_index` 的 `latest_frame`，截断未来 live frames，并把 action 写入 `perturbation_provenance`。
  - Web Inspector 新增 paused-only velocity perturbation controls，支持选中 dynamic body 或 collider owner，展示 requested delta、computed target velocity、accepted / rejected state 与当前 frame provenance。
  - UI 文案明确 V1 是 paused-only absolute velocity perturbation，不是 continuous force / torque、mouse joint 或 running-world editor。
- Reviewer / fix：
  - 首轮 reviewer 发现 running/created gate、SSE frame hash、tampered preview fields 与 docs rejection list 等风险；fix worker 已修复。
  - Web reviewer 发现 stale preview / commit race、availability gate 顺序、failed status 文案和 rejection reason parity 风险；fix worker 已补 guarded request token、server-refreshed latest frame 应用、完整 reason union 与 contract tests。
  - 二轮 reviewer 无 blocker / major finding；残余风险是部分 fault-injection 分支如 `body_patch_failed`、`query_sync_status = stale` 仍未直接造故障覆盖。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test server_routes`：PASS，`9 passed`。
  - `rtk proxy cargo test -p picea-lab`：PASS。
  - `rtk proxy cargo test -p picea --test core_model_world`：PASS，`18 passed`。
  - `rtk proxy cargo test -p picea --test world_step_review_regressions`：PASS，`11 passed`。
  - `rtk proxy cargo test -p picea --test query_debug_contract`：PASS，`26 passed`。
  - `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`：PASS。
  - `cd crates/picea-lab/web && rtk proxy npm run test:i18n`：PASS。
  - `cd crates/picea-lab/web && rtk proxy npm run build`：PASS，`1676 modules transformed`。
  - `rtk proxy git diff --check`：PASS。
- browser-use 验收：
  - `browser-use:browser` / `iab` 已重新激活，`rtk proxy just picea-lab-web` 启动到 backend `http://127.0.0.1:8080`、Web `http://127.0.0.1:5173`。
  - 浏览器打开 `http://127.0.0.1:5173/`，页面标题 `Picea Lab Workbench`，初始 console warn/error 为 0。
  - `Rust 实时会话` 创建成功；运行中 Inspector 的 `速度扰动` panel 显示 `请先暂停实时会话`，`预览` / `提交` 禁用，明确不是运行中连续施力或扭矩。
  - 暂停 live session 后，选中动态 `物体 1`，填写 `x 增量 = 1.25`、`y 增量 = -0.50`；`预览` 成功后 `提交` 从禁用变为可用，并显示请求增量、目标速度、查询同步和拒绝原因槽位。
  - `提交` accepted 后，当前帧 `当前帧扰动来源` 展示 world revision、session epoch、frame index、请求增量、目标速度、`查询同步：已同步`、`拒绝原因：已接受`。
  - reset 后先 `前进一帧` 生成 authoritative latest frame，再 preview / submit / `前进一帧`，页面推进到后续帧并继续显示 trajectory summary 与 accepted perturbation provenance。
  - 全流程 browser console warn/error：0。
- 风险 / 后续：
  - M37-B 仍是 absolute velocity perturbation V1；continuous force / torque / drag constraint / mouse joint 需要后续 core API milestone，不在本轮偷加。

### 2026-05-05 - M38 Rigid-Body Lattice / Grid Simulation Proxy 验收

- 状态：通过；M38 已完成，允许进入 M39。
- Commit：none
- 实现摘要：
  - 新增 `lattice_grid` 场景：4x3 dynamic circle nodes、4 个 world-anchor joints、23 个 distance joints，artifact replay 和 live session 都能导出 joint / island / solver row facts。
  - `DebugRenderFrame` 以 serde default 方式新增 `joints` carrier，旧 debug-render JSON 仍可反序列化。
  - Web 新增 lattice layer、`格点` timeline tab、Inspector lattice group、demo frames 和 i18n / UI contract；文案明确这是 rigid-body joint lattice proxy，不是 true soft-body、cloth 或 FEM solver。
  - Reviewer 发现 lattice overlay 曾退化为通用 joint overlay、canvas 命中过度优先 joint、live `lattice_grid` route 覆盖不足；已修复为仅在 `deriveLatticeProxy(...).enabled` 时绘制/命中格点层，节点点击选 body、边段点击选 joint，并补 live route 行为锁。
- Subagent 闭环：
  - explorer 确认 M38 V1 可复用现有 scene fixture、distance/world-anchor joints、`DebugJoint`、island 和 `joint_row_count` facts，不需要改 core solver / pipeline。
  - worker 完成首版实现；main Codex 集成并修复 reviewer 高优先级 findings。
  - reviewer 复核未发现 core solver / pipeline 行为漂移；schema 兼容路径有 serde default 和旧 payload 覆盖。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run lattice_grid`：PASS。
  - `rtk proxy cargo test -p picea-lab --test artifact_run`：PASS，18 passed。
  - `rtk proxy cargo test -p picea-lab --test server_routes live_lattice_grid_session_steps_with_joint_proxy_facts -- --exact`：PASS，1 passed。
  - `rtk proxy cargo test -p picea-lab --test server_routes`：PASS，10 passed。
  - `rtk proxy cargo test -p picea-lab`：PASS，lib 27 passed，artifact_run 18 passed，server_routes 10 passed。
  - `rtk proxy cargo test -p picea --test core_model_world`：PASS，18 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep`：PASS，7 passed。
  - `cd crates/picea-lab/web && rtk proxy npm run build`：PASS，`1676 modules transformed`。
  - `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`：PASS。
  - `cd crates/picea-lab/web && rtk proxy npm run test:i18n`：PASS。
  - `rtk proxy cargo fmt --check`：PASS。
  - `rtk proxy git diff --check`：PASS。
- browser-use 验收：
  - `browser-use:browser` / `iab` 已激活当前 Codex in-app browser，`rtk proxy just picea-lab-web` 启动到 backend `http://127.0.0.1:8080`、Web `http://127.0.0.1:5173`。
  - `Rust 产物回放` 下选择 `刚体格点代理` 后，场景层级显示 12 个 dynamic bodies / 27 个 joints；`格点 / 网格代理` tab 显示 12 节点、27 边、4 锚点、最大拉伸比 `1.000`、27 求解行、0 接触点、活动 / 休眠岛 `1/0`。
  - `格点代理` layer menu item 可从 `aria-checked=true` 切到 `false` 再切回 `true`。
  - `Rust 实时会话` 下同一场景可运行到 playing 状态；live frame 中 `格点` tab 显示 12 节点、27 边、4 锚点、27 求解行，最大拉伸比随 live steps 变化（实测 frame 86 为 `1.077`）。
  - 选择 `物体 0` 后 Inspector 展示 lattice node facts；选择 `关节 4 距离关节` 后 Inspector 展示当前长度、参考长度和拉伸比。
  - 切到非 lattice 的 `世界锚点关节` 场景时，`格点` tab 只显示 `缺少证据`，确认 M38 overlay 没有泛化成通用 joint overlay。
  - 浏览器 console warn/error：0。
- 验证限制：
  - `tab.playwright.screenshot({ fullPage: false })` 和 CUA screenshot 在当前 browser-use 通道会超时并重置 Node REPL；本次浏览器验收以 DOM / role / tabpanel / console state 为准，未记录截图作为证据。
- 风险 / 后续：
  - M38 仍是 rigid-body lattice proxy。true soft-body / particle / deformable mesh 进入 M39 RFC gate，不在本 milestone 中实现。

### 2026-05-05 - M39 Soft-Body / Particle / Deformable Mesh RFC Gate 验收

- 状态：通过；M39 已完成，允许进入 M40。
- Commit：none
- 设计输出：
  - 新增 `docs/design/deformable-body-roadmap.md`，明确当前 Picea 是 2D rigid-body engine，M38 是 rigid-body lattice proxy，不是 true soft-body / cloth / FEM / particle simulation。
  - RFC 推荐如果进入真实 deformable physics，第一条路线应先在设计上收敛为 CPU 2D XPBD/PBD particle-grid 或 particle-mesh family；FEM、3D tet mesh、GPU、tearing/cutting 和 runtime topology mutation 均不属于首个实现 slice。
  - 文档定义 future core surface 候选：`DeformableHandle`、`ParticleHandle`、可选 `ElementHandle` / `ConstraintHandle`、`DeformableDesc`、material model、pipeline phase、rigid coupling、invalid topology failure modes。
  - 文档定义 future query / selection boundary：不要把 deformable state 静默塞进现有 rigid `QueryPipeline`；必须先决定 whole deformable、particle、element / constraint 或 collision proxy 的 hit 语义，以及 revision-aware handle 映射。
  - 文档定义 artifact/debug facts、lab authoring/UI 需求、verification gates、open questions 和 implementation decision checklist。
  - 更新 `docs/design/README.md`、`docs/ai/index.md`、`docs/ai/repo-map.md`、`docs/ai/doc-catalog.yaml`，让后续 soft-body / particle / cloth / FEM / XPBD / PBD 问题先路由到 RFC，而不是直接落到 `picea-lab-web`。
- Subagent 闭环：
  - explorer 确认当前 core/lab/web 只有 rigid body / joint / lattice proxy surface，没有 soft-body / particle / FEM / XPBD / PBD core 入口；建议 M39 保持 doc/RFC gate。
  - reviewer 无 blocker；主要 finding 是 query / selection public surface 太薄，以及计划状态未同步。已补 query / selection boundary，并把计划状态与进度记录同步。
- 验证：
  - `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`：PASS，`yaml ok`。
  - `rtk proxy git diff --check`：PASS。
  - `rtk proxy scripts/validate-ai-context.sh`：未运行；当前仓库没有可执行脚本路径，按 M39 验收约定记录原因并人工复核 `docs/ai/index.md` / `docs/ai/repo-map.md` / `docs/ai/doc-catalog.yaml` routing。
- browser-use 验收：
  - M39 为 design/RFC gate，不新增页面行为；browser acceptance 已在 M38 验证 rigid lattice proxy 的非 soft-body 边界，最终综合浏览器验收留到 M40。
- 风险 / 后续：
  - M39 不选择最终 deformable implementation route，也不授权实现 solver。若后续要做 true deformable，第一个执行 milestone 必须先补 behavior locks、debug/artifact schema 和 query/selection contract。

### 2026-05-05 - M40 Industrial Debugger UX Closeout 静态门与 reviewer 修复

- 状态：实现与静态验证通过；最终 browser-use 综合验收待恢复。
- Commit：none
- 实现摘要：
  - 场景选择器改为前端派生分组，不要求 server scenario schema 新增 group 字段。
  - Toolbar 新增 overlay preset 菜单：stack stability、trajectory focus、perturbation review、lattice grid；原 granular layer toggles 保留。
  - Copy debug context 扩展为当前 workbench 派生 payload：scenario name/description/group、source/status/session/run/frame/state hash、live latest-frame authority、selection、layers、camera、trajectory settings/summary/window-local markers、stack stability summary、lattice proxy summary、当前帧 perturbation provenance、preview/commit state、perf summary。
  - Evidence panel 增加 closeout summary rows，并用 `Web-derived` / `proxy (not soft-body)` 文案区分派生展示、刚体代理与 authoritative Rust facts。
  - README 和 AI routing 补充 `rtk proxy just picea-lab-web`、实际 dev URL、browser-use 验收路径。
- Subagent 闭环：
  - explorer 确认剩余缺口集中在 scenario grouping、overlay presets、copy debug context payload、docs/routing 和 contract。
  - worker 完成 M40 收口实现并跑过 UI/i18n/build/diff gates。
  - reviewer 无 blocker；发现 trajectory markers 使用全局尾部、live authority 未写入 payload、evidence summary 边界文案不足、contract 偏 presence-oriented。已修复并加强 contract 行为锁。
- 验证：
  - `cd crates/picea-lab/web && rtk proxy npm run test:dev-server`：PASS。
  - `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`：PASS。
  - `cd crates/picea-lab/web && rtk proxy npm run test:i18n`：PASS。
  - `cd crates/picea-lab/web && rtk proxy npm run build`：PASS，仍有 Vite chunk size warning（主 JS 约 516 KB），不阻塞本 milestone。
  - `rtk proxy cargo test -p picea-lab --test server_routes`：PASS，10 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run`：PASS，18 passed。
  - `rtk proxy cargo test -p picea-lab`：PASS，lib 27 passed，artifact_run 18 passed，server_routes 10 passed。
  - `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`：PASS，`yaml ok`。
  - `rtk proxy git diff --check`：PASS。
- browser-use 状态：
  - `rtk proxy just picea-lab-web` 已启动到 API `http://127.0.0.1:8081`、Vite `http://127.0.0.1:5174/`。
  - 当前 `browser-use:browser` / `iab` backend discovery 失败：`No Codex IAB backends were discovered`，6 个 pipe candidates 均 timeout；因此未完成 M40 最终浏览器验收。
- 风险 / 后续：
  - 需要恢复 Codex in-app browser backend 后，继续执行 demo / artifact / live、scenario grouping、overlay presets、stack / trajectory / perturbation / lattice、copy debug context 和 console warn/error 验收。
