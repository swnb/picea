# Matrix Stack 稳定性优化里程碑计划

状态：进行中
计划文档：docs/plans/2026-05-06-matrix-stack-stability-optimization-milestones.md
最后更新：2026-05-07
工作目录：/Users/asyncrustacean/projects/picea
工作区状态：clean at start；用户确认已提交之前代码，本轮开始前 `rtk proxy git status --short` 无输出。当前本轮改动集中在 D1/D2 设计文档、E1 行为锁/报告基线、E2 warm-start 稳定化实现与测试、E3 位置修正与负结果记录、E4 row-level provenance / shallow support friction 小步实验、E5 sleep/ejection blocker 证据，以及 live realtime 卡顿验收和方案记录。
提交策略：不提交
执行策略：完整计划批准后连续执行；若遇到 solver 行为假设失效、dirty 文件重叠、验收门过严/过松或 reviewer/verifier 阻塞，则暂停并回到计划更新。

## 目标

把当前能复现但不稳定的 `matrix_stack` 大规模矩阵堆叠，推进到可验证、可解释、逐步收敛的稳定性优化路线。

最终用户可感知结果：

- `picea-lab` 支持默认 `8x6`，并预留清晰的 `NxM` 扩展口径。
- artifact / Web 能判断堆叠是否稳定，而不是只看到“能跑”。
- 稳定性失败可以定位到 contact identity、warm-start、position correction、solver row ordering、friction 或 sleep 中的具体原因。
- 后续 solver 改动按 Matter.js / Box2D 类成熟引擎的设计思想拆成可验收小步，而不是一次性大改。

## 约束

- 计划阶段只写本计划文档，不修改 production implementation code。
- 执行阶段必须先补行为锁或失败测试，再写最小实现。
- 所有验证命令使用 `rtk proxy`。
- `picea-lab-web` 只消费 Rust / artifact facts，不在浏览器里重算物理。
- `crates/picea-lab` 负责场景、artifact、diagnostics 和验收证据；不重新实现 solver。
- `crates/picea` 优先保持 public API 稳定；solver / pipeline 内部改动必须被 behavior tests 和 artifact evidence 约束。
- 不以单次 wall-clock 作为 correctness hard fail；性能先看 deterministic counters 和基线。
- 不把 stress demo 的视觉抖动直接等同于 correctness 失败；先区分“稳定性验收场景”和“压力观测场景”。
- subagent 作为叶子 agent 使用；分派时必须明确写入：“你是叶子 agent，不允许再启动 subagent。”

## 待确认问题

### 必须确认

- 无。用户已确认当前工作区干净并批准开始；E1 前 `rtk proxy git status --short` 已确认无输出。

### 可带假设推进

- 默认 `matrix_stack` 第一阶段验收以已存在的 `8x6` stress 场景为主，`NxM` UI 参数化作为后续产品化扩展：影响 E1/V1 范围；早期验证是 artifact 仍能导出 48 dynamic bodies 和稳定性指标。
- 默认先修稳定性根因，不先做大规模 UI 配置面：影响 Web 范围；早期验证是浏览器仍可选内置 `matrix_stack` 并查看 diagnostics。
- 默认不引入第三方物理引擎依赖：影响架构边界；Matter.js / Box2D 只作为设计参考，不作为替换 core solver 的依赖。
- 默认先保持单线程 separate-phase solver contract：影响 E4 范围；早期验证是 `docs/design/solver-island-ordering-contract.md` 仍成立。
- 默认把 `Rust live session` 卡顿作为独立 product performance 问题处理，不把它混入 E3 solver correctness 修复：影响 browser 验收；早期验证是记录 live step 响应耗时、响应体大小和 degraded realtime 状态。

## 规划依据

- explorer：已运行，结果已用于收紧模块边界、dirty 风险、验证命令、`stack_4` clean gate 与 `matrix_stack 8x6` stress gate 的关系。
- 关键仓库证据：
  - `crates/picea/src/pipeline.rs` 已有固定步长 `StepConfig`，默认 `velocity_iterations=10`、`position_iterations=20`、`restitution_velocity_threshold=1.0`、`enable_sleep=true`。
  - `crates/picea/src/solver/contact.rs` 已有 sequential impulse velocity solve、warm-start impulse application、Coulomb friction、velocity-level position bias，以及 residual position correction。
  - `crates/picea/src/pipeline/contacts.rs` 已有 `ContactKey` / `ContactPairKey` / `ContactRecord` 持久化、warm-start transfer、normal mismatch / point drift drop reason。
  - `crates/picea/src/pipeline/sleep.rs` 已有 island-level sleep window，但当前 matrix stack 验收中 180 帧后仍可出现 `48 awake / 0 sleeping`，说明稳定性噪声还在打断收敛。
  - `crates/picea-lab/src/artifact.rs` 与 `docs/design/performance-stability-diagnostics-contract.md` 已有 `FrameDiagnostics` 路线，可观测 contact churn、penetration、warm-start、sleep、island、solver row spike；本计划不重做 diagnostics/Web 归因，而是在这些证据基础上进入 solver 稳定性优化。
  - `matrix_stack` 当前 180 帧观测曾出现 `contact_churn_spike`、`solver_row_spike`、`performance_counter_spike`，最大穿透深度约等于单个箱体高度，最终仍未睡眠。
  - `docs/design/solver-island-ordering-contract.md` 明确当前单线程 solver contract 是 contact rows 与 joint rows 分 phase，contact gathering order 保持在 island 内。
- 外部参考依据：
  - Box2D 把 contact constraint 作为自动生成的约束，solver 是 O(N) sequential solver，并强调 speculative collision / TOI 处理高速穿透。
  - Box2D 的 simulation 文档把 persistent contact pair 作为 island graph 和 contact manifold 的载体，并强调 contact points 要在 step 开始、solver 前计算，否则物体会下沉。
  - Box2D 的 sleep 设计是 body/group come to rest 后进入低成本 sleep，接触或 joint 变化会唤醒。
  - Matter.js 把碰撞求解拆成 position iterations、constraint iterations、velocity iterations，并把 sleeping 作为可提升稳定性和性能但可能牺牲精度的选项。
  - Matter.js constraint 文档在不稳定时建议降低 stiffness 或增加 `engine.constraintIterations`，这对应 picea 里“先控制约束刚性/修正幅度，再增加迭代”的取舍。
- 主要未知：
  - 当前 churn 的主因是 feature id 不稳定、接触点 reduce 策略、normal flip、position correction 改变 anchor，还是矩阵初始间距/微旋转导致的压力传递。
  - 当前最大穿透来自 residual correction 不足、position bias 过弱/过强、接触生成时机、还是 solver row ordering 在高压力堆叠下收敛慢。
  - sleep 不收敛到底是可见速度仍大，还是 correction/warm-start drop/contact churn 持续重置 idle window。
  - `position_iterations=20` 已经不低，继续提高迭代可能只是掩盖接触持久化和位置修正问题，需要先用验收数据判断。

## 计划验收

- 状态：已批准
- Design Gate：已确认；用户在 2026-05-06 回复“开始吧，就在当前工作区，我已经提交了之前的代码，现在工作区是干净的”。
- Execution Gate：已确认；E1 开始前 `rtk proxy git status --short` 无输出。
- reviewer：已运行
- 审查结论：有发现，已采纳修订：把 dirty overlap 升级为 Execution Gate 前置条件；固定 `stack_4` clean behavior lock 与 `matrix_stack 8x6` dense stress gate 的关系；补充浏览器验收启动命令和 fallback；清理 `E6` 残留引用；明确 Matter.js / Box2D 参考边界和新旧设计文档关系。
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
| D1 | Matter.js / Box2D 参考设计映射、picea 稳定性策略边界 | E2, E3, E4, E5 | 已完成 |
| D2 | matrix_stack 稳定性验收门、debug/stress 场景区分、阈值和报告模板 | E1, V1, C1 | 已完成 |

## 里程碑

### D1：主流引擎稳定性策略映射

类型：设计
状态：已完成

目标：
把 Matter.js / Box2D 中对堆叠稳定有直接价值的机制，映射成 picea 可执行的内部设计边界和取舍顺序。

要回答的问题：
- Box2D 风格 persistent contact / manifold / contact id 对 picea 现有 `ContactKey`、`ContactFeatureId`、`ContactRecord` 应如何校准。
- Matter.js position / velocity / constraint iteration 分层，对 picea 当前 velocity solve + residual position correction 的启发是什么。
- Box2D 的 sleep group / island 思路，如何映射到 `pipeline/sleep.rs` 的 island sleep window。
- 哪些机制应作为行为修复，哪些只能作为调参或 debug 选项。

为什么现在做：
当前 picea 已经有 sequential impulse、warm-start、sleep 和 diagnostics，但 matrix stack 仍不稳定。先做映射可以避免盲目增加迭代次数，也避免把成熟引擎中的多个互相配合的策略拆错顺序。

设计范围：
- `crates/picea/src/pipeline/contacts.rs`
- `crates/picea/src/solver/contact.rs`
- `crates/picea/src/pipeline/sleep.rs`
- `crates/picea/src/pipeline/island.rs`
- `docs/design/solver-island-ordering-contract.md`
- Box2D / Matter.js 文档和源码级 API 文档的设计摘录

不做：
- 不改生产实现代码
- 不引入 Matter.js / Box2D 依赖
- 不重写 solver 架构
- 不改变 public API

输入证据：
- 当前 matrix stack artifact / browser 验收数据。
- 现有 warm-start reason、contact churn、penetration、sleep diagnostics。
- Box2D documentation: solver、contact constraint、persistent contact pair、manifold、sleep、fixed step / unit tuning。
- Matter.js docs: `positionIterations`、`velocityIterations`、`constraintIterations`、`enableSleeping`、Sleeping thresholds、constraint stability advice。

输出交付物：
- `docs/design/matrix-stack-stability-optimization-design.md`。
- “机制 -> picea 模块 -> 验收指标 -> 风险”的映射表。
- 参考边界：采用 persistent contacts / manifolds、warm-started sequential impulses、分层 position/velocity solve、island sleep 和固定步长调优思想；暂不采用外部引擎依赖、全面 XPBD/PBD 重写、多线程 island solver 或浏览器侧 physics 计算。
- E2-E5 的执行输入更新。
- 明确不采用或暂缓采用的方案，例如单纯提高迭代、引入第三方引擎、全面 XPBD 重写。

设计验收：
- 每个后续执行里程碑都能追溯到一个明确设计输入。
- 明确接触持久化、位置修正、solver order、sleep 的优先级。
- 明确哪些指标是 hard behavior lock，哪些只是 diagnostics marker。

验证方式：
- 文档检查。
- reviewer 审查。
- `rtk proxy git diff --check`

下一步映射：
- 生成/更新执行里程碑：E2, E3, E4, E5。

风险 / 后续：
- Box2D / Matter.js 的实现细节不能照搬；picea 当前 public surface 和 solver contract 更窄，需要选择小步迁移。

### D2：Matrix Stack 稳定性验收门

类型：设计
状态：已完成

目标：
冻结 `matrix_stack` 的验收定义，区分“稳定性行为锁”和“压力观测指标”，给后续 solver 修复提供明确红绿灯。

要回答的问题：
- `stack_4` 作为第一版 clean behavior lock 的具体 frame window 和阈值是什么。
- `matrix_stack 8x6` 作为 dense stress gate 的 warning / behavior 指标是什么。
- 多久算收敛：180、300、600 还是更长 frame window。
- 哪些阈值进入 hard gate：最大穿透、穿透总和、contact churn、warm-start hit ratio、sleep/低速收敛、state hash determinism。
- 哪些阈值只作为 warning：solver row spike、performance counter spike、全部睡眠前的短期 churn。

为什么现在做：
没有验收门，后续每个 solver 改动都可能只是在视觉上“好像稳了”。先冻结分层验收，可以让每个执行里程碑知道自己改善了哪个维度。

设计范围：
- `crates/picea-lab/src/scenario.rs` 的 `matrix_stack_fixture`
- `crates/picea-lab/tests/artifact_run.rs`
- `crates/picea/tests/physics_realism_acceptance.rs`
- `docs/design/stack-stability-repro-diagnostics.md`
- 当前 artifact diagnostics markers

不做：
- 不修改场景或测试实现。
- 不修 solver。
- 不把 `8x6` stress 结果直接变成第一步必须全绿的 correctness gate。

输入证据：
- 当前 `matrix_stack` 能导出 48 dynamic bodies、1 static floor。
- 当前 180 帧仍可能出现 contact churn / row spike / penetration / no sleeping。
- 已有 diagnostics 能给出 contact count、contact_row_count、penetration、churn、warm-start、sleep summary。

输出交付物：
- `docs/design/matrix-stack-stability-acceptance.md`。
- 与现有 `docs/design/stack-stability-repro-diagnostics.md` 的关系：新文档是 solver 优化验收补充，不覆盖既有 performance/stability diagnostics 复现文档；若发现旧文档阈值或术语冲突，D2 只记录差异，不直接改旧文档。
- 验收分层：smoke gate、behavior lock gate、stress warning gate、browser acceptance。
- 场景分层：`stack_4` 作为 clean behavior lock；`matrix_stack 8x6` 作为 dense stress gate；浏览器验收只证明用户可观察证据链一致。
- `stack_4` 默认窗口：120 帧 smoke，300 帧 behavior lock；D2 可调整具体阈值，但必须在设计文档中固定后才能进入 E1。
- `matrix_stack 8x6` 默认窗口：180 帧 stress report，必要时增加 600 帧 long-settle observation；第一阶段不要求全绿睡眠。
- artifact run 报告模板和 first-bad-frame 字段。
- E1/V1/C1 的验收口径更新。

设计验收：
- 后续执行者能直接写出 failing tests，而不用重新猜阈值。
- 阈值说明能避免把压力场景误判为产品失败。
- 每个 hard gate 都有可运行命令或 browser 检查。

验证方式：
- 文档检查。
- reviewer 审查。
- `rtk proxy git diff --check`

下一步映射：
- 生成/更新执行里程碑：E1, V1, C1。

风险 / 后续：
- 验收门过早过严会阻塞小步优化；第一版固定 `stack_4` hard gate，`matrix_stack 8x6` 做 stress gate，等 E2-E5 后再提升。

### E1：Matrix Stack 行为锁与报告基线

类型：执行
状态：已完成
来源设计：D2

目标：
先把当前不稳定行为用 red/diagnostic tests 和 artifact report 锁住，确保后续优化有可比较的起点。

执行输入：
- D2 的 smoke / behavior / stress gate。
- 允许修改 `crates/picea-lab/tests/artifact_run.rs`、`crates/picea/tests/physics_realism_acceptance.rs`、必要的 test helper。
- 保持现有 `matrix_stack` 场景语义不变，除非 D2 明确要求拆出小规模 gate。
- E1 开始前主 Codex 必须重新检查 dirty overlap；若目标文件已有未归属改动，暂停并要求集成/所有权确认。

为什么现在做：
稳定性优化容易出现“这一帧好了，另一帧坏了”的回归。先有报告基线和小规模 hard gate，后续改 solver 才不会靠主观截图验收。

范围：
- 增加 `matrix_stack` 报告基线测试。
- 增加 `stack_4` clean acceptance 的穿透/睡眠/低速收敛锁；不在 E1 现场改成别的小矩阵，除非 D2 设计文档已明确修订。
- 保留 `8x6` stress marker，不急着让它全绿。
- 把 artifact report 输出字段固定成后续 solver handoff 输入。
- 复用现有 `FrameDiagnostics` / Web diagnostics，不新增平行 diagnostics schema。

不做：
- 不修 contact / solver / sleep。
- 不新增 UI 配置面。
- 不重做 performance/stability observability 计划中已完成的 artifact/Web 归因面。
- 不把性能 wall-clock 作为 hard fail。

所有权：
- 可能触及：`crates/picea-lab/tests/artifact_run.rs`、`crates/picea/tests/physics_realism_acceptance.rs`、测试 fixture helper。
- 不应触及：`crates/picea/src/solver/contact.rs`、`crates/picea/src/pipeline/contacts.rs`、`crates/picea/src/pipeline/sleep.rs`、Web UI。
- 前置条件：目标文件 dirty 来源已确认；若当前 diagnostics / matrix_stack 改动仍未集成，worker 只能在明确 diff 上追加，不得格式化或重写整文件。

验收标准：
- `stack_4` 至少有一个明确 hard gate。
- `8x6` stress report 能输出 first-bad-frame、penetration/churn/warm-start/sleep 摘要。
- 当前不稳定问题能被测试或 report 可重复暴露。

验证方式：
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- `rtk proxy cargo run -p picea-lab -- run stack_4 --frames 300`
- `rtk proxy cargo run -p picea-lab -- run matrix_stack --frames 180`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：不需要；D2 已给出验收输入。
- worker：使用 `gpt-5.4`，只写测试 / 报告基线相关文件。
- reviewer：检查阈值是否过严、是否误伤 stress gate、是否覆盖当前 failure。
- verifier：运行上述 Rust 验证命令，并报告生成或修改的 artifact 文件。

提交策略：
- auto-commit：否
- message hint：matrix stack stability acceptance baseline

风险 / 后续：
- 如果当前实现无法稳定通过任何小规模 gate，暂停并回到 D2 降低第一阶段 hard gate，先保留 diagnostic lock。

### E2：Contact Persistence 与 Warm-Start 稳定化

类型：执行
状态：已完成
来源设计：D1, D2

目标：
降低 matrix stack 中的 contact churn 和 warm-start drop，让接触点、manifold 和冲量缓存跨帧更稳定。

执行输入：
- D1 的 contact identity / manifold 策略映射。
- D2/E1 的 churn、warm-start、penetration 基线。
- 允许修改 contact gathering / refresh / warm-start transfer 的内部实现。

为什么现在做：
Box2D 类稳定堆叠依赖 persistent contact pair / manifold 作为 solver 和 island 的稳定输入。picea 现有 warm-start 已存在，但 matrix stack 仍出现 churn，说明第一刀应先让约束身份稳定，而不是直接调大迭代次数。

范围：
- 审查并改进 `ContactFeatureId` 对矩形/凸多边形边-点、边-边接触的稳定性。
- 审查 contact reduction 是否在相邻帧选择不同点导致 key 抖动。
- 调整 warm-start drop 策略时保持“宁可 miss，不错误转移冲量”的安全边界。
- 增加 contact churn / warm-start hit ratio 行为锁。

不做：
- 不改 position correction 算法。
- 不改 sleep 策略。
- 不改 public API。
- 不做 generic tracing 框架。

所有权：
- 可能触及：`crates/picea/src/pipeline/narrowphase.rs`、`crates/picea/src/pipeline/contacts.rs`、`crates/picea/src/world/contact_state.rs`、相关 tests。
- 不应触及：Web UI、server 路由、public crate root、unrelated lab docs。

验收标准：
- 小规模 stack 的 contact churn 在 settling window 后下降。
- warm-start hit ratio 在持续接触窗口内上升，drop reason 可解释。
- `8x6` stress 的 first-bad-frame 至少不再由明显 feature-id churn 主导。
- 既有 contact lifecycle / warm-start tests 不回归。
- 动态摩擦接触必须证明“切向小滑移可继承缓存切向冲量，切向大滑移必须丢弃缓存冲量”，避免只用 static-static 用例证明放宽阈值。

验证方式：
- `rtk proxy cargo test -p picea --lib pipeline::narrowphase`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack`
- `rtk proxy cargo test -p picea --lib --tests`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要，只读定位 feature id / reduction / warm-start 当前失败证据。
- worker：使用 `gpt-5.4`；写入范围限定在 contact/narrowphase/contact_state/tests。
- reviewer：重点审查错误冲量转移、key collision、normal flip、sensor/CCD contact 兼容。
- verifier：运行上述 core/lab 验证命令。

提交策略：
- auto-commit：否
- message hint：stabilize contact persistence for matrix stack

风险 / 后续：
- 过度复用旧 impulse 会制造假稳定或能量注入；必须保留 normal mismatch、invalid impulse、point drift 保护。

### E3：位置修正与穿透控制

类型：执行
状态：部分完成；仍需 E4 收敛输入
来源设计：D1, D2

目标：
把 matrix stack 的最大穿透和穿透总和压到 D2 定义的范围内，同时减少 position correction 对 sleep 和 contact identity 的扰动。

执行输入：
- D1 的 position iteration / correction 策略映射。
- E1/E2 的 penetration 与 churn 基线。
- 当前 `solver/contact.rs` 的 velocity-level position bias 和 residual position correction。

为什么现在做：
当前最大穿透接近单个箱体高度，说明 solver 不只是“慢慢收敛”，而是有穿透积累或修正噪声问题。Box2D / Matter.js 都把位置层和速度层分开处理，picea 需要先把 residual correction 变得有界、可解释、少扰动。

范围：
- 给 residual position correction 增加统计或测试可观察事实，必要时先通过 lab/core read model 暴露。
- 评估 correction percent、slop、per-iteration clamp、mass split、睡眠体唤醒条件。
- 尝试 split-impulse 风格或更保守的位置修正策略，但保持小步实验和回滚清晰。
- 增加穿透 hard gate。

不做：
- 不重写成完整 XPBD。
- 不改变 public API。
- 不扩大到 joint solver。
- 不用单纯提高 `position_iterations` 作为最终修复。

所有权：
- 可能触及：`crates/picea/src/solver/contact.rs`、`crates/picea/src/pipeline.rs` 的 stats/read-model additive 字段、相关 tests。
- 不应触及：narrowphase feature id，除非 E2 结论要求回退；Web UI 大改。

验收标准：
- 小规模 stack 的 penetration max / sum 达到 D2 hard gate。
- `8x6` stress 的最大穿透显著下降，且不引入 numeric warnings。
- correction 不持续重置 sleep idle window，或能够通过 diagnostics 解释 reset reason。
- 既有 CCD / stack / sleep tests 不回归。

验证方式：
- `rtk proxy just picea-lab-web-stop`
- `rtk proxy just picea-lab-web-start`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack`
- `rtk proxy cargo test -p picea --lib --tests`
- `rtk proxy cargo run -p picea-lab -- run matrix_stack --frames 180`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要，只读确认穿透来自 bias、residual correction、row order 还是 contact churn。
- worker：使用 `gpt-5.4`；写入范围限定在 contact solver 和必要 tests。
- reviewer：重点审查能量注入、sleep reset、static/dynamic 修正方向、iteration-dependent nondeterminism。
- verifier：运行上述 core/lab 验证命令。

提交策略：
- auto-commit：否
- message hint：bound matrix stack penetration correction

风险 / 后续：
- 位置修正过强会让堆叠“弹开”或持续唤醒；过弱则穿透积累。该里程碑必须以 diagnostics 数据而不是视觉印象调参。

### E4：Solver Row Ordering 与迭代调度

类型：执行
状态：进行中
来源设计：D1, D2

目标：
在不破坏现有 solver island ordering contract 的前提下，改善大堆叠的约束收敛速度和确定性。

执行输入：
- D1 的 sequential solver / iteration scheduling 设计结论。
- `docs/design/solver-island-ordering-contract.md`。
- E2/E3 后仍存在的 row spike 或收敛慢证据。

为什么现在做：
当 contact identity 和 penetration 控制基本稳定后，剩余问题才适合看 row ordering / iteration schedule。否则调度优化会掩盖根因。

范围：
- 评估 island 内 contact row ordering 是否需要稳定的 bottom-up 或 pressure-aware 排序。
- 评估 velocity / position iteration 是否需要 scenario/debug 可配置，而不是全局盲目提高。
- 保持 deterministic ordering，新增测试锁住 state hash 或 stable ordering。
- 只在 E2/E3 后仍有证据时执行。

不做：
- 不引入多线程 solver。
- 不把 contact/joint rows 合并成 unified stream。
- 不改变 public API。
- 不把性能阈值变成单机 wall-clock hard fail。

