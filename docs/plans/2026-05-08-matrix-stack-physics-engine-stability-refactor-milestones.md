# Matrix Stack 物理引擎稳定性重构里程碑计划

状态：进行中
计划文档：docs/plans/2026-05-08-matrix-stack-physics-engine-stability-refactor-milestones.md
最后更新：2026-05-08
工作目录：/Users/asyncrustacean/projects/picea
工作区状态：dirty；已有 `picea-lab` live session、gravity controls、AI routing、matrix-stack 设计文档等未提交改动。本轮 solver stability 执行只触碰明确记录的测试 / solver 文件，不吸收 web/server/AI routing WIP。
提交策略：不提交
执行策略：计划已获用户确认并进入当前工作区执行；执行里程碑开始前必须先完成 V0 工作区 baseline gate。若执行中发现设计假设不成立、dirty 文件重叠、行为锁退化、或需要改变 public API / step cadence / 数据语义，则暂停并回到计划更新。

## 进展记录

### 2026-05-08 - E5 resting dense island sleep 收敛

- 状态：已验证，`/goal` 的“多堆叠最终可静止”核心门已转绿；仍保留 early-frame diagnostics warning 作为后续 D4/E6 可观测优化问题。
- Commit：none
- 红锁：新增 1200-frame `matrix_stack_long_run_acceptance_requires_resting_sleep_convergence`，用 `PICEA_MATRIX_STACK_E5_SLEEP_ACCEPTANCE=1` 才执行。该门先红：first floor exit frame `800`、final outside floor bodies `5`、final awake/sleeping `48/0`、late support gap 从 frame `1000` 持续到结束，说明 600-frame no-ejection 还没有覆盖长窗慢漂移。
- 实现：在 `pipeline/sleep.rs` 新增 dense supported island 的 relaxed sleep eligibility。只有 active contact 数达到 `16`，并且岛内每个 dynamic body 都仍处在活跃支撑接触中，才使用较宽的 resting-stack sleep 阈值；普通 sparse island 继续使用原 `0.04 / 0.08` strict quiet gate。进入 sleep 时将 linear / angular velocity 置零，避免 sleeping body 在 debug/read model 里继续显示残余速度。
- 行为锁：新增 `dense_supported_island_can_sleep_after_relaxed_resting_window` 与 `sparse_island_keeps_strict_sleep_threshold`，明确该机制只属于 dense supported stack，不会让稀疏/普通接触走宽阈值。
- 测试更新：180-frame `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 不再要求 sleep 后仍有 late normal impulse；现在要求 48 个 dynamic body 在 endpoint 全部 sleeping，同时仍保留 late contact facts 和 no support gap。
- 结果：1200-frame E5 gate 通过，final awake/sleeping 为 `0/48`，floor ejection `none`，final outside floor bodies `0`，late correction/churn 中 position correction 归零。600-frame E4 acceptance、240-frame support-gap gate、180-frame matrix gate 也随之通过。
- 已验证：`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --lib pipeline::sleep::tests -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test physics_realism_acceptance sleep -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --lib solver::contact::tests -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin PICEA_MATRIX_STACK_E5_SLEEP_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_run_acceptance_requires_resting_sleep_convergence -- --ignored --nocapture`。
- 后续风险：该 slice 明确不解决 early-frame contact churn / solver-row spike，也不改变 manifold lifecycle / warm-start identity。它把成熟引擎里的 island sleep 思路落到现有架构上，但 D4/E6 仍可继续优化早期 churn 和 diagnostics warning。

### 2026-05-08 - E4/E5 接壤：浅层 support 与 speculative support 实验

- 已保留：dense resting-contact graph 在 velocity solve 后做 tangent / angular damping，`20.0/s` 是当前 180/240/600 gate 的稳定基线。
- 已保留：浅层 separating support 从 static-only 放宽到 dynamic/static 或 dynamic/dynamic，只在 `0 <= relative_normal_speed <= 0.25`、浅穿透、非 restitution、带摩擦时启用。600-frame gate 从纯 dense damping 的 `7dda9770a550459e` 变为 `f4a9b99b49d33fc3`，保持 no ejection / final outside 0，并把 final blocker 从 `linear 0.718565 / angular 2.623579` 降到 `linear 0.717971 / angular 0.669892`；仍未达成 sleep convergence。
- 已拒绝：把 shallow support 判定改成 `abs(relative_normal_speed) <= 0.25`。该实验在 600-frame gate 退化为 frame 510 floor ejection，说明闭合接触也加 support damping 会破坏边缘支撑平衡。
- 已拒绝：velocity-only speculative support row（仅求解器内部使用、不中断 contact event/position correction）。该实验显著变慢，并在 600-frame gate 退化为 frame 486 floor ejection / final outside 2；因此 E5 不应通过“一帧保留已消失 contact”来凑静止。
- 当时结论：E4 已守住 no-ejection/no-runaway，E5 仍需单独处理 sleep convergence；该结论已被上方 `E5 resting dense island sleep 收敛` 记录更新为 1200-frame gate 通过。

## 目标

把 `matrix_stack 8x6` / 多矩阵折叠从“局部补丁和诊断驱动”推进到参考 Box2D / Matter.js 成熟方向的 engine-level 稳定性重构。

最终用户可感知结果：

- `Rust 实时会话` 和 artifact replay 中，大矩阵堆叠不再在长窗口内产生 floor ejection 或 runaway velocity。
- `stack_4`、aligned matrix、`matrix_stack 8x6`、240/600-frame support-gap gate 能同时解释并约束稳定性。
- solver 结构里明确区分 contact persistence / warm-start、velocity impulse solve、dense position-row / pseudo-pose correction、source-row pressure propagation、persistent manifold 和 sleep convergence。
- 不再用“提高 iteration”“复用 non-dense block solve”“前一帧合成支撑”这类局部补丁掩盖根因。

## 约束

- 规划阶段只允许新增/更新计划文档、设计文档或测试计划，不修改生产实现代码。
- 执行阶段必须通过 `subagent-current-workspace` 逐里程碑执行。
- 所有验证命令必须使用 `rtk proxy` 前缀；环境变量使用 `rtk proxy env KEY=value ...` 形式。
- 不引入 Matter.js / Box2D 运行时依赖；它们只作为设计参照。
- 默认保持 public API、`World` / `SimulationPipeline` surface 和 M28 separate-phase solver ordering contract；若需要改变，必须先更新设计并重新过 Plan Gate。
- 默认不改变全局 `dt` / sub-step cadence；若需要 sub-step，应作为单独设计决策和执行里程碑，不混进 position-row 修复。
- `picea-lab-web` 只消费 Rust facts，不在浏览器重算物理。
- 现有 dirty worktree 不可被 revert、格式化或覆盖；V0 必须确认本计划执行不会吸收或覆盖他线 WIP。

## 待确认问题

### 必须确认

- 执行前 dirty baseline：当前工作区已有 server/web/live gravity、AI routing、matrix-stack 设计文档等未提交改动；完整计划批准不等于批准把这些改动混入 solver 重构。V0 必须先确认这些改动的归属、是否已完成、是否需要提交/搁置，或者明确哪些文件允许继续在 dirty workspace 中协作。

### 可带假设推进

- 假设第一阶段不改变 public API：影响执行范围；早期验证是 `rtk proxy cargo test -p picea --test v1_api_smoke` 或核心 API 测试不需因 solver 重构改期望。
- 假设第一阶段不改变全局 step cadence / sub-stepping：影响 Box2D-style TGS 路线；早期验证是 D1 明确记录“position-row 和 contact lifecycle 先行，sub-step 后置”的取舍。
- 假设 `stack_4` hard gate 优先于 `matrix_stack` 单项指标改善：影响补丁取舍；早期验证是任何 E2-E7 改动先跑 `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`。
- 假设 browser 现场验收可以在 solver 验证后作为产品检查，而不是 correctness oracle：影响 V1 范围；早期验证是 artifact / ignored red lock 先过，再用 in-app browser 看 `matrix_stack 8x6` live/artifact 表现。

## 规划依据

- explorer：已运行，结果已合并。
- reviewer：已运行，必须修改项已采纳。
- 关键证据：
  - explorer 结论：这条重构的 seam 不是某一个单独 solver 函数，而是 `step` 级事实流里的 `contacts -> warm-start -> solver -> island -> sleep`，应继续围绕现有 `stack_4` clean lock、`matrix_stack 8x6` stress gate 和 `FrameDiagnostics` 证据层做小步重构。
  - reviewer 结论：原草稿过早把 position-row 设为主线，和既有“contact persistence 先于 position correction”的顺序冲突；本版已把 V0 dirty gate、E2 contact-persistence evidence、D3/D4 设计门、E3 solver-local position-row 边界和更广验证门补入计划。
  - `docs/plans/2026-05-06-matrix-stack-stability-optimization-milestones.md` 已记录从 E1 到 E4c 的渐进路线和局部实验结果。
  - `docs/design/matrix-stack-stability-optimization-design.md` 已记录 Box2D / Matter.js 对照复核：Picea 已有 SAT + clipped manifold、feature-id persistence、warm-started sequential impulse、Coulomb friction、resting restitution threshold、island solve 和 residual position correction；缺口在 contact lifecycle 与完整 dense position-row / pseudo-position 求解边界。
  - `crates/picea/src/pipeline/step.rs` 是事实流总装配点，持有 previous poses、sleep states、wake reasons、numeric warnings、broadphase、warm-start、solver、sleep 到 commit 的 step context。
  - `crates/picea/src/pipeline/contacts.rs` 拥有 contact gathering、persistent contact map、warm-start transfer、`source_row_continuity_candidate`。
  - `crates/picea/src/solver/contact.rs` 当前拥有 velocity sequential impulse、warm-start application、dense shallow support friction、residual position correction、`ContactPositionRow`、`QueuedPositionTranslation`；其中注释已把 pseudo-translation 标为 later Box2D-style position row seam。
  - `crates/picea/src/pipeline/narrowphase.rs` 是 contact feature 生成 seam；只有当 D4/E6 证明 lifecycle 输入不足时才考虑触碰，不作为第一执行切入点。
  - `crates/picea/src/pipeline/island.rs` 拥有 deterministic dense island-local slots；`crates/picea/src/pipeline/sleep.rs` 拥有 island sleep window。
  - `crates/picea-lab/tests/artifact_run.rs` 已有 `matrix_stack_artifacts_capture_nxm_grid_stack_facts`、aligned matrix lock、180-frame first-bad-frame observation、240-frame E4c support-gap red lock、600-frame acceptance env gate。
  - `FrameDiagnostics` 已把 penetration、contact churn、warm-start、sleep、island、marker source 做成 lab-owned facts；Web 只展示事实，不应成为 solver root-cause 的计算层。
  - 近期负结果显示：dense graph 直接启用 non-dense two-point block normal solve、全局提高 default velocity iterations、简单 source-row residual correction 过滤都会破坏现有 gate，不能保留。
- 主要未知：
  - 当前主因到底是 contact identity / feature-id lifecycle，还是 stale position-row depth，还是二者都参与。
  - position-row pass 应该作为旧 residual correction 的替代、包裹层，还是先并行 shadow mode 再切换。
  - source-row pressure propagation 需要多少 contact lifecycle 输入才足够安全：local anchor、normal dot、same-pair、edge-swap、counterpart motion、tangent/rotation energy、frame-level correction budget。
  - 是否需要把 persistent manifold 从 `ContactObservation` 级别前移到 gather/reduce 层，而不是只在 solver row 里做 fallback。
  - 长窗口 no-ejection 后 sleep 是否自然收敛，还是需要独立 sleep threshold / wake reason 调整。

## 计划验收

- 状态：已批准，执行中
- Design Gate：已确认
- Execution Gate：已确认；V0 dirty baseline 未完成前不得执行 E1-E7/V1/C1。
- reviewer：已运行
- 审查结论：有发现，已采纳修订：补 V0 dirty baseline gate；撤掉 position-row 先行预设；把 D3/D4 拆成 position eligibility 与 persistent manifold 两道设计门；E3 限定为 solver-local pseudo-state；修正 `rtk proxy env` 命令；补 `world_step_review_regressions` / `query_debug_contract` / aligned matrix / browser start gate。
- 用户确认：已确认，按 `subagent-current-workspace` 在当前工作区开始动工；最后通过 `browser-use:browser` 新 tab 验收 web 效果。

## 设计 / 执行分离规则

- 设计里程碑只允许修改计划文档、设计文档、调研记录或测试计划，不修改生产实现代码。
- 执行里程碑必须引用已批准的设计结论，或者显式说明为什么不需要设计前置。
- 如果执行中发现设计假设不成立，暂停执行并回到对应设计里程碑更新计划。
- 设计里程碑完成不等于允许实现；实现仍需经过 Execution Gate 或用户确认。
- 一个设计里程碑可以生成多个执行里程碑；一个执行里程碑必须绑定清晰的设计输入。

## 里程碑映射

| 设计里程碑 | 产出 | 解锁的执行里程碑 | 状态 |
| --- | --- | --- | --- |
| D1 | step 事实流、contact persistence、position-row、sub-step、sleep 的架构边界与排序决策 | E1, E2, E3, E7 | 待确认 |
| D2 | 验收矩阵、红锁升级策略、negative patch guard、browser acceptance 边界 | E1, V1, C1 | 待确认 |
| D3 | position-row eligibility / source-row pressure 输入契约 | E3, E4, E5 | 待确认 |
| D4 | persistent manifold / contact lifecycle 前移契约 | E6；若 E2 证明 identity churn 是主因，则也会重新排序 E3-E5 | 待确认 |

## 里程碑

### V0：工作区 Baseline Gate

类型：验证
状态：已完成

目标：
在任何执行里程碑前确认 dirty worktree 的归属和文件所有权，避免把 live gravity / web / AI routing 等他线 WIP 混入 solver 重构。

范围：
- 读取 `git status --short`。
- 列出 dirty 文件与本计划可能触碰文件的交集。
- 给出执行前建议：先提交/搁置他线 WIP、限制本计划文件范围，或由用户明确批准在 dirty workspace 中继续。

不做：
- 不 revert、stash、format、commit 或清理用户改动。
- 不修改实现代码。

验收标准：
- 执行里程碑的文件所有权和 dirty 交集被明确记录。
- 若存在重叠，暂停等待用户确认处理方式。
- 本次用户已确认“开始动工”，但该确认不等于允许吸收所有既有 dirty WIP；当前 solver stability 执行先避开已 dirty 的 web/server/AI routing/live gravity 文件。

V0 结论：
- `git status --short` 显示已有 `crates/picea-lab` server/web/runtime controls、`docs/ai/*`、`docs/design/*`、`crates/picea/src/recipe.rs` / `world/*` 等未提交改动。
- `rtk proxy git diff --check` 已通过。
- E1/E2 立即执行范围限定为未 dirty 或可独立审查的测试 / contact evidence 文件，例如 `crates/picea-lab/tests/artifact_run.rs`、`crates/picea/tests/physics_realism_acceptance.rs`、`crates/picea/src/pipeline/contacts.rs`、`crates/picea/src/world/contact_state.rs`；不触碰当前 dirty 的 web/server/AI routing 文件。
- `docs/design/matrix-stack-stability-optimization-design.md` 当前已 dirty，E1 的 negative patch guard 优先通过测试名 / 新计划进度记录表达；除非后续明确需要，不把该文档纳入 worker 写入范围。
- V1 浏览器验收允许使用当前 web 现场做产品检查，但若发现问题，需要先区分是本轮 solver 改动、既有 web WIP，还是产品交互遗留。

验证方式：
- `git status --short`
- `rtk proxy git diff --check`

Subagent 执行计划：
- verifier：可选；只读报告 dirty overlap。

提交策略：
- auto-commit：否

风险 / 后续：
- 当前已有重叠风险；V0 是 Execution Gate 的硬前置。

### D1：Step 事实流与 Solver 重构架构边界

类型：设计
状态：计划中

目标：
明确 Picea 参考成熟物理引擎后的内部重构路线，决定 contact persistence、position-row、sub-step、sleep 的先后顺序和不变量。

要回答的问题：
- `contacts -> warm-start -> solver -> island -> sleep` 事实流中，哪些事实必须先稳定，哪些可以 shadow 记录。
- 是否继续保持 M28 separate-phase contact/joint ordering。
- contact persistence / feature-id lifecycle 是否必须先于 position-row 行为变更。
- dense position-row pass 是替代 residual correction，还是先作为 shadow / gated path 进入。
- sub-step / TGS-like cadence 是否进入本轮，还是后置为独立 milestone。

为什么现在做：
用户接受大重构，但成熟引擎的做法不是“先挑一个函数重写”。D1 先把事实流和不变量固定，避免把 position-row、identity lifecycle、sleep、sub-step 混在同一个执行 diff 里。

设计范围：
- `crates/picea/src/pipeline/step.rs`
- `crates/picea/src/pipeline/contacts.rs`
- `crates/picea/src/solver/contact.rs`
- `crates/picea/src/pipeline/island.rs`
- `crates/picea/src/pipeline/sleep.rs`
- `docs/design/matrix-stack-stability-optimization-design.md`
- `docs/design/solver-island-ordering-contract.md`

不做：
- 不改生产实现代码
- 不改变 public API
- 不引入外部物理引擎依赖
- 不调整默认 iteration / `dt` / sleep threshold

输入证据：
- 2026-05-06 / 2026-05-07 matrix-stack 设计和负结果记录。
- explorer 对 step-level facts flow 的只读结论。
- reviewer 对 position-row 先行假设的审查意见。
- Box2D PGS / NGS / warm start / sub-step / contact anchor update 思路。
- Matter.js position / velocity solve 分层。

输出交付物：
- 架构决策：默认顺序是先 E2 contact persistence evidence，再 D3/E3 position-row shadow/enable；若 E2 证明 identity churn 是主因，则暂停 E3 并前移 D4/E6。
- 执行文件边界和不变量列表。
- E1-E7 的设计输入更新。
- 明确可接受大重构范围和禁止路线。

设计验收：
- 每个执行里程碑都能引用明确设计输入。
- 不变量覆盖 public API、solver ordering、determinism、validation gates。
- sub-step、iteration、sleep 不再混入同一个执行 milestone。
- 明确哪些执行里程碑可以触碰 `step.rs` / `contacts.rs` / `contact.rs` / `island.rs` / `sleep.rs`，哪些必须避开 public API 和 Web。

验证方式：
- 文档检查
- reviewer 审查
- `rtk proxy git diff --check`

下一步映射：
- 生成/更新执行里程碑：E1, E2, E3, E7。

风险 / 后续：
- 如果 D1 发现必须改变 step cadence 或 solver ordering，应暂停并扩展计划，而不是直接进入 E2。
- 如果 D1 发现 E2/E3 缺少事实输入，应把 D3/D4 前移为阻塞设计门，而不是让 worker 在实现中补猜。

### D2：稳定性验收矩阵与红锁升级

类型：设计
状态：计划中

目标：
把旧计划中的 `stack_4`、aligned matrix、180-frame matrix、240-frame E4c、600-frame acceptance 整理成大重构验收矩阵。

要回答的问题：
- 哪些测试是 hard gate，哪些是 ignored red lock，哪些只做 observation。
- negative patch guard 是否需要写入固定测试或设计文档。
- browser acceptance 在 correctness 验收中的位置是什么。
- shared solver/pipeline 改动需要哪些更广回归门。

为什么现在做：
大重构容易造成某一指标改善、另一指标退化。验收矩阵必须先清楚，worker 才能知道什么时候撤补丁而不是继续微调。

设计范围：
- `docs/design/matrix-stack-stability-acceptance.md`
- `docs/design/matrix-stack-stability-optimization-design.md`
- `crates/picea-lab/tests/artifact_run.rs` 的测试计划和现有 test names
- `crates/picea/tests/physics_realism_acceptance.rs` 的测试计划和现有 test names

不做：
- 不修改测试实现
- 不新增阈值代码
- 不把 browser 视觉结果当成唯一 correctness gate

输入证据：
- `matrix_stack_artifacts_capture_nxm_grid_stack_facts`
- `aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock`
- `matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window`
- `matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed`
- 近期被撤销的三个负补丁结果

输出交付物：
- 大重构验收矩阵。
- 红锁升级顺序：先保 baseline，后解 240-frame support-gap，再启用 600-frame acceptance。
- browser acceptance 检查项：先用 `rtk proxy just picea-lab-web-start` 起服务，再通过 in-app browser 检查 artifact/live；不要求 UI 自行判断物理正确性。

设计验收：
- E1-E7 都有明确验证命令。
- 负结果不会被误判为成功。
- 第一执行 milestone 的测试范围足够小。

验证方式：
- 文档检查
- reviewer 审查
- `rtk proxy git diff --check`

下一步映射：
- 生成/更新执行里程碑：E1, V1, C1。

风险 / 后续：
- 如果阈值过严，可能阻塞架构切换；计划允许先把 600-frame gate 作为 env-controlled acceptance，再逐步升级。

### D3：Position Eligibility / Source-row Pressure 输入契约

类型：设计
状态：计划中

目标：
冻结 position-row solve 和 source-row pressure propagation 需要的输入事实、拒绝条件和 solver truth 边界，避免 E3/E4 靠单个 frame 的 retain/drop heuristic 决策。

要回答的问题：
- `source_row_continuity_candidate` 是否足够，还是必须拆成 geometry、pressure、motion、energy、budget 多个字段。
- pseudo-position support skin 如何从 artifact dry-run 进入 solver，而不重跑 full narrowphase。
- counterpart motion / tangent energy / rotation energy 的 gate 应该在哪里计算。
- E3 的 solver-local pseudo-state 能输出哪些 evidence，哪些必须留给 E4。

为什么现在做：
旧 E4c 证据显示，局部 anchor 连续不等于应该保留支撑；不先冻结输入契约，E3/E4 很容易再走向“合成支撑”或“误删 correction”。

设计范围：
- `crates/picea/src/pipeline/contacts.rs`
- `crates/picea/src/solver/contact.rs`
- `crates/picea-lab/tests/artifact_run.rs`
- `docs/design/matrix-stack-stability-optimization-design.md`

不做：
- 不改生产实现代码
- 不新增 public debug schema
- 不把 artifact dry-run 事实直接当 solver truth
- 不设计 persistent manifold identity fallback；那属于 D4

输入证据：
- E4c support-gap upstream trace。
- `source_row_dry_run_facts` / `source_row_pseudo_position_facts` 测试侧 dry-run。
- `ContactObservation.source_row_continuity_candidate`。

输出交付物：
- Source-row / position-row 输入字段和 gate 列表。
- 哪些字段留在 lab diagnostics，哪些进入 core solver row。
- E3/E4 的实现边界。

设计验收：
- 执行者能从契约写出 targeted tests。
- E3 被限定为 solver-local pseudo-state，不做 lifecycle eligibility。
- source-row 不再只用布尔 candidate 决定 position correction。
- 明确禁止“前一帧合成 retained support”。

验证方式：
- 文档检查
- reviewer 审查
- `rtk proxy git diff --check`

下一步映射：
- 生成/更新执行里程碑：E3, E4, E5。

风险 / 后续：
- 如果需要新增 core diagnostics 字段，必须保持 additive，并更新 lab/web schema 验证。

### D4：Persistent Manifold / Contact Lifecycle 输入契约

类型：设计
状态：计划中

目标：
冻结 persistent manifold / contact lifecycle 语义前移的条件，明确什么时候可以从 `ContactObservation`/solver row 层前移到 gather/reduce/narrowphase identity 层。

要回答的问题：
- previous same-pair / local-anchor / normal-dot / edge-swap / feature-id miss 的权重边界是什么。
- 哪些 lifecycle transition 可以影响 warm-start / manifold identity，哪些只能保留 diagnostics。
- 一旦需要改 `pipeline/narrowphase.rs`，是小范围 feature-id 稳定化，还是必须另开 SAT/feature-id 设计门。

为什么现在做：
成熟引擎稳定堆叠依赖 persistent contacts / manifold identity，但 reviewer 已指出这不应和 E3 position-row 混在一起。D4 是 E6 的前置设计门，只有 E2 或 E4 证明 identity lifecycle 是 blocker 时才进入实现。

设计范围：
- `crates/picea/src/pipeline/contacts.rs`
- `crates/picea/src/world/contact_state.rs`
- `crates/picea/src/solver/contact.rs`
- `docs/design/matrix-stack-stability-optimization-design.md`
- `crates/picea/src/pipeline/narrowphase.rs` 只允许做只读分析

不做：
- 不改生产实现代码
- 不扩大 warm-start fallback
- 不重写 narrowphase SAT / clipping

输入证据：
- E2 contact persistence evidence。
- E4c feature churn / support-gap lifecycle evidence。
- reviewer 对 D3/E5 边界模糊的审查意见。

输出交付物：
- Persistent manifold 输入契约。
- 是否允许 E6 触碰 `pipeline/contacts.rs` / `world/contact_state.rs`。
- 若需要改 `pipeline/narrowphase.rs`，必须回 D4 更新设计门；执行里程碑不得悄悄纳入。

设计验收：
- E6 的所有权和拒绝条件清晰。
- contact lifecycle 不再和 position eligibility 混成同一个 milestone。
- narrowphase identity 改动必须有单独 stop condition。

验证方式：
- 文档检查
- reviewer 审查
- `rtk proxy git diff --check`

下一步映射：
- 生成/更新执行里程碑：E6；必要时重排 E3-E5。

风险 / 后续：
- 如果 D4 证明必须改 SAT/reduce feature-id，当前计划应暂停并拆出新计划，而不是继续执行 E6。

### E1：重构前行为锁与 Negative Patch Guard

类型：执行
状态：已完成
来源设计：D2

目标：
在大重构前固定当前红锁、基线、负补丁保护，确保后续 worker 不能用已知失败路线“修绿一处、弄坏一处”。

执行输入：
- V0 已确认工作区 baseline。
- D2 的验收矩阵。
- 允许修改测试 / 设计文档，不修改 solver 行为。

为什么现在做：
大重构第一步应该先保护现有事实；如果直接改 solver，很难判断退化来自架构还是阈值漂移。

范围：
- 补强或整理现有 artifact / physics realism 测试。
- 把已撤销的负补丁路线写成明确 guard 或设计文档禁区。
- 保证所有测试命名能指向 E2-E7。

不做：
- 不改 solver 行为
- 不改场景初始条件来规避失败
- 不做浏览器产品调整

所有权：
- 可能触及：`crates/picea-lab/tests/artifact_run.rs`, `crates/picea/tests/physics_realism_acceptance.rs`, `docs/design/matrix-stack-stability-acceptance.md`, `docs/design/matrix-stack-stability-optimization-design.md`
- 不应触及：`crates/picea/src/solver/contact.rs`, `crates/picea/src/pipeline/*`, `crates/picea-lab/web/src/*`

验收标准：
- `stack_4` hard gate 保持通过。
- aligned matrix gate 保持通过。
- 180-frame `matrix_stack` baseline 保持可重复。
- 240-frame E4c red lock 仍能暴露当前 support-gap / ejection blocker。
- 已知失败路线在文档或测试名中清楚可见。

验证方式：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --ignored --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要；确认现有测试覆盖和最小 guard 缺口。
- worker：`gpt-5.4`；只改测试/文档所有权文件。
- reviewer：检查是否误改 solver 或扩大阈值。
- verifier：运行本 milestone 验证命令。

提交策略：
- auto-commit：否
- message hint：matrix stack stability guard baseline

风险 / 后续：
- 240-frame red lock 当前预期失败；本 milestone 只要求它暴露 blocker，不要求转绿。

### E2：Contact Persistence Evidence / Shadow Guard

类型：执行
状态：已完成
来源设计：D1, D2

目标：
在不改变 solver 数值行为的前提下，补足 contact identity / warm-start / feature-id lifecycle 的证据和 shadow guard，判断是否必须先走 D4/E6 persistent manifold 路线。

执行输入：
- V0 已确认工作区 baseline。
- D1 的 step facts flow。
- D2 的 baseline gates。

为什么现在做：
既有路线把 contact persistence 放在 position correction 前面；reviewer 指出不能未经证明就反转顺序。E2 先把 identity churn 是否是主因变成可验证事实。

范围：
- 强化 `source_row_continuity_candidate`、warm-start reason、same-pair/local-anchor/normal-dot evidence 的测试侧或 additive diagnostics。
- 增加 shadow-only evidence，证明当前 blocker 是 identity lifecycle、position-row stale depth，或二者交织。
- 不改变 normal/tangent impulse、position correction 或 contact persistence 行为。

不做：
- 不启用新的 warm-start fallback。
- 不改变 feature-id 生成。
- 不改 position-row solve。
- 不改 web。

所有权：
- 可能触及：`crates/picea/src/pipeline/contacts.rs`, `crates/picea/src/world/contact_state.rs`, `crates/picea-lab/tests/artifact_run.rs`, additive diagnostics if needed
- 不应触及：`crates/picea/src/solver/contact.rs` 数值行为, `crates/picea/src/pipeline/narrowphase.rs`, `crates/picea-lab/web/src/*`

验收标准：
- `stack_4` 不退化。
- 180-frame `matrix_stack` baseline 不退化。
- 报告能判断是否应继续 E3 position-row，或暂停转 D4/E6。
- shadow evidence 不改变 final state hash，除非 D1/D2 已明确允许。

验证方式：
- `rtk proxy cargo test -p picea --lib pipeline::contacts`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo fmt --all --check`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要；确认现有 contact evidence 与 minimal additive path。
- worker：`gpt-5.4`；只做 shadow/evidence，不改 solver 数值。
- reviewer：重点审查 behavior equivalence 和 diagnostics ownership。
- verifier：运行本 milestone 验证命令。

提交策略：
- auto-commit：否
- message hint：capture dense contact lifecycle evidence

风险 / 后续：
- 如果 E2 证明 identity churn 是 primary blocker，暂停 E3，先执行 D4 并重排 E6。

### E3：Dense Position-Row 数据结构 Shadow Path

类型：执行
状态：已完成
来源设计：D1, D3

目标：
把旧 residual correction 里的 dense pseudo-translation seam 拆成可测试的 position-row 数据结构和纯计算路径，只做 solver-local pseudo-state / shadow path，不做 lifecycle eligibility，不改变默认 solver 行为。

执行输入：
- V0 已确认工作区 baseline。
- E2 未证明 identity lifecycle 是先行 blocker，或 D4 已更新排序。
- D3 的 position eligibility 输入契约。

为什么现在做：
真正的 Box2D-style position solve 需要独立 row/state 边界；但按 reviewer 意见，本 milestone 只建立 solver-local pseudo-state，不偷渡 source-row eligibility 或 persistent manifold。

范围：
- 引入 `DensePositionRow` / `PositionSolveBody` / pseudo-pose 或等价内部结构。
- 把 residual correction depth 计算、pseudo translation accumulation、stats 写入拆成小函数。
- 添加小型 unit tests 验证 pseudo-state depth re-evaluation。
- 可做 shadow-only comparison，记录 stale-depth 与 pseudo-depth 差异。

不做：
- 不改变 `position_iterations` 默认值。
- 不启用 source-row retain/drop。
- 不改 warm-start identity。
- 不改 sleep threshold。
- 不改变默认 solver 行为。

所有权：
- 可能触及：`crates/picea/src/solver/contact.rs`，必要时新增 `crates/picea/src/solver/position.rs` 或等价内部模块；`crates/picea/src/solver/mod.rs`
- 不应触及：`crates/picea/src/pipeline/contacts.rs` lifecycle 语义，`crates/picea/src/pipeline/narrowphase.rs`, public API files, scenario setup, web files

验收标准：
- `stack_4` gate 不退化。
- `matrix_stack` 180-frame baseline 不退化。
- 新增 unit tests 证明 pseudo-state re-evaluation 会降低 stale depth，不写 velocity。
- 默认 behavior hash 若改变，必须回到 D1/D3。

验证方式：
- `rtk proxy cargo test -p picea --lib solver`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo fmt --all --check`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：可选；若 E2 后文件边界仍清晰可跳过。
- worker：`gpt-5.4`；限定 solver 内部结构拆分。
- reviewer：重点审查行为等价、默认路径、public API 未泄漏。
- verifier：运行本 milestone 验证命令。

提交策略：
- auto-commit：否
- message hint：split dense position row shadow state

风险 / 后续：
- 结构拆分可能导致大量移动代码；必须避免顺手重构 velocity impulse solve。
- non-dense `stack_4` 保持 legacy path 是首轮风险控制；何时统一必须由后续 D1/D3 更新决定，不能在 E3 内顺手做。

### E4：启用 Dense Position-Row / Pseudo-Pose Solve

类型：执行
状态：已验证，仍有后续 blocker
来源设计：D1, D2, D3

目标：
在 dense contact graph 中启用 solver-local position-row / pseudo-pose pass，使每轮 position iteration 基于 pseudo-state 重新评估 separation；不处理 source-row lifecycle eligibility。

执行输入：
- E3 的 shadow path。
- D3 的 position eligibility 边界。
- D2 的验收矩阵。

为什么现在做：
这是成熟引擎方向里 position solve 的核心切片，但必须保持纯 solver-local，避免和 contact lifecycle 混成不可审查大 diff。

范围：
- dense-only position-row solve。
- 每轮 iteration 只写 pseudo pose / queued pose translation，不写 velocity。
- 保持 non-dense `stack_4` legacy path，除非 D1/D3 明确允许统一。
- 更新 position correction stats 和 diagnostics。

不做：
- 不复用 non-dense two-point block normal solve 到 dense graph。
- 不提高默认 velocity iterations。
- 不做 source-row pressure propagation。
- 不调整 sleep。
- 不改 contact persistence / lifecycle。

所有权：
- 可能触及：`crates/picea/src/solver/contact.rs`，E3 新增 solver 内部模块，必要时 additive diagnostics。
- 不应触及：`crates/picea/src/pipeline/contacts.rs` lifecycle 语义，`crates/picea/src/pipeline/narrowphase.rs`, `crates/picea-lab/web/src/*`

验收标准：
- `stack_4` hard gate 通过。
- aligned matrix 不退化。
- 180-frame `matrix_stack` support gap 不早于 retained baseline，duration 不变长，no floor ejection。
- 240-frame E4c 若仍红，只允许显示 solver-local stale-depth 已改善但仍需要 E5/E6；不能新增更早 ejection。

验证方式：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --ignored --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo test -p picea --test query_debug_contract`
- `rtk proxy cargo fmt --all --check`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要；确认 E3 后代码 seam 和 minimal implementation path。
- worker：`gpt-5.4`，若 D1 认为实现高风险可升到继承主模型；文件所有权仅 solver。
- reviewer：重点审查 pseudo-state 数学符号、mass weighting、velocity 不变性、dense-only gate。
- verifier：运行本 milestone 验证命令。

提交策略：
- auto-commit：否
- message hint：enable dense position row solve

风险 / 后续：
- 如果 E2 已经证明 identity churn 是 blocker，本里程碑不得绕过 D4/E6 直接启用 position-row。
- 如果 180-frame 退化，立即回滚或回到 D1/D3。

### E5：Source-Row Pressure Propagation Gate

类型：执行
状态：部分完成，触发 D4/E6
来源设计：D3

目标：
把 source-row pressure propagation 从测试侧 dry-run 变成 solver 内部受约束的 gate，处理 240-frame support-gap / ejection blocker。

执行输入：
- D3 的 source-row 输入契约。
- E4 的 position-row pass。

为什么现在做：
E4c 证据表明真正 blocker 不只是 stale depth，而是 dense pressure 在 contact lifecycle 和 position correction 之间传递时缺少受控 gate。

范围：
- 将 geometry continuity、pseudo-state support skin、counterpart motion、tangent/rotation energy、frame-level correction budget 引入 position-row eligibility。
- 只在 dense contact graph 和 source-row pressure source 上启用。
- 保留 diagnostics 说明 retain/drop/reject 原因。

不做：
- 不在前一帧合成 retained support。
- 不扩大 warm-start fallback。
- 不改变 non-dense contact pair behavior。
- 不改 persistent manifold identity；那属于 D4/E6。

所有权：
- 可能触及：`crates/picea/src/pipeline/contacts.rs`, `crates/picea/src/solver/contact.rs`, additive diagnostics in `crates/picea/src/events.rs` / lab artifact if needed
- 不应触及：scenario initial layout, frontend rendering logic；`crates/picea/src/pipeline/narrowphase.rs` 必须回 D4 重开设计门后才允许纳入。

验收标准：
- 240-frame E4c red lock 转绿，或失败时不早于当前 first floor exit frame 且 blocker 更具体。
- 180-frame `matrix_stack` 无退化。
- `stack_4` 与 aligned matrix 通过。
- source-row eligibility/reject reason 可从 artifact 报告解释。

验证方式：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --ignored --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`
- `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo test -p picea --test query_debug_contract`
- `rtk proxy cargo fmt --all --check`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要；确认 D3 字段和现有 dry-run facts 映射。
- worker：`gpt-5.4` 或高风险时继承主模型；文件所有权覆盖 contacts/solver，避免 web。
- reviewer：重点审查 speculative support、energy gate、diagnostics 与 solver truth 边界。
- verifier：运行本 milestone 验证命令。

提交策略：
- auto-commit：否
- message hint：gate dense source row pressure propagation

风险 / 后续：
- 这是最可能需要重构 contact lifecycle 的 milestone；如果布尔 candidate 不够，暂停进入 D4/E6，不继续堆条件。

### E6：Persistent Manifold / Contact Lifecycle 深化

类型：执行
状态：待执行，已前移
来源设计：D4

目标：
如果 E2/E5 证明 `source_row_continuity_candidate` 或现有 warm-start lifecycle 不足，则把 persistent manifold / contact lifecycle 语义前移到 gather/reduce 层，降低 dense stack 的 feature churn 和 warm-start drop。

执行输入：
- D4 的 lifecycle 契约。
- E2/E5 的失败或剩余风险证据。

为什么现在做：
成熟引擎稳定堆叠依赖 persistent contacts / manifold identity；如果只在 solver 末端兜底，会继续出现 feature-id churn 和冷启动 row。

范围：
- 明确 manifold point identity / local anchor continuity / feature-id transition 的保守 fallback。
- 只对经 D4 证明安全的 lifecycle transition 生效。
- 保持 ContactKey / ContactPairKey public-debug 兼容。

不做：
- 不把 edge-swap candidate 全量当成 warm-start hit。
- 不把 artifact dry-run 数字搬成无条件 solver truth。
- 不重写 narrowphase SAT / clipping。

所有权：
- 可能触及：`crates/picea/src/pipeline/contacts.rs`, `crates/picea/src/world/contact_state.rs`, `crates/picea/src/solver/contact.rs`, `crates/picea-lab/tests/artifact_run.rs`
- 不应触及：`crates/picea/src/pipeline/narrowphase.rs`；一旦需要改 narrowphase identity，必须回 D4 重开设计门。

验收标准：
- warm-start miss/drop 在 matrix stack 中改善，同时不引入早期 ejection。
- `stack_4` 不退化。
- 240/600-frame gates 至少不比 E5 退化。
- lifecycle diagnostics 能说明 fallback 生效范围。

验证方式：
- `rtk proxy cargo test -p picea --lib pipeline::contacts`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`
- `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo test -p picea --test query_debug_contract`
- `rtk proxy cargo fmt --all --check`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：需要；确认 contact_state 和 contacts ownership。
- worker：架构敏感，优先继承主模型或显式 `gpt-5.5`；只在 lifecycle 文件范围内改动。
- reviewer：重点审查 warm-start 错配、feature-id fallback 过宽、contact event 语义。
- verifier：运行本 milestone 验证命令。

提交策略：
- auto-commit：否
- message hint：harden persistent manifold lifecycle

风险 / 后续：
- 如果必须修改 narrowphase reduce identity，计划需要回到 D4 扩展范围或拆出新计划。

### E7：Long-Window Convergence / Sleep 收口

类型：执行
状态：计划中
来源设计：D1, D2

目标：
在 no-ejection 和 pressure propagation 稳定后，处理长窗口 residual motion 与 sleep 收敛，避免用 sleep 掩盖穿透或 ejection。

执行输入：
- E4/E5/E6 已让 240/600-frame no-ejection 接近或通过。
- D2 明确 sleep 只是最后一层收敛目标。

为什么现在做：
sleep 在成熟引擎里是稳定和性能的组成部分，但它不能先于 contact/position solve 修正，否则只是把错误冻结。

范围：
- 分析 wake reason、position correction reset idle、island sleep thresholds。
- 若有必要，调整 dense stack 下的 idle reset 或 sleep eligibility。
- 增加 long-window sleep/convergence behavior lock。

不做：
- 不在 ejection 未解决前调 sleep threshold。
- 不让 sleeping body 跳过必要 collision response。
- 不改 live session cadence。

所有权：
- 可能触及：`crates/picea/src/pipeline/sleep.rs`, `crates/picea/src/solver/contact.rs`, `crates/picea/tests/physics_realism_acceptance.rs`, `crates/picea-lab/tests/artifact_run.rs`
- 不应触及：contact lifecycle fallback，web UI

验收标准：
- 600-frame matrix stack no ejection。
- 低速/睡眠指标改善，且没有冻结穿透。
- `physics_realism_acceptance sleep` 和 `stack` 不退化。

验证方式：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack -- --nocapture`
- `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo test -p picea --test query_debug_contract`
- `rtk proxy cargo fmt --all --check`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：可选；若 E5/E6 输出足够可跳过。
- worker：`gpt-5.4`；限定 sleep/contact wake reason。
- reviewer：重点审查是否掩盖 solver 错误。
- verifier：运行本 milestone 验证命令。

提交策略：
- auto-commit：否
- message hint：settle dense stack sleep convergence

风险 / 后续：
- 如果 no-ejection 还没过，不执行本 milestone。

### V1：Artifact / Browser 产品验收

类型：验证
状态：计划中

目标：
确认核心稳定性修复能被 `picea-lab` artifact 和 `picea-lab-web` live/artifact 两条用户路径看到，并且不会重新引入实时卡顿或解释混乱。

范围：
- 验证 artifact replay 的 matrix stack 报告。
- 验证 `Rust 实时会话` 下矩阵堆叠运行可解释，若性能 degraded，UI 诚实显示 degraded realtime。
- 验证底部 diagnostics / evidence / run settings 不被本轮 solver 改动破坏。

不做：
- 不新增 UI 功能。
- 不把 web 视觉结果当 solver correctness 唯一证据。
- 不修 unrelated live performance 问题。

验收标准：
- artifact replay 和 live session 都能选择 `matrix_stack 8x6`。
- 画布不会出现明显 ejection / runaway drift。
- diagnostics 能解释当前 frame 的 source、contacts、stability facts。
- browser use 不可用时，记录 fallback 和未完成项。

验证方式：
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- `rtk proxy just picea-lab-web-start`
- in-app browser：打开 `http://127.0.0.1:5173/?picea-profile=1`，检查 `Rust 生成产物并回放` 和 `Rust 实时会话` 的 `matrix_stack 8x6`。
- `rtk proxy just picea-lab-web-stop`
- `rtk proxy git diff --check`

Subagent 执行计划：
- verifier：运行 web build/contracts；browser acceptance 由主 Codex 或 browser-use 执行。

提交策略：
- auto-commit：否
- message hint：verify matrix stack product acceptance

风险 / 后续：
- 当前 Browser Use 工具可能不可用；如果不能浏览器验证，必须如实记录。
- V1 必须在 V0 确认 server/web dirty baseline 后执行，否则结果可能混入他线 WIP。

### C1：文档路由与计划收尾

类型：收尾
状态：计划中

目标：
把最终 solver 路线、验证命令、剩余风险同步到 AI routing 和设计文档，方便后续 session 接续。

范围：
- 更新本计划进度记录。
- 必要时更新 `docs/ai/repo-map.md` / `docs/ai/index.md` 对 matrix stack stability routing。
- 更新 `docs/design/matrix-stack-stability-optimization-design.md` 的最终决策和负结果。

不做：
- 不修改实现。
- 不改 unrelated docs。

验收标准：
- 后续 Codex 能从 docs routing 找到当前 solver stability plan。
- 最终验证命令和结果被记录。
- 剩余风险明确。

验证方式：
- `rtk proxy git diff --check`
- 文档检查

Subagent 执行计划：
- reviewer：只读审查文档路由是否准确。

提交策略：
- auto-commit：否
- message hint：document matrix stack stability refactor

风险 / 后续：
- 如果用户要求提交，应在所有 gate 后由主 Codex 统一 commit。
- C1 必须在 V0 确认 docs/ai dirty baseline 后执行，否则容易吸收他线 routing WIP。

## 进度记录

### 2026-05-08 - E4 dense resting-contact damping 落地

- 状态：部分完成，目标未完成
- Commit：none
- 背景：前一帧 retained support contact / synthetic `ContactObservation` 实验已撤回。三档实现均无法保留：`SUPPORT_RETENTION_SKIN=0.006`、`0.003`、以及 zero-depth/no-warm-start retained row 都会让 180-frame matrix hard gate 的 penetration 退化到约 `0.050571..0.050572`，说明 retained row 本身会污染 dense-stack dynamics；若未来需要 speculative contact，必须是独立 constraint 模型，不能塞回普通 `ContactObservation`。
- 实现：在 `contact.rs` 增加 Box2D/Matter 风格的 resting-contact energy dissipation：dense contact island 在 velocity solve 后耗散接触切向速度和角速度；另加窄 static resting support 判定，只对“动态体接静态支撑、浅穿透、低分离速度、非 restitution、且有摩擦”的接触给浅层支撑摩擦/阻尼入口。该路径不改 narrowphase、warm-start identity、dt/substep、scene fixture 或 public API。
- 接受的参数：dense tangent / angular damping 均为 `20.0/s`。该档通过 180-frame matrix hard gate、240-frame support-gap gate、600-frame long-settle no-ejection acceptance，且 180 late correction total 降到约 `0.734675`，低于旧 retained baseline。
- 拒绝的参数：`30.0/s` 通过 600 no-ejection，但 180 late correction total 升到约 `0.743574`，超过旧 hard gate；`40.0/s` 把最终速度压得更低，但 180 late correction total 约 `0.753520`，600 penetration 可升到约 `0.052508`，说明单纯加阻尼会把堆叠压深，不能作为 E5 sleep 收敛解。
- 当前结果：E4 的“不飞散/不出地面支撑”已推进：600-frame `first_floor_exit_frame=None`、`final_outside_floor_body_count=0`。但 E5 “最终静止/全量 sleep”尚未完成：600-frame 仍有 awake bodies，最大速度窗口仍高于 sleep 阈值。因此 `/goal` 仍未完成。
- 已验证：`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --lib solver::contact::tests -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --nocapture`。
- 下一步：E5 不应继续盲目提高 damping。应把 sleep 收敛作为单独机制设计：让 dense resting island 的位置修正压力、warm-start churn 和低速阈值共同决定 sleep eligibility，同时保持 180/240/600 hard gates 不回退。

### 2026-05-08 - 后续稳定性实验复核与拒绝记录

- 状态：进行中
- Commit：none
- 目标对齐：用户明确 `/goal` 为多堆叠稳定并最终静止；本轮允许大规模重构，但所有改动必须服务于该目标。
- 已撤回实验：
  - 全局/非门控 two-point block normal solve：`stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling` 退化，settled penetration 达到约 `0.2897398`，违背小堆叠行为锁。
  - 场景 fixture damping：`0.8/4.0` 虽能延后 240-frame ejection，但 600-frame 仍 first exit frame `397`，且 penetration 上升到约 `0.075`；这是场景调参，不是 engine 稳定性修复。
  - matrix-stack naive `substeps=4`：180-frame artifact gate 退化为 first floor exit frame `77`，final outside floor bodies `1`，说明当前 support-friction / residual-correction 语义不能直接缩小 dt 后复用。
  - dense-scene support-friction 扩散：把 dense 判定从 island 扩到全场景后，240-frame late support gap 从 frame `238` 提前到 frame `235`，没有移除 support gap。
  - dense-only 低比例 position correction：180/240 均明显退化，180 first floor exit frame `80`，quiet-window max linear 超过 `15`，说明简单降低 Baumgarte 不能保持支撑。
  - edge-swap source-row gating candidate：不迁移 impulse，仅扩展 source-row candidate 到 EdgeSwap 后，180 不飞出但 support gap 提前到 frame `155`，破坏当前 retained baseline。
- 验证仍通过：撤回失败实验后，`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --lib pipeline -- --nocapture` 通过；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture` 通过。
- 当前基线：240-frame red lock 已从“直接 ejection”推进到“no ejection but late support gap”；600-frame long-settle 仍会 runaway/eject，目标尚未完成。
- 下一步决策：停止保留已经证伪的全局 knobs。后续应设计更结构化的 contact-manifold / position-solver contract，例如 persistent manifold identity 与 residual position correction 的输入分离，而不是继续叠加 damping、substep、全局 friction 或宽 warm-start。

### 2026-05-08 - D4/E6 source-row provenance-first 分类

- 状态：已完成
- Commit：none
- 改动范围：`crates/picea/src/events.rs`、`crates/picea/src/debug.rs`、`crates/picea/src/lib.rs`、`crates/picea/src/pipeline/contacts.rs`、`crates/picea/tests/query_debug_contract.rs`、`crates/picea-lab/tests/artifact_run.rs`、`crates/picea-lab/web/src/types.ts`、本计划文档；未改 `crates/picea/src/solver/contact.rs`、`crates/picea/src/pipeline/narrowphase.rs`。
- 实现：把原私有 `SourceRowContinuityClassification` 升格为 serde-friendly `SourceRowContinuityReason` 并透传到 `ContactEvent` / `DebugContact` / web type；`ContactObservation` 现在保存 structured reason，`source_row_continuity_candidate` 仍只在 `warm_start_reason == MissFeatureId` 且 reason 为 `Candidate` 时为真。该字段严格属于 provenance/diagnostics，不代表 warm-start impulse transfer 或 solver row 复用。
- 行为锁：`pipeline::contacts` 单元测试覆盖 source-row candidate、edge-swap rejection、anchor-drift rejection、pair-mismatch、empty no-previous-pair，并继续锁定 candidate 路径下 `warm_start_normal_impulse` / `warm_start_tangent_impulse` / solver seed impulse 都为 0，避免重引入已撤销的 impulse transfer 实验。
- reviewer 修正：`SourceRowContinuityReason` 已进入 prelude / query debug contract；web live ring 字段保持 optional，避免这条 provenance slice 强制吸收 runtime-control WIP；live pause/resume 浏览器记录已移回 runtime-control 计划。
- 验证通过：`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --lib pipeline::contacts -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test query_debug_contract -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test world_step_review_regressions -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`；`rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`；`rtk proxy npm --prefix crates/picea-lab/web run test:i18n`；`rtk proxy npm --prefix crates/picea-lab/web run build`；`rtk proxy cargo fmt --all -- --check`；`rtk proxy git diff --check`。
- 预期红锁：`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture` 仍失败，first floor exit 为 frame 223；upstream trace 已消费 core-owned `continuity_reasons [Candidate]`，但残留 blocker 已不再是 source-row correction-pressure dominated。
- 决策：本 slice 仍是 provenance-first additive schema，不改变 warm-start transfer、solver math、contact identity allocation 或 narrowphase。后续 D4/E6 应继续围绕 lifecycle/manifold 输入设计推进，而不是把 source-row continuity 升级成 warm-start hit。

### 2026-05-08 - E1/E2/E3 执行闭环

- 状态：进行中
- Commit：none
- 改动范围：`crates/picea-lab/tests/artifact_run.rs`、`crates/picea/src/solver/contact.rs`、`crates/picea/tests/physics_realism_acceptance.rs`。
- E1/E2：新增 `E2 shadow direction` 报告，把 feature churn 与 support-gap lifecycle 事实合成 observation-only 路由提示；reviewer 指出不能把当前 `intertwined` / `reject_counterpart_motion` 写成长期 hard gate，已改为只断言报告字段存在，并把 `Suspected next milestone` 派生为 `E3 with D4/E6 watch`。
- E3：把 dense position correction 的 pseudo-position shadow state 明确为 `PositionSolveBody` / `PositionRowCorrection` 与纯计算 helper；新增 solver-local unit tests 和真实 step dense-threshold 行为锁 `contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface`。
- Reviewer/Explorer 结论：E3 diff 未发现 velocity impulse、warm-start identity、source-row retain/drop、sleep 或 public API 漂移；D4/E6 当前不是 E4 的硬阻塞，但应继续 watch。reviewer 要求补真实 step 行为锁，已补并验证。
- 验证：subagents 已通过 `rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --lib solver`、`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test physics_realism_acceptance contact_position_correction -- --nocapture`、`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture`、`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`、`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test world_step_review_regressions`、`rtk proxy cargo fmt --all --check`、`rtk proxy git diff --check`。
- 风险 / 后续：E4 先作为验证 gate 处理，因为当前 `contact.rs` 默认路径已经有 dense pseudo-depth re-evaluation；若 240-frame support-gap 红锁仍红，进入 E5 source-row pressure gate，而不是重复实现 E4。

### 2026-05-08 - E4/E5 gate 结果

- 状态：进行中
- Commit：none
- E4 gate：`aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock` 通过；180-frame `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 已通过；240-frame `matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window` 仍红，说明已有 dense pseudo-position pass 不足以消除 support-gap / ejection blocker。
- E5 实验：在 dense residual position correction 中增加 source-row pressure gate，只对 `source_row_continuity_candidate` 且 correction pressure dominated、无 velocity position bias / support-friction、并有 high-motion counterpart/tangent risk 的 row 跳过 residual correction。该 gate 不改 warm-start fallback、contact identity、dt/iterations、sleep 或 non-dense legacy 行为。
- E5 结果：240-frame 红锁仍失败，但 first floor exit 从 frame 214 推迟到 frame 223；原 upstream source frame 177 的 correction pressure 被切掉，dry-run decision 从 `reject_counterpart_motion` 变为 `observe_not_pressure_source`。残留 blocker 转移到后段 `retain_candidate_correction_active` / `retain_candidate_needs_position_re_evaluation` 链和 persistent lifecycle 解释。
- 验证：`solver::contact::tests`、`physics_realism_acceptance contact_position_correction` 通过；`matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window --ignored --nocapture` 仍红，符合“失败不早于当前 first floor exit 且 blocker 更具体”的 E5 部分验收。
- 决策：停止在 E5 继续堆 heuristics；前移 D4/E6，聚焦 persistent manifold / contact lifecycle 输入契约和后段 support-gap 链。

### 2026-05-08 - D4/E6 warm-start 扩展实验被拒绝

- 状态：进行中
- Commit：none
- 实验：尝试把非 edge-swap、同 pair、normal/local-anchor 连续的 feature miss 从 diagnostics-only `source_row_continuity_candidate` 升级为 warm-start transfer。
- 结果：`pipeline::contacts` 边界测试可通过，且 edge-swap 仍保持 identity-only；但 240-frame matrix 红锁明显退化，first floor exit 提前到 frame 175，早于 E5 后的 frame 223，也早于原 frame 214。
- 决策：撤回该实验。D4/E6 后续不得直接把 source-row continuity 升级为 impulse hit；应优先做 lifecycle provenance / identity pin-down，或者设计更窄的 manifold-state 输入，而不是扩大 warm-start fallback。
- 验证：撤回后 `rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --lib pipeline::contacts -- --nocapture` 通过。

### 2026-05-08 - E6 lifecycle provenance-only 小步

- 状态：已验证，保留 240-frame 红锁
- Commit：none
- 拒绝的实验：尝试把 edge-swap persistent lifecycle 也升级为 warm-start transfer。`pipeline::contacts` 边界测试可通过，但 180-frame `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 退化为 first floor exit frame 134、final outside floor bodies=1；240-frame 红锁同样退化为 first floor exit frame 134、final outside floor bodies=3。已撤回。
- 实现：新增 core-owned `ContactLifecycleReason`，透传到 `ContactEvent` / `DebugContact` / prelude / web type。`refresh_contact_events` 现在区分 `Started`、`ExactFeature`、`PersistentEdgeSwap`，但这个字段只描述 contact/manifold lifecycle，不改变 warm-start transfer、solver math、narrowphase feature-id、dt/iteration 或 sleep。
- 行为锁：新增 `refresh_contact_events_reports_edge_swap_lifecycle_without_warm_start_transfer`，明确 edge-swap 可以保留 `contact_id` / `manifold_id`，但 `warm_start_reason` 仍是 `MissFeatureId`，且 transferred normal/tangent impulse 仍为 0。180-frame artifact gate 额外断言 matrix stack 导出过 `PersistentEdgeSwap` provenance。
- 验证通过：`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --lib pipeline::contacts -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test query_debug_contract -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test world_step_review_regressions -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`；`rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`；`rtk proxy npm --prefix crates/picea-lab/web run test:i18n`；`rtk proxy npm --prefix crates/picea-lab/web run build`；`rtk proxy cargo fmt --all -- --check`；`rtk proxy git diff --check`。
- 预期红锁：`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window -- --ignored --nocapture` 仍失败，first floor exit 回到 frame 223，说明 E6 schema 小步没有引入 edge-swap warm-start 实验的 frame 134 退化。
- 决策：D4/E6 继续走“先把 lifecycle 事实钉牢，再设计更窄 manifold-state 输入”的路线；当前证据反对把 edge-swap 或 source-row continuity 直接当作 warm-start hit。

### 2026-05-08 - 用户确认并开始执行

- 状态：进行中
- Commit：none
- 验证：`git status --short` 已复核；`rtk proxy git diff --check` 已通过。
- 决策：V0 已记录当前 dirty 边界。先执行 E1/E2，限制写入范围到 matrix-stack behavior locks 和 contact persistence evidence；暂不触碰当前 dirty 的 web/server/AI routing/live gravity 文件。
- 风险 / 后续：如果 E2 证明 identity lifecycle 是 primary blocker，暂停 E3，回到 D4/E6；如果 E2 证明 position-row stale depth 是 primary blocker，再进入 E3/E4。

### 2026-05-08 - Persistent manifold warm-start 窄化落地

- 状态：部分完成，目标未完成
- Commit：none
- 背景：dense position angular effective-mass 实验被验证为不可保留。`0.25` scale 保持 180-frame no-ejection 但 penetration / support gap 退化；`0.5` scale 继续提前 support gap 并增加角速度；full scale 会产生 ejection。结论是不能靠 position correction 角向自由度单独稳定大堆叠。
- 实现：`persistent_manifold_lifecycle_candidate` 现在在 warm-start 准备阶段也可作为前任记录候选；但 edge-swap 路径只继承 normal impulse，显式丢弃 tangent impulse。这样把 Box2D-style persistent manifold 接入 solver seed，同时避免 edge-swap tangent impulse 在密集堆叠里注入切向能量。
- 行为锁：`edge_swap_manifold_can_warm_start_without_source_row_continuity` 锁定 edge-swap 可以 warm-start normal impulse，但不会被标记为 `source_row_continuity_candidate`；180-frame matrix stack gate 更新为要求 `late_support_gap none`、no ejection、penetration `<= 0.041646`、quiet linear / angular 不回退。
- 验证通过：`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --lib pipeline::contacts::tests -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --lib solver::contact::tests -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture`；`rtk proxy env RUSTUP_TOOLCHAIN=stable-aarch64-apple-darwin cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`。
- 长窗观察：`matrix_stack_long_settle_observation_reports_residual_e4_risk --ignored` 通过新的 observation lock，但仍明确失败：600-frame first floor exit 为 frame `245`，body `BodyHandle(41)`；support gap 从 frame `239` 开始，最终仍有 runaway speed / outside floor bodies。因此 `/goal` 尚未完成。
- 决策：下一步不要再扩大 warm-start fallback，也不要恢复 tangent impulse transfer；应沿 frame 239 support-retention 证据继续设计窄的 dynamic-support retention / pseudo-position support re-evaluation，目标是让长窗 no-ejection 后再进入 sleep 收敛。

### 2026-05-08 - Position-row pressure heuristics 复核与拒绝

- 状态：进行中，目标未完成
- Commit：none
- 探索结论：最新只读探索确认 residual / support-gap 的决策面应落在 `ContactPositionRow` 输入契约，不应继续把 `contact lifecycle` / `manifold state` 做成 solver truth。`contact lifecycle` 继续只负责 identity / provenance；position-row 负责判断当前支撑是否还值得参与 residual correction 或 pressure handoff。
- 已撤回实验：
  - 全局 dense support-friction 扩展：把 support-friction 从局部 dense island 扩大到更宽的 dense scene 后，240-frame support gap 仍从 frame `239` 开始；body41 获得了少量 `support_friction`，但没有移除 normal separation。
  - gravity-derived support friction budget：试图用重力派生浅支撑切向预算后，180/240 均退化，出现更早 ejection 和更高 penetration；这是把场景负载误塞进 tangent budget，不能保留。
  - non-source separating pressure gate：用 separating normal speed 扩大 pressure gate 可以把 600-frame ejection 延后到约 frame `293/329`，但会破坏当前 180-frame hard gate，产生新的 late support gap；说明简单按“正在分离”跳过 correction 会丢掉真实支撑。
  - dense shallow-support velocity bias：只对 dense、warm-start hit、浅重叠且轻微分离的支撑行加小 normal bias 后，`stack_4` 和 solver unit 仍过，但 180-frame matrix hard gate 退化为 first floor exit frame `73`，quiet linear `18.688015`，240-frame final outside floor bodies `3`。该实验证明“给浅分离 contact 人工 normal budget”会把早期动态接触粘成能量源，已撤回。
  - dense position-row contact-count mass conditioning：参考 Matter.js `totalContacts` 思路，把 dense position correction 按每个 body 的 row count 做有效逆质量分配后，solver unit 和 `stack_4` 仍过，但 180-frame matrix hard gate 退化为 first floor exit frame `84`，240-frame final outside floor bodies `1`。说明当前 blocker 不是单纯 per-body correction 分摊不足；该实验已撤回。
  - shallow support-friction scale 提升：把已有 dense shallow support tangent budget 从 `0.6` 提升到 `1.2` 后，solver unit 仍过，但 180-frame matrix hard gate 退化为 first floor exit frame `80`，quiet linear `16.144411`。说明当前不能靠放大切向预算解决滚动/滑移；该实验已撤回。
  - 全局 SAT clipping skin：把 `CLIP_EPSILON` 从 `1e-4` 提升到 `2.5e-3` 后，narrowphase unit 仍过，但 180-frame matrix hard gate 退化为 first floor exit frame `92`，penetration max 升到 `0.081138`。说明成熟引擎式 contact skin 不能作为全局窄相常量无条件开启；若继续走 skin 路线，必须是 previous-manifold / persistent-only 的 narrow support retention。
- 当前验证基线：撤回后 `solver::contact::tests`、`stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling`、180-frame `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 仍通过；240-frame support-gap 红锁仍失败，600-frame 仍可在 frame `245` 附近 ejection。
- 决策：停止继续叠全局 friction / gravity budget / separating-speed gate。下一步只允许做更窄的 position-row 输入契约：保留 lifecycle provenance 边界，把支撑有效性、pseudo-depth、normal impulse / correction pressure、counterpart motion 明确拆成可测试的 row-level 判定。

### 2026-05-08 - 计划草稿

- 状态：进行中
- Commit：none
- 验证：`git status --short` 已运行；当前 dirty 状态已记录。`rtk proxy git diff --check` 已通过。
- Reviewer/Verifier 发现：reviewer 已指出 position-row 先行假设、D3/E3/E4/E5 边界、dirty baseline、验证门、browser start gate 等问题；本版已采纳。
- 风险 / 后续：Plan Gate 前不执行实现；用户批准后第一步仍是 V0 dirty baseline gate。
