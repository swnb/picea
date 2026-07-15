## ADDED Requirements

### Requirement: Revolute point constraint remains the frozen solver contract
系统 SHALL 以一个 logical Revolute row 执行对称 2x2 point constraint，使用 inverse mass、
inverse inertia、local COM 与两个 local anchors；系统 MUST 保持 contact/joint separate
streams、现有 phase 相对顺序、`joint_row_count == 1` 和 atomic finite write-back。

#### Scenario: Finite off-center row
- **WHEN** 两个有效 endpoint 以偏心 anchor 形成可逆的 2x2 effective mass
- **THEN** 系统按冻结符号与 COM-to-origin rebuild 同时提交两端 correction，并只计一个 joint row

#### Scenario: Singular or non-finite row
- **WHEN** effective mass 不可安全求逆或任一候选 delta 非有限
- **THEN** 系统跳过整行且不部分写回、不虚假 wake、不增加第二 row

### Requirement: Post-contact reconciliation consumes authoritative pose and latest velocity
系统 SHALL 在 contact 与 optional joint velocity projection 之后、final position integration
之前，仅对 mandatory plan 中的 Revolute rows 执行 ADR-S5-5 reconciliation；系统 MUST 从
authoritative post-contact current pose 与 latest stored velocity 构造 evaluation pose，并保持
Distance/WorldAnchor position rows不重复执行。

#### Scenario: CCD contact mutates latest velocity
- **WHEN** CCD-clamped Revolute endpoint 的 contact solver 改变最新 angular 或 linear velocity
- **THEN** reconciliation 使用 contact 后 current pose 和 subject 自身 latest velocity，最终 full-pose error 不超过 `1e-4`、pivot drift 不超过 `0.01`

#### Scenario: Existing joint position rows are not repeated
- **WHEN** 同一步还存在 Distance 或 WorldAnchor row
- **THEN** post-contact pass 不再次应用其 position correction/damping，且其 logical row stats 不重复累计

### Requirement: Shadow oracle gates only pose-producing contact effects
RED/GREEN acceptance SHALL 使用 WorldAnchor shadow 恢复 otherwise-unobservable `P_contact`，但
cross-world 等价门 MUST 只覆盖 role/CCD facts、residual position-correction facts 与相关
StepStats；scaled tolerance MUST 保持 `1e-6`。

#### Scenario: Quantized velocity response diverges without pose mutation
- **WHEN** subject/shadow mandatory pose 相差 1 ULP，并导致 nearest-point 量化后的 speed、bias 或 impulse 数值不同，但 residual position correction 均为零
- **THEN** velocity-only 数值仅作为 finite 诊断输出，subject normal impulse另保持positive hard gate，且这些数值不使 pose-effect equivalence 失败

#### Scenario: Pose-producing contact state diverges
- **WHEN** CCD role/geometry或 residual correction depth、body translation、input/body count、max/total translation 超出 `1e-6` scaled envelope
- **THEN** pose-effect equivalence 失败，并阻止 SOLVER-5 验收

#### Scenario: Comparator negative control
- **WHEN** 测试复制一组finite pose-effect facts并只把`position_correction_input_max_depth`增加`1e-3`
- **THEN** comparator必须返回false并输出`S5_CONTACT_POSE_EFFECT_SELF_TEST`、最大差字段、delta与tolerance，证明gate不是常量true

### Requirement: Contact response remains real and observable
放宽 cross-world velocity 等价 MUST NOT 删除 subject contact真实性门。两边 contact facts MUST
finite，必须真实进入 CCD/contact solver；subject normal impulse MUST为正，shadow normal impulse
只要求finite并保留诊断。subject latest velocity mutation、两边 velocity delta 与最大差字段 MUST
保留在可复跑输出中。

#### Scenario: Shadow is not a valid contact control
- **WHEN** shadow 缺失 CCD trace/contact row、存在 numeric warning或输出非有限，或者subject normal impulse非正
- **THEN** 测试以 harness failure 停止，不把它归类为批准 RED/GREEN signature

### Requirement: Optional projection preserves post-contact active-island semantics
当 `joint_velocity_projection` 启用时，系统 SHALL 在 contact 后按最新 sleep/wake state 重建
active joint rows用于 velocity projection；mandatory plan仍是 row stats 与 Revolute-only post pass
的唯一来源。

#### Scenario: Contact wakes a sleeping Distance island
- **WHEN** contact solver 在本帧唤醒此前 sleeping 且由 Distance joint 连接的 island
- **THEN** 现有exact `post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows`中的新增子case证明optional Distance radial velocity projection在同一帧执行、enabled径向速度`<=1e-4`、disabled对照`>1e-3`，并输出`S5_REVOLUTE_REPLAN_BOUNDARY:contact_woken_distance_projection`

#### Scenario: Mandatory stats remain stable
- **WHEN** velocity projection 重建 active rows
- **THEN** rebuilt rows不重复增加 `joint_row_count`、body slots或其他 mandatory solver stats

### Requirement: RED-5 and SOLVER-5 remain scope bounded
change MUST 按 `S5-REPLAN-3 -> S5-BEHAVIOR-RED-5 -> S5-SOLVER-5` 顺序执行；RED-5
必须先独立提交 acceptance artifacts，SOLVER-5 不得修改 committed tests/scope、contact solver、
CCD、`integrate.rs`、public API/schema、StepConfig 字段/default 或批准阈值。

#### Scenario: Clean baseline classification
- **WHEN** RED-5 在 immutable clean baseline 上逐条运行批准 unit/integration exact tests
- **THEN** unit只出现批准的8个缺失symbol RED，既有16条integration严格分类为 `12 RED / 4 GREEN`，第16条在同一exact内包含Distance contact-wake GREEN子case，两条目标测试只命中批准的pose-effect signature

#### Scenario: Solver acceptance
- **WHEN** committed RED-5 应用于 SOLVER-5 production patch
- **THEN** unit为 `8/8`、16条integration为 `16/16`，targeted/broad/fmt/scope/hygiene gates 全部通过后才可提交