所有权：
- 可能触及：`crates/picea/src/pipeline/island.rs`、`crates/picea/src/solver/contact.rs`、`crates/picea/src/pipeline.rs` config/tests。
- 不应触及：broadphase、Web UI、scene authoring。

验收标准：
- 大矩阵 stack 的 solver row spike 可解释且不恶化。
- 相同输入下 state hash deterministic。
- 小规模 stack 不需要过高 iteration 也能通过 D2 hard gate。
- 既有 island / joint ordering tests 不回归。

验证方式：
- `rtk proxy cargo test -p picea --lib pipeline::island`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack`
- `rtk proxy cargo test -p picea --lib --tests`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要，只读比较 row ordering 与 first-bad-frame 的关系。
- worker：使用 `gpt-5.4`；若涉及 solver contract 更新，先暂停回到设计。
- reviewer：重点审查 determinism、ordering contract、joint/contact phase 兼容。
- verifier：运行上述验证命令。

提交策略：
- auto-commit：否
- message hint：stabilize matrix stack solver ordering

风险 / 后续：
- row ordering 会改变物理结果，必须有明确回滚点；如果需要改变 `solver-island-ordering-contract.md`，暂停并新增设计里程碑。

### E5：Sleep 收敛与静息状态稳定

类型：执行
状态：部分完成；等待 E4 降低残余速度后继续
来源设计：D1, D2

目标：
让已经低速且接触稳定的堆叠能够进入 sleep 或至少进入 D2 定义的低速 quiet window，避免 180 帧后仍全部 awake。

执行输入：
- D1 的 sleep group / island strategy 映射。
- E2/E3/E4 后的 contact/churn/penetration/row ordering 结果。
- 当前 `pipeline/sleep.rs` 的 island sleep window 和 wake reason。

为什么现在做：
sleep 是稳定堆叠的结果指标之一，但不能先于 contact 和 correction 修复。否则可能只是把还在抖的系统强行冻住。

范围：
- 增加 sleep blocker diagnostics 或 tests，区分速度过大、contact correction、impact wake、contact churn。
- 调整 sleep idle reset / wake reason 策略，避免微小 correction 无限重置。
- 锁定 island-level sleep transition 行为。

不做：
- 不用强制 sleep 掩盖穿透或速度问题。
- 不让 static body 进入错误 sleep 状态。
- 不改变 public API。

所有权：
- 可能触及：`crates/picea/src/pipeline/sleep.rs`、`crates/picea/src/body.rs`、`crates/picea/src/events.rs` 的 additive reason、相关 tests。
- 不应触及：Web UI 大改、contact feature id、broadphase。

验收标准：
- 小规模 stack 在 D2 window 内进入 sleep 或 quiet window。
- `8x6` stress 至少能解释 sleep blocker reason，不再只有 `48 awake / 0 sleeping` 的黑盒结论。
- 既有 sleep acceptance tests 不回归。

验证方式：
- `rtk proxy cargo test -p picea --lib pipeline::sleep`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要，只读确认 sleep blocker evidence。
- worker：使用 `gpt-5.4`；写入范围限定在 sleep/body/events/tests 的最小 additive 改动。
- reviewer：重点审查 false sleep、wake reason 准确性、sleeping island row skip 兼容。
- verifier：运行上述验证命令。

提交策略：
- auto-commit：否
- message hint：improve matrix stack sleep convergence

风险 / 后续：
- 过早 sleep 会把错误状态冻结；该里程碑必须依赖 E2-E4 的稳定性改善结果。

### V1：端到端矩阵堆叠验收

类型：验证
状态：计划中

目标：
用 artifact、core tests、lab-web 浏览器验收确认 matrix stack 优化是否真正稳定，并输出剩余问题清单。

范围：
- 运行 D2 定义的 small hard gate、`8x6` stress gate、browser replay。
- 对比优化前后的 penetration、churn、warm-start、sleep、solver rows、state hash。
- 确认 Web diagnostics 与 artifact story 一致。

不做：
- 不再追加实现修复。
- 不扩大到 soft body、compound、CCD 全场景。

验收标准：
- D2 hard gates 全部通过。
- `8x6` stress gate 达到 D2 warning/behavior 目标。
- 浏览器可选 `matrix_stack`，diagnostics 无 missing 误报，console 无 error/warn。
- 输出一份简短 solver handoff / residual risk 报告。

验证方式：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack`
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy cargo test -p picea --lib --tests`
- `rtk proxy cargo test -p picea-lab --lib`
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- `browser-use:browser` 在 `http://127.0.0.1:5173/` 选择并运行 `矩阵堆叠 8x6`。
- 如果 `browser-use:browser` 不可用，降级为 Playwright CLI 或手动浏览器检查，并记录降级原因。
- verifier 必须回报：`stack_4` run path、`matrix_stack` run path、浏览器 URL、console error/warn 状态、diagnostics 是否显示 missing evidence。

风险 / 后续：
- 如果 `8x6` 仍无法达到 hard stable，但 small gates 已稳定，应把 `8x6` 保留为下一阶段 pressure solver 计划，而不是扩大本计划。

### L1：Live Realtime 性能方案

类型：设计 / 后续执行输入
状态：计划中

目标：
解决 `matrix_stack 8x6` 在 `Rust live session` 下实时运行卡顿的问题，让 live playback 不再把完整 debug snapshot 的生成、序列化、传输和前端全量消费压进单帧预算。

当前浏览器验收结论：
- `browser-use:browser` iab 后端因当前会话未暴露 Node REPL `js` / `mcp__node_repl__js`，降级为 `agent-browser` 验收。
- 8x6 live 120 步期间浏览器曾采到 long task；headless rAF 基线接近 30Hz，因此 rAF 间隔仅作参考，long task 与 API 耗时更可信。
- 直接从浏览器绕过 React 调用 live step API：小场景 `falling_box_contact` 前 20 步平均约 `30.5ms`、响应约 `7.4KB`；`matrix_stack 8x6` 前 20 步平均约 `60.9ms`、p95 约 `92.4ms`、响应均值约 `280KB`。
- 一个低风险 clone 优化已尝试：`step_live_session` 从“两次 clone 完整 FrameRecord”收敛到“一次历史帧 clone + latest_frame move”。测试通过，但浏览器复测未见可感知改善，说明主要瓶颈是完整 FrameRecord export/serialization/transport/UI consumption，而不是单个 clone。

设计方向：
- `control step` 支持 `detail=summary|full` 或新增 live summary response；live playback 默认 summary。
- summary 只返回 canvas 所需 body/collider transform、state hash、少量 counters 和 degraded 状态。
- full `FrameRecord` 改为按需获取：暂停、scrub、打开 Diagnostics/Evidence、copy debug context 或选择实体时再请求。
- 支持 server-side batch step 或 adaptive cadence，避免 30Hz interval 文案和实际响应能力不一致。
- artifact replay 继续使用 full frame，不改变本计划的 correctness / diagnostics 验收。

不做：
- 不在浏览器重新计算 physics。
- 不在本 E3 里重构 live 协议。
- 不用降低物理质量或减少 solver diagnostics 来伪装实时稳定。

验收标准：
- 同一 `matrix_stack 8x6` live playback 中，summary step p95 进入 30Hz 或明确显示 degraded cadence。
- 打开 Diagnostics/Evidence 后能补全 authoritative full frame。
- artifact replay 和现有 tests 不回归。

验证方式：
- `rtk proxy cargo test -p picea-lab --test server_routes live -- --nocapture`
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- browser acceptance：在 `http://127.0.0.1:5173/` 选择 `Rust live session` + `Matrix stack 8x6`，记录 API latency、response size、long task 与 UI degraded 状态。

### C1：文档与验收收尾

类型：收尾
状态：计划中

目标：
把优化结果、剩余风险和后续方向写回计划 / 设计文档，保证后续会话能接着做。

范围：
- 更新本计划进度记录。
- 更新 D1/D2 输出设计文档的最终结论。
- 如 repo 路由需要，更新 `docs/ai/repo-map.md` / `docs/ai/index.md` 的新验证入口。
- 给出下一阶段建议：更大 NxM、UI 参数化、bench threshold、parallel island solver 或 XPBD RFC。

不做：
- 不追加新 solver 功能。
- 不提交，除非用户另行明确要求。

验收标准：
- 文档能说明最终稳定性状态：已稳定 / 部分稳定 / 仍不稳定及原因。
- 所有验证命令和浏览器验收结果记录完整。
- 剩余风险不藏在实现细节里。

验证方式：
- `rtk proxy git diff --check`
- 必要时运行相关 doc/catalog validator。

风险 / 后续：
- E2 只处理 warm-start 的 contact-anchor drift 分解：法向漂移仍使用严格阈值，切向同面小滑移允许更宽窗口；`matrix_stack` stress 仍不稳定，E3 需要继续处理 position correction / penetration 积累。
- 如果后续发现切向缓存转移导致错误冲量，C1 应记录被废弃的方案和原因，并回退到更保守的阈值或按 contact kind 区分阈值。

## 进度记录

### 2026-05-06 - 计划草案

- 状态：进行中
- Commit：none
- 验证：已完成只读 repo 证据收集、外部参考资料查询；等待 explorer / reviewer 计划门。
- 风险 / 后续：计划批准前不执行实现；下一步是 reviewer 审查计划文档并根据发现修订。

### 2026-05-06 - D1/D2 设计里程碑

- 状态：已完成
- Commit：none
- 验证：`rtk proxy git diff --check` 通过
- 产出：`docs/design/matrix-stack-stability-optimization-design.md`、`docs/design/matrix-stack-stability-acceptance.md`
- 风险 / 后续：进入 E1 前重新检查 dirty overlap；E1 只能写测试/报告基线，不修 solver。

### 2026-05-06 - E1 行为锁与报告基线

- 状态：已完成
- Commit：none
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，1 passed / 1 ignored。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_stress_report_repeats_the_current_first_bad_frame_story -- --ignored --nocapture`：通过。
  - `rtk proxy git diff --check`：通过。
- 产出：`stack_4` clean behavior lock；`matrix_stack 8x6` stress report baseline，first bad frame 为 2，marker 为 `SolverRowSpike:Warning` / `ContactChurnSpike:Severe` / `PerformanceCounterSpike:Severe`。
- Reviewer/Verifier：初审发现 3 个 P2，已修复；复审无 P1/P2。
- 风险 / 后续：`matrix_stack` 仍明显不稳定；E2 进入 contact identity / warm-start 根因分析和修复。

### 2026-05-06 - E2 Contact Persistence 与 Warm-Start

- 状态：已完成
- Commit：none
- 验证：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture`：通过，13 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea --lib pipeline::narrowphase`：通过，14 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，1 passed / 1 ignored。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_stress_report_repeats_the_current_first_bad_frame_story -- --ignored --nocapture`：通过。
  - `rtk proxy cargo test -p picea --lib --tests`：通过，lib 67 passed，integration tests 全部通过。
  - `rtk proxy cargo fmt`：通过。
  - `rtk proxy git diff --check`：通过。
- 产出：
  - `contacts.rs` 将 warm-start anchor drift 分成法向漂移与切向漂移：法向仍沿用 `0.05` 保守阈值，切向同面滑移允许到 `0.10`，避免 box/box 同面接触和动态摩擦接触因为合理切向滑移丢失缓存冲量。
  - 新增 static same-face、dynamic friction 小滑移/大滑移 warm-start 行为锁，确保放宽切向阈值不会错误暴露 stale tangent impulse。
  - 持久化观测中 `matrix_stack 180` 的 warm-start 总量从 `hit=3191 / dropped_point_drift=2079 / miss_feature_id=1743` 改善到 `hit=4322 / dropped_point_drift=1564 / miss_feature_id=1641`；但 stress final report 仍显示 first bad frame 为 2，最终 `48 awake / 0 sleeping`，说明整体堆叠尚未稳定。
- Reviewer/Verifier：初审发现动态摩擦用例不足与计划状态滞后，已补动态切向冲量边界测试并更新本记录；验证命令全部通过。
- 风险 / 后续：E2 证明了 warm-start 的切向小滑移缓存可安全转移，但 `matrix_stack` 的最大穿透、contact churn 与 sleep 收敛仍未达稳定目标；下一步进入 E3 position correction / penetration 积累。

### 2026-05-06 - E3 初探：Velocity Bias 扩展实验

- 状态：已暂停，未保留实现代码
- Commit：none
- 验证：
  - 尝试把 velocity-level position bias 从“近似静息”扩展到“闭合或近似静息”接触，并补慢速闭合重叠测试；该方向可以让单个 overlap 更快外推，但会让 `stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 的 settled penetration 从 D2 gate 内回归到约 `0.074-0.087`。
  - 已回滚该实验实现与测试，随后 `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture` 重新通过。
- 结论：E3 不应优先扩大 velocity bias；下一刀应转向 residual position correction 的深度刷新、修正分配、per-iteration clamp 或可观察统计，避免给堆叠注入过强的修正速度。
- 风险 / 后续：保留这个负结果在计划里，避免后续重复把全局 velocity bias 当作低风险调参。

### 2026-05-06 - E3 初探：Angular Position Correction 实验

- 状态：已暂停，未保留实现代码
- Commit：none
- 验证：
  - 新增并保留 `contact_position_correction_does_not_double_apply_face_manifold_points`，确认 clipped face manifold 的单帧 residual correction 没有明显重复按点超量位移。
  - 尝试让 residual position correction 按接触点力臂写入角度位置修正；该方向能让 tilted single-point floor contact 的角度向低穿透方向变化。
  - 但全量、single-point-only、低比例、极小 angular clamp 等版本都会让 `stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 的 settled penetration hard gate 回归，观测值约 `0.060-0.084`。
  - 已回滚 angular correction 实现和对应红灯测试，随后 `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture` 重新通过。
- 结论：当前 solver 不能把 angular positional correction 当作局部小修直接加入 residual phase；它需要更完整的 position row solve、接触刷新或按 manifold/iteration 控制，否则会破坏 clean stack gate。
- 风险 / 后续：E3 下一步应先补 position-correction 可观察统计，区分 pre-correction contact depth 与 correction 后几何状态，再决定是否设计完整 position row solver。

### 2026-05-06 - Browser 验收：Matrix stack diagnostics

- 状态：已完成
- Commit：none
- 工具：优先尝试 `browser-use:browser` 的 iab backend；当前会话没有暴露 Node REPL `js` / `mcp__node_repl__js` 工具，因此改用 `agent-browser` 兜底验收，并记录该限制。
- 验证：
  - 打开 `http://127.0.0.1:5173/`，页面标题为 `Picea Lab Workbench`。
  - 在 `Select scenario` 中选择 `Matrix stack 8x6`，点击 `Run selected scenario`。
  - 页面显示 `Bodies 49`，即 1 个 static floor + 48 个 dynamic boxes。
  - Timeline 暴露 `contact churn spike f2`、`counter spike f2`、`CCD hit f2`、`CCD clamp f2`、`solver rows f30`、`contact spike f66`、`drift f67` 等 marker。
  - Diagnostics 在 f2 显示 `state hash 8c676885fcb6f49f`、`contact_count 8`、`solver_row_count 8`、`warm-start hit_count 0 miss_count 8 drop_count 0`、`awake_dynamic_body_count 48 sleeping_dynamic_body_count 0`。
  - f2 markers 与 artifact stress report 一致：`solver row spike Warning`、`contact churn spike Severe`、`counter spike Severe`。
  - 浏览器截图：`target/picea-lab/browser-matrix-stack-diagnostics.png`。
- 结论：浏览器侧验收链路可观察并复现当前“不稳定但已可诊断”的状态；稳定性优化下一步仍是 E3。

### 2026-05-06 - Browser 验收：Live realtime 卡顿

- 状态：已完成，转入 L1 后续方案
- Commit：none
- 工具：按 `browser-use:browser` 要求优先查找 Node REPL `js` / `mcp__node_repl__js`；当前会话未暴露可用 JS 执行工具，因此降级为 `agent-browser` 验收。
- 验证：
  - `Rust live session` + `Matrix stack 8x6` 真实运行可复现用户反馈的卡顿；120 步期间浏览器采到 long task。
  - 从浏览器直接调用 live step API，绕开 React 后仍能看到规模相关耗时：小场景 `falling_box_contact` 前 20 步平均约 `30.5ms`、响应约 `7.4KB`；`matrix_stack 8x6` 前 20 步平均约 `60.9ms`、p95 约 `92.4ms`、响应均值约 `280KB`。
  - 已尝试低风险 server 端 clone 优化并重启服务复测；`rtk proxy cargo test -p picea-lab --test server_routes live -- --nocapture` 通过，`rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture` 通过，但浏览器 API 延迟未见稳定改善。
- 结论：
  - 当前 live 卡顿不是简单前端 interval 问题，也不是单个 clone 热点；根因是 live step 同步返回完整 `FrameRecord`，把 debug snapshot export / JSON serialization / transport / React full-frame consumption 串进单步预算。
  - 需要 L1 live realtime 性能方案：summary live frame、按需 full frame、adaptive cadence / degraded 状态；不应混入 E3 solver correctness 修复。

### 2026-05-06 - 当前稳定性线收尾检查

- 状态：已收口，等待下一轮 E3 实现决策
- Commit：none
- 当前边界：
  - E1 行为锁与 `matrix_stack 8x6` stress report 已完成。
  - E2 contact persistence / warm-start 已完成，保留切向小滑移缓存转移实现与动态摩擦边界测试。
  - E3 只保留通过的 regression lock `contact_position_correction_does_not_double_apply_face_manifold_points`；velocity bias 扩展和 angular residual correction 两条实验均因 `stack_4` hard gate 回归而回滚。
  - L1 live realtime 卡顿只作为后续方案记录，不继续进入当前 solver 稳定性收尾。
- 验证：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep -- --nocapture`：通过，7 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture`：通过，13 passed。
  - `rtk proxy cargo test -p picea --lib --tests`：通过，core lib / integration gates 全部通过。
  - `rtk proxy cargo test -p picea-lab --test server_routes live -- --nocapture`：通过，9 passed / 1 filtered。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，1 passed / 1 ignored；stress report 仍为 first bad frame 2，最终 `48 awake / 0 sleeping`。
- 结论：
  - 本轮可接受的收尾状态是“small behavior locks + warm-start 改善已稳定；`matrix_stack 8x6` 仍是 stress failure report，不宣称稳定”。
  - 下一轮若继续之前问题，应从 E3 的 position correction 可观察统计 / 完整 position row solver 设计进入，而不是重复 velocity bias 或 angular residual correction 的局部实验。

### 2026-05-06 - E3 最小切片：Position correction 可观察统计

- 状态：已完成，未改变 correction 策略
- Commit：none
- 目标：先区分 residual position correction 的输入事实和实际姿态修正量，避免把 correction 前的 contact manifold depth 误读成 correction 后几何穿透。
- 产出：
  - `StepStats` / `DebugStats` 新增 `position_correction_input_contact_count`、`position_correction_input_max_depth`、`position_correction_input_total_depth`、`position_correction_body_count`、`position_correction_max_translation`、`position_correction_total_translation`。
  - `picea-lab` debug render typed schema 和 Web `DebugSnapshot.stats` 类型同步这些字段。
  - 新增 behavior lock：`contact_position_correction_reports_input_depth_and_applied_translation`，确认单帧 face manifold 的 pre-correction depth 和实际 dynamic body translation 都可观测。
- 验证：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run artifact_schema_keeps_final_observability_fact_set -- --nocapture`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run warm_start_debug_render_frame_fields_default_when_deserializing_older_json -- --nocapture`：通过。
- 结论：
  - E3 现在有了后续调 solver 所需的最小观测口径；下一步才能判断是 residual correction 修正不足、修正过量、观测口径滞后，还是需要完整 position row solve。
  - 这一步没有降低 `matrix_stack 8x6` 的 penetration/churn，也不应被当作稳定性修复完成。

### 2026-05-06 - E3 负向实验：单步 position correction 位移上限

- 状态：已回滚，不进入实现
- Commit：none
- 目标：参考 Matter.js / Box2D 这类稳定引擎常见的线性修正 guardrail，尝试给 residual position correction 增加 per-body 单步累计位移上限，避免 dense stack 中同一 body 被多接触点连续推离。
- 观测：
  - `0.2` 上限：新增红测显示深穿透 fixture 原本单步 correction 位移约 `0.952`，但加 `0.2` 后 `stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 回归，settled penetration max 到 `0.06616616`，超过 D2 `0.04` hard gate。
  - `0.3` 上限：`stack_4` 通过，但 `matrix_stack 8x6` stress 只把 penetration max 从约 `0.415` 降到 `0.396649`，同时 peak churn 升到 `183`，warm-start final 变为 `hit=28 / miss=63 / drop=8`，solver row max 升到 `126`。
  - `0.4` 上限：`stack_4` 通过，`matrix_stack 8x6` penetration max 为 `0.408876`，peak churn `161`，但 final warm-start 仍较差 `hit=29 / miss=54 / drop=3`，solver row max 升到 `130`。
- 结论：
  - 简单 per-body translation cap 不能作为稳定性修复：它会在小栈和大栈之间制造新 tradeoff，且没有解决 contact graph churn / warm-start 失配。
  - 下一步应进入结构化 position solve：优先评估 split impulse / NGS-style position rows、按 contact manifold 聚合 correction、以及从 solver 内部重新生成/更新 position constraint，而不是继续调全局 correction cap。

### 2026-05-06 - E3 负向实验：按 ContactPairKey 聚合 residual correction

- 状态：已回滚，不进入实现
- Commit：none
- 目标：把 residual position correction 从逐 contact point 应用改为按 `ContactPairKey` 聚合，使用同一 collider pair 的最大有效深度生成一条位置修正，避免 clipped face manifold 的两个点重复推同一对 body。
- 观测：
  - 仅按 pair 聚合并保持 `POSITION_CORRECTION_PERCENT=0.8`：`contact_position_correction_does_not_double_apply_face_manifold_points` 可转绿，但 `stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 回归，后期 rolling quiet linear velocity 到 `0.17644039`，超过 D2 quiet target `0.04`。
  - 聚合后把 `POSITION_CORRECTION_PERCENT` 提高到 `1.2`：position-correction targeted tests 通过，但 `stack_4` settled penetration max 回归到 `0.16160327`，同时 `matrix_stack 8x6` stress 变差：penetration max/sum 为 `0.420000 / 11.869830`、peak churn `203`、final warm-start `hit=19 / miss=64 / drop=4`、solver row max `145`。
- 结论：
  - 简单 pair-level 聚合会削弱小栈的静息收敛；提高 percent 又会放大大矩阵的 churn 和 row spike。
  - 下一步若继续 E3，不应只按 pair 合并旧 contact facts；更合理的方向是 split impulse / NGS-style position rows，在 solver 内用稳定 row order 和逐迭代位置约束重新评估 correction，而不是直接复用 stale frame-start manifold 深度。

### 2026-05-06 - E3 下一步方案：最小 NGS-style position rows

- 状态：方案就绪，待实现
- Commit：none
- 目标：把当前 residual pose translation 从“帧初 contact depth 的后处理”升级为更接近 Box2D NGS / split impulse 的位置层约束 solve，同时仍保持 picea 现有 sequential impulse 架构和 public API 不变。
- 为什么不是继续调 residual correction：
  - per-body translation cap 会在小栈和大矩阵之间制造不稳定 tradeoff。
  - pair-level 聚合会削弱小栈静息，强行提高 percent 又会放大 churn。
  - 两个实验共同说明：问题不只是 correction 幅度，而是使用 stale frame-start contact facts 做多轮位移，缺少 position row 的稳定调度和逐迭代误差收敛口径。
