## Why

S5-SOLVER-4 已运行子集中的唯一 physics assertion failure来自 RED-4 的 WorldAnchor shadow
将 velocity-only contact响应也纳入数值等价：WorldAnchor mandatory correction 的 1 ULP
舍入差跨过nearest-point量化边界，放大为 tangent impulse 差；该子集内Revolute production
pose、冻结2x2 oracle、最终pose与drift均已匹配。完整16条/broad尚未运行、fmt已知失败，且
optional velocity active-plan另有待验风险；因此现在先修正 acceptance oracle 的职责边界并补齐
existing-behavior lock，再对production做完整验收。

## What Changes

- 新增 `S5-REPLAN-3` receipt：保持 ADR-S5-5、Revolute solver、contact/CCD/integrate 与
  public surface 不变，把 shadow 等价门收窄到能改变 `P_contact` 的 CCD、role、residual
  position-correction 和 StepStats facts。
- 新增独立 `S5-BEHAVIOR-RED-5`：在 clean baseline 上提交修正后的 acceptance-as-code；
  velocity speed/bias/impulse 继续要求 finite、subject normal impulse 为正并输出诊断，但不再作为
  cross-world pose-state 等价条件；既有 `1e-6` pose-state comparator、full-pose `1e-4`、drift
  `0.01` 及所有负向 oracle 阈值保持不变。
- 新增 `S5-SOLVER-5`：只消费 committed RED-5，复用已审计的 production patch，使 unit、16 条
  integration exact、targeted/broad、format、scope 和 hygiene gates 全绿。
- 更新 Revolute design的状态/acceptance owner、OpenSpec tasks与living spec receipts；不进入
  S5-LAB-RED，不执行archive/deploy/push/merge。

## Capabilities

### New Capabilities

- `revolute-joint-solver`: 定义 Revolute point constraint、post-contact pose
  reconciliation、shadow oracle 职责、既有 joint 兼容边界和验收证据。

### Modified Capabilities

无。仓库此前没有 OpenSpec capability；冻结的 Revolute public/solver 行为不变。

## Impact

- REPLAN-3 exact 8：Revolute design、living spec、`openspec/config.yaml`及本change的
  `.openspec.yaml`、proposal、design、spec、tasks。
- RED-5 exact 5：`crates/picea/tests/physics_realism_acceptance.rs`、
  `crates/picea/tests/world_step_review_regressions.rs`、
  `crates/picea/tests/verify_revolute_scope.rb`、living spec、OpenSpec tasks。
- SOLVER-5 required exact 7：现有 `pipeline.rs`、`pipeline/step.rs`、`pipeline/island.rs`、
  `pipeline/joints.rs`、`solver/body_state.rs` patch、living spec、OpenSpec tasks；
  `pipeline/joints/tests.rs`仅用于reviewer触发的sub-EPSILON unit lock。
- 不得扩大到contact/CCD/integrate/API/lab。
- 无新依赖、无 public API/schema breaking change。
