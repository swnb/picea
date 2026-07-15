## Context

导入基线为 `29bafce59a0913be7b4d37055ab57ed5e5233116`。S5-SOLVER-4 已验证
production Revolute mandatory/post 2x2 pose与冻结 oracle 一致，第二条 CCD exact 的 pose
error=`0`、drift=`0.000917`；已运行子集中的唯一 physics assertion failure 是 WorldAnchor
shadow mandatory center 比 subject 低1 ULP，几何量化把它放大成 tangent impulse
delta=`0.000020315`，超过 RED-4 对所有 contact floats共用的 `1e-6` 门。完整16条与broad gates
未运行，不能据此排除其他失败。

另一个只读代码审计发现，保留的 production patch 把 optional velocity projection 从
“contact 后按最新 wake state 重建 active rows”改成复用 contact 前 mandatory plan。这可能漏掉
本帧由 contact 唤醒的 sleeping Distance island，是尚待 RED-5 验真的兼容风险。

## Goals / Non-Goals

**Goals:**

- 修正 shadow oracle 的职责：它证明能恢复 pose-producing `P_contact`，不证明两个不同 joint
  solver经过量化 contact geometry 后的 velocity impulse bitwise/near-bitwise相同。
- 保持 subject contact真实性、原阈值、full-pose/negative candidates 与所有冻结 solver边界。
- 在现有第16条no-repeat exact中扩展子case，验证并保留既有Distance contact-wake projection语义，
  因此总数保持16，clean分类保持`12 RED / 4 GREEN`。
- 让保留的 production patch通过16条exact与完整门禁。

**Non-Goals:**

- 不改变 2x2、COM rebuild、ADR-S5-5、wake、one-row/no-stats、phase/stream顺序。
- 不修改 contact/narrowphase/CCD/`integrate.rs`，不引入 production test hook或public config。
- 不提高 `1e-6`、`1e-4`、`0.01` 等批准阈值，不让Revolute模拟WorldAnchor欠修正。
- 不进入 lab/Web、archive、deploy、push 或后续 milestone。

## Decisions

### 1. 把 contact comparison 拆为 pose-effect gate 与 velocity diagnostics

`P_contact`由 shadow final pose撤回shadow latest advance得到；subject post evaluation随后使用
subject自己的latest velocity。因此 gate比较两边CCD role/geometry、residual position correction
及StepStats pose-mutation facts，继续使用scaled `1e-6`。normal/tangent speed、bias、impulse继续
要求finite并完整打印，subject normal impulse继续要求positive，但这些velocity-response数值不参与
cross-world等价布尔值。

未采用“提高全字段 tolerance”，因为它掩盖字段职责且未来量化边界仍可能放大；未采用
“Revolute under-relaxation”，因为它违反完整2x2/no-stiffness合同；未采用新增public phase hook，
因为现有shadow和authoritative facts足以验收。

### 2. RED-5保留 clean RED，同时增加既有 Distance GREEN boundary lock

两条 CCD exact 在 clean baseline 仍必须以新的pose-effect signature失败；production compatibility
witness必须在提交RED-5前证明两条目标exact可转绿。现有第16条
`post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows`扩展一个clean baseline
本就GREEN的子case：contact在本帧唤醒sleeping Distance island后，optional projection仍消费
最新active membership。enabled径向速度必须`<=1e-4`，disabled对照必须`>1e-3`，输出固定
`S5_REVOLUTE_REPLAN_BOUNDARY:contact_woken_distance_projection`；不新增第17条exact。

pose-effect comparator还必须运行独立negative control：复制finite facts，只把
`position_correction_input_max_depth`增加`1e-3`，要求comparator false并输出
`S5_CONTACT_POSE_EFFECT_SELF_TEST`及最大差字段/delta/tolerance。这样clean RED不能只靠row=0
掩盖一个错误地恒为true的comparator。

这条边界锁不会把RED与实现混合：它锁住S5 patch不得破坏的既有行为，主Revolute缺口仍由clean
baseline RED证明。

### 3. Mandatory plan与velocity plan分责

mandatory joint phase建立的plan只拥有mandatory stats与Revolute-only post reconciliation rows。
optional velocity phase在contact后重新读取live world和wake reasons构造不计stats的active plan，
从而保留HEAD既有Distance语义，并让active Revolute projection使用latest membership。post pass仍
只遍历mandatory Revolute carriers，不重复existing position rows或stats。

### 4. 删除无关 existing-kind 重构

WorldAnchor damping恢复原内联point angular velocity表达；通用helper只服务Revolute。该变化不改
数值，只收紧review scope，避免把existing joint代码重构混入SOLVER-5。

### 5. OpenSpec拥有新链进度

本change的`tasks.md`是REPLAN-3/RED-5/SOLVER-5唯一task checkbox与完成进度源。living spec只
追加冻结合同、immutable node Start HEAD、前序node commit identity与可执行证据；scope verifier
把它当身份/证据注册表，不从PENDING/SHA推导任务完成度，也不在其中维护并行task checklist。
final S5-SOLVER-5 SHA只进入commit后的外部只读receipt，不回写living。

## Risks / Trade-offs

- [shadow仍有1 ULP pose差] -> full-pose oracle保留`1e-4`且pose-effect fields继续受`1e-6`约束；
  velocity差完整输出，不能静默消失。
- [收窄comparator误删真实contact门] -> 两边CCD/contact与velocity-response facts维持finite hard
  assertions，subject normal impulse维持positive，subject latest velocity mutation保持独立门。
- [重建velocity plan增加一次island/row构造] -> 这是HEAD既有语义；不纳入stats，性能优化另开milestone。
- [当前主worktree已有production patch] -> docs/test节点在clean detached worktree完成；只在RED-5
  commit后将production patch带入SOLVER-5，避免scope混提。

## Migration Plan

1. 在clean `29bafce...` detached worktree按exact allowlist导入current living blob
   `6bc4dc8...`及6个OpenSpec paths，复核design blob仍为`f945b0f...`后提交REPLAN-3。
2. 提交RED-5 acceptance和scope transition，验证clean分类及production compatibility witness。
3. 将detached commits用cherry-pick带回主worktree的当前
   `feat/vnext-s5-revolute-joint`分支；操作前移开同内容untracked OpenSpec、恢复已被commit覆盖的
   local living copy，并复核5个production blob不变。不得创建merge commit或推送。
4. 通过review/verifier和全部gates后提交SOLVER-5；失败则保留node STOP receipt，不进入lab。

回滚按node commit边界进行；不重写历史S5-SOLVER-4 STOP证据。

## Open Questions

- RED-5行为锁能否在clean baseline保持严格`12 RED / 4 GREEN`并只命中批准signature，尚待实跑。
- 第16条内的contact-wake Distance子case是否会在保留production patch上真实失败，当前是源码推断，尚待RED-5实跑。
- 修正plan职责后16条exact与broad gates能否全绿，尚待SOLVER-5验证。
