# Picea Performance 与堆叠稳定性可观测性里程碑计划

状态：已完成
计划文档：docs/plans/2026-05-06-performance-stability-observability-milestones.md
最后更新：2026-05-06
工作目录：/Users/asyncrustacean/projects/picea
工作区状态：dirty；当前变更包含本计划、diagnostics 实现、AI 路由文档和设计文档；`.gitignore` 是本轮前已存在的 unrelated dirty 文件，未触碰。
提交策略：不提交
执行策略：完整计划批准后连续执行；若遇到 dirty 文件重叠、设计假设失效或验证失败，暂停并回到计划更新。

## 目标

从 performance 和多物体堆叠稳定性两个角度，把现有 `picea` / `picea-lab` / `picea-lab-web` 观测能力推进到可定位问题根因的状态。

完成后，用户应该能在 `picea-lab-web` 或导出的 artifact 中回答：

- 哪一帧开始出现接触数量、求解行、穿透、漂移、睡眠状态或性能计数异常。
- 异常关联的是哪些 body / collider / contact / island，而不是只看到“堆叠倒了”。
- performance 变化是 deterministic work counters 变化、artifact / Web 处理开销变化，还是 wall-clock 噪声。
- Web 面板展示的是 Rust authoritative facts、lab artifact facts，还是明确标注的 Web-derived 聚合。
- 这些证据是否足以进入后续 solver / sleep / contact stabilization 修复，而不是先凭截图猜测。

## 与现有计划的关系

本计划继承并补充现有 observability / lab-web 路线，不替代或重开已完成的 Web UX 工作。

- `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md` 中 M23-M30 已完成 performance counters、lab-web process visualization foundation、performance threshold policy 和 public beta 收口。本计划不重做这些基础。
- `docs/plans/2026-05-03-picea-lab-web-debuggability-milestones.md` 与 `docs/plans/2026-05-04-picea-lab-web-industrial-ux-milestones.md` 已覆盖 live controls、canvas、stack stability observatory、trajectory、perturbation、copy-debug-context 等 Web 工作台能力。本计划不重做 M35/M40 的 UI/UX 面。
- 本计划的新增价值是：把 performance 与堆叠稳定性的现有显示面收束成可审查的 diagnostics contract；先补 lab/artifact authoritative evidence；只有 D1 证明现有 facts 不足时才进入最小 core read-model carrier；最后用真实 stack 场景和浏览器验收产出后续 solver 修复输入。
- 因此，本计划的风险主要是“与既有 milestone scope 重叠”，不是当前 git dirty 文件重叠。执行时必须优先复用现有 M35/M40 面板和 copy context，不另起一套平行 UI。

## 约束

- 工作区：只在 `/Users/asyncrustacean/projects/picea` 当前主工作区规划和执行。
- 计划阶段只写本计划文档，不修改 production implementation code。
- 所有验证命令使用 `rtk proxy`。
- `crates/picea` 只导出 consumer-neutral facts；不引入 UI、浏览器、artifact 路径或 benchmark interpretation。
- `crates/picea-lab` 负责 artifact capture / schema / run-level summary；不重新实现 physics。
- `picea-lab-web` 只消费和聚合 Rust / artifact facts；Web-derived 指标必须标注来源，不能伪装成 solver authoritative facts。
- 不在本计划里修 solver row math、narrowphase、broadphase heuristics、CCD 或 running-world patch 语义。
- 任何 Rust schema 改动必须 additive，并使用 serde default / missing evidence fallback 保持旧 artifact 可读。
- performance 先用 deterministic counters 和基线解释，不把单次本机 wall-clock 变成 hard fail。
- 浏览器验收优先使用 `browser-use:browser`；如果当前 in-app browser 后端不可用，允许降级 Playwright CLI 并报告产物。
- subagent 作为叶子 agent 使用；分派时必须明确写入：“你是叶子 agent，不允许再启动 subagent。”

## 待确认问题

### 必须确认

- 无。Plan Gate 审批放在 `计划验收`，不是未决架构问题。

### 可带假设推进

- 假设允许新增最小 lab artifact diagnostics carrier：影响 E1 范围；早期验证是 D1 先证明现有 `FrameRecord` / `DebugSnapshot` / `PerfArtifact` 的聚合缺口，并把字段归属写清楚。
- 假设只有 D1/E1 证明现有 facts 不足时才新增最小 core read-model carrier：影响 E2 是否执行；早期验证是 E1 产出明确缺口记录，且 E2 不触碰 solver behavior。
- 假设第一版 performance 不做 hard threshold：影响验收口径；早期验证是继续遵守 `docs/design/performance-threshold-policy.md` 的 baseline / warn / fail 分层。
- 假设稳定性第一版只做定位和证据，不直接修堆叠：影响用户可见结果；早期验证是 V1 产出可以指向后续 solver 修复的 first-bad-frame / body-contact-island 证据。

## 规划依据