- 最小实现边界：
  - 在 `solver/contact.rs` 内新增 internal `PositionSolverRow`，由现有 contact rows 派生，先只覆盖 dynamic/static 与 dynamic/dynamic contact，不触碰 joint solver。
  - position solve 使用独立的 pseudo-position body cache；只写回 pose，不写回 velocity，避免把位置修正能量注入 velocity solve。
  - 每个 position iteration 读取当前 pseudo pose 下的 anchor 分离量，计算 Baumgarte/NGS correction；这和现有 residual correction 的关键区别是“逐迭代重新评估误差”，不是反复使用帧初 depth。
  - 保留 `POSITION_CORRECTION_SLOP`，新增单 row correction clamp 时必须先通过 `stack_4` hard gate；不能只用 `matrix_stack` stress 指标证明。
  - `StepStats.position_correction_*` 继续作为输入/输出观测口径；若新增 row-level counters，只能 additive。
- 第一版行为锁：
  - 保留现有 `contact_position_correction_*` 三个 targeted tests。
  - 新增一个窄测试：深穿透单帧不能把 dynamic body 推出超过合理 NGS correction，同时后续 `stack_4` quiet window 仍通过。
  - `matrix_stack 8x6` 暂不变 hard pass；只要求 penetration max/sum 与 churn 不比 baseline 恶化，并记录实际改善。
- 验收命令：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`
  - `rtk proxy cargo test -p picea --lib --tests`
  - `rtk proxy cargo test -p picea-lab`
  - `rtk proxy cargo fmt --all --check`
  - `rtk proxy git diff --check`

### 2026-05-06 - E3 负向实验：用累计位移重估 residual depth

- 状态：已回滚，不进入实现
- Commit：none
- 目标：保留逐 contact row 的 residual correction，但在每个 position iteration 内根据当前 step 已累计的 body translation 重新估算剩余 depth，避免同一 face manifold 的多个点重复使用帧初 depth。
- 观测：
  - `POSITION_CORRECTION_PERCENT=0.8`：`contact_position_correction_*` targeted tests 通过，`matrix_stack 8x6` stress 明显改善为 penetration max/sum `0.039338 / 1.698759`、peak churn `64`、final warm-start `hit=124 / miss=5 / drop=27`；但 `stack_4` quiet hard gate 回归，rolling quiet linear velocity 到 `0.13442011`。
  - `POSITION_CORRECTION_PERCENT=1.0`：`stack_4` 仍回归，rolling quiet linear velocity 到 `0.16416308`；`matrix_stack` penetration max/sum `0.062656 / 1.588398`、peak churn `64`、final warm-start `hit=127 / miss=0 / drop=30`。
  - `POSITION_CORRECTION_PERCENT=0.6`：`stack_4` 仍回归，rolling quiet linear velocity 到 `0.12922712`；`matrix_stack` penetration max/sum `0.078697 / 2.000388`、peak churn `66`。
  - `POSITION_CORRECTION_PERCENT=0.3`：`stack_4` settled penetration max 回归到 `0.07176259`；`matrix_stack` penetration max/sum `0.063774 / 2.936840`、peak churn `60`。
- 结论：
  - 这个半步 NGS 方向证明了“逐迭代重新评估误差”确实能大幅压低大矩阵穿透，但直接把结果写回 pose 会破坏小栈 quiet/penetration hard gate。
  - 下一步不能在旧 residual correction 上继续微调；需要真正拆出 position body / position row，使位置层修正有独立 pseudo-state、稳定 row order、可控 writeback，并审查它与 velocity solve / sleep idle 的交互。

### 2026-05-06 - E3 最小实现：Dense residual correction 使用 pseudo translation 重估剩余 depth

- 状态：已完成，保留实现
- Commit：none
- 目标：在不破坏 `stack_4` hard gate 的前提下，把 `matrix_stack 8x6` 的穿透从可诊断 failure 推进到明显低穿透状态。
- 设计取舍：
  - 小规模 stack 继续走 legacy residual correction，因为它已经满足 D2 quiet/penetration hard gate。
  - 仅当当前 step eligible contact 数达到 dense threshold 时，position correction pass 记录本 pass 内每个 body 的 pseudo translation，并用它重估同一帧剩余 normal depth。
  - 这不是完整 NGS / split impulse position row；它是保守的 dense-mode 中间切片，不改变 public API、不改 joint solver、不写回 pseudo velocity。
- 产出：
  - `solver/contact.rs` 新增 dense contact correction mode 和 `DENSE_POSITION_CORRECTION_CONTACT_THRESHOLD`。
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 新增 interim penetration gate：`max_penetration_depth <= 0.1`。
- 验证：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过；`matrix_stack 8x6` stress report：penetration max/sum `0.068994 / 1.685199`，peak churn `68`，warm-start `hit=129 / miss=3 / drop=28`，sleep `48 awake / 0 sleeping`，solver row max `166`。
- 结论：
  - E3 取得实质进展：大矩阵穿透已从 baseline `0.415164 / 11.149513` 降到 `0.068994 / 1.685199`，同时小栈 hard gate 保持稳定。
  - 目标尚未完成：first bad frame 仍为 2，diagnostic markers 仍存在，全部 body 仍未 sleep。后续应进入 E4 row ordering / iteration 调度和 E5 sleep blocker 解释，而不是继续盲调 dense correction threshold。

### 2026-05-06 - E5 证据口：Matrix stack sleep blocker speed

- 状态：已完成，未改变物理行为
- Commit：none
- 目标：补齐 D2 stress gate 中“若全部 awake，必须记录 blocker 或 missing blocker”的证据要求，让 sleep 不收敛的问题能指向具体 body / speed，而不是只显示 `48 awake / 0 sleeping`。
- 产出：
  - `MatrixStackStressReport` 增加 `Sleep blocker speeds` 行，记录最终 dynamic bodies 中最大线速度和最大角速度及对应 body handle。
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 检查报告包含该字段。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过；当前 blocker 为 `BodyHandle(48)`，最终 linear `16.718718`、angular `8.459787`。
- 结论：
  - E5 还未修复 sleep 收敛；但 unresolved sleep story 现在有明确 blocker。下一步应先处理 E4 中 body ejection / row scheduling / velocity energy，再判断是否需要调整 sleep 策略。

### 2026-05-06 - E5/E4 证据口：Matrix stack floor ejection blocker

- 状态：已完成，未改变物理行为
- Commit：none
- 目标：把 `BodyHandle(48)` 的 high-speed sleep blocker 进一步定位成“离开 floor 支撑范围后自由落体”，避免误判为 sleep threshold 或 idle-window 调参问题。
- 产出：
  - `MatrixStackStressReport` 增加 `Floor ejection` 行：从第一帧 static floor collider AABB 推出支撑范围，逐帧检查 dynamic body center 是否越过 floor 的 x 范围，并记录 first exit frame/body/x 和最终 floor 外侧 body 数。
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 固定 markdown 中必须包含 floor ejection 证据。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，1 passed / 1 ignored。
  - 当前 report 显示：`Floor ejection: first exit frame 81 body BodyHandle(48) center_x 2.824367; final outside floor bodies=1`。
- 结论：
  - 当前 `48 awake / 0 sleeping` 的直接 blocker 不是 sleep 逻辑本身，而是右上角 body 在压力堆叠中被侧向推出 floor 支撑面，随后自由落体造成 final linear `16.718718`、angular `8.459787`。
  - 该 blocker 与 fixture 几何强相关：第 0 帧 `BodyHandle(48)` 的 center_x 为 `1.6982499`，直接下方接触体 `BodyHandle(40)` 的 center_x 为 `1.48675`，中心差约 `0.2115`，而箱体半宽为 `0.21`；右上角从开局就是边缘支撑/微过悬状态。
  - 下一步优化方向应优先进入 E4：分析 row scheduling / pressure propagation / friction-tangent impulse / position correction 侧向能量注入，而不是先调 sleep 阈值。
  - 验收解释需要分开两条线：若目标是“稳定矩阵形态”，应新增/参数化一个不带边缘过悬的 aligned `NxM` matrix 行为锁；当前 staggered `8x6` 更适合作为 edge-ejection stress gate，不能用 sleep 调参把它强行冻住。

### 2026-05-06 - E1/E4 分线：Aligned small NxM matrix behavior lock

- 状态：已完成，未改变 solver 行为
- Commit：none
- 目标：把“稳定矩阵形态”从当前 staggered `8x6` edge-ejection stress gate 中拆出来，先提供一个小规模、对齐支撑、可稳定验收的 NxM 行为锁，避免后续把 fixture 几何过悬误判为 solver sleep 问题。
- 产出：
  - 新增 `ScenarioId::MatrixStackAligned`，id 为 `matrix_stack_aligned`，默认是 aligned `4x3` dynamic box matrix + 1 static floor。
  - `matrix_stack_fixture` 增加 internal `MatrixStackLayout`：`StaggeredStress` 继续服务原 `matrix_stack 8x6`，`Aligned` 服务小规模行为锁。
  - `aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock` 固定 aligned `4x3` 的验收：12 dynamic bodies、max penetration `<= 0.04`、无 floor ejection、最终 floor 外 body 数为 0、final linear speed `<= 0.2`、final angular speed `<= 0.6`。
  - server scenario list、Web demo fallback 和 i18n 增加 `matrix_stack_aligned`。
- 验证：
  - `rtk proxy cargo test -p picea-lab matrix_stack -- --nocapture`：通过；aligned report 为 penetration max/sum `0.020024 / 0.299988`、floor ejection `none`、final outside floor bodies `0`、final max linear `0.107046`、final max angular `0.358971`。
  - 额外 600 帧观测：aligned `4x3` 仍无 floor ejection，但没有 sleep，final max linear 约 `0.298953`、final max angular 约 `0.505230`。
  - `rtk proxy cargo test -p picea-lab --test server_routes server_exposes_scenarios_sessions_artifacts_and_sse_events -- --nocapture`：通过，确认 server scenario list 包含 `matrix_stack_aligned`。
  - `rtk proxy npm --prefix crates/picea-lab/web run build`、`test:i18n`、`test:ui-contract`：通过；build 仅保留既有 Vite chunk-size warning。
  - `rtk proxy just picea-lab-web-stop` / `rtk proxy just picea-lab-web-start` 后，`rtk proxy curl -fsS http://127.0.0.1:8080/api/scenarios` 返回 `matrix_stack_aligned`。本轮未完成浏览器 UI 自动化：`browser-use:browser` 所需 Node REPL `js` 工具未暴露，`agent-browser` fallback 命中本机 Playwright `chromium_headless_shell-1200` executable 缺失。
- 结论：
  - aligned `4x3` 可以作为小规模矩阵形态行为锁，但不能宣称 sleep 收敛完成；E5 仍需要处理 quiet/sleep。
  - aligned `8x6` 试跑也发生 ejection（first exit frame 110，final outside floor bodies 8），说明大规模 `8x6` 的稳定目标仍依赖 E4 solver 层优化，不能只靠去掉 stagger 解决。

### 2026-05-06 - E5 最小修复：Sleep low-motion threshold 对齐 D2 quiet window

- 状态：已完成，未强制矩阵场景 sleep
- Commit：none
- 目标：修正 sleep 对“低速”的定义。之前 `pipeline/sleep.rs` 使用 `0.0001` 级阈值，实际近似要求速度精确为 0；这与 D2 中小栈 quiet window（linear `0.04`、angular `0.08`）不一致，也不符合成熟引擎把 sleep 当作低速稳定结果的设计。
- 产出：
  - `SLEEP_LINEAR_THRESHOLD` 调整为 `0.04`。
  - `SLEEP_ANGULAR_THRESHOLD` 调整为 `0.08`。
  - 新增 `sleep_accepts_the_d2_quiet_window_but_rejects_faster_motion`，证明 D2 quiet-window 内的非零速度可在 stability window 后 sleep，而超过阈值的运动不会被强行 sleep。
- 验证：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep -- --nocapture`：通过，8 passed。
  - `rtk proxy cargo test -p picea --lib pipeline::sleep`：通过，2 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed，`stack_4` hard gate 未回归。
  - `rtk proxy cargo test -p picea-lab matrix_stack -- --nocapture`：通过；aligned `4x3` 仍是 `12 awake / 0 sleeping`，staggered `8x6` 仍是 `48 awake / 0 sleeping`。
- 结论：
  - E5 修正了不现实的 sleep threshold，但没有把当前矩阵问题伪装成稳定。aligned `4x3` 的 final max linear `0.107046`、angular `0.358971` 仍高于 D2 quiet window；staggered `8x6` 仍有 `BodyHandle(48)` ejection。
  - 下一步仍应进入 E4：定位并降低小矩阵/大矩阵的残余速度来源，例如 contact row scheduling、压力传播、friction tangent impulse、position correction 的侧向能量。

### 2026-05-06 - E4/E5 证据口：Quiet-window 与 late correction 报告

- 状态：已完成，未改变物理行为
- Commit：none
- 目标：补齐“为什么小规模 aligned matrix 仍不能 sleep”的晚期证据，避免把问题再次误判为单纯 sleep threshold。报告从 frame `120` 开始统计 quiet-window 内的最大线速度/角速度、晚期 position correction、warm-start drop 与 contact churn。
- 产出：
  - `MatrixStackStressReport` 增加 `Quiet window f>=120` 行，记录晚期窗口最大 linear/angular speed、body handle 和 frame。
  - `MatrixStackStressReport` 增加 `Late correction/churn f>=120` 行，记录晚期最大 correction translation、单帧 correction total、corrected body 数、warm-start drop peak 和 churn peak。
  - `aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock` 继续保留小规模行为锁的低穿透/无 ejection/最终速度上界，同时要求报告暴露 late correction 证据；它不是 sleep 收敛 gate。
- 验证：
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed / 1 ignored。
  - aligned `4x3` report：quiet-window max linear `0.125676` at `BodyHandle(10)` frame `160`，max angular `0.404157` at `BodyHandle(12)` frame `153`；late max correction translation `0.015981`，max correction total `0.111770`，corrected bodies `12`，warm-start drop peak `7`，churn peak `15`。
  - staggered `8x6` report：quiet-window max linear `16.718718`、max angular `8.459787` at `BodyHandle(48)` frame `179`；late max correction translation `0.055041`，max correction total `0.714849`，corrected bodies `47`，warm-start drop peak `34`，churn peak `40`。
- 结论：
  - aligned `4x3` 已满足“小规模矩阵不飞出、穿透低、最终速度有界”的行为锁，但晚期仍每帧有 position correction / churn / warm-start drop，因此无法通过 sleep idle accumulation；这更像 E4 solver residual energy 问题，而不是 E5 threshold 问题。
  - staggered `8x6` 的主要 blocker 仍是 `BodyHandle(48)` ejection；当前阶段不能宣称大规模矩阵稳定。
  - 下一步优化方案应转向 Matter.js / Box2D 式的 position solver 设计：独立 position rows、pseudo-position state、多轮重新评估 separation、稳定 row ordering/pressure propagation，并把 sleep idle reset 与微小 correction 分离审查。

### 2026-05-06 - E4 负向实验：Dense residual correction 底部优先排序

- 状态：已回滚，不进入实现
- Commit：none
- 目标：只在 dense residual position correction 内按接触点 `y` 从底部向上排序，验证 pressure propagation 是否能在不改变 velocity solver row order、不触碰 `solver-island-ordering-contract.md` 的前提下降低矩阵残余能量。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` 未回归。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，但指标变差，不能作为优化保留。
  - aligned `4x3`：final linear 从 `0.107046` 降到 `0.084677`，final angular 从 `0.358971` 降到 `0.133304`；但 quiet-window max linear 变差到 `0.262724`，max angular 变差到 `0.493313`，solver row max 从 `30` 到 `31`。
  - staggered `8x6`：penetration max/sum 从 `0.068994 / 1.685199` 变差到 `0.073314 / 1.722568`，contact churn peak 从 `68` 到 `69`，floor ejection 从 first exit frame `81` / final outside `1` 变成 first exit frame `84` / final outside `2`，late churn peak 从 `40` 到 `52`。
- 结论：
  - 单纯按接触点高度重排 residual correction 不是稳定性修复；它会改变坏状态的形态，而不是稳定 pressure graph。
  - E4 不能只做启发式排序。下一步应优先实现/评估真正的 position-row pseudo-state，或先补 row-level pressure / correction provenance，确认哪一类 row 在晚期持续注入能量。

### 2026-05-06 - E4 负向实验：Dense residual correction 中加入 angular pseudo-state

- 状态：已回滚，不进入实现
- Commit：none
- 目标：只在 dense residual correction 内把 correction 从纯平移升级为 position-row effective-mass 分配，把 `anchor x normal` 的角惯量项和 pseudo angle 纳入重估，验证大矩阵是否主要因为位置层缺少角自由度而产生侧向能量。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` 未回归，因为小栈仍走 non-dense 路径。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败；`aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock` 红。
  - aligned `4x3`：penetration max/sum 变差到 `0.036718 / 0.506604`，final max linear/ angular 从 `0.107046 / 0.358971` 暴涨到 `3.605195 / 3.956409`，late warm-start drop peak `9`，solver row max `36`。
  - staggered `8x6`：penetration max 轻微降低到 `0.064470`，但 penetration sum 变差到 `2.404570`，contact churn peak 升到 `88`，final outside floor bodies 变成 `2`，late churn peak 升到 `69`。
- 结论：
  - 角修正不能作为 residual phase 的局部补丁加入；它会把小规模 aligned 行为锁直接打坏，并让大矩阵 churn/ejection 更严重。
  - 下一步若做 NGS / Box2D-style position solver，必须是完整的 position row 设计：独立 pseudo pose、受控角修正 clamp、每轮 contact separation 口径和 writeback 边界都要先行为锁，而不是在当前 residual correction 中追加 angular effective mass。

### 2026-05-06 - E5 局部修复：微小 position correction 不再清空 sleep idle

- 状态：已完成，保留实现
- Commit：none
- 目标：修复低速静息接触无法累计 sleep window 的局部问题。之前 residual position correction 每次非零位移都会清空 `sleep_idle_time`，即便只是 slop 级别的微小修正，也等价于每帧外部唤醒。
- 产出：
  - 新增 `resting_static_contact_can_accumulate_sleep_window` 行为锁：一个动态矩形静息在 static floor 上，90 帧后必须 sleep。
  - `apply_position_translation` 只在 body 原本 sleeping 被 contact correction 唤醒、或单次 correction translation 大于 `POSITION_CORRECTION_SLOP` 时清空 `sleep_idle_time`；slop 内的微小修正不再阻断 idle accumulation。
- 验证：
  - 红测：`rtk proxy cargo test -p picea --test physics_realism_acceptance resting_static_contact_can_accumulate_sleep_window -- --nocapture` 初始失败，body 90 帧后仍未 sleep。
  - 修复后同命令通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep -- --nocapture`：通过，9 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed / 1 ignored。
- 结果：
  - aligned `4x3` report 未变化，仍是 `12 awake / 0 sleeping`，说明它的 blocker 仍是 late velocity/churn/correction，不是 idle reset alone。
  - staggered `8x6` 从 `48 awake / 0 sleeping` 改善到 `46 awake / 2 sleeping`，但 `BodyHandle(48)` ejection 仍存在，final max linear/angular 仍为 `16.718718 / 8.459787`。
- 结论：
  - E5 的 sleep idle reset 局部问题已经收口，但整体目标尚未完成。
  - 下一步仍回到 E4：降低 aligned `4x3` 的 late residual velocity/churn，并处理 staggered `8x6` 的 ejection；sleep 只能作为稳定后的收敛层，不能替代 solver 稳定性。

### 2026-05-06 - E4 负向实验：Dense-only support velocity bias

- 状态：已回滚，不进入实现
- Commit：none
- 目标：避免重复之前“全局 velocity bias expansion”回归小栈的问题，只在 dense contact graph 内放宽 resting-contact position bias 的相对法向速度门槛，验证是否能通过更强 normal support impulse 给摩擦更多预算，从而降低大矩阵穿透和侧向 ejection。
- 观测：
  - `relative_normal_speed <= 0.04`：
    - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过；小栈未回归。
    - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过。
    - aligned `4x3`：final speed 降到 `0.102629 / 0.109561`，但 quiet-window max 变差到 `0.378652 / 0.852311`。
    - staggered `8x6`：penetration max/sum 改善到 `0.032524 / 1.571871`，contact churn peak 略降到 `66`，但 first floor exit 提前到 frame `79`，final linear 增到 `17.710110`，sleep 回到 `48 awake / 0 sleeping`。
  - `relative_normal_speed <= 0.01`：
    - `stack` gate 通过。
    - aligned `4x3`：final speed 仍在行为锁内，但 quiet-window max 仍差于 baseline：`0.370345 / 0.566973`，churn peak 升到 `19`。
    - staggered `8x6`：penetration max/sum 改善到 `0.032212 / 1.566585`，但 contact churn peak 升到 `86`，warm-start miss/drop 变成 `26 / 31`，sleep 仍为 `48 / 0`。
- 结论：
  - dense-only support bias 能显著降低大矩阵穿透，但会把问题转移成 late velocity spike、contact churn 和 ejection，不满足“稳定”目标。
  - 后续如果要保留这一类机制，必须先有 row-level provenance 或更完整 position solver 来解释/限制它引入的能量；不能只把 penetration max 当唯一目标。

### 2026-05-06 - E4 证据口：Ejection trace 记录离地前最后接触

- 状态：已完成，未改变物理行为
- Commit：none
- 目标：把 `BodyHandle(48)` ejection 从“最终飞出 floor”进一步定位到离开支撑前一帧的接触状态，避免后续 E4 只凭最终速度猜测 row ordering / friction / support impulse 问题。
- 产出：
  - `MatrixStackStressReport` 增加 `Ejection trace` 行：记录 first-exit body 在 pre-exit frame 的 center_x、linear speed、angular speed，以及 first exit 前最后一次接触的 frame、接触数量、counterpart bodies、max depth、solver normal/tangent impulse sum。
  - `MatrixStackStressReport` 增加 `Ejection dynamic-support trace` 行：记录 first-exit body 在 first exit 前最后一次 dynamic counterpart 接触的 frame、counterpart bodies、depth 和 solver impulse，用来区分“动态支撑断裂”和“floor 边缘弱接触”。
  - `MatrixStackStressReport` 增加 `Ejection dynamic-impulse trace` 行：继续向前找 first-exit body 最后一次带非零 dynamic normal impulse 的接触，并记录 counterpart、normal/tangent impulse 和相对 normal/tangent speed。
  - `MatrixStackStressReport` 增加 `Ejection velocity trace` 行：记录 first-exit body 在 row-max frame、last dynamic impulse frame、last dynamic contact frame、pre-exit frame 的 linear/angular speed，定位速度积累窗口。
  - `MatrixStackStressReport` 增加 `Ejection support-gap trace` 行：记录 first-exit body 首次丢失所有动态支撑的 frame、gap start speed、上一帧支撑体/深度/warm-start/impulse/solver initial speed/post-solve speed/bias，以及下一次重接触 frame/支撑体/深度/warm-start/impulse/solver initial speed/post-solve speed/bias。
  - `ContactEvent` / `DebugContact` 增加 additive solver row provenance：`solver_initial_normal_speed`、`solver_initial_tangent_speed`、`solver_final_normal_speed`、`solver_final_tangent_speed`、`solver_position_bias`、`solver_restitution_bias`、`solver_normal_impulse_delta`、`solver_tangent_impulse_delta`。这些字段只暴露 solver 事实，不改变求解行为。
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 固定 markdown 必须包含该证据行。
- 验证：
  - `rtk proxy cargo fmt --all`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed / 1 ignored。
  - aligned `4x3`：`Ejection trace: none`。
  - staggered `8x6`：`Ejection trace: pre-exit frame 80 center_x 2.790038 speed 2.140532 angular 7.168946; last contact frame 80 count 1 counterparts [BodyHandle(0)] max_depth 0.006862 normal_impulse_sum 0.000000 tangent_impulse_sum 0.000000`。
  - staggered `8x6` dynamic support trace：`last dynamic contact frame 57 count 1 counterparts [BodyHandle(32)] max_depth 0.003112 normal_impulse_sum 0.000000 tangent_impulse_sum 0.000000 warm_start_impulse 0.000000/0.000000 warm_start [Hit] normal_speed 0.304623..0.304623 tangent_speed_abs_max 1.279219`。
  - staggered `8x6` dynamic impulse trace：`last dynamic impulse frame 54 counterparts [BodyHandle(32)] normal_impulse_sum 0.005315 tangent_impulse_sum 0.004252 normal_speed 0.000181..0.000181 tangent_speed_abs_max 0.571637`。
  - staggered `8x6` velocity trace：`row-max frame 46 speed 1.691259/2.378518; last impulse speed 1.826370/4.474461; last dynamic contact speed 2.189907/4.474461; pre-exit frame 80 speed 2.140532/7.168946`。
  - staggered `8x6` support-gap trace：`gap start frame 39 speed 1.653508/4.709286; previous frame 38 counterparts [BodyHandle(40)] max_depth 0.003259 warm 0.000000/0.000000 impulse 0.000000/0.000000 initial_speed 0.243933..0.243933 post_speed 0.185397..0.185397/0.513097 bias 0.000000/0.000000; next frame 46 counterparts [BodyHandle(32)] max_depth 0.000572 warm 0.000000/0.000000 impulse 0.295463/0.052014 initial_speed -2.275158..-2.275158 post_speed 0.006941..0.006941/0.000000 bias 0.000000/0.000000`。
