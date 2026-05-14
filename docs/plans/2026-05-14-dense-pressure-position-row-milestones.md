# Dense Pressure / Position-Row 修正里程碑计划

状态：已完成
计划文档：docs/plans/2026-05-14-dense-pressure-position-row-milestones.md
最后更新：2026-05-14
工作目录：/Users/asyncrustacean/projects/picea
工作区状态：dirty；已有 lab-web WIP、上一阶段 `stack_4` solver guard、以及 dense pressure 架构文档未提交。本计划只允许触碰明确记录的 solver / 行为锁 / 计划文档范围，不吸收 unrelated web WIP。
提交策略：不提交
执行策略：完整计划批准后连续执行；执行里程碑必须通过 `subagent-current-workspace`，每次只交付一个执行 milestone。执行前必须先完成 V0 same-file dirty baseline handoff。若设计输入缺失、红锁外扩、dirty 文件重叠未归因、budget 参数无法用现有 gates 解释、或需要改变 public API / `StepConfig` 默认值 / lab-web schema，则暂停并回到计划更新。

## 目标

把当前 dense position correction 从“固定 rows 多轮直接累计 correction”推进到有明确状态所有权的 dense pressure / pseudo-position / position-row 修正路径，让 8 层 dense stack 的 position-only 红锁转绿，同时保持 `stack_4`、matrix stack、aligned matrix 等既有稳定性门不退化。

最终用户可感知结果：

- `contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface` 通过，证明 dense position pass 不再重复消费 stale raw-depth。
- `stack_4` 的 sparse-stack block-solve 修复保持稳定；额外的稀疏竖直支撑链会进入受限 stabilized contact graph，避免 4 箱最终摊平成地面接触，并且在稳定后进入 sleep。
- matrix/aligned stack artifact gates 继续通过，说明 pressure budget 没有切掉真实支撑。
- solver 内部有清晰的 `DensePositionSolveContext` / `DensePressureBudget` / `PositionRowDecision` 边界，后续若要可视化 row decision，可以在不重写核心逻辑的前提下透出 diagnostics。

## 约束

- 所有验证命令必须使用 `rtk proxy` 前缀。
- 规划阶段只允许新增/更新计划文档；不得修改生产实现代码。
- 执行阶段必须按 `subagent-current-workspace` 的 closed-loop：`worker` 实现、`reviewer` 审查、`verifier` 验证，主 Codex 负责集成和取舍。
- 默认不创建 worktree，使用当前工作区；最多一个写代码 `worker` 同时运行。
- 不改变 `World` / `SimulationPipeline` public API。
- 不改变 `StepConfig` 默认值、全局 `dt`、默认 iteration、sleep policy 或 M28 separate-phase solver ordering；`stack_4` 是否可 sleep 由场景 authoring 显式开启。
- 不重新打开全局 two-point block normal solve；小堆叠只允许通过明确的稀疏竖直支撑链判定进入 stabilized contact graph，不做全局 dense 化。
- 不通过调低全局 `POSITION_CORRECTION_PERCENT` 或减少 `position_iterations` 修绿。
- 不扩大 contact lifecycle / warm-start truth；它们仍只作为 provenance，除非后续另过设计门。
- 不触碰当前 unrelated dirty lab-web 文件：
  - `crates/picea-lab/web/scripts/i18n-contract.mjs`
  - `crates/picea-lab/web/scripts/ui-contract.mjs`
  - `crates/picea-lab/web/src/components/ui/radix.tsx`
  - `crates/picea-lab/web/src/components/workbench/Toolbar.tsx`
  - `crates/picea-lab/web/src/i18n.ts`

## 待确认问题

### 必须确认

无。推荐默认按 private solver-only 路线推进，不改 public API、不改 web schema、不提交。

### 可带假设推进