- explorer：已运行。
- 关键证据：
  - `crates/picea/src/pipeline.rs` 已有 `StepStats`，覆盖 broadphase traversal/prune、tree depth、island、solver rows、warm-start、CCD、sleep transition、numeric warnings 等 counters。
  - `crates/picea/src/debug.rs` 已有 `DebugStats` / `DebugSnapshot` / `DebugIsland` / `DebugBroadphaseTree`，并明确是 consumer-neutral read model。
  - `crates/picea/src/events.rs` 已有 contact lifecycle、warm-start、solver impulse、sleep/wake reason 等 event facts。
  - `crates/picea-lab/src/artifact.rs` 已有 `FrameRecord`、`DebugRenderArtifact`、`PerfArtifact` 和 run-level `PerfCounterSummary`。
  - `crates/picea-lab/tests/artifact_run.rs` 已有 stack tower artifact facts 锁点，但当前锁的是“facts 存在”，不是“稳定性可解释”。
  - `crates/picea-lab/web/src/types.ts` 已 typed 消费 `report` / `stats` / `events` / `snapshot` / `perf.json`。
  - `crates/picea-lab/web/src/components/workbench/stackStability.ts` 明确当前堆叠指标是 Web-derived 聚合，不是 authoritative solver facts。
  - `crates/picea-lab/web/src/components/workbench/Timeline.tsx` 已展示 stack markers、perf summary 和 copy debug context，但 performance / stability 还没有统一归因契约。
  - `docs/design/picea-lab-observability-architecture.md` 明确 core expose facts、lab capture/render/compare/summarize、viewer 不运行 physics。
  - `docs/design/debug-observability-design.md` 要求 phase-scoped trace、enum-like reason fields、opt-in trace collection，不 dump whole world。
  - `docs/design/performance-threshold-policy.md` 明确 performance 先用 Criterion 与 deterministic counters 解释，不用单次 wall-clock hard fail。
- 主要未知：
  - 哪些 stability facts 能从现有 contact / event / snapshot 安全派生，哪些必须新增 Rust authoritative carrier。
  - 是否需要 pipeline phase timing；若需要，第一版应落在 lab/debug-only 还是 core opt-in recorder。
  - first-bad-frame 的阈值应如何定义，才能帮助 debug 而不把 stress demo 错当 correctness oracle。
  - 当前 git dirty 只有 `.gitignore` 与本计划文件；真正的范围风险来自与既有 M35-M40 Web milestone 的功能面重叠，执行时需要优先复用已有能力。

## 计划验收

- 状态：已批准
- Design Gate：已确认
- Execution Gate：已确认；用户在 2026-05-06 回复“开始”后进入 E1。
- reviewer：已运行
- 审查结论：有发现，已采纳修订：补充与既有计划关系、修正 dirty 现场、拆分 lab/artifact carrier 与可选 core carrier、收拢 V1 总验收门。
- 用户确认：已确认

## 设计 / 执行分离规则

- 设计里程碑只允许修改计划文档、设计文档、调研记录或测试计划，不修改生产实现代码。
- 执行里程碑必须引用已批准的设计结论，或者显式说明为什么不需要设计前置。
- 如果执行中发现设计假设不成立，暂停执行并回到对应设计里程碑更新计划。
- 设计里程碑完成不等于允许实现；实现仍需经过 Execution Gate 或用户确认。
- 一个设计里程碑可以生成多个执行里程碑；一个执行里程碑必须绑定清晰的设计输入。

## 里程碑映射

| 设计里程碑 | 产出 | 解锁的执行里程碑 | 状态 |
| --- | --- | --- | --- |
| D1 | Performance / stability diagnostics contract、字段归属、schema tier、UI 映射 | E1, E2, E3, V1 | 已完成 |
| D2 | stack 复现窗口、first-bad-frame 口径、验收阈值和后续 solver 输入格式 | E1, E2, V1, C1 | 已完成 |

## 里程碑

### D1：Diagnostics Contract 与事实归属

类型：设计
状态：已完成

目标：
冻结 performance 与 stability 可观测性的第一版契约，明确哪些事实来自 core、哪些来自 lab artifact、哪些只是 Web-derived 聚合。

要回答的问题：
- Performance 线：哪些 deterministic counters 已足够，哪些还需要 per-frame 或 run-level 归因字段。
- Stability 线：穿透、漂移、contact churn、warm-start、position correction、sleep reset / wake reason、island active/sleep 应由哪里导出。
- Schema 线：新增字段是扩展 `StepStats` / `DebugSnapshot`，还是挂在 lab-owned `FrameDiagnostics` / `PerfArtifact` 下。
- Timing 线：第一版是否只做 lab runner / artifact / Web handling timing，还是引入 core opt-in phase recorder。
- UI 线：Performance panel、Stability panel、timeline marker 和 copy-debug-context 分别消费哪些字段。

为什么现在做：
现有 Web 已能显示 stack / perf summary，但 attribution 还不够清楚。先做契约可以避免把 Web 派生指标误升级成 solver facts，也避免为了观测而先改物理算法。

设计范围：
- `docs/design/picea-lab-observability-architecture.md`
- `docs/design/debug-observability-design.md`
- `docs/design/performance-threshold-policy.md`
- 本计划文档
- 现有 `StepStats`、`DebugSnapshot`、`FrameRecord`、`PerfArtifact`、Web types 的只读盘点