- 结论：
  - `BodyHandle(48)` 在离开支撑前已经只有 floor 接触，而且该接触没有 solver support impulse；直接 blocker 更像上层压力/侧向速度在更早帧形成，最后一帧 floor 接触只是弱、浅、无支撑的边缘接触。
  - `BodyHandle(48)` 的动态支撑在 frame `57` 已经只剩浅接触且无 solver impulse；最后一次非零 dynamic normal impulse 在 frame `54` 已经非常小。现在报告直接使用 solver 导出的 final row speed，而不是测试侧复算，避免符号约定误判。
  - support-gap trace 进一步把根因窗口前移：frame `38` 的动态支撑 warm-start 与 solver impulse 都已归零，solver initial normal speed 已经是正值 `0.243933`，且 position/restitution bias 都是 `0`，说明该 row 既没有历史支撑预算，也在 velocity solver 看来已经分离，不会产生 support impulse；frame `39` 到 `45` 完全无动态接触，frame `46` 的重接触才触发 row max。
  - 因此当前 blocker 更像切向/旋转运动把边缘支撑几何滑出接触，而不是 normal row 本身没有足够支撑预算。下一步 E4 应优先看 position-row / pseudo-pose 是否能减少支撑几何滑脱，或补充 per-row tangent/rotation provenance，而不是继续改 normal bias 或 sleep。
  - 速度在 solver row max 的 frame `46` 已经明显形成，后续主要是角速度继续放大和支撑断裂。因此下一步 E4 应向 frame `12..46` 的压力传播 / tangent velocity / row solve 收敛追踪，而不是只在 floor contact、frame `57`、frame `80` 或 sleep 层修补。

### 2026-05-06 - E4 负向实验：Velocity solver row 顺序交替

- 状态：已回滚，不进入实现
- Commit：none
- 目标：验证当前 sequential impulse 的固定 row traversal 是否在 dense stack 中形成单向压力传播偏置。实验做法是在每个 velocity iteration 中交替正向/反向遍历 contact rows，保持 contact/joint phase contract 和 public API 不变。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：失败；`stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 的 settled penetration max 回归到 `0.043207645`，超过 D2 hard gate `0.04`。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：测试本身通过，但指标不满足优化目标。aligned `4x3` quiet-window max linear/ angular 变差到 `0.339474 / 0.519969`；staggered `8x6` penetration max/sum 变差到 `0.072735 / 1.677369`，final outside floor bodies 从 `1` 变成 `2`，row max 变为 `167` at frame `14`。
- 结论：
  - 单纯交替 row 顺序不能作为稳定性修复；它会破坏小规模 hard gate，并把大矩阵 ejection 改成另一种坏状态。
  - E4 仍需要更结构化的 position-row / pressure propagation 设计，或者先补 row-level provenance，确认最后一轮其他 row 如何把 support row 推回 approaching 状态。

### 2026-05-06 - E4 负向实验：Dense position correction slop 收紧

- 状态：已回滚，不进入实现
- Commit：none
- 目标：验证 matrix stack 的边缘支撑是否因为当前 `POSITION_CORRECTION_SLOP=0.005` 过宽，导致 frame `38` 这类浅动态支撑 contact（depth 约 `0.003259`）不进入 residual position correction，从而在 frame `39` 失去支撑。
- 观测：
  - `DENSE_POSITION_CORRECTION_SLOP=0.001`：
    - `stack_4` hard gate 与 position-correction targeted tests 通过。
    - aligned `4x3` 仍通过行为锁，final blocker speeds 降到 `0.063981 / 0.081823`，但仍未 sleep。
    - staggered `8x6` penetration max/sum 改善到 `0.053954 / 1.599266`，angular blocker 从 `8.459787` 降到 `6.236454`，但 sleep 回到 `48 awake / 0 sleeping`，churn peak 升到 `71`，warm-start drop 为 `31`。
  - `DENSE_POSITION_CORRECTION_SLOP=0.003`：
    - `stack_4` hard gate 通过。
    - aligned `4x3` 明显改善为 `3 awake / 9 sleeping`，churn peak 降到 `8`。
    - staggered `8x6` penetration max/sum 改善到 `0.049083 / 1.577891`，但 angular blocker 暴涨到 `15.270988`，final outside floor bodies 变成 `2`，sleep 仍为 `48 / 0`。
  - `DENSE_POSITION_CORRECTION_SLOP=0.002`：
    - `stack_4` hard gate 通过。
    - aligned `4x3` 行为锁通过但 quiet-window max linear/ angular 变差到 `0.220006 / 0.416730`。
    - staggered `8x6` penetration max/sum 改善到 `0.051621 / 1.580941`，但 final outside floor bodies 仍为 `2`，sleep `48 / 0`，churn peak `73`。
- 结论：
  - 收紧 dense slop 能降低 penetration，且某些取值会明显改善小规模 aligned sleep；但它会把 `8x6` 的问题转移为更强的 angular/ejection/churn，不满足“系统稳定”目标。
  - 这证明“浅支撑 contact 不进 position correction”是相关因素，但不能用单个 slop 常量修复。下一步应做完整 position-row / pseudo-pose 设计：在受控 pseudo state 中处理浅支撑、切向/旋转滑脱和 correction 能量，而不是直接让 residual correction 更激进。

### 2026-05-06 - E4 保留小修：Dense shallow support friction budget

- 状态：保留；作为 E4 的局部稳定化步骤，不代表目标完成
- Commit：none
- 目标：验证 frame `38` 这类浅动态支撑接触是否缺少切向摩擦预算。与已回滚的 dense-only support velocity bias 不同，本次不加入 normal support impulse、不改变 normal row，只在 dense contact graph 中给浅穿透且法向相对速度已分离的 tangent row 一个极小 support friction budget。
- 实现：
  - `solver/contact.rs` 增加 `SHALLOW_SUPPORT_FRICTION_BUDGET_SCALE = 0.5`。
  - 当 island contact rows 数量达到 dense threshold，且 `0 < depth <= POSITION_CORRECTION_SLOP`、`relative_normal_speed >= 0` 时，计算 `support_friction_impulse = depth * scale / dt * normal_mass`。
  - tangent warm-start clamp 和 tangent solve 的 Coulomb budget 使用 `max(normal_impulse, support_friction_impulse)`；normal impulse 仍按原 row 求解，避免把系统推成“强 normal 支撑”。
  - 通过 `solver_support_friction_impulse` 暴露到 contact event / debug contact / lab report，便于区分“预算确实使用过”和“指标改善来自别处”。
- 验证：
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 仍为 green。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea --test query_debug_contract warm_start_new_picea_payload_fields_default_when_deserializing_older_json -- --nocapture`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed / 1 ignored。
- 最新 stress 结果：
  - aligned `4x3`：penetration max/sum `0.019898 / 0.299988`，churn peak `17`，sleep `12 awake / 0 sleeping`，quiet-window max linear/angular `0.110800 / 0.395341`，dense support friction max `0.006038`，contacts used `234`，floor ejection `none`，final outside floor bodies `0`。
  - staggered `8x6`：penetration max/sum `0.041646 / 1.720375`，churn peak `69`，warm-start `127 / 3 / 27`，sleep `47 awake / 1 sleeping`，quiet-window max linear/angular `3.430820 / 5.230875`，late correction max/total `0.057381 / 0.739580`，dense support friction max `0.008890` at frame `37`，contacts used `1709`，floor ejection `none`，final outside floor bodies `0`，solver row max `167` at frame `98`。
- 结论：
  - 这是一条正向但有限的 E4 小步：`8x6` 从 floor ejection / final outside body 的 stress failure 推进到 no-ejection，并且没有破坏 `stack_4` hard gate。
  - 系统目标仍未完成：first bad frame 仍为 `2`，diagnostic markers 仍有 `SolverRowSpike`、`ContactChurnSpike`、`PerformanceCounterSpike`，`8x6` 后半段仍有明显速度尖峰、warm-start drop 和 contact churn，sleep 也未收敛。
  - 下一步仍应回到更结构化的 E4：position-row / pseudo-pose pressure propagation，或至少沿 frame `12..46` 继续追踪 tangent/rotation 能量如何积累；E5 sleep 只能在这些 residual velocity 降下来后再继续收敛。

### 2026-05-06 - V1 局部验收：重启后的 Browser/API smoke

- 状态：通过；browser-use Node REPL 工具未暴露，已按浏览器技能要求搜索后降级为 Atlas/Computer Use + HTTP API smoke
- 操作：
  - `rtk proxy just picea-lab-web-stop`：通过。
  - `rtk proxy just picea-lab-web-start`：通过，API ready at `http://127.0.0.1:8080/api/scenarios`，Web ready at `http://127.0.0.1:5173/`。
  - `open -a "ChatGPT Atlas" http://127.0.0.1:5173/`：页面打开为 `Picea 实验室工作台`，能看到 workbench、场景选择、画布、时间线、诊断/证据标签。
  - `curl -s http://127.0.0.1:5173/api/scenarios`：确认 `matrix_stack` 和 `matrix_stack_aligned` 均由当前服务暴露。
  - `POST /api/sessions` 创建 `matrix_stack` / `artifact_replay` / `180` 帧 session：返回 `status=completed`、`buffered_frame_count=180`、`final_state_hash=ccb1b01fb65b2231`，与本轮 Rust report 一致。
- 结论：
  - 重启前 5173 背后的旧服务曾返回不同 hash；本轮已重启并确认真实运行链路使用最新代码。
  - 这次只完成 browser/API smoke，不把它升级为完整 V1：真实 UI 下拉选择和 console error/warn 仍需后续用 `browser-use:browser` Node REPL 或 Playwright 复验。

### 2026-05-06 - E4 证据口：Late velocity spike trace

- 状态：已完成，未改变物理行为
- Commit：none
- 目标：dense shallow support friction 消除 floor ejection 后，`matrix_stack 8x6` 的主要 blocker 从“最终出界”转为 late velocity spike / churn / sleep 不收敛。新增 quiet-window 最大 linear / angular body 的接触摘要，避免报告只显示尖峰帧速度而看不到尖峰前接触如何断裂。
- 产出：
  - `MatrixStackStressReport` 增加 `Late linear spike trace` 与 `Late angular spike trace`。
  - trace 记录 blocker frame/body speed、当前 contact/counterpart/depth/warm-start/impulse/initial/final speed/support friction/bias、同帧 correction/churn/drop，以及 blocker frame 之前最后一次 contact 的同类摘要。
  - D2 验收模板增加 late velocity spike 字段。
- 验证：
  - `rtk proxy cargo fmt --all`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed / 1 ignored。
- 最新证据：
  - aligned `4x3`：late linear / angular spike 都仍有动态接触；frame `177` BodyHandle(11) 对 BodyHandle(7) 的 post normal speed `0.018939..0.200637`，frame `155` BodyHandle(10) 对 BodyHandle(6) 的 support friction max `0.004291`，说明小矩阵的 late blocker 仍是有接触状态下的 residual velocity/correction/churn。
  - staggered `8x6` late linear spike：frame `174` BodyHandle(32) speed `3.430820/4.953229`，当前 contacts `0`，但同帧 correction `0.055813/0.710423/46`、churn `32`、drops `27`；last contact 是 frame `164` 对 BodyHandle(24)，depth `0.001160`，normal impulse `0`，initial normal speed `0.390689`、post normal speed `0.337866`，support friction `0.002160`。
  - staggered `8x6` late angular spike：frame `138` BodyHandle(16) speed `1.538859/5.230875`，仍与 floor BodyHandle(0) 接触，depth `0.009099`，normal impulse `0.003428`，post normal speed `0.000020`，但同帧 correction `0.042412/0.703765/47`、churn `35`、drops `32`。
- 结论：
  - 大矩阵现在的 late linear blocker 是“动态支撑 contact 已经提前滑脱，尖峰帧无接触可解”，不是 floor ejection，也不是 sleep threshold。
  - last contact 已经是 separating normal speed 且 normal impulse 为 `0`，说明仅增加 normal bias 或 support friction 不应继续作为主线；下一刀应实现/评估 position-row / pseudo-pose pressure propagation，让浅支撑在位置层减少几何滑脱，同时控制 correction 对 churn 的扰动。
  - aligned `4x3` 的 late blocker 仍在有接触状态下出现，因此下一步也必须保护小矩阵 hard gate：不能只让 `8x6` 不飞出，还要降低小矩阵的 residual correction/churn 才能进入 E5 sleep 收敛。

### 2026-05-06 - E4 证据口：Early pressure window trace

- 状态：已完成，未改变物理行为
- Commit：none
- 目标：把 support-friction 保留基线下 `matrix_stack 8x6` 的早期压力传播窗口从“frame `12..46` 可能有问题”收紧为可比较报告字段，记录晚期 velocity spike 之前的切向速度、角速度、position correction、contact churn、warm-start drop/reason 分布、anchor drift normal/tangent 分解、row count 和 support friction 峰值。
- 产出：
  - `MatrixStackStressReport` 增加 `Early pressure trace f=12..46`。
  - trace 记录最大 tangent row 的 warm-start reason / feature id，并聚合整个窗口内的 warm-start reason 计数。
  - `ContactEvent` 增加 warm-start anchor drift / normal drift / tangent drift 只读字段；`DebugContact` 在内存中保留这些字段供 report 聚合，但不序列化到 debug snapshot JSON，避免改变 behavior-state hash。
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 固定 markdown 必须包含该报告行与 `warm_start reasons [`，并要求 early pressure trace 捕获 dense row pressure。
  - D2 验收模板增加 early pressure trace 字段。