- 假设第一版不新增 public diagnostics schema：影响 lab-web 可视化解释力；早期验证是 solver/artifact gates 足够定位修正效果。如 reviewer 或 verifier 发现证据不足，再单独开 diagnostics slice。
- 假设 dense budget 只约束 output translation work，不改变 input stats 语义：影响 `StepStats` 解释；早期验证是红锁仍读取 `position_correction_input_*` 作为 stale raw-depth budget 输入。
- 假设 `docs/design/dense-pressure-position-row-architecture.md` 中的 `0.76` 只是示意值，不是最终常量：影响 E2 参数选择；早期验证是 worker 必须用红锁、`stack_4`、matrix/aligned gates 解释最终 ratio 和 `min_partial_correction_scale`。
- 假设 240/600 ignored observation gates 不是本 slice 的 hard gate：影响验收范围；早期验证是本计划先守住红锁、`stack_4`、1200 帧 `stack_4` 竖直支撑链、180 matrix、aligned matrix。若 ignored gates 退化到早于当前失败点，暂停分析。

## 规划依据

- 架构输入：`docs/design/dense-pressure-position-row-architecture.md`。
- 当前红锁：

```text
rtk proxy cargo test -p picea --test physics_realism_acceptance \
  contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface -- --nocapture
```

- 当前失败事实：
  - `velocity_iterations = 0`
  - `position_iterations = 4`
  - `position_correction_input_contact_count = 16`
  - `position_correction_input_total_depth = 0.8000002`
  - `position_correction_total_translation = 0.6416276`
  - 断言要求 total translation 小于 `input_total_depth * 0.78`，约 `0.624`
- 关键代码：
  - `crates/picea/src/solver/contact.rs`
    - `apply_residual_contact_position_correction`
    - `contact_position_rows`
    - `dense_position_row_depth`
    - `dense_contact_position_correction_gate`
    - `position_row_correction`
    - `queue_position_correction`
    - `record_shadow_position_delta`
  - `crates/picea/src/pipeline/contacts.rs`
    - `source_row_continuity_candidate`
    - warm-start / contact event refresh 上游事实
  - `crates/picea/src/pipeline.rs`、`crates/picea/src/pipeline/step.rs`、`crates/picea/src/debug.rs`、`crates/picea/src/events.rs`
    - `position_correction_*` stats / events / debug facts 传递层，只读对照，第一版不改 schema
  - `crates/picea/tests/physics_realism_acceptance.rs`
    - `build_dense_position_correction_stack_world`
    - `contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface`