不做：
- 不改生产实现代码
- 不新增 Rust structs / TS types
- 不执行重构、迁移、提交或产品行为变更
- 不决定 solver 修复方案

输入证据：
- `crates/picea/src/pipeline.rs`
- `crates/picea/src/debug.rs`
- `crates/picea/src/events.rs`
- `crates/picea-lab/src/artifact.rs`
- `crates/picea-lab/web/src/types.ts`
- `crates/picea-lab/web/src/components/workbench/stackStability.ts`
- `crates/picea-lab/web/src/components/workbench/Timeline.tsx`

输出交付物：
- 字段归属表：core authoritative / lab-owned / Web-derived。
- Schema tier：always-on cheap counters / debug-only trace / lab-only timing / UI-only aggregation。
- Performance panel 与 Stability panel 的数据映射。
- E1 / E2 / E3 / V1 的执行输入和验收口径更新。
- 设计文档：`docs/design/performance-stability-diagnostics-contract.md`。

设计验收：
- 每个新增或复用字段都有 owner、来源、缺失时 UI 行为和 verification。
- 明确哪些 Web 指标只能用于观察，不能作为 solver correctness oracle。
- 明确第一版是否需要 Rust authoritative carrier；若需要，字段必须 additive 且可 serde default。
- reviewer 审查通过或发现已记录处理。

验证方式：
- 文档检查。
- reviewer 审查。
- `rtk proxy git diff --check`

下一步映射：
- 生成/更新执行里程碑：E1, E2, E3, V1。

风险 / 后续：
- 如果 D1 发现现有 facts 已足够，则 E1 应收窄为 artifact / web 接线，不新增 core carrier。
- 如果 D1 发现必须新增 core carrier，需要优先控制为最小字段集，避免变成通用 tracing 框架。

### D2：Stack 复现窗口与 first-bad-frame 口径

类型：设计
状态：已完成

目标：
把多物体堆叠不稳定的复现和验收口径固定下来，让后续观测字段知道要解释什么。

要回答的问题：
- 第一版稳定性诊断以 `stack_4`、`stack_stability_tower`，还是新增更小的 stable-stack repro 作为主对象。
- first-bad-frame 如何定义：穿透峰值、漂移峰值、角速度/角漂移、contact churn、sleep never converged、position correction reset、还是组合 marker。
- 哪些指标是 stress demo 观察指标，哪些未来可以成为 behavior lock。
- V1 应产出什么格式的 solver 修复输入。

为什么现在做：
没有固定复现窗口时，观测面板会堆很多数字，却仍然不知道哪一帧开始值得调。先确定口径，可以把后续 execution 变成可验收的证据链。

设计范围：
- `crates/picea-lab/src/scenario.rs` 的 stack 场景只读盘点。
- `crates/picea-lab/tests/artifact_run.rs` 的 stack artifact 测试只读盘点。
- `crates/picea/tests/physics_realism_acceptance.rs` 的 stack / sleep 相关测试只读盘点。
- 本计划文档中的 V1 验收更新。

不做：
- 不新增或修改场景。
- 不新增 behavior test。
- 不修 solver / sleep / contact。
- 不把 stress tower 的视觉结果直接定义成 correctness failure。

输入证据：
- `stack_4` 与 `stack_stability_tower` 已能暴露多接触、多 island、solver rows。
- 当前 artifact test 只验证 stack facts 存在，不验证稳定性是否收敛。
- 当前 Web stack 指标能显示 drift、angular drift、jitter proxy、quiet window，但还不是 authoritative diagnostic contract。

输出交付物：
- 首选 repro 场景和 frame window。
- first-bad-frame marker 列表与阈值来源。
- 哪些指标只用于 debug，哪些未来可迁移为 behavior lock。
- V1 solver handoff 报告模板。
- 设计文档：`docs/design/stack-stability-repro-diagnostics.md`。

设计验收：
- V1 可以不读代码也知道应生成哪些 artifact、看哪些 frame、复制哪些 debug context。
- 明确 stable-stack 和 stress-tower 的区别。
- 明确本计划不以观测替代后续失败测试。

验证方式：
- 文档检查。
- reviewer 审查。
- `rtk proxy git diff --check`

下一步映射：
- 生成/更新执行里程碑：E1, E2, V1, C1。

风险 / 后续：
- 阈值过严会把当前 stress scenario 当成 correctness oracle；阈值过松会失去定位价值。D2 只固定 debug marker，不直接冻结 solver acceptance。

### E1：Lab / Artifact Diagnostics Carrier

类型：执行
状态：已完成
来源设计：D1, D2

目标：
按 D1 决定的字段归属，优先在 `picea-lab` artifact 层补齐最小 diagnostics carrier，让 performance 和 stability 归因能从已有 authoritative facts 出发。

执行输入：
- D1 的 owner / tier / schema 表。
- D2 的 repro window 与 first-bad-frame marker。
- 允许修改的文件/模块限定在 lab artifact / scenario / tests；不触及 `crates/picea/src/*`。
- 必须保持旧 artifact serde 兼容。