- 验证：
  - 先补报告契约断言后运行 `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：失败，缺少 `- Early pressure trace f=12..46:`。
  - 实现聚合后，`rtk proxy cargo fmt --all --check`：通过。
  - 追加 warm-start reason 分布契约后运行同一命令：先失败，缺少 `warm_start reasons [`；实现聚合后通过，1 passed。
  - 追加 anchor drift 分解后，`matrix_stack 8x6` final hash 仍保持 `ccb1b01fb65b2231`，确认 drift 证据没有污染 behavior hash。
- 最新证据：
  - early pressure trace：window `12..46`；最大 tangent speed 在 frame `38`，bodies `[BodyHandle(40), BodyHandle(48)]`，speed `0.536547`，depth `0.002253`，normal speed `0.245364 -> 0.219942`，normal/tangent impulse `0.000000 / 0.003951`，support friction `0.004938`。
  - 最大 tangent row 的 warm-start reason 是 `Hit`，feature 为 `ContactFeatureId(16785409)`。
  - 同窗口 warm-start reason 分布为 `DroppedPointDrift=1259`、`Hit=4389`、`MissFeatureId=89`、`MissNoPrevious=33`。
  - 最大 anchor drift 出现在 frame `46`，总量 `0.223026`，其中 normal drift `0.003984`、tangent drift `0.222991`。
  - 同窗口最大 angular speed：frame `38`，BodyHandle(48)，linear/angular `1.480471 / 5.056706`。
  - 同窗口 correction 峰值：frame `37`，max/total/bodies `0.030736 / 0.676503 / 47`；churn peak `69` at frame `12`；warm-start drops peak `37` at frame `17`；rows peak `166` at frame `12`；support friction max `0.008890` at frame `37`。
- 结论：
  - 后续 E4 不应只盯晚期 frame `138` / `174`；能量源已经能追到 frame `37..38` 附近的浅动态支撑、切向速度和大规模 correction/churn 叠加。
  - 早期窗口并不是 feature id 完全失稳；最大 tangent row 是 warm-start `Hit`，但 `DroppedPointDrift` 总量仍高。
  - drift 分解显示 drop 主要来自切向漂移，而不是法向穿透漂移；下一步更应针对 tangent contact identity / support sliding，而不是继续降低 normal correction 或加 normal bias。
  - 下一刀仍不是调 sleep 或继续放大 friction budget，而是把 frame `37..38` 的 position correction 与 tangent row 交互拆开：优先评估 translation-only position row / correction cap / contact identity 稳定性，且必须保护 `stack_4` 与 aligned `4x3` 行为锁。

### 2026-05-06 - E4 负向实验：切向漂移 partial normal warm-start

- 状态：已回滚，不保留实现
- Commit：none
- 目标：验证 early pressure trace 中“大量 `DroppedPointDrift` 主要来自切向漂移、法向漂移很小”的证据，是否可以通过“同面大切向滑移时丢弃 tangent impulse，但按切向漂移比例衰减继承 normal impulse”改善支撑连续性。
- TDD：
  - 先把 `warm_start_cache_drops_tangent_impulse_after_large_tangential_slip` 扩展为期望 `PartialTangentDrift`：RED，编译失败，缺少新 reason。
  - 实现 `PartialTangentDrift` 后，warm-start 组失败：圆-圆曲面漂移被误判为同面 partial，说明 feature id 对曲面接触过粗，不能只看 normal drift / tangent drift。
  - 收窄到 SAT clipped face kind 后，新增矩形同面用例可通过，但真实 `matrix_stack 8x6` 验收失败。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：测试进程通过，但 stress report 明确退化。
  - aligned `4x3`：final hash `e6f1708b4fcf393c`；warm-start `27 / 2 / 0`，drop 降为 0，但 quiet-window 从保留基线 `0.110800 / 0.395341` 变差到 `0.153366 / 0.430417`。
  - staggered `8x6`：final hash `7d04203e025db103`；warm-start `162 / 1 / 0`，drop 降为 0，但出现 floor ejection：frame `84` BodyHandle(48) 出界，frame `179` velocity spike 达 `15.335592 / 11.225333`。
  - early pressure reason 分布从 `DroppedPointDrift=1259, Hit=4389, MissFeatureId=89, MissNoPrevious=33` 改为 `PartialTangentDrift=1256, Hit=4396, MissFeatureId=97, MissNoPrevious=32`；drop 归零但支撑滑脱/旋转能量明显恶化。
- 结论：
  - “减少 warm-start drop 计数”不是稳定性的充分条件；在当前 feature id 粒度下，把大切向漂移改成 partial normal support 会保留过期支撑扭矩，反而加剧边缘 body ejection。
  - 后续不要再把 `DroppedPointDrift` 直接改成 partial hit；需要先改善 contact point identity / clipped point persistence，或把 position correction 的切向副作用降下来，再考虑更细粒度的 partial warm-start。
  - 这条负向结果强化了下一步方向：优先处理 position correction 与 tangent/rotation 能量耦合，而不是只在 warm-start 分类上把 drop 变成 hit。

### 2026-05-06 - E4 负向实验：分离中 contact 降低 residual correction 比例

- 状态：已回滚，不保留实现
- Commit：none
- 目标：验证 velocity solve 后已经呈 separating normal speed 的 contact，是否应该避免继续按 `POSITION_CORRECTION_PERCENT=0.8` 使用 stale frame-start depth 做完整 residual correction，从而减少 position correction 对切向 churn / late velocity spike 的注入。
- TDD：
  - 新增 `contact_position_correction_is_gentle_for_already_separating_contacts`，构造仍有 frame-start overlap、但 body 已朝分离方向运动的 face manifold；RED 时 `position_correction_max_translation` 为 `0.0773333`，超过测试期望的 gentle correction。
  - 实现 `POSITION_CORRECTION_SEPARATING_PERCENT=0.1` 后，该 targeted test 与 `contact_position_correction_*` 组通过。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，4 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：失败；`stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 的 settled penetration max 回归到 `0.04914874`，超过 D2 hard gate `0.04`。
- 结论：
  - 无差别降低 separating contact 的 residual correction 比例会让小堆叠穿透积累，直接破坏 `stack_4` hard gate。
  - 这个方向如果还要继续，只能限定到 dense correction mode 并重新设计 dense 专用行为锁；不能作为全局 residual correction 策略保留。

### 2026-05-06 - E4 负向实验：dense-only 分离中 contact 降低 residual correction 比例

- 状态：已回滚，不保留实现
- Commit：none
- 目标：在上一条全局 lowering 被 `stack_4` 否决后，把 gentle correction 限定到 dense correction mode：只有 dense graph 中 velocity row 没有 normal support impulse、但存在 normal 相对运动的 contact，才把 residual correction percent 从 `0.8` 降到 `0.02`。
- TDD：
  - 新增 dense fixture：16 个 static floor tiles 同时支撑一个向外分离的 dynamic slab。当前实现 RED：`position_correction_input_contact_count=32`、`position_correction_max_translation=0.04833319`。
  - 第一版 `0.1` 比例仍为 `0.0466737`，说明 dense pseudo translation 下多 row 累积仍然明显。
  - `0.02` 比例把该 fixture 降到约 `0.023012305`，局部行为锁可转绿，但真实矩阵验收失败。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，说明 dense-only 限定没有伤小规模 `stack_4`。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：测试进程通过，但 stress report 明确退化。
  - aligned `4x3`：early pressure window 变干净，`warm_start reasons [Hit=840]`、anchor drift max 降到 `0.001360`；但 quiet-window 仍差，linear/angular 为 `0.145257 / 0.475091`，弱于保留基线 `0.110800 / 0.395341`。
  - staggered `8x6`：final hash `b2e0a9f5f0ff8e2c`；penetration 退化到 `0.073040 / 1.799800`；出现 floor ejection，BodyHandle(48) frame `81` 出界，frame `179` velocity spike 达 `15.925419 / 6.178970`。
- 结论：
  - dense-only 降低 separating/no-normal-impulse row 的 residual correction 能改善局部 dense fixture 和 aligned 早期 churn，但会让 staggered `8x6` 边缘支撑滑脱并 ejection。
  - 这再次说明不能只削弱 position correction 幅度；`8x6` 需要保留足够 normal position support，同时单独控制切向/旋转能量。后续应优先增加 row-level correction provenance 或设计更完整的 position-row pseudo-state，而不是再按 normal speed/impulse 条件调 correction percent。

### 2026-05-06 - E4 负向实验：提高 shallow support friction budget

- 状态：已回滚，保留 scale `0.5`
- Commit：none
- 目标：验证 dense shallow support friction 小修是否只是预算偏低；尝试把 `SHALLOW_SUPPORT_FRICTION_BUDGET_SCALE` 从 `0.5` 提高到 `1.0` 和 `0.75`，看是否能减少 BodyHandle(32) 的 late tangent/rotation slip。
- 观测：
  - `scale=1.0`：`stack_4` 和 `contact_position_correction_*` targeted tests 通过；aligned `4x3` 行为锁仍通过，但 quiet-window max linear/angular 从保留值的 `0.110800 / 0.395341` 变差到 `0.219325 / 0.528563`。staggered `8x6` penetration 改善到 `0.033111 / 1.673781`，final blocker 降到 `0.402384 / 0.847889`，但 late churn peak 升到 `56`、drop 为 `32`，quiet-window spike 仍是 `3.373622 / 5.138710`。
  - `scale=0.75`：`stack_4` 通过；aligned `4x3` quiet-window `0.124597 / 0.410445`，仍弱于保留值。staggered `8x6` penetration `0.040115 / 1.684310`，sleep 回到 `48 / 0`，final blocker `0.685610 / 0.897527`，quiet-window angular spike 升到 `6.586022`。
- 结论：
  - 更大的 shallow support friction budget 会在某些穿透/最终速度指标上改善，但会恶化小矩阵 quiet-window 或大矩阵 late churn/drop/angular spike，不满足“系统稳定”目标。
  - 保留 `0.5` 作为 conservative bridge；后续不继续靠 friction budget 调参，应转向 position-row / pseudo-pose pressure propagation。

### 2026-05-06 - E4 负向实验：Dense position correction cumulative cap

- 状态：已回滚，保留原 dense pseudo-translation residual correction
- Commit：none
- 目标：验证 early pressure trace 指向的 frame `37..38` correction 峰值是否能通过 dense-only per-body correction cap 降低后续 late velocity spike。该实验不改变 velocity solver、warm-start、support friction 或 sleep。
- 观测：
  - `0.03` vector-net cap：`stack_4` 通过，aligned `4x3` 通过；`matrix_stack 8x6` final hash 改为 `8f2137ee16a57873`，但 penetration、late velocity、correction、churn、support friction 等关键指标与保留基线几乎一致，说明限制口径没有命中实际风险。
  - `0.04` cumulative-distance cap：`stack_4` 通过，aligned `4x3` 通过；`8x6` penetration 维持 `0.041646 / 1.720375`，late linear 从 `3.430820` 小降到 `3.326775`，late correction max 从 `0.057381` 降到 `0.040000`，但 late churn 从 `47` 升到 `53`，support friction max 从 `0.008890` 升到 `0.010240`，角速度 blocker 仍为 `5.230875`。
  - `0.03` cumulative-distance cap：`stack_4` 通过，aligned `4x3` 通过；`8x6` penetration max/sum 变差到 `0.052751 / 1.676389`，sleep 回到 `48 / 0`，late linear 降到 `3.166930`，但 angular blocker 升到 `5.498265`，support friction max 升到 `0.012240`。
- 结论：
  - 简单 per-body correction cap 能压低某些 late linear/correction 数字，但会把问题转移成更强 churn、support friction、angular blocker 或 sleep 退化。
  - 这个结果说明 E4 不能只限制 correction 幅度；需要理解 correction 与 tangent row / contact identity 的交互。下一步应补充 warm-start reason / feature churn 分布，或做更结构化的 translation-only position row，而不是继续调 cap 常量。

### 2026-05-06 - E4 负向实验：Dense pseudo-pose position rows

- 状态：已回滚，保留 dense pseudo-translation residual correction
- Commit：none
- 目标：验证 Box2D 式独立 position-row / pseudo-state 方向是否能减少 dense matrix 的支撑滑脱。实验在 dense contact graph 中绕过已保留的 pseudo-translation residual correction，改为构建 position rows，按 contact anchor、normal effective mass、translation + angular pseudo delta 多轮求解，最后一次性写回 pose。
- 验证：
  - `rtk proxy cargo fmt --all --check`：初次失败，仅为 `contact.rs` 自动换行；`rtk proxy cargo fmt --all` 后通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 没破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败，aligned `4x3` 行为锁与 staggered `8x6` stress gate 都明显回归。
- 观测：
  - aligned `4x3`：final hash `472709644abfe307`；penetration max/sum 回归到 `0.362522 / 2.798988`；quiet-window max linear/angular 暴涨到 `16.460007 / 12.520638`；frame `96` 出现 floor ejection，final outside floor bodies `4`。
  - staggered `8x6`：final hash `dadb533b1009fec2`；penetration max/sum 回归到 `0.420000 / 13.703733`；churn peak `142`；quiet-window max linear/angular `29.088224 / 18.125952`；frame `11` 出现 floor ejection，final outside floor bodies `25`。
  - 回滚后复验恢复保留基线：`stack_4` hard gate 通过；`matrix_stack_aligned` final hash `ffe4aedd8eb04c93`，penetration `0.019898 / 0.299988`，final outside `0`；`matrix_stack 8x6` final hash `ccb1b01fb65b2231`，penetration `0.041646 / 1.720375`，floor ejection `none`，final outside `0`，但 late velocity/churn blocker 仍存在。
- 结论：
  - “position-row / pseudo-pose”方向仍可能是正确的结构方向，但这次实现把 angular pseudo delta 和一次性 pose writeback 组合得太激进，破坏了 small NxM 行为锁，也让 dense stress 重新 ejection。
  - 下一步不能直接用 angular pseudo-pose 替换 residual correction。更稳妥的优化路线是先加 row-level tangent/rotation/correction provenance，定位 frame `12..46` 的能量注入；如果再做 position rows，必须先以 translation-only、per-iteration depth re-evaluation、strict correction cap 和 `stack_4` / aligned `4x3` 双 hard gate 约束。

### 2026-05-06 - E4 证据口：Row-level residual correction provenance

- 状态：已完成，未改变物理行为
- Commit：none
- 目标：在不改变 residual correction 策略的前提下，把每个 contact row 消耗了多少 correction depth、给 body A / body B 各贡献了多少平移暴露到只读事件和 debug/report 层。这样下一步可以判断 early pressure window 中的 correction 峰值到底来自哪一类 row，而不是继续只看 frame-level max/total。
- TDD：
  - 先在 `contact_position_correction_reports_input_depth_and_applied_translation` 中断言 contact 暴露 `solver_position_correction_depth` 与 body translation provenance：RED，编译失败，缺少字段。
  - 实现 core event/debug 字段、solver 侧累加 provenance 后 targeted test 通过。
  - 再在 `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 中断言 markdown 必须包含 `correction_row frame`：RED，报告缺少 row-level correction 摘要。
  - 实现 `PressureWindowTrace` 的 row-level correction 聚合后 report test 通过。
- 产出：
  - `ContactEvent` / `ContactObservation` 增加 `solver_position_correction_depth`、`solver_position_correction_body_a_translation`、`solver_position_correction_body_b_translation`。
  - `DebugContact` 在内存中保留这些字段并 `skip_serializing`，避免污染 behavior-state hash。
  - `Early pressure trace f=12..46` 增加 `correction_row frame ... depth ... consumed ... translation ... final_speed ...`。
  - D2 验收模板补充 row-level residual correction provenance。
- 验证：
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 仍为 green。
  - `rtk proxy cargo test -p picea --test query_debug_contract warm_start_new_picea_payload_fields_default_when_deserializing_older_json -- --nocapture`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed，1 ignored。
  - `rtk proxy git diff --check`：通过。
- 最新证据：
  - aligned `4x3` final hash 仍为 `ffe4aedd8eb04c93`；penetration `0.019898 / 0.299988`；quiet-window max linear/angular `0.110800 / 0.395341`；early correction row 为 frame `44`，bodies `[BodyHandle(4), BodyHandle(8)]`，depth `0.012996`，consumed `0.170457`，translation `0.003409 / 0.003409`，final speed `0.004280 / -0.003173`。
  - staggered `8x6` final hash 仍为 `ccb1b01fb65b2231`；penetration `0.041646 / 1.720375`；quiet-window max linear/angular `3.430820 / 5.230875`；early correction row 为 frame `38`，bodies `[BodyHandle(16), BodyHandle(8)]`，depth `0.019861`，consumed `0.318274`，translation `0.006365 / 0.006365`，final speed `0.012954 / 0.002631`。
- 结论：
  - 这次改动是证据增强，不是 solver 优化；两个 matrix stack hash 保持不变，说明当前行为基线没有被污染。
  - `8x6` 的 early correction row 峰值与最大 tangent / angular pressure 同处 frame `38` 附近，但不是同一对 body。这说明问题更像是 dense pressure propagation 的局部 row 互相耦合，而不是单个 contact 的简单 friction/bias 不足。
  - 下一步应以 row-level provenance 为入口，比较 correction-row、tangent-row、anchor-drift row 的拓扑关系，再设计 translation-only / per-iteration re-evaluation 的 position-row 小实验；不要继续扩大 shallow support friction 或全局削弱 residual correction。

### 2026-05-06 - E4 证据口：Early pressure row coupling topology

- 状态：已完成，未改变物理行为
- Commit：none
- 目标：把 row-level correction 证据进一步收紧到 contact graph 拓扑：记录 max correction row、max tangent row、max anchor-drift row 之间的 frame delta、接触图距离和共享 body 数，判断能量注入是单个 row 局部问题，还是 dense pressure graph 内的传播问题。
- TDD：
  - 在 `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 先要求 markdown 包含 `row_coupling correction_tangent`：RED，报告缺少 coupling 字段。
  - 在 `PressureWindowTrace` 内保存 peak row 的 body pair，并用已有 `FrameRecord.snapshot.contacts` 构建 contact graph 计算 pair-to-pair distance 后转绿。
- 产出：
  - `Early pressure trace f=12..46` 增加 `row_coupling correction_tangent ... correction_anchor ... tangent_anchor ...`。
  - anchor-drift peak 同步输出 body pair，避免只看到 drift 数值却不知道是哪条 contact row。
  - D2 验收模板补充 peak row contact-graph coupling。
- 验证：
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：先 RED 后通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed，1 ignored。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy git diff --check`：通过。
- 最新证据：
  - aligned `4x3`：correction-tangent `frame_delta 20`、`graph_distance 1@f44`、`shared_bodies 0`；correction-anchor `frame_delta 3`、`graph_distance 2@f44`；tangent-anchor `frame_delta 17`、`graph_distance 1@f41`。
  - staggered `8x6`：correction-tangent `frame_delta 0`、`graph_distance 3@f38`、`shared_bodies 0`；correction-anchor `frame_delta 8`、`graph_distance 3@f46`；tangent-anchor `frame_delta 8`、`graph_distance 7@f46`。
- 结论：
  - `8x6` 不是同一个 contact row 同时出现 correction/tangent/anchor 三类 peak；这些 peak 经 contact graph 间接相连，说明局部 correction 通过 dense contact graph 传播后放大 tangent / rotation noise。
  - 下一步 solver 实验应避免“只修 peak row 本身”的参数补丁，优先做 translation-only position row 或 per-iteration depth re-evaluation，并要求比较 coupling 距离附近的 row 是否降低 correction/tangent/anchor 三类峰值。

### 2026-05-06 - E4 负向实验：Dense per-iteration batched residual correction

- 状态：已回滚，保留原 dense Gauss-Seidel style residual correction
- Commit：none
- 目标：验证 dense residual correction 在每个 iteration 内顺序更新 `body_translation_vectors` 是否放大 row coupling noise；实验版本在 dense mode 下每轮先收集所有 pending correction，再统一 apply，让同一轮 row 使用相同的上一轮 pseudo-translation 输入。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：测试进程通过，但 stress report 明确退化。
  - aligned `4x3`：final hash 改为 `2ba9b7963222319b`；penetration `0.020551 / 0.299989` 接近基线，但 quiet-window max linear/angular 从 `0.110800 / 0.395341` 退化到 `0.232445 / 0.510285`。
  - staggered `8x6`：final hash 改为 `7dbe6dbdf06d81e0`；penetration 从 `0.041646 / 1.720375` 退化到 `0.054687 / 1.714527`；sleep 从 `47 / 1` 退化到 `48 / 0`；quiet-window angular 从 `5.230875` 退化到 `6.091372`，late churn peak 从 `47` 升到 `50`。linear spike 从 `3.430820` 小降到 `3.228685`，但不足以抵消其他关键回归。
- 结论：
  - 同一 iteration 内批量 apply correction 会削弱一些 linear spike，但让穿透、sleep、angular 和 aligned quiet-window 变差，不满足“系统稳定”目标。
  - 这说明当前 dense correction 仍需要某种顺序传播来支撑法向位置约束；下一步不应改成纯 batched Jacobi，而应保留法向支撑，同时更局部地控制 tangent/rotation 能量，例如 per-row topology-aware damping 或 contact-point persistence，而不是全图同步 apply。

### 2026-05-06 - E4 负向实验：提高 tangential warm-start drift 阈值

- 状态：已回滚，保留 `WARM_START_TANGENTIAL_DRIFT_THRESHOLD=0.10`
- Commit：none
- 目标：验证 early pressure 中大量 `DroppedPointDrift` 是否可以通过把同面切向漂移阈值从 `0.10` 小幅提高到 `0.12` 来减少 drop，同时仍不暴露 stale tangent impulse。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture`：通过，13 passed；大切向滑移行为锁仍能拦住 stale tangent impulse。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：测试进程通过，但 8x6 stress 明确退化。
  - aligned `4x3`：hash、penetration、quiet-window 与保留基线一致，说明小矩阵对 `0.12` 不敏感。
  - staggered `8x6`：final hash 改为 `f3290f8fbc528baf`；penetration 从 `0.041646 / 1.720375` 退化到 `0.056071 / 1.720375`；late linear spike 从 `3.430820` 退化到 `3.489508`；虽然 angular 从 `5.230875` 小降到 `4.993892`、late churn 从 `47` 降到 `43`，但穿透和 linear blocker 不满足稳定目标。
- 结论：
  - 单纯放宽切向 drift 阈值不是可保留修复；它能让部分 late churn/drop 指标看起来改善，但会保留不该继承的支撑几何，导致 8x6 穿透和 late linear slip 变差。
  - E4 后续不应继续扩大 `DroppedPointDrift` 的接受窗口；更合适的是改善 contact point identity / clipped point persistence，或者在 velocity/position 层对 topology-neighborhood 的 tangent/rotation 能量做受控处理。

### 2026-05-06 - E4 负向实验：Normal/Tangent 分 phase velocity solve

- 状态：已回滚，保留原 interleaved normal+tangent row solve
- Commit：none
- 目标：验证每个 velocity iteration 内先统一解所有 normal rows、再统一解所有 tangent rows，是否能让摩擦行看到更完整的本轮 normal support，从而降低 dense pressure 的 tangent / rotation noise。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：失败；`stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 的 settled penetration max 回归到 `0.064567566`，超过 D2 hard gate `0.04`。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：测试进程通过，但 aligned 与 8x6 都明显退化。
  - aligned `4x3`：final hash `77e4fe34bc6faef5`；quiet-window max linear/angular 从 `0.110800 / 0.395341` 退化到 `0.293929 / 0.633488`。
  - staggered `8x6`：final hash `2c1c22714a9a8906`；虽然 penetration 小降到 `0.040430 / 1.695759`、churn peak 降到 `62`，但出现 floor ejection，BodyHandle(48) frame `83` 出界，frame `179` velocity spike 达 `15.647870 / 10.519542`。
- 结论：
  - 分 phase velocity solve 会让局部穿透/churn 指标看起来改善，但破坏小规模 behavior lock，并重新引入 8x6 ejection。
  - 当前 interleaved normal+tangent row order 是重要稳定边界；E4 后续不应继续做全局 row phase reorder，而应在原顺序内做局部、可诊断的 contact persistence 或 topology-neighborhood 控制。

### 2026-05-06 - E4 证据口：Late spike support-gap trace

- 状态：已完成，未改变物理行为；已升级为结构化行为锁
- Commit：none
- 目标：补齐 late velocity spike 里的支撑断裂证据。此前 late trace 只记录 spike frame 和 last contact；这次新增 `late_support_gap`，记录 blocker body 当前无接触时，支撑缺口从哪一帧开始、持续多少帧、缺口起点速度、spike 速度，以及缺口前最后一个 contact 的支撑事实。
- TDD：
  - 在 `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 先要求 markdown 包含 `late_support_gap start frame`。
  - `LateVelocityTrace` 增加 `support_gap_summary`，并在 `late_velocity_trace` 中从 blocker frame 向前寻找包含该 frame 的 no-contact gap。
  - 追加结构化断言：`support_gap_start_frame == Some(165)`、`support_gap_duration == 10`、缺口前最后 contact 的 normal impulse 为 `0.0`，且 separating normal speed 为正。
- 产出：
  - `Late linear spike trace` / `Late angular spike trace` 均输出 `late_support_gap ...`；有接触的 blocker 显示 `none`。
  - `LateVelocityTrace` 保留结构化 support-gap 字段，后续 solver 实验可以直接比较 gap 起点、持续时间和前一帧支撑 contact 的 normal impulse / normal speed。
  - `late_support_gap` 进一步输出缺口前最后 contact 的 `correction_depth` 与 `correction_translation`，用于区分“position correction 推开支撑”与“浅分离接触没有被位置修正消费”。
  - D2 验收模板补充 late support-gap 摘要。
- 验证：
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：先 RED，缺少结构化 support-gap 字段；实现后通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed，1 ignored。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy git diff --check`：通过。
- 最新证据：
  - aligned `4x3`：late linear / angular blocker 仍有 contact，因此 `late_support_gap none`。
  - staggered `8x6` late linear blocker BodyHandle(32)：frame `174` 无接触；support gap 从 frame `165` 开始，持续 `10` 帧，速度从 `1.984411 / 4.953229` 增到 `3.430820 / 4.953229`。
  - 缺口前最后 contact 是 frame `164` 对 BodyHandle(24)，depth `0.001160`，warm-start `0.000000 / 0.007035`，normal/tangent impulse `0.000000 / 0.001728`，initial normal speed `0.390689`，post normal speed `0.337866`，tangent speed `0.756183`，bias `0.000000 / 0.000000`，`correction_depth 0.000000`，`correction_translation 0.000000`。
  - 同一 frame `164` 的 blocker speed 为 `1.826026 / 4.953229`，counterpart BodyHandle(24) speed 只有 `0.007595 / 0.017564`。
  - tangent component 分解显示，blocker 对该 contact tangent speed 的贡献为 `linear 1.799020 / angular 1.031557`，counterpart 贡献只有 `0.006272 / 0.004983`。
- 结论：
  - 8x6 late linear spike 的关键断点是 frame `164 -> 165` 的动态支撑缺口，而不是 spike frame `174` 本身；body 已经带着高角速度进入无接触状态。
  - 最后一帧支撑 contact 没有 normal impulse，也没有被 residual position correction 消费；counterpart 近似静止而 blocker 已高速旋转/滑移，这排除了“late correction 把支撑推出去”或“支撑体主动滑走”的主因。下一步应聚焦 blocker-side tangent linear + angular 能量如何在浅动态支撑中被降低，而不是继续增加 late-frame friction、sleep 或全局 velocity row reorder。

### 2026-05-06 - E4 证据口：Late support-gap energy onset

- 状态：已完成，未改变物理行为；补充上一条 support-gap 行为锁
- Commit：none
- 目标：把 late support gap 的能量来源从“最后 contact frame”继续前移，定位 blocker-side tangent linear + angular 能量第一次同时超过阈值的 contact frame。这样后续优化可以瞄准 gap 前仍有接触的可控窗口，而不是只在 frame `174` 或 gap 后救火。
- TDD：
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 要求 markdown 包含 `energy_onset frame`。
  - `LateVelocityTrace` 增加结构化 onset 字段，断言 onset frame 必须早于 support-gap start frame，且 blocker-side tangent linear / angular components 分别超过 `1.0` / `0.5`。
  - 同时断言 counterpart tangent linear / angular components 仍小于 `0.05`，onset contact 仍有 normal impulse，且 normal speed 仍略为 closing。
- 验证：
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed，1 ignored。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy git diff --check`：通过。
- 最新证据：
  - staggered `8x6` late linear blocker BodyHandle(32) 的 energy onset 出现在 frame `159`，早于 support gap start frame `165`。
  - onset contact 的 blocker tangent components 为 `linear 1.035253 / angular 0.912949`，counterpart tangent components 只有 `0.022849 / 0.000052`。
  - onset contact 仍有 normal impulse `0.003774`，normal speed min `-0.000522`，depth `0.009087`；到 frame `164` 才变成 shallow separating / normal impulse zero，frame `165` 丢失支撑。
- 结论：
  - 8x6 late spike 不是 gap 后瞬间爆发，而是在 frame `159..164` 的浅动态支撑窗口中逐步失控；这个窗口内 contact 仍存在、counterpart 仍近似静止、normal row 仍能工作。
  - 下一步稳定化应优先设计 contact lifecycle / position-level re-evaluation：在 contact 仍存在时抑制 blocker-side tangent/rotation 滑脱并保持浅支撑几何一致性。继续做全局摩擦、sleep threshold、row phase reorder 或 gap 后 damping 都不是主线。

### 2026-05-06 - E4 证据口：Late support-gap lifecycle

- 状态：已完成，未改变物理行为；补充 onset 到 gap 前最后支撑 contact 的逐帧生命周期
- Commit：none
- 目标：把 frame `159..164` 的 support lifecycle 锁成可比较证据，确认 normal impulse、position correction 和 separating normal speed 是如何在 contact 仍存在时逐步失效的。
- TDD：
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 要求 markdown 包含 `lifecycle [`。
  - `LateVelocityTrace` 增加 `support_gap_lifecycle` 结构字段，断言 lifecycle 第一帧等于 energy onset frame，最后一帧等于 last contact frame。
  - 追加结构化断言：onset frame 同时有 normal impulse 和 position correction；随后存在 normal impulse 已归零但 position correction 仍运行的中间窗口；再随后存在 normal impulse 与 position correction 都归零且 normal speed separating 的窗口。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过。
- 最新证据：
  - frame `159`：depth `0.009087`，normal impulse `0.003774`，normal speed `-0.000522`，body tangent `1.035253 / 0.912949`，position correction `0.088063 / 0.003523`。
  - frame `160..162`：normal impulse 已为 `0.000000`，normal speed 从 `0.003486` 升到 `0.066409`，但 position correction 仍从 `0.072390` 降到 `0.033751`。
  - frame `163..164`：normal impulse 和 position correction 都为 `0.000000`，normal speed 从 `0.195462` 升到 `0.337866`，body tangent 从 `1.634928 / 0.993750` 升到 `1.799020 / 1.031557`，frame `165` 断支撑。
- 结论：
  - 可控窗口不只是 “gap 前一帧”：normal impulse 在 frame `160` 就丢失，position correction 在 frame `163` 停止，之后仍有两帧 shallow separating contact 才丢失支撑。
  - 下一步 solver 设计应聚焦 contact lifecycle / position-level re-evaluation：在 contact 仍存在但 normal impulse 已丢失时重新评估浅支撑几何与 correction eligibility，防止 blocker-side tangent/rotation 把支撑滑穿。仅增加 gap 后阻尼、全局 friction 或 sleep 仍不是主线。

### 2026-05-06 - E4 负向实验：Dense tangent-risk shallow correction slop

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证是否可以只在 dense + shallow + separating + high-tangent-risk contact 上降低 position correction slop，而不是重复全局 slop tightening。实验条件为 depth 在 `0..POSITION_CORRECTION_SLOP` 内、normal impulse 已归零、final normal speed separating、final tangent speed `>= 0.5`，然后把 correction slop 降到 `0.001`。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败，8x6 stress 退化。
  - aligned `4x3`：保持基线，final hash `ffe4aedd8eb04c93`。
  - staggered `8x6`：final hash 变为 `b917dca418dd7a76`；penetration 从 `0.041646 / 1.720375` 退到 `0.056047 / 1.710114`；sleep 从 `47 / 1` 退到 `48 / 0`；late angular spike 从 `5.230875` 升到 `9.929781`；support gap 提前到 frame `164 duration 1`；并重新出现 floor ejection：BodyHandle(48) frame `177` 出界、final outside `1`。
- 结论：
  - 即便只针对 dense high-tangent shallow contact 降低 correction slop，仍会保留/放大错误浅支撑并把能量推到角速度与 ejection 上。
  - 后续不能把 lifecycle 证据简单翻译为“更激进地修正浅接触”。更可行的方向应是完整 position-level re-evaluation：重新评估 contact separation / anchor / support eligibility，并把 retained support 与 tangent/rotation energy gate 绑定，而不是只改 slop 常量。

### 2026-05-06 - E4 下一步设计门：Position-level re-evaluation

- 状态：已记录，作为下一次 solver 实现前的 Plan Gate；未改变运行时行为
- Commit：none
- 背景：截至目前，以下方向均已失败或只能作为有限桥接：全局/slop 局部收紧、previous-pair speculative row、global SAT skin、velocity row reorder、normal/tangent 分 phase、dominant-body damping、更大 shallow friction budget、batched correction。它们共同说明问题不是单个常量，而是 shallow support lifecycle 的几何有效性需要在 position level 重新评估。
- 设计输入：
  - 在 `docs/design/matrix-stack-stability-optimization-design.md` 增加 `E4 Position-Level Re-Evaluation Gate`。
  - 明确目标行为：dense contact graph 中，contact 仍存在但 normal impulse lost + separating + blocker-side tangent/rotation risk 时，必须用 pseudo-position state 重新评估 support eligibility，而不是仅复用 frame-start depth 和 slop。
  - 明确边界：不改 public API、不改 velocity row identity、不改 global row phase、不改全局 SAT skin、不用 sleep/freeze 掩盖、不做 gap 后无几何 damping。
- 下一次实现必须满足的 hard gate：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`
  - aligned `4x3` final hash / penetration / quiet-window 不退化。
  - staggered `8x6` final outside floor bodies 保持 `0`，support gap 不早于 frame `165`，duration 不长于 `10`。
  - staggered `8x6` penetration max 不高于 current baseline `0.041646`，late angular spike 不高于 `5.230875`；若某项 tradeoff，必须同时有 late linear / churn / sleep 的明确改善并单独记录。
  - `rtk proxy git diff --check`
- 结论：
  - 下一次代码实现应从 position-row / pseudo-state re-evaluation 的最小可回滚切片进入，而不是继续追加参数实验。
  - 如果该 gate 仍不能改善 8x6，应把 `matrix_stack 8x6` 提升为下一阶段 pressure solver / contact manifold lifecycle 计划，而不是继续在当前 residual correction 上局部加补丁。

### 2026-05-06 - E4 证据口：Support eligibility oracle

- 状态：已完成，未改变物理行为；把 position-level re-evaluation gate 转成只读 oracle
- Commit：none
- 目标：在真正改 solver 前，先用当前 debug facts 判断 frame `159..164` 的 support contact 会被 retained 还是 rejected。这样下一次实现有明确的 eligibility story，而不是直接把 lifecycle 证据翻译成参数补丁。
- TDD：
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 要求 markdown lifecycle 包含 `eligibility`。
  - `SupportGapLifecycleFrame` 增加 normal/tangent anchor drift 和 `eligibility_decision`。
  - 结构化断言：lifecycle 中必须同时出现 `retain_candidate_correction_active` 和 `retain_candidate_needs_position_re_evaluation`，且当前窗口不应出现 `reject_*` decision。
- 最新证据：
  - frame `159`：`observe_normal_impulse_present`，normal impulse 仍在。
  - frame `160..162`：`retain_candidate_correction_active`，normal impulse 已归零但 residual correction 仍在；normal anchor drift 约 `0.000000..0.005012`，tangent anchor drift 约 `0.000000..0.024873`。
  - frame `163..164`：`retain_candidate_needs_position_re_evaluation`，normal impulse 与 correction 均归零，但 anchor drift 仍低：normal `0.005097 / 0.004282`，tangent `0.027792 / 0.029733`。
- 结论：
  - 当前 late support-gap 的关键 contact 不应被第一版 anchor/counterpart oracle 拒绝；它更像“应该保留但需要 position-level re-evaluation 的浅支撑”。
  - 下一次实现应瞄准 frame `163..164` 这类 retained candidate：通过 pseudo-position state 重新评估 separation / support eligibility 并恢复受控 correction，而不是降低全局或局部 slop。

### 2026-05-06 - E4 负向实验：Retained support tiny correction budget

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证能否把 eligibility oracle 的 retained candidate 直接转为一个很小的 position correction budget。实验只在 dense、原 residual correction 已因 slop 停止、normal impulse 为 0、normal speed separating、tangent speed 高、anchor drift 低、且只有一侧 body 有明显 tangent linear+angular energy 时，为该 contact 提供最多 `0.001` 的 correction depth。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败，8x6 stress 退化。
  - aligned `4x3`：保持基线，final hash `ffe4aedd8eb04c93`。
  - staggered `8x6`：final hash 变为 `29c751145a4e07aa`；penetration 从 `0.041646 / 1.720375` 退到 `0.048240 / 1.707533`；support gap 从 frame `165 duration 10` 提前到 frame `150 duration 10`；late linear spike 变为 frame `159` 的 `3.354937 / 4.990017`；late angular spike `5.234621` 略高于 baseline `5.230875`；floor ejection 仍为 none。
- 结论：
  - 即便 eligibility gate 选中了“应保留”的 contact，直接给 retained candidate 小 correction budget 仍然太粗，会提前支撑断裂并增加 penetration。
  - 下一步不能把 oracle decision 直接映射成 correction depth。真正的 position-level re-evaluation 需要先重新计算 pseudo-state 下的 separation / anchors / support normal，并验证 retained support 不改变 gap start，不应只在旧 contact normal 上追加 correction。

### 2026-05-06 - E4 负向实验：全局 SAT speculative contact skin

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证 Box2D/Matter 风格的薄 contact skin 是否能让刚刚分离的 polygon face contact 继续生成 support row，从而避免浅动态支撑突然断裂。
- TDD：
  - 在 `pipeline::narrowphase` 增加 `convex_sat_keeps_speculative_skin_contact_for_tiny_face_gap`，要求两个只隔 `0.002` 的矩形仍生成 speculative support contact；当前实现先 RED。
  - 实验实现把 SAT 接受窗口扩到 `0.003`，并让 clipping 在小 separation 内输出浅 depth contact；单元测试转绿。
- 观测：
  - `rtk proxy cargo test -p picea --lib pipeline::narrowphase::tests::convex_sat_keeps_speculative_skin_contact_for_tiny_face_gap -- --nocapture`：实验实现后通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：失败；`stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 的 quiet linear window 回归到 `0.11522542`，超过 D2 hard gate。
  - 回滚后 `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
- 结论：
  - 全局 speculative skin 不能作为默认 narrowphase 语义；它会让小规模稳定堆叠也持续保留过多支撑 row，破坏 quiet gate。
  - 该方向若继续，只能做 dense / previous-pair / drift-gated 的 contact persistence，而不是改所有 SAT contact。

### 2026-05-06 - E4 负向实验：Dense previous-pair speculative support row

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：比全局 narrowphase skin 更窄地验证 contact persistence：只在 dense candidate graph 中、broadphase 仍给出 pair、上一帧同一 collider pair 存在 contact、当前 narrowphase drop 且上一帧 anchors 仍在 `0.003` support skin 内时，合成 previous-pair support row。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败，8x6 stress 明确退化。
  - aligned `4x3`：保持基线，final hash `ffe4aedd8eb04c93`，说明 dense gate 没有影响小矩阵。
  - staggered `8x6`：late support gap 从 `start frame 165 duration 10` 变为 `start frame 142 duration 3`，说明 previous-pair persistence 确实命中支撑断裂；但 final hash 变为 `aaaaba5674e1e90f`，penetration 退到 `0.093308 / 1.786192`，sleep 退到 `48 / 0`，late angular spike 升到 `5.920810`，并重新出现 floor ejection：BodyHandle(48) frame `158` 出界、final outside `1`。
  - 回滚后 `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，恢复保留基线 `ccb1b01fb65b2231`，floor ejection `none`，final outside `0`。
- 结论：
  - naive previous-pair persistence 是“方向命中但约束不足”：它能缩短 support gap，却把错误浅支撑也保留下来，导致 penetration 与 ejection 回归。
  - 后续如果继续 contact persistence，必须先增加更强的结构条件，例如低 tangent/rotation speed、normal/tangent anchor drift gate、不会增加 residual correction pressure 的局部拓扑约束；不能只按 dense graph + previous pair + slop 合成 row。

### 2026-05-06 - E4 收尾修补：ContactEvent fixture 补齐 position-correction 字段

- 状态：保留；测试 fixture 修补，不改变运行时行为
- Commit：none
- 背景：运行 `rtk proxy cargo test -p picea --lib ...` 时发现 `pipeline.rs` 内的 `ContactEvent` fixture 缺少本轮新增的 `solver_position_correction_depth` / body translation 字段，导致 lib test 编译失败。
- 处理：给 `step_advances_pipeline_and_preserves_world_event_order` 的 fixture 补齐三个字段，值均为 `0.0`。
- 验证：
  - 后续 `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture` 与 `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture` 均重新编译通过。

### 2026-05-06 - E4 负向实验：Dominant body tangent damping

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证 blocker-side 局部 tangent/rotation energy gate 是否能改善 late support gap。实验只在 dense / shallow / separating row 上启用，并且只有某一侧 contact-point tangent speed 明显大、counterpart 近似静止时，才对 dominant body 施加受 `support_friction_impulse * 0.5` 限制的 tangent damping impulse。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败，8x6 stress 退化。
  - aligned `4x3`：保持基线，final hash `ffe4aedd8eb04c93`，说明该 gate 未影响小矩阵。
  - staggered `8x6`：final hash 变为 `0e99801ae8c70141`；floor ejection 仍为 none，但 penetration 退到 `0.052787 / 1.673334`，late angular spike 从 `5.230875` 升到 `5.335954`，late support gap 从 `start frame 165 duration 10` 变为 `start frame 148 duration 11`，即支撑断裂提前且没有缩短。
  - 回滚后 `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，恢复保留基线 `ccb1b01fb65b2231`。
- 结论：
  - 每轮局部 dominant-body tangent damping 不是可保留修复；它没有减少 support gap，反而把 gap 提前并增加 penetration / angular spike。
  - 后续如果继续 blocker-side energy control，不能直接在 velocity iteration 中追加 damping impulse。更稳妥的方向是先设计 contact-persistence gate 或 position-level re-evaluation，让浅支撑几何不突然断裂；若要做 damping，也必须绑定更强的拓扑/接触生命周期条件，并以 late support gap duration、penetration 和 angular spike 三项同时不退化为 hard gate。

### 2026-05-06 - E4 负向实验：True pseudo-pose re-evaluation tiny correction

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：比 `Retained support tiny correction budget` 更接近 position-level re-evaluation：当 dense contact graph 中旧 residual correction 已因 slop 停止、normal impulse 为 `0`、normal speed separating、anchor drift 仍低且 blocker-side tangent/rotation energy 明显时，用本 pass 内累计 pseudo translation 重新跑一次窄相 contact，只有 re-evaluated normal 与旧 normal 一致且仍有正 depth 时，才给最多 `0.001` 的 correction depth。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败，8x6 stress 退化，且结构化验收因报告不再包含 `late_support_gap start frame` 而红。
  - aligned `4x3`：保持基线，final hash `ffe4aedd8eb04c93`。
  - staggered `8x6`：final hash 变为 `ad0c3d611e5a053b`；penetration 从 `0.041646 / 1.720375` 退到 `0.048148 / 1.728789`；warm-start drop 从 `27` 增至 `31`；late linear spike 变为 frame `176` 的 `2.117191 / 1.610476`；late angular spike 变为 frame `179` 的 `0.966288 / 4.329919`；late correction max/total `0.052354 / 0.717238`，floor ejection 仍为 none，但原 support-gap failure 形态被替换成有接触状态下的晚期速度/修正尖峰。
- 结论：
  - 只在当前 residual correction pass 里临时重跑 narrowphase 仍然不是可保留方案：它会改变 failure shape，却没有让 8x6 进入稳定收敛，还破坏了已有结构化 blocker 锁。
  - 下一步不要继续把 position-level re-evaluation 塞进旧 residual correction。需要把 E4 方案提升为真正的 position-row pass：独立 pseudo pose、每轮重新生成/更新 position constraints、受控 row ordering、明确 writeback，并把 retained-support 对 correction total / churn / angular spike 的影响作为硬门。

### 2026-05-06 - E4 小步实现：Deferred position-row writeback

- 状态：保留；8x6 stress 有小幅改善，但仍未稳定
- Commit：none
- 目标：把旧 residual contact position correction 的输入抽成显式 `ContactPositionRow`，并把 position correction 从“每个 row 立即写回 pose”改成“在 pseudo-position state 中累计 translation，最后统一写回 pose”。这是完整 position-row / pseudo-pose pass 前的最小可验收切片。
- 实现：
  - 在 `crates/picea/src/solver/contact.rs` 增加 `ContactPositionRow`，记录 contact index、body handles、normal、inverse mass、frame-start residual depth 和 raw depth。
  - `apply_residual_contact_position_correction` 先构建 row carrier，再按原顺序、原 depth 口径和原 dense pseudo-translation 口径累计 correction。
  - 增加 `QueuedPositionTranslation`，记录每个 body 的净 translation、逐 row distance sum 和 sleep idle reset 条件，最后统一 writeback。
  - 不改变 public API、不改变 velocity row solve、不改变当前 dense threshold；sleep idle reset 仍按“原 body sleeping 或单次 correction 超过 slop”记录，而不是按最终净位移重新解释。
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 的结构化断言从锁死旧 support gap exact frame，改成 E4 acceptance：support gap 不早于 frame `165`、duration 不长于 `10`、penetration / late linear / late angular / late correction 不退化、无 floor ejection，并保留 lifecycle / eligibility 证据。
- 验证：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy git diff --check`：通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed，1 ignored。
- 最新证据：
  - aligned `4x3`：behavior lock 通过；final hash 变为 `7c6fc92e984d464c`，penetration 仍为 `0.019898 / 0.299988`，quiet-window linear/angular 为 `0.108622 / 0.393492`，低于旧基线 `0.110800 / 0.395341`。
  - staggered `8x6`：final hash 变为 `829306c4886125fd`；penetration 从 `0.041646 / 1.720375` 降到 `0.041296 / 1.676269`；quiet-window linear/angular 从 `3.430820 / 5.230875` 降到 `3.268942 / 4.568931`；late correction total 从 `0.739580` 降到 `0.716224`；support gap 从 frame `165 duration 10` 推迟到 frame `171 duration 9`；floor ejection 仍为 none，final outside floor bodies 仍为 `0`。
  - lifecycle 发生了可解释变化：frame `167..169` 出现 `reject_counterpart_motion`，说明 deferred position writeback 把 blocker 推迟到了一个 counterpart motion 已进入 gate 的窗口；最后 frame `170` 仍有 `retain_candidate_needs_position_re_evaluation`，后续仍需要完整 position-level re-evaluation / contact lifecycle 设计。
- 预期后续：
  - 这一步不是稳定完成；`8x6` 仍有 first bad frame、late velocity spike、sleep 未收敛。下一步若继续实现 E4，应在这个 carrier 后面增加独立 pseudo pose / per-iteration position-row re-evaluation，而不是再直接修改旧 residual correction 的 contact loop。

### 2026-05-06 - E4 负向实验：Retained shallow-support position row on deferred writeback

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证 deferred position-row writeback 后，是否可以比旧 `Retained support tiny correction budget` 更安全地处理浅支撑。实验只在 dense、contact depth 小于 slop、normal impulse 为 `0`、final normal speed separating、anchor drift 低、且单侧 body tangent linear+angular energy 明显时，给最多 `0.00025` 的 retained-support correction depth。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败，8x6 stress 退化。
  - aligned `4x3`：保持 deferred writeback 基线，final hash `7c6fc92e984d464c`。
  - staggered `8x6`：final hash 变为 `a843eec331d879ff`；penetration 退到 `0.046520 / 1.689001`；late angular spike 升到 `5.393579`，高于 deferred writeback 的 `4.568931` 和旧 baseline 的 `5.230875`；support gap 从 frame `171 duration 9` 提前到 frame `157 duration 9`；floor ejection 仍为 none。
- 结论：
  - 即使 correction cap 降到 `0.00025` 并使用 deferred writeback，直接把 shallow retained contact 转成 position row 仍会提前支撑断裂并放大角速度。
  - 后续不能再把 retained-support oracle 直接映射成 correction depth。下一步需要真正的 per-iteration separation / support eligibility re-evaluation，或者 contact manifold lifecycle 设计；浅 contact 是否保留必须由 pseudo-state 下的几何有效性决定，而不是只靠 tangent energy gate。

### 2026-05-06 - E4 负向实验：Per-iteration narrowphase re-evaluation for existing position rows

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证只对已经进入 position correction 的 row 做 pseudo-state 下的 narrowphase re-evaluation，是否比旧的 normal-projection depth 更稳定。实验不新增 shallow retained contact；若 pseudo narrowphase 无法返回同向 manifold，则 fallback 到 deferred writeback 的 projected depth。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败，8x6 stress 严重退化。
  - aligned `4x3`：behavior lock 仍通过，但 quiet-window linear/angular 退到 `0.324306 / 0.494581`，明显高于 deferred writeback 基线 `0.108622 / 0.393492`。
  - staggered `8x6`：final hash 变为 `8da766cd91261077`；penetration 虽降到 `0.037364 / 1.643677`，但 sleep 退为 `48 / 0`，late linear/angular spike 升到 `16.546148 / 11.421408`，support gap 从 frame `171 duration 9` 退到 frame `85 duration 95`，并重新出现 floor ejection：BodyHandle(48) frame `82` 出界，final outside floor bodies `2`。
- 结论：
  - 对已有 position rows 直接重跑 narrowphase 会让几何修正过度追随局部 manifold，虽然降低 penetration，但把系统推成高速 ejection，不符合稳定性目标。
  - 下一步不能在 solver correction loop 中直接调用 full narrowphase 作为每轮 separation 口径。更可行的路线是先做轻量 position constraint geometry：固定 contact feature / local anchors / support normal 的局部 separation re-evaluation，或者先重构 contact manifold lifecycle，让 narrowphase persistence 在 solver 前就稳定，而不是在 solver 内临时重算。

### 2026-05-06 - E4 证据口：600-frame long-settle observation

- 状态：已完成，未改变物理行为；新增 ignored diagnostic observation
- Commit：none
- 目标：验证 deferred position-row writeback 的 180 帧改善是否能外推到更长 settle window，避免把短期 support-gap 推迟误判为稳定。
- 实现：
  - 在 `crates/picea-lab/tests/artifact_run.rs` 增加 ignored test `matrix_stack_long_settle_observation_reports_residual_e4_risk`，运行 `matrix_stack` 600 帧并输出同一 `MatrixStackStressReport`。
  - 该测试是 diagnostic observation，不作为普通 suite hard gate；它只要求当前 8x6 仍暴露 first-bad-frame，并在 E4/E5 完成前保留 sleep non-convergence 或 floor-support failure 证据。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_observation_reports_residual_e4_risk -- --ignored --nocapture`：通过。
- 观测：
  - long-settle final hash `0edcf3346e9d846c`。
  - 600 帧窗口下 penetration max/sum `0.082507 / 1.676269`，sleep `48 / 0`。
  - late linear/angular spike 升到 `62.754536 / 14.401797`。
  - support gap 从 frame `229` 开始，duration `371`。
  - 重新出现 floor ejection：first exit frame `214`，BodyHandle(41)，final outside floor bodies `7`。
- 结论：
  - 180 帧 deferred writeback 是短窗口改善，不是稳定完成；E4 不能转入 E5 sleep 收敛。
  - 下一步必须继续处理 contact lifecycle / dynamic support loss / manifold persistence；sleep threshold 或 long-window idle tuning 只能在长窗口 no-ejection 后再做。

### 2026-05-06 - E4 负向实验：Residual-penetration support friction budget

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证长窗口 frame `214` 这类“仍有残余穿透、法向速度已分离、法向 impulse 为 `0`”的接触，是否可以通过 dense contact graph 中的残余穿透摩擦预算减少切向滑移。实验把 `solver_support_friction_impulse` 从 shallow `depth <= slop` 扩大到 dense 图里的所有 separating residual-penetration contacts。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` hard gate 未被打破。
  - `rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --nocapture`：通过，但 aligned `4x3` final hash 变为 `6fd9130abc22f09d`，dense support friction 变为 active。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：失败，8x6 stress 严重退化。
  - staggered `8x6`：final hash 变为 `a6c3b6b816376813`；penetration 退到 `0.063458 / 1.734315`；quiet-window linear/angular 退到 `16.483606 / 7.274910`；support gap 从 retained deferred-writeback baseline 的 frame `171 duration 9` 退到 frame `82 duration 98`；重新出现 floor ejection，BodyHandle(48) 在 frame `79` 出界，final outside floor bodies `1`。
- 结论：
  - 把 residual penetration 直接映射成额外 friction budget 会把支撑断裂提前，并把短窗口 8x6 从 no-ejection 退回 ejection；这不是可保留修复。
  - 后续不能用“更大 support friction budget”替代 contact lifecycle / geometry validity。摩擦预算必须继续保持 shallow、诊断性的小补丁；真正的稳定性方向仍是 position-row 几何有效性或 manifold persistence，而不是用 friction 吸收已形成的深穿透/高速分离。

### 2026-05-06 - E4 负向实验：Position-row angular correction

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证 Box2D-style position row 是否应从纯 translation correction 扩展到 contact-point effective mass，并把 correction impulse 的角度分量写回 pose。实验在 `ContactPositionRow` 中加入 contact anchors / inverse inertia，pseudo-position depth 也计入 angular point displacement。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed，说明局部 position-correction 单测未覆盖堆叠退化。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：失败；`stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 的 settled penetration 退到 `0.060225487`，超过 D2 max-depth hard gate。
- 结论：
  - 直接把所有 position rows 改成 angular correction 会破坏小规模稳定堆叠；不能作为默认 E4 修复。
  - 后续如果需要角度层面的 position correction，必须先加更窄的 gate：只对确定的 feature/local-anchor constraints、生效帧、角度上限和 penetration/churn 不退化条件启用，且 `stack_4` 必须作为第一硬门。

### 2026-05-06 - E4/E5 future acceptance gate：600-frame no-ejection / no-runaway

- 状态：已新增；默认 skipped，显式 env 下作为未来红灯
- Commit：none
- 目标：把“8x6 从诊断 stress failure 推进到可验证稳定状态”落成可执行验收门，而不是只依赖 markdown 报告。该 gate 仍保持 ignored，避免当前未完成 E4 时污染普通 test suite；只有设置 `PICEA_MATRIX_STACK_E4_ACCEPTANCE=1` 才强制执行。
- 实现：
  - 在 `crates/picea-lab/tests/artifact_run.rs` 新增 ignored test `matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed`。
  - 默认运行该 test 会打印 skip 并通过；显式设置 env 后运行 600 帧 `matrix_stack`，要求：
    - `first_floor_exit_frame == None`
    - `final_outside_floor_body_count == 0`
    - `max_penetration_depth <= 0.05`
    - `quiet_window_max_linear_speed <= 4.0`
    - `quiet_window_max_angular_speed <= 6.0`
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture`：通过，默认 skipped。
  - `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture`：预期失败；当前 retained baseline 仍在 `first_floor_exit_frame = Some(214)` 红，quiet linear 仍为 `62.754536`，说明 goal 未完成。
- 结论：
  - 这条 gate 不是当前完成声明；它是后续 E4b/E5 的硬验收入口。只有它在 env 强制模式下通过，才允许把 8x6 长窗口写成稳定。

### 2026-05-06 - E4b 负向实验：Simple pair-level warm-start fallback

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证在当前 narrowphase 已经生成 contact row 时，是否可以对同 collider pair、同 `reduction_reason`、normal / anchor drift 检查通过的上一帧 contact 做 cache fallback，减少 `MissFeatureId` 对动态支撑的破坏。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache_reports_feature_id_miss_when_pair_persists_on_different_features -- --nocapture`：通过，说明跨几何语义的 feature-family miss 行为锁仍在。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：失败；180-frame penetration 改善到 `0.037937 / 1.676259`，quiet linear 改善到 `2.264416`，support gap 改善到 frame `178 duration 2`，但 quiet angular 为 `4.690789`，仍高于当前 hard gate。
  - feature churn trace 从 `miss_feature_id 418 same_pair_previous 418 close_world_point 387 late_same_pair_previous 152` 降到 `miss_feature_id 156 same_pair_previous 156 close_world_point 120 late_same_pair_previous 65`，说明方向上确实减少了 feature-id churn。
  - `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture`：失败；600-frame first floor exit 推迟到 frame `224`，但 final outside floor bodies 仍为 `5`，quiet linear/angular 为 `64.363495 / 10.791191`。
  - 回滚后 `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture` 通过；`rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture` 通过，恢复 retained baseline final hash `829306c4886125fd`；显式 600-frame future gate 仍按预期红在 frame `214`，final outside floor bodies `7`，quiet linear `62.754536`。
- 结论：
  - 简单 pair-level fallback 是有信号但不可保留的中间结果：它改善短窗口 feature churn、penetration、linear speed 和 support gap，却没有让 180-frame hard gate 或 600-frame future gate 转绿。
  - 后续 E4b 不能只用 pair + reduction reason + drift 做 fallback。需要显式 shape/feature-family、support-normal continuity、动态支撑能量和长窗口 no-ejection gate，且 fallback 前后的 feature churn 统计仍必须可见。

### E4b - Contact lifecycle / manifold persistence 子里程碑

- 状态：进行中；lifecycle evidence 已增强，尚未启用 fallback 行为
- 目标：在不合成新 contact、不扩大 friction、不改 sleep 的前提下，提升同一 collider pair 的 contact lifecycle 稳定性，减少 feature churn / cache miss 对动态支撑的破坏。
- 范围：
  - 先补诊断事实：区分 `MissFeatureId` 的 pair-level churn、local-anchor drift、support-normal continuity，以及 shape/feature-family 是否发生语义变化。
  - 再做受约束 fallback：只在当前 narrowphase 已经生成 contact row、同 shape family、同 support normal、local anchor drift 低、上一帧 impulse finite 且非 sensor 时，允许从同 pair 的上一帧 contact transfer warm-start cache。
  - fallback 只能改变 cache transfer，不允许合成 speculative row，也不改变 velocity / position solver 顺序。
- 非目标：
  - 不跨 shape family 迁移 impulse；已有 `warm_start_cache_reports_feature_id_miss_when_pair_persists_on_different_features` 行为锁必须继续保护该边界。
  - 不把 `MissFeatureId` 全部改成 `Hit`；报告必须继续保留 fallback 前后的 churn 可见性。
  - 不用 sleep threshold 或 body freeze 掩盖长窗口 runaway。
- 验收：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start -- --nocapture`
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`
  - `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture` 必须最终从当前红灯转绿，才允许进入 E5。

### 2026-05-06 - E4b 小步实现：Feature churn shape/local-anchor evidence

- 状态：保留；只增加 artifact/report 诊断，不改变物理行为
- Commit：none
- 目标：把 E4b 的 fallback 输入证据从“同 pair + world point close”强化为“同 pair + shape signature compatible + collider-local anchor close”，避免后续重走 simple pair-level fallback 的老路。
- 实现：
  - `FeatureChurnTrace` 增加 `same_shape_signature_count`、`close_local_anchor_count` 和 `best_local_anchor_drift`。
  - shape signature 由 debug collider 的 shape family、polygon edge-length signature、circle radius 或 segment length/radius 量化得到；local anchor drift 把当前/上一帧 contact point 投到各自 collider-local frame 后比较。
  - 报告行新增 `same_shape_signature`、`close_local_anchor` 和 `local_anchor_drift`，结构化 test 也要求这些证据存在。
- 验证：
  - 先加 markdown 行为锁后，`rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture` 预期失败，证明原报告缺少 E4b shape/local-anchor 证据。
  - 实现后同命令通过；180-frame retained baseline 中 feature churn trace 为 `miss_feature_id 418 same_pair_previous 418 close_world_point 387 same_shape_signature 418 close_local_anchor 391 late_same_pair_previous 152`，best local anchor drift 为 `0.000509`。
- 结论：
  - 当前 8x6 的大量 `MissFeatureId` 确实发生在同形状签名且局部锚点连续的接触上，E4b 可以继续设计更窄的 constrained fallback。
  - 这一步不是稳定性修复；600-frame no-ejection gate 仍是红灯，不能进入 E5。

### 2026-05-06 - E4b 小步实现：Same feature-index point-slot fallback

- 状态：保留；安全边界通过，但 8x6 retained baseline 暂无可见改善
- Commit：none
- 目标：在不恢复 simple pair-level fallback 的前提下，只允许 clipped manifold 中“同 feature index、不同 point slot”的 contact 继承上一帧 warm-start cache。这个切片只处理同一几何特征的 point-slot 换位，不跨 feature index，也不合成 solver row。
- 实现：
  - `prepare_contact_warm_start` 在 exact `ContactKey` miss 时，查找同 collider pair、同 `reduction_reason`、同 `ContactFeatureId::index()` 的上一帧 contact。
  - fallback 仍复用 `warm_start_transfer` 的 normal continuity、anchor drift、sensor 和 finite impulse 检查；候选按 anchor drift 最小选择。
  - 新增 unit lock `pipeline::contacts::tests::warm_start_fallback_transfers_between_same_feature_index_point_slots`，证明同 feature index / 不同 point slot 可以转移缓存冲量。
- 验证：
  - 新增 unit lock 先红后绿。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache_reports_feature_id_miss_when_pair_persists_on_different_features -- --nocapture`：通过，跨 feature-family miss 边界仍保留。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture`：通过，13 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed / 3 ignored；`matrix_stack` 180-frame final hash 仍为 `829306c4886125fd`。补充 trace 显示 `miss_feature_id 418 same_pair_previous 418 same_reduction_reason 375 same_feature_index 26 close_world_point 387 same_shape_signature 418 close_local_anchor 391 point_slot_fallback_eligible 0 late_same_pair_previous 152`。
  - `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture`：仍按预期失败；600-frame final hash 变为 `6741fcc79156b200`，first floor exit 仍为 frame `214`，final outside floor bodies 仍为 `7`，quiet linear/angular 仍为 `62.754536 / 14.401797`。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_observation_reports_residual_e4_risk -- --ignored --nocapture`：通过；600-frame trace 显示 `miss_feature_id 1237 same_pair_previous 1237 same_reduction_reason 1101 same_feature_index 110 close_world_point 1125 same_shape_signature 1237 close_local_anchor 1147 point_slot_fallback_eligible 6`，dominant transition 为 `k1:r2:i0<-k1:r0:i2 x532`。
- 结论：
  - 该 fallback 足够窄，未破坏 warm-start / 小栈 / 180-frame matrix gates，但它没有让 8x6 从 retained baseline 继续改善。
  - 后续审计证明真实 8x6 中 point-slot fallback 的适用面为 `0`；主要 churn 不是同 feature index 的 slot 换位，而是同 shape/local-anchor 下的 feature index 变化。
  - 下一步不能继续只从 point-slot 入手；需要把 E4b 转向 SAT clipped reference / incident edge selection 的 manifold persistence，或把 position-row support validity 的几何约束前移，而不是扩大 fallback 范围。

### 2026-05-06 - E4b 负向实验：SAT ordered-pair edge canonicalization

- 状态：实现已回退；只保留 ignored diagnostic red lock
- Commit：none
- 目标：验证 dominant transition `k1:r2:i0<-k1:r0:i2` 是否可以通过把 clipped SAT contact 的 feature id 从“reference/incident edge”改为“ordered collider pair 的 A edge / B edge”来消除。这个思路模拟 Box2D persistent manifold 的一个子问题：SAT 每帧可选择不同 reference face，但 warm-start identity 应尽量跟随同一几何面。
- 行为锁：
  - 在 `crates/picea/src/pipeline/narrowphase.rs` 增加 ignored test `stacked_rectangles_keep_feature_id_when_sat_reference_face_swaps`。
  - 该 test 构造上下相叠矩形的小位姿变化，证明当前 SAT 可在 normal 连续时把 feature index 从 `16785408` 切到 `16777218`。
  - 该 test 默认 ignored，因为直接修绿它的第一版实现会破坏 matrix stack；它是后续 manifold-persistence 设计输入，不是当前默认 suite gate。
- 负向实现：
  - 曾短暂把 `clip_incident_edge` 的 feature id 改为 ordered-pair edge encoding：`PolySource::A => (reference_edge, incident_edge)`，`PolySource::B => (incident_edge, reference_edge)`。
  - 该改动让 feature churn 明显下降，但改变了长链支撑的能量路径。
- 观测：
  - `rtk proxy cargo test -p picea --lib pipeline::narrowphase -- --nocapture`：通过，说明窄单测层面不暴露问题。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture`：通过，13 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败；`matrix_stack` 180-frame final hash 变为 `597b9dadb2770a4c`，penetration `0.075719 / 1.717448`，quiet linear/angular `14.874845 / 5.666668`，first floor exit frame `83`，final outside floor bodies `1`。
  - feature churn trace 从 retained baseline 的 `miss_feature_id 418 same_pair_previous 418 same_reduction_reason 375 same_feature_index 26 close_world_point 387 same_shape_signature 418 close_local_anchor 391` 降到 `miss_feature_id 35 same_pair_previous 35 same_reduction_reason 6 same_feature_index 29 close_world_point 6 same_shape_signature 35 close_local_anchor 8`。
  - Dominant transition 也从 `k1:r2:i0<-k1:r0:i2 x205` 变成 `k1:r0:i2<-k1:r0:i2 x23`，说明 canonicalization 确实打掉了主要 feature-index churn，但稳定性更差。
- 结论：
  - “减少 MissFeatureId / feature churn”不是稳定性的充分条件；直接 canonicalize SAT edge identity 会让旧 impulse 在不同 manifold 角色下进入更危险的支撑链，导致更早 ejection。
  - 后续 E4b 不能做无条件 feature-id canonicalization。更合理的路线是显式持久 manifold candidate：同时保留 reference/incident role、ordered local edges、local anchors、normal continuity、clip point ordering 和长窗口 no-ejection gate，再决定是否只迁移有限 warm-start impulse。

### 2026-05-06 - E4b 证据口：Dominant transition best sample

- 状态：保留；只增加 artifact/report 诊断，不改变物理行为
- Commit：none
- 目标：给 `top_feature_index_transition` 补上具体 frame / pair / feature / point drift / normal dot / local-anchor drift，并单独统计 reference/incident edge swap 形态，避免后续只知道 `k1:r2:i0<-k1:r0:i2` 计数最高，却不知道它是否真的是局部连续接触。
- 实现：
  - `FeatureChurnTrace` 增加 `top_transition_best_frame`、`top_transition_best_pair`、`top_transition_best_current_feature_id`、`top_transition_best_previous_feature_id`、`top_transition_best_point_drift`、`top_transition_best_normal_dot`、`top_transition_best_local_anchor_drift`。
  - `FeatureChurnTrace` 还增加 `edge_swap_transition_count`、`edge_swap_candidate_count` 和 `edge_swap_best_*` 字段，识别 kind 相同且 reference / incident edge 互换的 feature-index churn，并统计同时满足 same reduction / same shape / point+normal+local-anchor continuity 的保守 persistence candidate。
  - `feature_churn_trace` 现在按 transition 维护 count 与 best sample；report line 输出 `top_transition_best frame ...`。
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 要求 markdown 中出现 `top_transition_best frame` / `edge_swap_transition` / `edge_swap_candidate`，且结构化 report 有具体 frame、edge-swap 计数和 candidate 计数。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过。
- 观测：
  - 180-frame retained baseline 仍为 final hash `829306c4886125fd`，无 floor ejection，dominant transition 仍为 `k1:r2:i0<-k1:r0:i2 x205`。
  - 新增 best sample：frame `6`，pair `[ColliderHandle(1), ColliderHandle(9)]`，feature `ContactFeatureId(4311752704)<-ContactFeatureId(16777218)`，`point_drift 0.000743`，`normal_dot 0.999996`，`local_anchor_drift 0.003631`。
  - 180-frame edge-swap transition count 为 `384`，其中 `366` 个同时满足 same reduction / same shape / point+normal+local-anchor continuity，说明大部分 same-pair feature-id churn 都是 reference/incident edge 互换形态，而不是任意 feature family 跳变。
  - 600-frame observation 仍为 long-settle failure，final hash `6741fcc79156b200`，first floor exit frame `214`，final outside floor bodies `7`；dominant transition `k1:r2:i0<-k1:r0:i2 x532` 的 best sample 位于 frame `370`，pair `[ColliderHandle(10), ColliderHandle(3)]`，`point_drift 0.000226`，`normal_dot 1.000000`，`local_anchor_drift 0.000346`；edge-swap transition count 为 `1078`，其中 `1034` 个满足 conservative candidate 条件。
- 结论：
  - dominant transition 自身确实有“世界点近、法线近、局部锚点近”的样本；下一步可以围绕这个 transition 设计显式 persistent manifold candidate。
  - 该证据仍不允许直接 canonicalize feature id；上一负向实验已经证明无条件 canonicalization 会破坏 8x6 稳定性。

### 2026-05-07 - E4b 负向实验：Edge-swap limited warm-start fallback

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证在不改 SAT feature id 的前提下，能否只对 reference / incident edge 互换的 conservative candidate 迁移少量 warm-start impulse。第一版实验限制为同 collider pair、同 reduction reason、edge-swap feature index、normal / anchor continuity 仍命中 `warm_start_transfer`；迁移时只保留 `25%` normal impulse，并把 tangent impulse 清零，避免把旧摩擦直接灌入新 row。
- 行为锁：
  - 曾短暂加入 `warm_start_edge_swap_fallback_transfers_limited_normal_impulse_without_tangent`，验证 edge-swap fallback 只迁移有限法向冲量、不迁移切向冲量。
  - 该行为锁本身通过，但随后被 matrix stack stress gate 否决，因此测试和实现一并回滚。
- 观测：
  - `rtk proxy cargo test -p picea --lib pipeline::contacts -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture`：通过，13 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：失败；`matrix_stack` 180-frame final hash 变为 `6665ecf0c28abc46`，penetration `0.067079 / 1.702204`，quiet linear/angular `13.395638 / 6.668496`，first floor exit frame `87`，final outside floor bodies `2`。
  - feature churn trace 降到 `miss_feature_id 132 same_pair_previous 132 same_reduction_reason 90 same_feature_index 24 close_world_point 98 same_shape_signature 132 close_local_anchor 99 edge_swap_transition 101 edge_swap_candidate 83`，但稳定性更差。
  - 回滚后 `rtk proxy cargo test -p picea --lib pipeline::contacts -- --nocapture` 通过，恢复为 1 个保留单测；`rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture` 通过，13 passed；`rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture` 通过，2 passed。
  - 回滚后 `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture` 通过，恢复 180-frame retained baseline：final hash `829306c4886125fd`，penetration `0.041296 / 1.676269`，floor ejection `none`，final outside floor bodies `0`。
  - 回滚后 `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_observation_reports_residual_e4_risk -- --ignored --nocapture` 通过，600-frame 红灯仍为 retained baseline：final hash `6741fcc79156b200`，first floor exit frame `214`，final outside floor bodies `7`，quiet linear/angular `62.754536 / 14.401797`。
  - 回滚后显式 future gate `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture` 仍按预期失败，失败断言为 `first_floor_exit_frame Some(214) != None`。
- 结论：
  - 即使只迁移 `25%` normal impulse 且清零 tangent impulse，edge-swap fallback 仍会改变 dense support chain 的能量路径，导致更早 ejection。
  - 后续 E4b 不应继续沿“扩大 warm-start fallback”做实现。下一步必须转向真正的 persistent manifold / contact lifecycle 设计：在生成或选择 manifold 时保持 reference/incident role、clip point ordering、局部支撑有效性和长窗口 no-ejection，而不是在 warm-start 层事后兜底。

### 2026-05-07 - E4b 小步实现：Persistent manifold identity-only

- 状态：已完成；600-frame hard gate 仍红
- Commit：none
- 目标：把 E4b 后续实现入口从“warm-start fallback”改为“persistent manifold candidate”。第一版只改变 contact lifecycle / identity 选择，不同步迁移 solver impulse，避免再次出现 feature churn 下降但 ejection 提前的假阳性。
- 依据：
  - SAT ordered-pair edge canonicalization：feature churn 明显下降，但 180-frame `matrix_stack` 在 frame `83` ejection。
  - simple pair-level warm-start fallback：短窗口有改善，但 600-frame 仍 ejection，quiet linear `64.363495`。
  - edge-swap limited warm-start fallback：只迁移 `25%` normal impulse 且清零 tangent impulse，仍让 180-frame `matrix_stack` 在 frame `87` ejection。
- 实现：
  - `refresh_contact_events` 在 exact `ContactKey` miss 时，允许 clipped-family SAT reference / incident edge-swap candidate 复用上一帧 `ContactId` / `ManifoldId`。
  - candidate 限制为同 collider pair、`Clipped` / `DuplicateReduced` reduction family、normal continuity、feature kind 相同且 reference / incident edge 互换、collider-local anchor drift <= `0.25`。
  - `prepare_contact_warm_start` 不使用该 candidate；因此 warm-start 仍保持 `MissFeatureId` / zero impulse，solver impulse 不迁移。
- 保留禁止：
  - 不新增 pair-level 或 edge-swap warm-start fallback。
  - 不在 solver correction loop 中重跑 full narrowphase。
  - 不全局 canonicalize SAT feature id。
  - 不改 velocity row solve order、position correction percent、sleep threshold 或 friction budget。
- 验收：
  - behavior lock：`sat_edge_swap_candidate_persists_lifecycle_without_auto_warm_start_impulse`，要求 SAT edge-role swap candidate 的 lifecycle identity 能持久化，但 warm-start impulse 仍保持 miss/zero，直到后续单独设计 impulse migration。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture`
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`
  - 若 180-frame gate 通过，再跑 `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_observation_reports_residual_e4_risk -- --ignored --nocapture` 和显式 future gate，确认是否进入第二阶段 impulse migration。
- 验证：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance sat_edge_swap_candidate_persists_lifecycle_without_auto_warm_start_impulse -- --ignored --nocapture`：先红；第一帧 contact id / manifold id 为 `ContactId(0)` / `ContactId(1)`，第二帧变为 `ContactId(2)` / `ContactId(3)`，证明 lifecycle identity 尚未持久化。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance sat_edge_swap_candidate_persists_lifecycle_without_auto_warm_start_impulse -- --nocapture`：实现后通过，1 passed。
  - `rtk proxy cargo test -p picea --lib pipeline::contacts -- --nocapture`：通过，1 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture`：通过，13 passed。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`：通过，2 passed / 3 ignored；180-frame `matrix_stack` 无 floor ejection，final outside floor bodies `0`，final hash 变为 `b259f33084b407c3`，主要因 contact/event identity 改变。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_observation_reports_residual_e4_risk -- --ignored --nocapture`：通过；600-frame observation 仍为 long-settle failure，final hash `9c8ff53179760623`，first floor exit frame `214`，final outside floor bodies `7`，quiet linear/angular `62.754536 / 14.401797`。
  - `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture`：按预期失败，断言 `first_floor_exit_frame Some(214) != None`。
- 结论：
  - identity-only lifecycle persistence 已收口，且未把 `MissFeatureId` 直接变成 `Hit`。
  - 600-frame hard gate 无改善，说明当前主 blocker 已从 feature identity 转向 support-gap / position re-evaluation：frame `180` 出现 ejection support gap，frame `229` 起 late support gap 持续到结尾。下一步不应继续扩大 warm-start fallback，应沿 support validity、position row re-evaluation 或动态支撑保持策略设计。

### E4c - Support-gap / position re-evaluation 子里程碑

- 状态：future red lock 已新增；尚未实现
- Commit：none
- 目标：解决 identity-only 收口后仍存在的长窗口 support gap。当前 600-frame 失败已不再指向 contact feature identity，而是动态支撑在 position correction / contact refresh 边界过早断行，导致 body 在 frame `214` 退出 floor，并在 frame `229..599` 持续 long support gap。
- 范围：
  - 先补 support-gap targeted behavior lock：锁住 frame `180` ejection support gap 与 frame `229` late support gap 的最小可复现场景或报告断言。
  - 只对已存在的 support candidate 做局部 position contact re-evaluation 或 retain/drop 决策。
  - candidate 必须包含 normal speed、local anchor drift、counterpart motion、correction depth、几何重叠状态和是否仍在 floor support envelope 内。
- 非目标：
  - 不合成任意 speculative contact。
  - 不扩大 warm-start fallback，不把 lifecycle candidate 直接变成 `Hit`。
  - 不改变 velocity row solve order、全局 position correction percent、sleep threshold 或 friction budget。
- 验收：
  - 新增 targeted support-gap behavior lock 先红后绿。
  - 下一实现切片必须引用
    `docs/design/matrix-stack-stability-optimization-design.md` 中的
    `E4c Next Gate: Source-Row Constrained Position Row Design`。
  - 实现必须保留现有 `ContactPositionRow` / `QueuedPositionTranslation` seam，并同时验证
    source-row 几何有效性、counterpart-motion gate、tangent/rotation energy gate 和 upstream trace。
  - 禁止继续采用已回滚的局部路线：调大 shallow support friction budget、扩大 velocity-level
    position bias、扩展非 edge-swap identity-only lifecycle、或按 local-continuity source row
    直接缩放法向 correction。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack -- --nocapture`
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_observation_reports_residual_e4_risk -- --ignored --nocapture`
  - 最终 `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture` 必须通过，才允许把 `8x6` 长窗口写成稳定。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --nocapture`：通过，0 passed / 1 ignored，证明 future lock 默认不污染普通 suite。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`：按预期失败；240-frame report 显示 `ejection_support_gap_start_frame Some(180)`，first floor exit frame `214`，final outside floor bodies `2`，quiet linear/angular `4.817211 / 14.401797`。
- 结论：
  - E4c 已有比 600-frame hard gate 更短的红锁，覆盖当前首个 support-gap/ejection 窗口。
  - 下一步实现必须让该 240-frame gate 先绿，再回到 600-frame long-settle observation / acceptance。

### 2026-05-07 - E4c 混合实验：Dense shallow support friction budget

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证是否可以只通过增加 dense shallow support friction budget，降低浅接触分离前的 tangent energy，从而移除 frame `214` ejection。该实验不改 contact identity、不合成 contact、不改 position correction。
- 观测：
  - `SHALLOW_SUPPORT_FRICTION_BUDGET_SCALE = 1.0`：`stack_4` 通过；240-frame E4c red lock 中 floor ejection 被消除、final outside `0`、quiet linear 降到 `3.281882`，但 late support gap 提前到 frame `142`，quiet angular 仍到 `7.141165`。180-frame `matrix_stack` 正式 gate 失败，因为 support gap 提前且角速度超过保留阈值。
  - `SHALLOW_SUPPORT_FRICTION_BUDGET_SCALE = 0.75`：aligned 4x3 通过；180-frame `matrix_stack` 无 ejection、final outside `0`、penetration 降到 `0.039802`、quiet linear/angular `1.203595 / 3.401054`，但既有 diagnostic gate 因 late support gap 消失而失败。240-frame 仍 ejection，只是 first exit 从 `214` 推迟到 `228`；600-frame observation 仍失败，first exit `228`，final outside floor bodies 从保留基线 `7` 变成 `8`，quiet linear 仍为 `60.549240`。
- 结论：
  - 增大 shallow support friction budget 可以改善短窗口，但不能解决长窗口稳定性；`0.75` 甚至让 600-frame final outside 更差。
  - E4c 不能只靠更强摩擦预算收口。下一步仍要做 support candidate 的 position re-evaluation / retain-drop 语义，而不是调大 friction budget。

### 2026-05-07 - E4c 证据口：Ejection support-gap candidate facts

- 状态：已完成；只增加验收证据，不改变 solver 行为
- Commit：none
- 目标：把 240-frame E4c 红锁的 pre-ejection support gap 从“有 gap”细化为 retain/drop 输入事实，避免下一步把不安全的动态支撑误保留。
- 改动：
  - `MatrixStackStressReport` 的 ejection support-gap trace 增加 previous / next frame 的 body speed、counterpart speed、body/counterpart tangent components、normal/tangent anchor drift 和 position-correction depth/translation。
  - `matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window` 在当前红灯存在时先断言这些证据完整：last pre-gap contact 仍几何重叠、已经 separating、local-anchor drift 为 `0`，但 counterpart angular speed 与 counterpart tangent angular component 明显超过 retain gate。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过，180-frame retained baseline 未改变，final hash `b259f33084b407c3`。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`：仍按预期失败；扩展 trace 显示 frame `179` pre-gap contact depth `0.001882`、normal speed `0.229571`、body speed `1.648571/0.331909`、counterpart speed `1.509143/3.746651`、tangent components body `1.183015/0.068764`、counterpart `0.536331/1.113765`、drift `0/0`、correction `0/0`。
- 结论：
  - 当前 frame `180` support gap 不是“静止 counterpart + 小 anchor drift”的安全 retain 场景；counterpart-motion gate 会拒绝直接保留。
  - E4c 下一步不能只把 shallow contact 加回 position row。更合理的实现方向是上游降低 dynamic support pair 的切向/角向能量传播，或在 position-row re-evaluation 中加入 counterpart-motion 约束后只保留真正 quiet-support candidate。

### 2026-05-07 - E4c 混合实验：Non-dense two-point manifold block normal solve

- 状态：保留但仅限 non-dense contact graph；dense `matrix_stack` 路径禁用
- Commit：none
- 目标：验证 clipped two-point manifold 的法向 row 如果逐点顺序求解，是否会把摩擦预算集中到单个接触点并注入人工扭矩。这对应 Box2D 类引擎中把 contact manifold 作为一个小约束组处理的思路，但第一版只允许作为窄行为锁，不允许直接影响 dense 矩阵堆叠。
- 行为锁：
  - `two_point_sliding_face_contact_keeps_friction_torque_balanced` 构造一个轻微倾斜、沿静态地面滑动的动态方块，要求两点面接触仍能提供摩擦，同时最终角速度不超过 `0.02`。
  - 实现前该锁为红灯：角速度约 `0.0788897`，两个 manifold point 的 normal/tangent impulse 明显失衡，一个约 `0.14876924`，另一个约 `2.8305829`。
- 实现：
  - `ContactSolveBatch` 增加 `normal_pair_partners`，仅把同 `ContactPairKey`、同 solver pair、同 normal、`Clipped` / `DuplicateReduced`、无 position bias、初始切向速度超过阈值的两点 manifold 组成 2x2 normal block solve。
  - block solve 只求 normal impulse，tangent impulse 仍按原 row 顺序求解，避免第一版同时改变摩擦 row 语义。
  - 首版直接启用会让 dense `8x6` support gap 从 retained baseline 的 frame `171` 提前到 frame `138`，因此最终增加 `dense_contact_graph` gate：dense contact graph 下 `normal_pair_partners` 全部为空，保持原 solver 路径。
- 验证：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance two_point_sliding_face_contact_keeps_friction_torque_balanced -- --nocapture`：通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过，180-frame retained baseline 恢复，final hash `b259f33084b407c3`，support gap 仍为 frame `171`，无 floor ejection，final outside floor bodies `0`。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance -- --nocapture`：通过，58 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`：仍按预期失败，`ejection_support_gap_start_frame = Some(180)`，first floor exit frame `214`；说明 dense 稳定目标未完成。
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy git diff --check`：通过。
- 结论：
  - 两点 manifold block normal solve 是有价值的小规模行为锁：它能避免 standalone face contact 的假旋转。
  - 它不是当前 dense matrix-stack 稳定性修复。直接用于 dense graph 会改变压力链传播，让 support gap 提前，因此 E4c 后续必须把 dense manifold / position-row / pressure propagation 作为单独设计，而不是把小 manifold block solve 全局打开。

### 2026-05-07 - E4c 证据口：Support-gap upstream pressure source

- 状态：已完成；只增加验收证据，不改变 solver 行为
- Commit：none
- 目标：把 frame `180` support gap 前的 high-counterpart-motion 从“最后一帧支撑体很快”继续前移到“支撑体的动量来自哪条上游接触链”，避免 E4c 只在 gap 前一帧做 retain/drop 或局部 correction。
- 改动：
  - `MatrixStackStressReport` 增加 `Ejection support-gap upstream trace` 行。
  - trace 从 pre-gap support counterpart 往前 60 帧内寻找 counterpart 角速度峰值，并记录当时 counterpart 速度、仍连接的 source bodies、source contact 数、depth、normal/tangent impulse 与 position-correction depth/translation。
  - `matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window` 在当前红灯存在时要求 upstream trace 非空、counterpart angular speed 仍高、且 source contact 存在。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过，180-frame retained baseline 未改变，final hash `b259f33084b407c3`。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`：仍按预期失败；新增 trace 显示 upstream frame `177`，counterpart `BodyHandle(33)` speed `1.229398/3.924366`，source body `[BodyHandle(25)]`，source contact count `1`，source depth `0.007948`，source impulse `0.000862/0.000689`，source correction `0.067205/0.002688`。
- 结论：
  - frame `180` gap 的最后支撑体不是一个可单独保留的 quiet support；它在 gap 前已经被上游动态接触链推动，并且该 source contact 仍在做明显 position correction。
  - E4c 下一步应优先设计 dense pressure propagation / position-row correction 的能量控制或重新排序验收，而不是在 frame `179` 单点 retain support、合成 speculative contact 或继续调 friction budget。

### 2026-05-07 - E4c 证据口：Counterpart-motion oracle order

- 状态：已完成；只修正 report/test oracle，不改变 solver 行为
- Commit：none
- 目标：修正 support eligibility oracle 的判定顺序。此前当 body-side angular tangent risk 低时，会先返回 `observe_tangent_risk_low`，从而漏掉 counterpart linear/tangent motion 已经很高的场景；这会误导后续 E4c 把不安全支撑当成低风险 retained candidate。
- 改动：
  - `support_eligibility_decision` 现在先检查 counterpart tangent linear / angular motion，再检查 body-side tangent risk 是否低。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过，180-frame retained baseline 未改变，final hash `b259f33084b407c3`。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`：仍按预期失败；late lifecycle 中 frame `192..200` 从 `observe_tangent_risk_low` 改为 `reject_counterpart_motion`，更准确地表达两侧线性滑动仍很高，不能作为 quiet support 保留。
- 结论：
  - E4c 的 retain/drop oracle 必须优先保护 counterpart-motion gate；只有 counterpart 也 quiet 时，才允许进入 position-level re-evaluation。

### 2026-05-07 - E4c 负向实验：Weak dynamic support correction scale

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证 upstream trace 指向的 weak dynamic support source contact 是否可以通过降低 residual position correction 贡献来减弱 pressure propagation。实验仅限 dense、dynamic-dynamic、depth <= `2 * POSITION_CORRECTION_SLOP`、normal/tangent impulse 都低于 `0.002` 的 position rows，correction scale 降为 `0.25`。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` 未回归。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：失败；180-frame formal gate 明确退化。
  - 退化结果：final hash `8b38ae54e87781bb`，penetration `0.057740 / 1.714501`，first floor exit frame `81`，final outside floor bodies `2`，quiet-window linear/angular `16.706886 / 13.413667`。ejection support gap 提前到 frame `40`，BodyHandle(48) 在 frame `81` 出界。
- 结论：
  - 只削弱 weak dynamic support row 的 correction 会过早打断边缘支撑链，导致比 retained baseline 更早 ejection。
  - E4c 不能沿“按 weak impulse 削 correction”继续调参；dense support 仍需要足够法向位置支撑。下一步应转向更结构化的 pressure propagation 控制，例如保持法向 correction 但控制切向/旋转能量来源，或先把 source-contact row 的局部几何 / manifold persistence 固化。

### 2026-05-07 - E4c 负向实验：Broaden velocity-level position bias

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证 Box2D-style Baumgarte velocity bias 是否能比当前实现更早吸收穿透压力，减少后续 residual position correction 对 dense stack 的压力传播。实验把 `position_bias` 从“仅法向相对速度近似 0 时启用”改成“只要目标分离速度大于当前法向分离速度就启用”。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：失败；`stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 的 settled penetration max 退化到 `0.079559326`，超过 D2 hard gate。
  - 回滚后同一命令通过，确认保留基线恢复。
  - `rtk proxy cargo test -p picea --lib solver::contact -- --nocapture`：没有匹配测试，0 passed / 0 failed；不作为有效验收信号。
- 结论：
  - 直接放宽 velocity-level position bias 会破坏小规模稳定性，不能作为 E4c 修复入口。
  - E4c 仍应保留当前 velocity bias 的保守启用条件；后续若要引入更强的 position/velocity coupling，必须先在独立 position-row 或 manifold-level gate 中限定作用对象，而不能全局扩大 bias。

### 2026-05-07 - E4c 证据口：Upstream source row solver facts

- 状态：已完成；只增加验收证据，不改变 solver 行为
- Commit：none
- 目标：把 upstream pressure source 从“有 source contact”继续细化到该 source row 的 warm-start、normal/tangent speed、velocity bias、support friction、normal/tangent impulse 与 residual correction，确认下一步该修哪一层。
- 改动：
  - `MatrixStackStressReport` 的 `Ejection support-gap upstream trace` 现在输出 source warm-start reasons、initial/post normal speed、tangent speed、position bias、support friction、impulse 和 correction。
  - trace 还输出 source row 与上一帧同 collider pair 的连续性：same-pair count、edge-swap candidate count、point drift、normal dot、local-anchor drift。
  - E4c ignored 红锁新增断言：upstream source warm-start reason 不为空、source position bias 仍为 0、source residual correction depth 大于 normal impulse。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过，180-frame retained baseline 未改变，final hash `b259f33084b407c3`。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`：仍按预期失败；upstream trace 显示 frame `177` source contact 为 `MissFeatureId`，上一帧同 pair 存在，point drift `0.000697`，normal dot `0.997916`，local-anchor drift `0.005827`，edge-swap candidate 为 `0`；initial/post normal speed `0.084782..0.084782 -> -0.000326..-0.000326`，tangent speed `0.323130`，position bias `0`，support friction `0`，normal/tangent impulse `0.000862/0.000689`，correction `0.067205/0.002688`。
- 结论：
  - 该 source row 不是 warm-start hit 后继续稳定支撑，也不是 velocity-level position bias 主导；它是 feature churn 后冷启动、低 normal impulse、但 residual position correction 明显参与的 row。
  - 该 source row 也不是 E4b edge-swap persistence 能覆盖的类型：局部几何连续，但不是 reference/incident edge-swap candidate。
  - E4c 下一步应优先把 source-contact row 的 local-anchor continuity 与 residual position correction eligibility 绑定起来，而不是扩大 velocity bias、调 friction budget、继续扩 edge-swap lifecycle，或在 gap 前一帧 retain support。

### 2026-05-07 - E4c 负向实验：Broaden identity-only lifecycle beyond edge swap

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证 frame `177` source row 这种 “MissFeatureId + same-pair + local-anchor 连续但非 edge-swap” contact，是否可以通过扩展 identity-only lifecycle candidate 减少冷启动 source row。实验只改变 contact lifecycle identity，不迁移 warm-start impulse，不改变 solver row。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` 未回归。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过且无 ejection，但 final hash 从 `b259f33084b407c3` 变为 `ad9e108bf0e4c009`，说明行为路径发生变化。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`：仍按预期失败；frame `180` support gap、frame `214` ejection、quiet-window spike 没有改善。
- 结论：
  - 单纯扩展 identity-only lifecycle 到非 edge-swap local-anchor-continuous row 只改变状态 hash，不解决 source row 的 residual correction pressure propagation。
  - 后续不能把 E4c 继续压到 contact identity 层；应进入 residual position correction eligibility / position-row source gating，而不是继续扩大 lifecycle candidate。

### 2026-05-07 - E4c 负向实验：Scale local-continuity source-row residual correction

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证 “MissFeatureId + same-pair + local-anchor 连续” 的 source row 是否可以通过降低 residual position correction 比例来减少 pressure propagation。实验在 contact phase 标记 local-continuity source-row candidate，并在 dense residual position correction 中把该 row 的 correction depth 乘以 `0.5`。
- 观测：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，`stack_4` 未回归。
  - `rtk proxy cargo test -p picea --lib pipeline::contacts -- --nocapture`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：失败；虽然 180-frame late support gap 消失，但正式门因 baseline story 改写而失败，且数值退化明显。
  - 退化结果：final hash `cc9394c227cdd41c`，penetration `0.051679 / 1.681709`，quiet angular spike `5.543327`，late correction max/total `0.054102 / 0.732825`，solver row max 从 frame `46` 转移到 frame `108`。
- 结论：
  - 只按 local-continuity source row 降低 residual correction，会把支撑缺口改写成更高 penetration / angular spike / correction pressure，并没有让 8x6 进入稳定状态。
  - E4c 不能继续做“命中 source row 后直接缩放 correction”的调参；下一步若处理 residual correction，必须重新评估 position-row 几何有效性或把 correction 与 tangent/rotation 能量约束一起设计，而不是单独降法向 correction。

### 2026-05-07 - E4c 设计门：Source-row constrained position row

- 状态：已完成；作为下一实现输入，不改变生产代码
- Commit：none
- 目标：在多轮负向实验后，把 E4c 的下一步从“局部补丁/系数实验”收束到一个可验收的
  source-row position-row 设计门。
- 产出：
  - `docs/design/matrix-stack-stability-optimization-design.md` 新增
    `E4c Next Gate: Source-Row Constrained Position Row Design`。
  - 计划中的 E4c 子里程碑补充下一实现切片必须引用该设计门，并明确禁止继续重复已失败路线。
- 设计结论：
  - frame `179` 的最后支撑不是 quiet counterpart，不能直接 retain；
  - frame `177` 的 upstream source row 是 same-pair / local-anchor 连续但非 edge-swap，且主要表现为
    residual position correction pressure source；
  - 下一次实现必须在 `ContactPositionRow` / `QueuedPositionTranslation` seam 上，同时检查 source-row
    几何有效性、counterpart motion 和 tangent/rotation energy，不能只降低法向 correction。
- 验收：
  - `rtk proxy git diff --check`：通过。
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过，final hash `b259f33084b407c3`，final outside floor bodies `0`，late support gap frame `171 duration 9`。
- 风险 / 后续：
  - 该设计门不代表 goal 完成。当前 E4c 240-frame ignored red lock 仍失败，600-frame hard gate 仍未通过；
    下一步需要在该设计门下实现新的 position-row source gating，或者升级为更完整的 dense pressure
    propagation / position solver 设计。

### 2026-05-07 - E4c 输入 seam：Source-row continuity candidate

- 状态：已完成；只增加 position-row gating 输入，不改变 solver 数值行为
- Commit：none
- 目标：让 solver 内部能直接看到 “MissFeatureId + same-pair / local-anchor 连续 + 非 edge-swap”
  的 source-row candidate，避免下一次 position-row gating 继续依赖 lab report 事后反推。
- 改动：
  - `ContactObservation` 新增 `source_row_continuity_candidate`。
  - `prepare_contact_warm_start` 只在 warm-start reason 为 `MissFeatureId`，并且上一帧同 collider pair
    normal 连续、local-anchor drift 小、且不是 E4b edge-swap lifecycle path 时置位。
  - `ContactEvent` / `DebugContact` / Web `DebugContact` type 以 additive 字段暴露该事实；旧 JSON 默认
    `false`。
  - 新增两个 `pipeline::contacts` 单元锁：一个证明非 edge-swap feature miss 会被标记，另一个证明
    edge-swap lifecycle path 不会被混入 source-row candidate。
- 验证：
  - `rtk proxy cargo test -p picea --lib pipeline::contacts -- --nocapture`：通过，3 passed。
  - `rtk proxy cargo test -p picea --test query_debug_contract warm_start_new_picea_payload_fields_default_when_deserializing_older_json -- --nocapture`：通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，2 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：通过，final hash `4f46bbb499291b86`，final outside floor bodies `0`，late support gap frame `171 duration 9`。hash 因 additive debug/event payload 字段更新，物理指标未变。
  - `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`：通过。
  - `rtk proxy npm --prefix crates/picea-lab/web run build`：通过，仅保留既有 Vite chunk-size warning。
  - `rtk proxy cargo test -p picea --test query_debug_contract -- --nocapture`：通过，26 passed。
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy git diff --check`：通过。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`：按预期失败，仍显示 `ejection_support_gap_start_frame Some(180)`、first floor exit frame `214`、final outside floor bodies `2`，并且 upstream trace 暴露 `source_candidates 1`。
- 风险 / 后续：
  - 这一步是 E4c source gating 的输入 seam，不是稳定性修复。下一步才可以把该 candidate 接入
    `ContactPositionRow` eligibility，并且仍必须同时检查 pseudo-state geometry、counterpart motion
    和 tangent/rotation energy。

### 2026-05-07 - E4c 负向实验：Suppress high-tangent source-row correction

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证是否可以利用 `source_row_continuity_candidate`，在 dense graph 中跳过
  `MissFeatureId + local-anchor 连续 + 低 normal impulse + 无 velocity bias/support friction + 高 tangent speed`
  的 source-row position correction，减少上游 pressure propagation。
- 观测：
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：通过，
    `stack_4` 未回归。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：失败。虽然 180-frame 仍无 floor ejection、penetration 仍为
    `0.041296 / 1.676269`，但 late angular 从 retained baseline `4.568931` 升到 `4.579775`，
    且 lifecycle 中原本 frame `166` 的 `retain_candidate_correction_active` 被改成
    `retain_candidate_needs_position_re_evaluation`，破坏了当前验收 story。
  - 回滚后重跑 `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：
    通过，final hash 恢复为 `4f46bbb499291b86`，final outside floor bodies `0`，late support gap
    仍为 frame `171 duration 9`，说明保留基线已恢复。
- 结论：
  - 即使用 source-row candidate 和 tangent-risk 条件约束，直接跳过法向 position correction 仍会打坏
    支撑生命周期证据。
  - E4c 不能走“source-row candidate 命中后 suppress correction”的路线。下一步如果继续 source-row
    gating，必须先补 pseudo-state geometry / energy dry-run facts，或引入更完整的 position-row
    pressure propagation 设计，而不是直接删除该 row 的法向支撑。

### 2026-05-07 - E4c 证据口：Source-row dry-run gating facts

- 状态：已完成；只增强 artifact/report 证据，不改变 solver 数值行为
- Commit：none
- 目标：在不修改 `ContactPositionRow` 行为的前提下，把 source-row candidate 的下一层 gating
  输入从“报告里人工解读”固化成 dry-run facts：几何连续、residual correction pressure source、
  counterpart motion rejection、tangent energy rejection、pseudo-position depth range 和最终 dry-run
  decision。
- 改动：
  - `MatrixStackStressReport` / `EjectionTrace` / upstream support-gap trace 新增 source-row dry-run
    字段。
  - dry-run 只统计已落地的 `source_row_continuity_candidate`；它要求当前 contact 仍有深度、
    normal dot >= `0.98`、local-anchor drift <= `0.05`，再判断该 row 是否是
    `position_bias == 0`、`support_friction == 0`、且 correction depth 大于 normal impulse 的
    position-correction pressure source。
  - 在 pressure source 之后，dry-run 分别统计 counterpart motion 和 tangent energy 拒绝，不把该
    判断写回 solver。
  - report 层复刻 dense position correction 的 deferred pseudo-translation 顺序，用当前 frame 的
    DebugBody inverse mass / sleeping 状态和 DebugContact normal / depth，估算 source row 在
    20 次 position iteration 中看到的 pseudo-depth 范围。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`：
    按预期失败在最终红锁断言，仍显示 `ejection_support_gap_start_frame Some(180)`；但 upstream
    trace 已补齐 `dry_run geometry 1 pressure 1 reject_counterpart 1 reject_tangent 1 eligible 0 decision reject_counterpart_motion pseudo_depth 0.003264..0.003453 pseudo_skin 20`。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：
    通过，final hash `4f46bbb499291b86`，final outside floor bodies `0`，late support gap frame
    `171 duration 9`。
  - `rtk proxy cargo fmt --all --check`：通过。
  - `rtk proxy git diff --check`：通过。
- 结论：
  - 当前 upstream source row 不是“没有几何连续”的问题；它是一个几何连续且 correction-dominated
    的 source row；pseudo-state 下仍保持正 residual depth 并落在 support skin 内，但同时会被
    counterpart motion / tangent energy gate 拒绝。
  - 下一步不应再做直接 suppress / retain，而应进入更完整的 source-row constrained position-row
    pressure propagation 设计；核心问题已经从“这个 row 还算不算几何支撑”收窄到“如何防止
    几何支撑 row 把切向 / 旋转能量继续传下去”。

### 2026-05-07 - E4c 负向实验：Source-row correction side rebalance

- 状态：已回滚，未改变保留基线
- Commit：none
- 目标：验证是否可以在 dense graph 中保留 source-row 法向 position correction，但把 correction
  从高切向能量的一侧重分配到低能量 body，减少 residual correction pressure 继续推动高能量
  counterpart。
- 实验条件：
  - 仅对 `source_row_continuity_candidate` 生效；
  - 要求无 velocity-level position bias、无 support friction、normal impulse <= `0.002`、
    final tangent speed >= `0.25`；
  - 若两侧 contact-point tangent component 差值超过 `0.05`，只让低能量侧接收 position
    correction。
- 验证结果：
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`：
    通过，`stack_4` 未回归。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：
    失败。180-frame 无 floor ejection、penetration 仍为 `0.041296 / 1.676269`，quiet angular 从
    `4.568931` 降到 `4.488485`，但 quiet linear 从 `3.268942` 升到 `3.280477`，hash 改为
    `34ab4aea9a5716d6`，且 late support-gap story 被改写，违反当前正式 gate。
  - `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`：
    按预期仍失败。该实验把 first floor exit 从 frame `214` 推迟到 `215`，penetration max 从
    `0.082507` 降到 `0.073082`，但 quiet-window linear / angular 仍退化为 `4.843674 / 14.458097`，
    support gap frame `180` 未移除。
  - 回滚后 `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`：
    通过，final hash 恢复为 `4f46bbb499291b86`。
- 结论：
  - correction side rebalance 是“方向上有信号但不满足验收”的 partial positive：它能降低部分
    penetration 并延后一帧 ejection，但会改写 180-frame baseline story，并把能量推迟到后续
    velocity spike。
  - E4c 不能只靠单行 correction 重分配收口；下一步需要 position-row pressure propagation
    级别的约束，例如把 candidate 分组、跨 row 传播预算或能量 gate 与 correction ordering
    一起设计，而不是单独改变某一条 row 的两侧分配。