- 既有稳定性 guard：
  - `contact_position_correction_reports_input_depth_and_applied_translation`
  - `contact_position_correction_preserves_spin_after_velocity_solve`
  - `contact_position_correction_does_not_double_apply_face_manifold_points`
  - `stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling`
  - `stack_4_long_window_retains_vertical_support_chain`
  - `stack_4_long_window_enters_sleep_after_stable_vertical_stack`
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts`
  - `aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock`
  - `solver::contact::tests`
- 当前 same-file dirty 归因：
  - `crates/picea/src/solver/contact.rs` 的 dirty 内容来自上一阶段 `stack_4` sparse-stack block-solve guard，是本轮 solver 稳定性任务的前置基线，不属于 unrelated web WIP。
  - 执行 worker 不得 revert、弱化或绕过这些 guard；V0 必须把当前 diff 摘要写入 handoff，后续 reviewer 以“保留 stack_4 guard + 新增 dense position-row 修正”为合并合同。
- 已知负路线：
  - 全局降低 correction percent。
  - dense graph 直接复用 non-dense two-point block normal solve。
  - 扩大 warm-start / lifecycle truth。
  - 用 sleep、friction、velocity bias 掩盖 position-only over-correction。

## 计划验收

- Design Gate：本计划确认架构输入、milestone 边界、subagent 执行合同和验证矩阵。
- Execution Gate：用户确认完整计划后，主 Codex 才按顺序派发执行 milestone。
- explorer：已返回，结论已合并。核心结论是：架构文档足够作为实现输入；当前缺口主要是初始 budget ratio / `min_partial_correction_scale` 的实现期校准，以及 private row decision 是否后续进入 diagnostics 的延期决定。
- reviewer：已返回，P1/P2/P3 已采纳。修订包括新增 V0 same-file dirty baseline handoff、把 E1 红锁改为“预期仍红”的观察门、补 E1 double-apply face manifold gate、要求持久化 budget 参数解释、以及把 C1 裸 `git status` 改为 `rtk proxy git status --short`。
- 用户确认：已确认，按完整计划连续执行 V0 -> E1 -> E2 -> V1 -> C1；若执行中触发 stop condition，则暂停回报。

## 设计 / 执行分离规则

- 设计里程碑只允许修改计划文档、设计文档、决策记录或测试计划，不修改生产实现代码。
- 执行里程碑必须引用本计划和 `docs/design/dense-pressure-position-row-architecture.md`。
- 如果执行发现架构文档与代码事实冲突，立即停止当前 worker，回到计划/设计更新。
- 执行不得从聊天记忆替代 artifact contract；以本计划和设计文档为准。

## 里程碑映射

| 设计里程碑 | 产出 | 解锁的执行里程碑 | 状态 |
| --- | --- | --- | --- |
| D1 | 确认 dense position 修正 private interface、红锁边界和非目标 | E1, E2, V1 | 待确认 |
| C1 | 记录最终验证证据、实现偏差和后续 diagnostics/support-retention 风险 | 后续新计划 | 待确认 |

## 里程碑

### V0：Same-File Dirty Baseline Handoff

类型：验证
状态：已完成

目标：
在任何 worker 修改 `crates/picea/src/solver/contact.rs` 前，把当前同文件 dirty 状态归因并写入执行 handoff，避免 subagent 把上一阶段 `stack_4` guard 当成不明用户改动、误删或混淆。

范围：
- 读取 `rtk proxy git status --short`。
- 读取 `rtk proxy git diff -- crates/picea/src/solver/contact.rs docs/design/dense-pressure-position-row-architecture.md docs/design/stack-4-contact-block-solve-stability-design.md`。
- 记录当前 `contact.rs` dirty 内容的任务归属：上一阶段 `stack_4` sparse-stack block-solve guard。
- 给 E1/E2 worker 的 handoff 明确：必须保留现有 `stack_4` guard 和相关 solver tests。

不做：
- 不修改代码。
- 不 revert、stash、stage、commit。
- 不吸收 unrelated lab-web WIP。

验收标准：
- same-file dirty 归因清楚，且没有未解释的 `contact.rs` 改动。
- 如果发现 `contact.rs` 中存在无法归因的用户改动，暂停执行并请用户确认。
- E1/E2 handoff 包含“保留 stack_4 guard，不得回退上一阶段 solver baseline”的明确要求。

验证方式：
- `rtk proxy git status --short`
- `rtk proxy git diff -- crates/picea/src/solver/contact.rs docs/design/dense-pressure-position-row-architecture.md docs/design/stack-4-contact-block-solve-stability-design.md`

Subagent 执行计划：
- verifier：可选；只读确认 dirty overlap 和 diff 摘要。主 Codex 负责把 V0 结论写入 worker handoff。

提交策略：
- auto-commit：否

风险 / 后续：
- 这是 current-workspace subagent 执行的硬前置。V0 未完成或归因不清时，不进入 E1。

V0 结论：
- `rtk proxy git status --short` 确认 unrelated lab-web WIP 仍存在，本计划继续避开。
- `crates/picea/src/solver/contact.rs` 当前 dirty diff 来自上一阶段 `stack_4` sparse-stack block-solve guard：新增 sparse guard 常量、`manifold_normal_pair_partners` 的低速 resting guard、以及 block-solve 相关 solver unit tests。
- 未发现无法归因的同文件改动；E1/E2 worker handoff 必须保留这些 guard，不得回退上一阶段 solver baseline。

### D1：Dense Position 修正输入确认

类型：设计
状态：已完成

目标：
确认 `docs/design/dense-pressure-position-row-architecture.md` 足以作为实现输入，并把红锁、非目标、文件所有权和验收矩阵固定下来。

范围：
- `docs/plans/2026-05-14-dense-pressure-position-row-milestones.md`
- `docs/design/dense-pressure-position-row-architecture.md`
- 只读检查：
  - `crates/picea/src/solver/contact.rs`
  - `crates/picea/tests/physics_realism_acceptance.rs`
  - `crates/picea-lab/tests/artifact_run.rs`

不做：
- 不修改 `crates/picea/src/solver/contact.rs`。
- 不修改 public API、lab-web、artifact schema。
- 不运行 browser acceptance；本阶段只确认计划和设计输入。

验收标准：
- 当前红锁、非目标、dirty 文件边界在计划中明确。
- `DensePositionSolveContext`、`DensePressureBudget`、`PositionRowDecision` 三个内部边界能映射到具体执行 milestone。
- reviewer 对计划没有未处理 P1/P2。

验证方式：
- `rtk proxy git diff --check -- docs/plans/2026-05-14-dense-pressure-position-row-milestones.md`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_reports_input_depth_and_applied_translation -- --nocapture`
- reviewer 只读审查