为什么现在做：
Web 现有 stack summary 已能观察现象，但仍有“为什么这一帧坏掉”的缺口。E1 先把可复用 facts 放进 artifact，E3 才能安全展示。把 lab/artifact 层单独拆出，可以避免第一步就进入 core 或 solver 内部。

范围：
- 增加或扩展 D1 认可的 lab-owned diagnostics carrier。
- Performance：复用 `StepStats` / `PerfCounterSummary`，必要时增加 lab-owned per-frame / run-level timing、counter delta 或 spike summary。
- Stability：基于 `FrameRecord` / `DebugSnapshot` / `WorldEvent` 可得事实，增加 first-bad-frame、contact churn、sleep/wake、island、warm-start、penetration summary 或 missing evidence summary。
- 给新增字段加 serde default / missing evidence 行为。
- 补 artifact schema / regression tests。

不做：
- 不改 solver row math。
- 不改 contact generation、broadphase heuristics、CCD 或 sleep policy。
- 不修改 `crates/picea/src/*`。
- 不改 Web UI。
- 不把 wall-clock timing 变成 hard fail。

所有权：
- 可能触及：`crates/picea-lab/src/artifact.rs`、`crates/picea-lab/tests/artifact_run.rs`、`crates/picea-lab/src/scenario.rs`。
- 不应触及：`crates/picea/src/*`、solver behavior、contact response algorithms、server running-world patch semantics、Web UI files。

验收标准：
- 新增 diagnostics 在 `frames.jsonl` / `debug_render.json` / `perf.json` 中可被旧 artifact 缺字段安全跳过。
- `stack_4` 或 `stack_stability_tower` 能导出至少一个 D2 定义的 first-bad-frame 相关事实。
- Performance summary 能区分 deterministic work counter 与 timing / missing evidence。
- 新增测试能证明字段存在、serde default 生效、旧 schema 不崩。
- 若 D1 证明 lab/artifact 层无法解释关键缺口，E1 必须显式记录缺口并暂停进入 E2，而不是在 E1 内临时改 core。

验证方式：
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- 如触及 stack evidence：`rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要；复核 D1 字段归属对应的当前代码落点。分派时必须明确写入：“你是叶子 agent，不允许再启动 subagent。”
- worker：叶子 agent；普通 lab artifact/schema 任务优先 `gpt-5.4`，只实现 E1 允许文件范围。
- reviewer：叶子 agent；审查 serde 兼容、事实归属、是否误改 core/physics behavior、是否把 timing 当 correctness。
- verifier：叶子 agent；运行 E1 验证命令并报告验证过程中产生或修改的文件。

提交策略：
- auto-commit：否。
- message hint：`Add lab performance and stack diagnostics carrier`。

风险 / 后续：
- 如果 D1 / E1 发现必须新增 core read-model facts，暂停并进入 E2；不能在 E1 里顺手触碰 solver 或 core hot path。

### E2：Core Authoritative Carrier（条件执行）

类型：执行
状态：已跳过
来源设计：D1, D2；依赖 E1 缺口结论

目标：
只有当 D1/E1 明确证明 lab/artifact 层无法解释关键稳定性或 performance 缺口时，补一个最小 core read-model carrier，并保持 additive、consumer-neutral、默认低成本。

执行输入：
- D1 的字段归属表，必须明确指出现有 `StepStats` / `DebugSnapshot` / `WorldEvent` 哪里不足。
- E1 的缺口记录，必须说明 lab-owned 派生无法可靠表达的事实。
- D2 的 first-bad-frame 口径，必须说明新增 core fact 如何帮助定位。

为什么现在做：
有些问题可能确实需要 core authoritative facts，例如 position correction 是否重置 sleep、某类 sleep wake reason 是否不可从现有事件安全推导。把它作为条件里程碑可以避免默认把观测工作推入 solver 内部。

范围：
- 最小扩展 `StepStats`、`DebugStats`、`DebugSnapshot` 或 `WorldEvent` 中 D1 批准的 read-model facts。
- 必须 additive，并使用 serde default。
- 如需观测 pipeline phase，只能先做 opt-in / debug-only / allocation-light carrier，不做 always-on tracing framework。
- 补 core read model / event / artifact 兼容测试。

不做：
- 不改 solver row math。
- 不改变 sleep / wake / contact / broadphase / CCD 行为。
- 不暴露 private solver storage ids、broadphase proxy ids 或 UI-only concepts。
- 不做通用 tracing framework。
- 不改 Web UI；Web 接线留给 E3。

所有权：
- 可能触及：`crates/picea/src/pipeline.rs`、`crates/picea/src/debug.rs`、`crates/picea/src/events.rs`、`crates/picea-lab/src/artifact.rs`、相关 tests。
- 只有 D1 明确批准、且字段是 read-only fact 时才可触及：`crates/picea/src/pipeline/step.rs`、`crates/picea/src/pipeline/sleep.rs`。
- 不应触及：`crates/picea/src/solver/contact.rs`，除非 D1 追加批准一个明确的 read-only observation point；即便触及也不得改变 behavior。

验收标准：
- 新增 core facts 在 `DebugSnapshot` / `StepReport` / artifact 中可被消费者稳定读取。
- 旧 artifact / 旧 JSON 缺字段时 serde default 通过。
- 相关 core tests 能证明 behavior 未改变，只增加观察事实。
- E2 产物足够让 E3 显示来源为 Rust authoritative facts。

验证方式：
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- 如触及 sleep facts：`rtk proxy cargo test -p picea --lib pipeline::sleep`
- 如触及 stack evidence：`rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要；复核 D1/E1 缺口和最小 core 文件落点。
- worker：叶子 agent；架构敏感 core carrier 任务优先继承主模型或显式 `gpt-5.5`，只做批准字段。
- reviewer：叶子 agent；审查 public/read-model 边界、serde 兼容、hot path 成本、是否误改 behavior。
- verifier：叶子 agent；运行 E2 验证命令并报告验证过程中产生或修改的文件。

