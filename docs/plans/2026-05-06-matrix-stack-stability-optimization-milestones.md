# Matrix Stack 稳定性优化里程碑计划

状态：草稿
计划文档：docs/plans/2026-05-06-matrix-stack-stability-optimization-milestones.md
最后更新：2026-05-06
工作目录：/Users/asyncrustacean/projects/picea
工作区状态：dirty；当前工作区已有多组 picea-lab diagnostics / matrix_stack / Web / docs 相关改动，以及若干本轮前已存在的 unrelated dirty 文件；本计划阶段只新增本计划文档，不覆盖既有 dirty 内容。
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

- 当前工作区 dirty 与 E1/E2 可能触及文件重叠：影响执行安全和文件所有权；建议默认：先批准并完成 D1/D2 设计文档，设计完成后再由主 Codex 重新检查 `git status --short` 和相关文件 diff，确认 E1 能否叠加到现有 diagnostics / matrix_stack 改动上；若仍有重叠且来源不清，暂停执行并先做集成/所有权确认。

### 可带假设推进

- 默认 `matrix_stack` 第一阶段验收以已存在的 `8x6` stress 场景为主，`NxM` UI 参数化作为后续产品化扩展：影响 E1/V1 范围；早期验证是 artifact 仍能导出 48 dynamic bodies 和稳定性指标。
- 默认先修稳定性根因，不先做大规模 UI 配置面：影响 Web 范围；早期验证是浏览器仍可选内置 `matrix_stack` 并查看 diagnostics。
- 默认不引入第三方物理引擎依赖：影响架构边界；Matter.js / Box2D 只作为设计参考，不作为替换 core solver 的依赖。
- 默认先保持单线程 separate-phase solver contract：影响 E4 范围；早期验证是 `docs/design/solver-island-ordering-contract.md` 仍成立。

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

- 状态：待确认
- Design Gate：待确认；D1/D2 可在当前 dirty 工作区执行，因为只写设计/计划文档。
- Execution Gate：待确认；E1 之前必须重新检查 dirty overlap，确认 `artifact_run.rs`、`physics_realism_acceptance.rs`、`scenario.rs` 等目标文件没有未归属冲突。
- reviewer：已运行
- 审查结论：有发现，已采纳修订：把 dirty overlap 升级为 Execution Gate 前置条件；固定 `stack_4` clean behavior lock 与 `matrix_stack 8x6` dense stress gate 的关系；补充浏览器验收启动命令和 fallback；清理 `E6` 残留引用；明确 Matter.js / Box2D 参考边界和新旧设计文档关系。
- 用户确认：待确认

## 设计 / 执行分离规则

- 设计里程碑只允许修改计划文档、设计文档、调研记录或测试计划，不修改生产实现代码。
- 执行里程碑必须引用已批准的设计结论，或者显式说明为什么不需要设计前置。
- 如果执行中发现设计假设不成立，暂停执行并回到对应设计里程碑更新计划。
- 设计里程碑完成不等于允许实现；实现仍需经过 Execution Gate 或用户确认。
- 一个设计里程碑可以生成多个执行里程碑；一个执行里程碑必须绑定清晰的设计输入。

## 里程碑映射

| 设计里程碑 | 产出 | 解锁的执行里程碑 | 状态 |
| --- | --- | --- | --- |
| D1 | Matter.js / Box2D 参考设计映射、picea 稳定性策略边界 | E2, E3, E4, E5 | 待确认 |
| D2 | matrix_stack 稳定性验收门、debug/stress 场景区分、阈值和报告模板 | E1, V1, C1 | 待确认 |

## 里程碑

### D1：主流引擎稳定性策略映射

类型：设计
状态：计划中

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
状态：计划中

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
状态：计划中
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
状态：计划中
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
状态：计划中
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
状态：计划中
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
状态：计划中
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
- 如果执行过程中设计假设变化较大，C1 应记录被废弃的方案和原因，避免后续重复踩坑。

## 进度记录

### 2026-05-06 - 计划草案

- 状态：进行中
- Commit：none
- 验证：已完成只读 repo 证据收集、外部参考资料查询；等待 explorer / reviewer 计划门。
- 风险 / 后续：计划批准前不执行实现；下一步是 reviewer 审查计划文档并根据发现修订。