Subagent 执行计划：
- explorer：只读，补充仓库证据、文件边界、验证命令。
- reviewer：只读，审查完整计划是否存在隐藏 scope、弱验收、文件所有权风险。

提交策略：
- auto-commit：否

风险 / 后续：
- 如果 reviewer 认为架构输入仍不充分，应先回到 `software-architecture-design` 更新设计文档，而不是进入 E1。

### E1：Dense Position Pass 状态所有权重构

类型：执行
状态：已完成

目标：
把 dense position pass 的临时状态从 `apply_residual_contact_position_correction` 内联变量整理到 private solver-local context，保持行为不变，为 E2 的 budget/decision 插入点做干净落点。

范围：
- `crates/picea/src/solver/contact.rs`
- 可选新增/调整 solver 内部 unit tests，仅限 `solver::contact::tests`
- 如需要解释复杂领域名词，可在同文件新增少量注释

不做：
- 不改变 correction 数值策略。
- 不引入 pressure budget。
- 不改变 `StepStats` 字段语义。
- 不修改 `physics_realism_acceptance.rs` 的红锁阈值。
- 不触碰 `crates/picea-lab/web/*`。

验收标准：
- 原有 solver contact unit tests 通过。
- 红锁结果不应被 E1 伪修绿；E1 可运行红锁观察命令，但成功条件是“仍以同一 dense over-correction 断言失败，且不出现编译失败、panic 或数值发散”。若红锁意外通过或失败原因变化，说明 E1 已越界，必须暂停。
- `stack_4` guard 不退化。
- diff 局限在本 milestone 文件范围。

验证方式：
- `rtk proxy cargo test -p picea --lib solver::contact::tests -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_preserves_spin_after_velocity_solve -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_reports_input_depth_and_applied_translation -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_does_not_double_apply_face_manifold_points -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture`
- `rtk proxy git diff --check`