提交策略：
- auto-commit：否。
- message hint：`Add minimal core diagnostics carrier`。

风险 / 后续：
- E2 是条件执行里程碑；若 E1 已能满足诊断需要，应标记为跳过，而不是为了“更完整”继续加 core 字段。

### E3：Lab-Web Performance / Stability Diagnostics Surface

类型：执行
状态：已完成
来源设计：D1, D2；依赖 E1；若 E2 执行则同时依赖 E2

目标：
把 E1 / E2 导出的 diagnostics 接入 `picea-lab-web`，让 performance 和 stability 归因能在 timeline、panel 和 copy-debug-context 中被稳定消费。

执行输入：
- D1 的 UI 数据映射。
- D2 的 first-bad-frame marker。
- E1 的 artifact schema 与 missing evidence 行为。
- E2 的 core read-model fields；若 E2 跳过，则显式记录 Web 只消费 lab/artifact diagnostics。

为什么现在做：
用户主要从浏览器 demo 观察问题。只有 artifact facts 接到 Web，调试体验才会从“看现象”变成“点 frame / contact / island 定位原因”。

范围：
- 扩展 Web `types.ts`，typed 消费 E1 / E2 diagnostics。
- Stability panel 优先显示 Rust / artifact diagnostics，缺失时保留 Web-derived fallback 并明确标注。
- Performance panel 展示 run-level summary、per-frame spikes、counter deltas、missing evidence 状态。
- Timeline marker 支持 D2 first-bad-frame / spike markers。
- Copy debug context 包含 scenario、source、run/session、frame window、selected entity、performance/stability diagnostics、marker、state hash。
- 更新 i18n 与 UI contract。

不做：
- 不在 Web 重新计算 physics。
- 不引入 visual editor、server autoplay、running-world patch 或 live rewind。
- 不改 Rust schema；若 schema 不足，暂停回到 E1/E2。
- 不做 benchmark explorer 或多 run 对比。

所有权：
- 可能触及：`crates/picea-lab/web/src/types.ts`、`crates/picea-lab/web/src/App.tsx`、`crates/picea-lab/web/src/api.ts`、`crates/picea-lab/web/src/components/workbench/Timeline.tsx`、`crates/picea-lab/web/src/components/workbench/Inspector.tsx`、`crates/picea-lab/web/src/components/workbench/stackStability.ts`、`crates/picea-lab/web/src/i18n.ts`、contract scripts。
- 不应触及：`crates/picea/src/*`、solver / pipeline behavior、artifact capture logic；若字段不足，暂停回到 E1/E2。

验收标准：
- Browser 中选择 stack 场景时，可以看到 performance 与 stability 诊断来源。
- 缺少 E1 / E2 新字段的旧 artifact 不崩溃、不显示为 0 成本或 0 风险，而是 missing evidence。
- marker 点击或 frame 切换能定位到 D2 定义的异常帧。
- copy-debug-context 能生成可粘贴的高信号调试上下文。
- Web-derived 与 Rust authoritative facts 的来源标签清楚。

验证方式：
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- 如涉及 dev server scripts：`cd crates/picea-lab/web && rtk proxy npm run test:dev-server`
- `rtk proxy git diff --check`
- 浏览器验收：
  - 首选启动：`rtk proxy just picea-lab-web`
  - 调试 fallback 后端：`rtk proxy cargo run -p picea-lab -- serve --bind 127.0.0.1:18080`
  - 调试 fallback Web：`cd crates/picea-lab/web && rtk proxy npm run dev -- --host 127.0.0.1 --port 5174`
  - 优先用 `browser-use:browser` 打开对应本地 URL，选择 stack 场景，检查 performance panel、stability panel、marker、copy-debug-context、console error。

Subagent 执行计划：
- explorer：需要；确认 E1/E2 diagnostics 到 Web typed surface 的映射。
- worker：叶子 agent；普通 Web 接线优先 `gpt-5.4`，只做 E3 文件范围。
- reviewer：叶子 agent；审查来源标签、missing evidence、UI 是否重新计算 physics、copy context 是否足够复现。
- verifier：叶子 agent；运行 Web build / contract / browser gates，报告是否产生 `.playwright-cli/`。

提交策略：
- auto-commit：否。
- message hint：`Surface performance and stack diagnostics in lab web`。

风险 / 后续：
- Web 面板容易膨胀。本里程碑只做 attribution 和 copy context，不做 benchmark explorer。

### V1：真实复现证据与浏览器验收

类型：验证
状态：已完成
来源设计：D1, D2；依赖 E1, E3；若 E2 执行则同时依赖 E2

目标：
用真实 stack 场景生成 artifact / Web 证据，并复跑总验证矩阵，确认 observability 已经足够支持后续 solver 修复。

范围：
- 运行 D2 选定的 stack 场景并保存 run path。
- 在 artifact 中检查 first-bad-frame、performance/stability diagnostics、state hash、selected frame context。
- 在浏览器中打开对应场景，完成 frame marker、panel、copy-debug-context 验收。
- 复跑 Rust artifact、必要 core read model、Web contract、browser acceptance 和 diff check。
- 产出简短 solver handoff：指出最可能的下一步修复方向和需要补的 failing test。

不做：
- 不修 solver。
- 不提交。
- 不扩大到所有 scenario。
- 不把本机 wall-clock 作为 correctness 结论。

所有权：
- 可能触及：本计划文档进度记录；必要时新增 `docs/reports/` 下的验证报告。
- 不应触及：implementation code。

验收标准：
- 至少一个 stack run 能生成完整 diagnostics evidence。
- 浏览器里能从 marker 跳到异常帧，并复制包含 performance/stability facts 的 debug context。
- 报告明确下一步 solver / sleep / contact 修复候选，但不声称已经修复堆叠。