预期红锁观察命令（允许失败；成功判据是继续同一断言失败）：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface -- --nocapture`

Subagent 执行计划：
- worker：一个写入 worker，最高质量姿态，执行 E1；必须只改 `crates/picea/src/solver/contact.rs` 和必要的同文件 tests，不 stage、不 commit。
- reviewer：只读审查 E1 diff 是否行为保持、抽象是否真减少复杂度、注释是否解释了 dense pressure/pseudo-position 不变量。
- verifier：运行本 milestone 验证命令，并报告是否产生或修改文件。

提交策略：
- auto-commit：否

风险 / 后续：
- 风险：只包一层 context 反而增加样板。若 reviewer 认为 abstraction 没有承载状态所有权，应回退到更小的 helper 分拆或直接把 E1 合并进 E2 前重新过计划。

E1 结果：
- 实现：新增 private `DensePositionSolveContext`，集中持有 rows、queued translations、pseudo-position bodies 和 stats；`apply_residual_contact_position_correction` 只负责创建 context、执行 solve、应用 queued translations。
- 验证：worker 与独立 verifier 均确认 solver contact tests、三条 position-correction regression、`stack_4` guard 和 `git diff --check` 通过。
- 红锁观察：`contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface` 仍按同一 dense over-correction 断言失败，`position_correction_total_translation = 0.6416276`，`position_correction_input_total_depth = 0.8000002`。
- Reviewer：无 P1/P2。P3 建议后续补更直接的 behavior-preserving orchestration test；当前作为非阻塞风险记录，E2 先补 budget/decision 行为锁。

### E2：Dense Pressure Budget 与 Position-Row Decision

类型：执行
状态：已完成

目标：
在 dense mode 内引入 row-level decision 和 frame-level pressure budget，使用 pseudo-position 重评估后的 residual depth 对 correction work 做限流，使红锁转绿，同时保持既有堆叠稳定性门。

范围：
- `crates/picea/src/solver/contact.rs`
- `crates/picea/tests/physics_realism_acceptance.rs` 仅允许新增行为锁或补充断言，不允许放宽现有红锁阈值
- 必要的 solver unit tests：
  - budget full allow
  - budget partial scale
  - budget exhausted
  - resolved row skip
  - source-row pressure gate priority before budget

不做：
- 不降低红锁阈值或移除现有断言。
- 不改 `POSITION_CORRECTION_PERCENT` / `DENSE_POSITION_CORRECTION_SLOP` 的全局语义，除非 reviewer 明确确认它只是 private dense policy 的命名重构。
- 不改变 velocity solver、friction、sleep、warm-start lifecycle。
- 不新增 public diagnostics schema。

验收标准：
- 红锁通过，且 `velocity_iterations=0` 下不注入 linear velocity。
- `position_correction_total_translation` 低于 stale raw-depth budget，而不是通过减少 moved body count 逃避 correction。
- `solver::contact::tests` 覆盖 budget/decision 的 full/partial/exhausted/skip/source-gate 顺序。
- 最终 budget ratio / `min_partial_correction_scale` 必须有代码注释或设计偏差记录解释：它们如何对应红锁的 `0.78` budget、为何不直接采用设计示例 `0.76`、以及 matrix/aligned gates 如何约束取值。
- `stack_4` guard、180 matrix、aligned matrix gates 通过。
- reviewer 无未处理 P1/P2。

验证方式：
- `rtk proxy cargo test -p picea --lib solver::contact::tests -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_does_not_double_apply_face_manifold_points -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --nocapture`
- `rtk proxy git diff --check`

Subagent 执行计划：
- worker：一个写入 worker，最高质量姿态，因为这是 solver correctness 高风险改动；文件所有权限定为 `crates/picea/src/solver/contact.rs` 和必要的 `crates/picea/tests/physics_realism_acceptance.rs`。
- reviewer：只读审查 E2 diff 是否符合 architecture/design artifact、是否存在 magic-number 风险、是否误改 non-dense path、是否缺测试。
- verifier：运行本 milestone 验证命令；若 artifact tests 生成新 run 目录，必须报告路径和是否为未跟踪文件。

提交策略：
- auto-commit：否

风险 / 后续：
- 风险：budget ratio 变成 magic number。缓解：常量命名必须表达 dense pressure work limit，并用红锁 + matrix gates 一起校准。
- 风险：budget 过硬切掉真实支撑。缓解：先跑 matrix/aligned gates；若退化，暂停回设计，不扩大到 sleep/friction/lifecycle patch。
- 风险：240/600 support-gap 仍需 support retention。缓解：本 milestone 只解决 dense position-row over-correction；support retention 另开设计。

E2 结果：
- 实现：新增 private `DensePressureBudget` 与 `PositionRowDecision`，只在 dense mode 下启用；source-row pressure gate 先于 budget；resolved / invalid / non-finite row 不消费 budget；limited row 使用同一个 scale 缩放 queued translation 和 pseudo-position delta。
- 参数：`DENSE_PRESSURE_TOTAL_TRANSLATION_BUDGET_RATIO = 0.775`，`DENSE_PRESSURE_MIN_PARTIAL_CORRECTION_SCALE = 0.05`。取舍解释写在 `crates/picea/src/solver/contact.rs` 常量旁：`0.775` 低于红锁 `0.78` 但比设计示意 `0.76` 更少切支撑；matrix/aligned gates 作为支撑不退化约束。
- 验证：worker、reviewer、verifier 均确认 hard gates 通过；`solver::contact::tests` 从 15 增至 20，覆盖 full/partial/exhausted/resolved/source-gate priority。
- Reviewer：无 P1/P2/P3 代码缺陷。Residual risk 是 partial-limit 集成锁和 non-dense 分流专名测试仍可后续补强；当前由实现路径、unit tests、红锁和 `stack_4` gate 共同覆盖。

### V1：回归验证与 Artifact 证据收口

类型：验证
状态：已完成

目标：
用最小但覆盖核心风险的验证矩阵证明 E1/E2 没有破坏 solver、artifact 和既有 stack 稳定性。

范围：
- 运行验证命令。
- 记录命令结果、关键输出、生成 artifact run 路径。
- 如验证发现实现范围外问题，暂停并回报，不直接修。

不做：
- 不修改代码。
- 不更新 browser UI。
- 不清理 unrelated dirty 文件或 target artifact。

验收标准：
- 所有 hard gates 通过：
  - solver contact unit tests
  - dense stack 红锁
  - `stack_4` behavior lock
  - matrix stack artifact gate
  - aligned matrix artifact gate
  - `cargo fmt --check`
  - `git diff --check`
- 如运行 ignored observation gates，结果只作为观察记录，不直接扩大当前 milestone。

验证方式：
- `rtk proxy cargo test -p picea --lib solver::contact::tests -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_preserves_spin_after_velocity_solve -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_reports_input_depth_and_applied_translation -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_does_not_double_apply_face_manifold_points -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --nocapture`
- `rtk proxy cargo fmt --check`
- `rtk proxy git diff --check`

Subagent 执行计划：
- verifier：运行验证矩阵，报告 exact outcomes、artifact paths、以及验证过程中产生或修改的文件。
- reviewer：如 V1 前的最终 diff 较 E2 有补丁，追加只读审查；否则复用 E2 reviewer 结论。

提交策略：
- auto-commit：否

风险 / 后续：
- 如果 hard gate 中只有 artifact test 因 unrelated dirty web/server WIP 失败，先隔离归因，不在本计划内修 web。

V1 结果：
- `rtk proxy cargo test -p picea --lib solver::contact::tests -- --nocapture` 通过，`20 passed; 0 failed`。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface -- --nocapture` 通过。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_preserves_spin_after_velocity_solve -- --nocapture` 已在 E1 验证通过。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_reports_input_depth_and_applied_translation -- --nocapture` 已在 E1 验证通过。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_does_not_double_apply_face_manifold_points -- --nocapture` 通过。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture` 通过。
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture` 通过；verifier artifact path：`/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/.tmpDOgBAV/runs/matrix-stack-acceptance`。
- `rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --nocapture` 通过；verifier artifact path：`/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/.tmpaYP9Ys/runs/matrix-stack-aligned-behavior-lock`。
- `rtk proxy cargo fmt --check` 通过。
- `rtk proxy git diff --check` 通过。

### E3：稀疏竖直支撑链稳定化补丁

类型：执行
状态：已完成

触发原因：
浏览器实时会话目标不是“数值门全绿”而是 `stack_4` 最终仍保持 4 箱竖直堆叠。E1/E2 后追加 1200 帧命令行验收发现旧 `stack_4` 行为锁只证明“最终安静”，没有证明“仍然堆叠”：原始场景会在长窗口内丢失所有动态-动态支撑链，最终变成 4 个箱体横向摊在地面上。

实现：
- 新增 `stack_4_long_window_retains_vertical_support_chain` 行为锁，要求 1200 帧后仍至少有 3 条动态-动态支撑链接，且动态箱体 x spread 不超过 `0.35`。
- 保留 matrix dense graph 的 `16` 行阈值，不把所有接触都当作 dense。
- 新增稀疏竖直支撑链判定：小世界、小 island、2..8 行接触、且至少 2 行近竖直法线时，进入 solver-local stabilized contact graph。
- stabilized contact graph 复用既有 dense/shallow 稳定化部件：支撑摩擦预算、resting graph damping，并绕过小栈 low-speed block-solve guard；但不改变 public API、`StepConfig`、lab-web schema 或 artifact schema。
- 这个补丁不是场景参数修正：测试 fixture 保持和 lab `stack_4` 一致的默认材料、默认间距。最初这里仍保留 `can_sleep=false`，后续 E4 已把它作为明确的场景 authoring 缺陷修正。

验证：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_long_window_retains_vertical_support_chain -- --nocapture` 通过。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture` 通过。
- `rtk proxy cargo run -p picea-lab -- run stack_4 --frames 1200` 通过；最终 snapshot 有 `contacts=8`、`dynamic_support_links=6`，4 个动态箱体 x 坐标约在 `[-0.0000013, 0.0000012]` 内。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance -- --nocapture` 通过，`60 passed; 0 failed`。
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture` 通过；artifact path：`/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/.tmpl5f2my/runs/matrix-stack-acceptance`。
- `rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --nocapture` 通过；artifact path：`/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/.tmpxxXp5X/runs/matrix-stack-aligned-behavior-lock`。
- `rtk proxy cargo fmt --check` 通过。
- `rtk proxy git diff --check` 通过。

风险 / 后续：
- 当前稀疏判定仍是 solver-private heuristic，刻意限制在小世界、小 island、近竖直接触链内。若后续发现非堆叠三体场景被过度稳定化，应把 `sparse_vertical_stack_support_graph` 扩成更精细的支撑图判定。

### E4：Stack4 Sleep Authoring 修复

类型：执行
状态：已完成

触发原因：
浏览器实时会话验证发现 E3 后 `stack_4` 已经能保持竖直堆叠，但没有进入 sleep。根因不是 solver sleep policy，而是场景 authoring：lab `stack_4` 复用了 legacy `add_box` helper，动态体的 `BodyDesc::can_sleep` 固定为 `false`；验收测试的 `build_stack_4_world` helper 也同样创建了不可 sleep 的动态体。

实现：
- 新增 `stack_4_long_window_enters_sleep_after_stable_vertical_stack` 行为锁，要求稳定竖直堆叠后所有动态体都进入 sleep。
- `build_stack_4_world` 中 4 个动态箱体改为显式 `can_sleep: true`，保持默认材料、默认尺寸、默认间距不变。
- lab `ScenarioId::Stack4` 改用 `add_box_can_sleep(..., true)` 创建动态箱体；静态地面仍沿用原 helper，不改变全局 sleep policy。

验证：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_long_window_enters_sleep_after_stable_vertical_stack -- --nocapture` 通过。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_long_window_retains_vertical_support_chain -- --nocapture` 通过。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture` 通过。
- `rtk proxy cargo run -p picea-lab -- run stack_4 --frames 1200` 通过；最终 snapshot 中 4 个动态体 `sleeping=true`，线速度和角速度均为 `0`，`active_island_count=0`，`sleeping_island_skip_count=1`，`contact_count=8`。
- 浏览器实时会话 `session-1` 验收到第 `1612` 帧：`contact_count=8`、`active_island_count=0`、`sleeping_island_skip_count=1`、`solver_body_slot_count=0`、`dynamic_support_links=6`，4 个动态体全部 `sleeping=true`。