验证方式：
- Rust artifact：`rtk proxy cargo test -p picea-lab --test artifact_run`
- Core stack gate：`rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- Core read model：如果 E2 执行或 E1/E3 发现 core surface 受影响，运行 `rtk proxy cargo test -p picea --test world_step_review_regressions`
- Core sleep gate：如果 E2 触及 sleep facts，运行 `rtk proxy cargo test -p picea --lib pipeline::sleep`
- Web build：`cd crates/picea-lab/web && rtk proxy npm run build`
- Web UI contract：`cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- Web i18n contract：`cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- Browser acceptance：首选 `rtk proxy just picea-lab-web` 启动本地服务；优先用 `browser-use:browser` 打开本地 URL，检查 stack 场景、performance/stability panel、marker、copy-debug-context、console error。若不可用，使用 E3 的双进程 fallback 并报告原因。
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：不需要。
- worker：不需要，除非需要写验证报告。
- reviewer：可选；审查 handoff 是否过度推断。
- verifier：叶子 agent；运行 V1 验证命令和浏览器验收，报告 run path、浏览器 URL、是否产生验证产物。

提交策略：
- auto-commit：否。
- message hint：无。

风险 / 后续：
- 如果 V1 发现 diagnostics 仍缺关键字段，应暂停并回到 D1/E1/E2，而不是直接进入 solver 修复。

### C1：文档路由与后续 solver 输入收口

类型：收尾
状态：已完成
来源设计：D1, D2；依赖 V1

目标：
把完成后的可观测性路线登记到 AI 路由和设计文档，并把后续 solver 修复输入收束成下一轮可执行任务。

范围：
- 更新本计划进度记录。
- 必要时更新 `docs/ai/doc-catalog.yaml`、`docs/ai/index.md`、`docs/ai/repo-map.md`。
- 必要时补充 `docs/design/picea-lab-observability-architecture.md` 或 `docs/design/debug-observability-design.md` 的当前状态。
- 写出下一轮 solver / sleep / contact stabilization 的建议入口、测试和非目标。

不做：
- 不修 physics。
- 不重写 M23-M40 历史计划。
- 不删除或清理 unrelated dirty files。

所有权：
- 可能触及：本计划文档、`docs/ai/doc-catalog.yaml`、`docs/ai/index.md`、`docs/ai/repo-map.md`、相关 `docs/design/*`。
- 不应触及：production implementation code。

验收标准：
- 未来会话能从 AI 路由找到 performance/stability observability 的入口。
- 后续 solver 修复任务有明确复现、first failing test 候选、验证门和非目标。
- 当前计划状态、验证证据、剩余风险记录清楚。

验证方式：
- `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：不需要。
- worker：叶子 agent；文档收口可用 `gpt-5.4`。
- reviewer：叶子 agent；审查路由是否准确、是否误称 solver 已修复。
- verifier：叶子 agent；运行 YAML 与 diff check。

提交策略：
- auto-commit：否。
- message hint：`Document performance and stack observability route`。

风险 / 后续：
- 如果执行中没有更新 AI 路由，后续会话可能重新走弯路；C1 应作为完成条件，而不是可选美化。

## 进度记录

### 2026-05-06 - 规划草稿

- 状态：进行中
- Commit：none
- 验证：已运行 `rtk proxy git status --short --branch`；已只读检查 repo routing、observability design、performance threshold policy、core/lab/web 相关文件；已运行 `rtk proxy git diff --check -- docs/plans/2026-05-06-performance-stability-observability-milestones.md`。
- Reviewer：已运行；发现已采纳，包括补充与 M35/M40 关系、修正 dirty 现场、拆分 lab/artifact carrier 与条件 core carrier、收拢 V1 总验证门。
- 风险 / 后续：当前 git dirty 只有 `.gitignore` 和本计划文件；scope 风险来自与既有 M35/M40 功能面重叠。执行前需要用户通过 Plan Gate，并在每个 execution milestone 中严格按文件所有权处理。

### 2026-05-06 - D1 Diagnostics Contract 与事实归属

- 状态：已完成
- Commit：none
- 变更文件：
  - `docs/design/performance-stability-diagnostics-contract.md`
  - `docs/design/README.md`
  - `docs/plans/2026-05-06-performance-stability-observability-milestones.md`
- 设计结论：E1 先做 lab-owned diagnostics carrier；E2 仅在 D1/E1 证明现有 facts 不足时执行最小 core read-model carrier；E3 复用现有 Web 面板并强制 source label / missing evidence 语义；V1 只产出 solver handoff，不修 solver。
- 验证：`rtk proxy git diff --check` 通过。
- 风险 / 后续：D2 仍需固定 stack repro window 与 first-bad-frame 阈值，避免 E1/E3 在实现时各自发明 marker。

### 2026-05-06 - D2 Stack 复现窗口与 first-bad-frame 口径

- 状态：已完成
- Commit：none
- 变更文件：
  - `docs/design/stack-stability-repro-diagnostics.md`
  - `docs/design/README.md`
  - `docs/plans/2026-05-06-performance-stability-observability-milestones.md`
- 设计结论：`stack_4` 是主 clean repro，`stack_stability_tower` 是 stress repro；first-bad-frame 是最早 warning/severe marker，marker payload 必须携带 source、threshold、handle attribution 和 missing evidence；debug marker 不等同于 solver correctness lock。
- 验证：`rtk proxy git diff --check` 通过。
- 风险 / 后续：E1 可以基于 D1/D2 开始 lab/artifact diagnostics carrier，但 Execution Gate 仍为待确认；不应直接进入实现。

### 2026-05-06 - E1 Lab / Artifact Diagnostics Carrier

- 状态：已完成
- Commit：none
- 变更文件：
  - `crates/picea-lab/src/artifact.rs`
  - `crates/picea-lab/src/lib.rs`
  - `crates/picea-lab/tests/artifact_run.rs`
  - `docs/plans/2026-05-06-performance-stability-observability-milestones.md`
- 实现结论：新增 lab-owned `FrameDiagnostics` carrier，并挂到 `FrameRecord`；`DebugRenderFrame` 只复制同一份 diagnostics 供 Web 消费。diagnostics 现在能表达 per-frame counter delta、penetration、contact churn、warm-start、impulse、sleep、island summary 和 marker；旧 artifact 缺字段时 source 默认 `missing`，不会被误读成 lab-derived zero fact。
- Source / missing 语义：`DiagnosticSource` 默认 `Missing`；首帧 `performance.counter_delta` 与 `contact_churn` 显式标记 `PreviousFrame` 缺失；warm-start counts / sleep transition / island counters 保留 Rust authoritative source，lab 聚合事实保留 lab-derived source。
- Reviewer：已运行两轮；第一轮阻塞 source 粒度、首帧 churn missing、projection 测试，第二轮阻塞 nested marker source default。所有阻塞点已修正，并补测试覆盖。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run` 通过，20 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack` 通过，1 passed。
  - `rtk proxy cargo fmt -- --check` 通过。
  - `rtk proxy git diff --check` 通过。
- 风险 / 后续：E1 没有触碰 `crates/picea/src/*` 或 Web。当前未证明必须执行 E2；下一步优先进入 E3，把 artifact diagnostics typed 接到 `picea-lab-web`，再由 V1 做浏览器和真实 stack run 验收。

### 2026-05-06 - E2 Core Authoritative Carrier

- 状态：已跳过
- Commit：none
- 跳过原因：E1 的 lab-owned `FrameDiagnostics` 已能用现有 `FrameRecord` / `DebugSnapshot` / `StepStats` / `WorldEvent` facts 表达当前 performance 与 stack stability 诊断；E3 未发现必须新增 core read-model carrier 的缺口。
- 边界：未触碰 `crates/picea/src/*`、solver、pipeline behavior 或 public API。
- 风险 / 后续：如果后续 solver 修复需要更精确的 contact identity、position correction 或 sleep transition 事实，应新开最小 core read-model milestone，而不是在 Web 中补推断。

### 2026-05-06 - E3 Lab-Web Performance / Stability Diagnostics Surface

- 状态：已完成
- Commit：none
- 变更文件：
  - `crates/picea-lab/web/src/types.ts`
  - `crates/picea-lab/web/src/App.tsx`
  - `crates/picea-lab/web/src/components/workbench/Timeline.tsx`
  - `crates/picea-lab/web/src/i18n.ts`
  - `crates/picea-lab/web/scripts/ui-contract.mjs`
  - `crates/picea-lab/web/scripts/i18n-contract.mjs`
- 实现结论：Web typed 消费 `FrameRecord.diagnostics`；Timeline marker rail 合并 artifact diagnostics、stack fallback、trajectory fallback；Diagnostics tab 展示 performance counter delta、penetration、contact churn、warm-start、impulse、sleep、island、markers、missing evidence；Evidence tab 与 copy-debug-context 增加 diagnostics summary。
- 兼容语义：旧 artifact / demo / partial diagnostics 不崩溃；缺少 diagnostics 时 debug context 记录 `available: false`、`marker_count: null`、`missing_evidence: ["frame_diagnostics"]`，Evidence 显示 `perf:missing missing miss:frame_diagnostics`，不再显示 `m0`。
- Reviewer：第一轮发现旧 artifact 被压成 `m0`、marker priority 未保留 D2 first-bad-frame severity、missing detail 硬编码英文、contract 不锁旧 artifact 路径；已全部修复。第二轮复审无 unresolved findings。
- 验证：
  - `cd crates/picea-lab/web && rtk proxy npm run test:i18n` 通过。
  - `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract` 通过。
  - `cd crates/picea-lab/web && rtk proxy npm run build` 通过；Vite 仍有既有 chunk > 500 kB warning。
  - `rtk proxy cargo test -p picea-lab --test artifact_run` 通过，20 passed。
  - `rtk proxy cargo fmt -- --check` 通过。
  - `rtk proxy git diff --check` 通过。
- 风险 / 后续：E3 未修改 Rust schema 或 core；正式 solver 修复仍留给后续 milestone。

### 2026-05-06 - V1 真实复现证据与浏览器验收

- 状态：已完成
- Commit：none
- Run path：`/Users/asyncrustacean/projects/picea/target/picea-lab/runs/run-1778040044555672000`
- Artifact 证据：`stack_4 --frames 120` 生成 120 帧。first warning marker 是 frame 11 `solver_row_spike`，`state_hash=11e167060e9ffde8`，`severity=warning`，`score=4`，source 为 `lab_derived`，evidence fields 为 `stats.contact_row_count`、`stats.joint_row_count`、`diagnostics.performance.counter_delta.solver_row_count`。frame 13 `state_hash=a876f3e3da897dae` 出现 `solver_row_spike` severe marker；frame 14 contact rows 增至 6、island count 降到 2、penetration total 约 0.0533。
- Browser acceptance：`rtk proxy just picea-lab-web` 临时启动到 API `http://127.0.0.1:8080`、Web `http://127.0.0.1:5173/`；`browser-use:browser` 的 `node_repl js` 执行工具未在本轮暴露，按计划降级到 Playwright。浏览器选择 `Four box stack`，运行 Rust artifact replay，点击 `solver row spike f13`，Diagnostics/Evidence/copy-debug-context 均显示 diagnostics available、marker_count 1、marker kind `solver_row_spike`、counter_delta_source `lab_derived`，console warn/error 为 0。旧 demo/no diagnostics 路径显示 `available: false` 与 `frame_diagnostics` missing，未显示 `m0`。
- 验证：
  - `rtk proxy cargo run -p picea-lab -- run stack_4 --frames 120` 通过并生成 run path。
  - `rtk proxy cargo test -p picea-lab --test artifact_run` 通过，20 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack` 通过，1 passed。
  - `cd crates/picea-lab/web && rtk proxy npm run test:i18n` 通过。
  - `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract` 通过。
  - `cd crates/picea-lab/web && rtk proxy npm run build` 通过；Vite chunk warning 未作为本 milestone 阻塞。
  - `rtk proxy cargo fmt -- --check` 通过。
  - `rtk proxy git diff --check` 通过。
- Solver handoff：下一轮应从 `stack_4` frame 11-14 的 solver row / contact churn / penetration 增长做 characterization test，再进入 contact persistence、island merge、solver row ordering 或 position correction 的最小修复；不要把本轮 diagnostics marker 当 correctness lock，也不要在 Web 中补物理推断。

### 2026-05-06 - C1 文档路由与 solver 输入收口

- 状态：已完成
- Commit：none
- 变更文件：
  - `docs/ai/index.md`
  - `docs/ai/repo-map.md`
  - `docs/ai/doc-catalog.yaml`
  - `docs/plans/2026-05-06-performance-stability-observability-milestones.md`
- 收口结论：AI 路由现在能从 performance / stack stability observability 问题定位到本计划、D1 diagnostics contract、D2 stack repro diagnostics、lab artifact carrier 和 Web diagnostics surface。repo-map 明确 `FrameDiagnostics` 是 lab/artifact diagnostics carrier，不代表 solver 已修复；Web 缺失 diagnostics 时必须显示 missing 而不是 0。
- 验证：
  - `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'` 通过。
  - `rtk proxy git diff --check` 通过。
- 风险 / 后续：下一轮 solver 修复应以 V1 的 frame 11-14 证据开局，先补 targeted characterization/failing test，再改 solver/contact/sleep 相关实现。