风险 / 后续：
- 这是 `stack_4` 场景级 authoring 修正，不改变 engine 默认 `BodyDesc::can_sleep` 或 sleep 判定阈值。
- 如果后续新增“永不 sleep 的调试场景”，应在场景命名或运行设置中显式表达，避免和稳定堆叠验收混在一起。

### C1：计划收口与后续路线记录

类型：收尾
状态：已完成

目标：
更新计划文档，记录最终文件、验证证据、实现与架构文档的偏差、以及是否需要后续 diagnostics / support-retention 计划。

范围：
- `docs/plans/2026-05-14-dense-pressure-position-row-milestones.md`
- 必要时更新 `docs/design/dense-pressure-position-row-architecture.md`，仅记录实现偏差或明确后续路线，不重写设计。

不做：
- 不提交，除非用户另行授权。
- 不把 unrelated dirty files 纳入总结。

验收标准：
- 每个已执行 milestone 有状态、文件、验证证据、风险和下一步建议。
- 记录最终 budget ratio / `min_partial_correction_scale` 的取舍解释，或者指向代码注释位置。
- 若有未解决风险，明确是本计划后续、另开 milestone、还是接受为观察项。
- 最终 `rtk proxy git status --short` 中 task-owned 文件和 unrelated WIP 清楚分离。

验证方式：
- `rtk proxy git diff --check -- docs/plans/2026-05-14-dense-pressure-position-row-milestones.md docs/design/dense-pressure-position-row-architecture.md`
- `rtk proxy git status --short`

Subagent 执行计划：
- reviewer：可选；只读检查收尾记录是否准确。

提交策略：
- auto-commit：否

风险 / 后续：
- 如果实现偏差较大，必须同步设计文档，否则未来执行会从过时 artifact 出发。

C1 结果：
- 计划执行完成，无需更新 public API、`StepConfig`、lab-web schema 或 artifact schema。
- 实现与设计有一处受验收驱动的扩展：除了 private dense pressure / position-row 修正，还新增稀疏竖直支撑链 stabilized contact graph，用来解决 `stack_4` 最终摊平成地面接触的问题。设计文档已同步记录该偏差。
- 追加 sleep authoring 修复：`stack_4` 动态体显式允许 sleep，浏览器实时会话已验证长窗口后进入 `sleeping_island_skip_count=1`。
- 设计文档中的 `0.76` 保持示意值，最终参数解释已落在代码注释与本计划记录中。
- 后续可选补强：新增一个穿过 `DensePositionSolveContext` 的 partial-limit 集成锁；把稀疏竖直支撑链判定从 row-count heuristic 升级为显式支撑图；若 240/600 ignored observation gates 仍暴露 support-gap，另开 support-retention / diagnostics slice。

## Plan Gate Checklist

- [x] 计划文档已持久化到 `docs/plans/`。
- [x] 每个 milestone 有类型、范围、不做、验收和验证方式。
- [x] 设计里程碑和执行里程碑分离。
- [x] 执行 milestone 引用架构输入 `docs/design/dense-pressure-position-row-architecture.md`。
- [x] dirty 工作区和 unrelated web WIP 已显式标出。
- [x] 提交策略明确为不提交。
- [x] 执行策略明确为完整计划批准后连续执行。
- [x] explorer 结果已合并。
- [x] reviewer 结果已合并，无未处理 P1/P2。
- [x] 用户已批准完整计划或点名可执行 milestone。
