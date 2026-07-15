# vNext Handoff §5 Revolute Joint Pin-only V1 Milestone

状态：S5-D/API-RED/API/BEHAVIOR-RED/S5-REPLAN/S5-BEHAVIOR-RED-2/
S5-BEHAVIOR-RED-3/S5-REPLAN-2/S5-BEHAVIOR-RED-4均已commit；RED-4 commit=
`29bafce59a0913be7b4d37055ab57ed5e5233116`。历史S5-SOLVER保持
`STOPPED / FROZEN CONTRACT CONFLICT / NOT COMMITTED`；S5-SOLVER-2、S5-SOLVER-3与
S5-SOLVER-4均为`STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`。
S5-SOLVER-4 immutable Start HEAD=`29bafce59a0913be7b4d37055ab57ed5e5233116`；unit=`8/8 GREEN`，
第一条RED-4 shadow exact GREEN，第二条仅因WorldAnchor shadow的1 ULP under-correction使
`solver_tangent_impulse` delta=`0.000020315 > 0.000001`而保持
`contact_equivalent=false`。该差异命中§15，未运行完整16条/broad gates，未派reviewer/verifier，
未提交production。用户本轮已批准继续
`S5-REPLAN-3 -> S5-BEHAVIOR-RED-5 -> S5-SOLVER-5`；S5-REPLAN-3以
`29bafce59a0913be7b4d37055ab57ed5e5233116`为immutable Start HEAD，review/verifier gates已闭合；
其commit identity由RED-5开始时登记。
新链唯一task checkbox/完成进度来源为
`openspec/changes/complete-s5-revolute-solver-5/tasks.md`；本文只保留冻结合同与
identity/evidence receipt。Chrome `NOT RUN / BROWSER PENDING`
初始日期：2026-07-14；S5-REPLAN：2026-07-15
基线：`feat/vnext-s5-revolute-joint@28867b5`
设计：`docs/design/2026-07-14-revolute-joint-v1-design.md`
交接：`docs/handoff-2026-07-14-vnext-s5-revolute-joint.md`
父计划：`docs/plans/2026-06-17-physics-realism-vnext-milestones.md`
Profiles：architecture-heavy, api-contract, ui-browser

## 1. 批准、目标与边界

用户已批准 handoff §5 的“完整 Pin-only V1”public API、字段、
`#[non_exhaustive]` 列表、wake semantics、lab 范围和延期项。不存在 Plan Gate
阻塞问题。2026-07-15用户又明确批准在S5-SOLVER-3 test-contract conflict后保持ADR-S5-5与
design不变，按`S5-REPLAN-2 -> S5-BEHAVIOR-RED-4 -> S5-SOLVER-4`继续；只有本计划列出的
stop condition可以中断连续执行。S5-SOLVER-4命中test-contract conflict后，用户本轮又明确
要求继续完成`S5-REPLAN-3 -> S5-BEHAVIOR-RED-5 -> S5-SOLVER-5`；沿用既有“各node gate
通过后commit、不push”授权。

目标：

1. 增加以两个 local anchors 定义的 pin-only revolute joint；只约束两个 world
   anchors 重合，相对旋转保持自由。
2. 完成 public descriptor/patch、World lifecycle、recipe、debug、fixture schema v1、
   deterministic lab scenario、artifact/server/Web 消费与 browser 验收。
3. 使用包含 inverse mass、偏心 lever arm 和 inverse inertia 的 2x2 point
   constraint；一个 revolute 始终计为一个 logical joint row。
4. 对所有 joint kinds 补齐 create、constraint patch、remove 与 body cascade 的
   lifecycle wake，并保持 `WorldCommands` scratch transaction 原子性。
5. 以 committed RED acceptance artifacts、独立 reviewer/verifier 和可复跑 receipt
   闭合每个节点。

非目标：

- 不做 motor、angle limits、`reference_angle`、stiffness、damping、break force、
  persistent joint warm-start cache 或 `collide_connected`。
- 不合并或重构contact/joint solver streams，不改contact solver；除用户批准的
  Revolute-only post-contact局部插入外，不重排既有phase。
- 不升级 fixture schema version，不增加 generic live joint patch protocol，不增加
  motor/limit UI。
- 不重新打开已冻结的 handoff §4，不把 S5 前缀解释为父计划 E5/E6 完成。
- 不 push；用户只授权通过 gate 后 commit。

## 2. 已验证事实、用户冻结边界与已通过S5-D验收的计划

### Fact

- 初始 branch=`feat/vnext-s5-revolute-joint`、HEAD=`28867b5`、worktree clean。
- `joint.rs` 现有 public joint surface 只有 Distance / WorldAnchor；批准列表中的 public
  enums 当前均未加 `#[non_exhaustive]`。
- create/patch/remove joint 当前没有 lifecycle wake；`WorldCommands` 已在 scratch world
  上全部成功后整体 commit。
- live step order 为 velocity integration -> CCD -> mandatory joint -> contact ->
  optional joint velocity projection -> position integration -> final contact -> sleep。
- `IslandSolveBatch` 已有 separate `contact_rows` / `joint_rows` 和共享 dense body slots。
- existing pair position helper 只平移；point impulse helper 已支持 inverse inertia 和
  Picea clockwise-positive angular sign。
- lab scenario 源码已拆分到 `crates/picea-lab/src/scenario/`；旧 AI 路由中的
  `scenario.rs` 是文档漂移。
- `JointDesc::Revolute` 会使 core `pipeline/island.rs` 的exhaustive match失配；
  `JointDesc`加`#[non_exhaustive]`后，`picea-lab`作为external crate的exhaustive
  consumers也必须迁移。Live `rg` 当前至少命中`scenario/scene_lattice.rs`与
  `scenario/fixture/tests.rs`。
- committed external fixture
  `crates/picea/tests/fixtures/revolute_public_api/src/lib.rs`从`picea_lab` crate root导入
  `SceneRevoluteJointFixture`；live `crates/picea-lab/src/scenario/mod.rs`与
  `crates/picea-lab/src/lib.rs`均通过显式`pub use`列表形成public re-export链。若S5-API
  只修改`scenario/fixture.rs`，external fixture必然以`E0432`失败。因此这两个public
  re-export文件是compiler-proven required paths，不适用external exhaustive consumer的
  optional机制。

### User-frozen Decision

- `RevoluteJointDesc/Patch`、enum variants、prelude 边界和
  `SceneRevoluteJointFixture` shape 严格按 design §7。
- 只有 `JointKind`、`JointDesc`、`JointPatch`、`JointBundle`、`DebugJointKind`、
  `SceneJointFixture` 增加 `#[non_exhaustive]`；public structs 不加。
- lifecycle mutation 使用 existing `SleepTransitionReason::UserPatch`；solver 实际
  correction 使用 `JointCorrection`，不增加 public enum variant。
- schema 保持 v1；new reader 兼容旧 v1，old reader 对新 `revolute` variant 明确拒绝。
- builtin capability 固定为 `ScenarioId::RevolutePendulum` / `revolute_pendulum`，不改变
  `ScenarioId` attribute policy。

### S5-D plan decisions（review / verifier 已通过，已commit）

- S5-API只做可编译checkpoint：`pipeline/island.rs`对Revolute显式skip，不创建
  `JointSolvePlanRow`/`JointSolverRow`，所以该checkpoint的revolute
  `joint_row_count == 0`。S5-BEHAVIOR-RED锁住缺口，S5-SOLVER才增加row/math。

### S5-SOLVER verified conflict（Fact，2026-07-14）

- 历史S5-SOLVER严格按design §9的2x2/COM/atomic/one-row/separate-stream合同实现后，
  focused unit=`8 passed`，但13条integration只有`7 GREEN / 6 FAIL`。
- verified metrics：two-dynamic首帧drift=`0.033333`；nonzero-COM max drift=
  `0.056089`；两条CCD drift分别为`0.021701`/`0.019774`；projection on/off
  contact fixture在pivot正确对齐后，按live `SharedShape::rect(width,height)` full-dimensions
  语义仍有`1.25` vertical gap，故`contact_frames=0`；不是切触。
- independent reviewer=`2 High / 0 Medium / 0 Low`，确认问题不是`K`、clockwise sign、
  COM-to-origin rebuild、atomic apply、wake、logical row或separate stream遗漏，而是mandatory
  position sampling早于contact/projection velocity mutation，以及fixture没有正穿透。
- 该production diff已由supervisor恢复到Start HEAD；旧S5-SOLVER未GREEN、未提交，必须永久
  保留为`STOPPED / FROZEN CONTRACT CONFLICT`历史证据。

### 2026-07-15 User-approved Decision

- 用户接受推荐路径：在contact与optional joint velocity projection之后、final position
  integration之前加入Revolute-only post-contact pose reconciliation，并把contact fixture改为
  pivot对齐后仍有解析正穿透。
- 该决定只supersede旧“禁止second position solve”的局部边界；它不是第二logical row或
  stats source，不允许重新solve Distance/WorldAnchor，也不授权改contact solver/CCD算法、
  冻结旧velocity、merge/reorder既有streams/phases。
- 原13条tests和`0.03/0.01`阈值保持不变；新增latest-velocity exact与existing-joint
  no-repeat boundary exact后，必须先完成独立S5-BEHAVIOR-RED-2 commit，再进入S5-SOLVER-2。

### S5-SOLVER-2 verified test-contract conflict（Fact，2026-07-15）

- Immutable Start HEAD=`07b2bd6cd3ff4b80d1b6c124a4752e36e8e8d88f`。Production patch严格实现
  ADR-S5-5 full post-contact symmetric 2x2 corrected-evaluation pose后，新
  `revolute_joint_post_contact_reconciliation_uses_latest_velocity` GREEN：两配置normal
  impulse均=`1.570796`，latest omega=`2.154278/0.065281`，post demand=
  `0.012817/0.019774`，expected/actual final drift=`0.000386/0.000917`，完整pose error=
  `0/0`，logical rows=`1/1`。
- 两条旧absolute final-angle oracle与同一accepted behavior互斥：rotating fixture actual/old
  expected angle=`0.106296/0.001185`、angle error=`0.105111`、drift=`0.001104`；contact
  full-step fixture actual/old expected angle=`0.096846/0.001088`、angle error=`0.095758`、
  drift=`0.000917`，normal impulse=`1.570796`。两条都只有旧angle equality先失败，既有
  `drift <= 0.01`实际满足。
- Independent reviewer裁决`CONFLICT CONFIRMED`：production不是本次失败根因，继续修改
  production会违背accepted full post 2x2 contract。S5-SOLVER-2因此冻结为
  `STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`。Production patch保留在主worktree供
  supervisor复核，但不进入`/tmp/picea-s5-red3`、不属于RED-3 scope，也不得提交为SOLVER-2。

### S5-SOLVER-3 verified test-contract conflict（Fact，2026-07-15）

- Immutable Start HEAD=`6ffa1e3f29e902b68708b62e11d5fae160db97ec`。现有production patch在主
  worktree复用并运行后，unit=`8/8 GREEN`；16条integration exact全部逐条真实输出
  `running 1 test`，结果=`15 GREEN / 1 FAIL`。唯一FAIL为
  `revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle`：full-pose error=
  `0.011336 > 1e-4`，但row/warning=`1/0`、actual drift=`0.001104 <= 0.01`；另一条CCD
  full-pose、latest、would-wake、projection enabled/disabled和no-repeat均GREEN。
- Independent reviewer裁决=`CONFLICT CONFIRMED`、severity=`1 High / 1 Medium / 0 Low`。
  失败fixture的contact residual correction translation=`0.00194835861`、contact depth=
  `0.0487089641`、position-correction input contact/body count=`1/1`；GREEN对照fixture的
  residual correction=`0`。Live phase事实为mandatory Revolute correction -> contact velocity
  solve + residual authoritative pose mutation -> Revolute post pass重新读取current -> final
  integration。
- RED-3 oracle从mandatory current直接拼post eval，遗漏了contact phase已经写入的pose mutation。
  Production post pass读取contact后的authoritative current，符合ADR-S5-5；强迫production忽略/
  撤回该mutation才会迎合旧oracle，属于§15禁止路径。该node因此冻结为
  `STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`；production patch仅保留在主worktree作
  证据，不得提交为S5-SOLVER-3。
- `cargo fmt --all --check`还独立报告dirty `pipeline/joints.rs` rustfmt diff；这属于未闭合hygiene
  gate，不改变physics/test-contract conflict分类，也不得冒充唯一FAIL的根因。

### S5-REPLAN-2 oracle route（Inference，尚待RED-4验真）

- 只读explorer确认当前没有public test hook能直接读取contact-post current。No-joint control缺少
  mandatory Revolute geometry，不能恢复同一contact输入；从subject final反推又会与待验的post
  correction形成循环依赖。
- 推荐的test-side `WorldAnchor shadow` control可在不新增public开关、不改production的前提下
  复现mandatory后的spinner origin，再让同一contact pipeline产生可观察的post-contact current
  `P_contact`。这是基于现有WorldAnchor行为与solver facts的合理推断；只有RED-4实际证明shadow/
  subject contact等价、唯一failure signature和clean baseline分类后，才能升级为已验证事实。

### 2026-07-15 User-approved RED-4 Decision

- 用户批准保持ADR-S5-5、design、production phase与public surface不变；新链只修正
  acceptance oracle对contact residual pose mutation的遗漏。
- S5-BEHAVIOR-RED-4必须先在clean `6ffa1e3…`上提交统一的两条CCD full-pose shadow-control
  oracle和scope state transition，再允许S5-SOLVER-4复用现有production patch。
- 不授权修改contact solver/CCD/`integrate.rs`、增加production test开关/public config、忽略或
  撤回contact residual correction，也不授权直接在S5-SOLVER-3继续改tests/production。

### S5-SOLVER-4 verified test-contract conflict（Fact，2026-07-15）

- 已运行子集内unit=`8/8`；第一条shadow exact GREEN。第二条row/warning=`1/0`、pose error=`0`、
  drift=`0.000917`，唯一physics assertion failure是`tangent_impulse` delta=
  `0.000020315 > 1e-6`使全字段`contact_equivalent=false`。
- LLDB定位WorldAnchor mandatory center比冻结2x2/subject低1 ULP；该差异跨过analytic
  circle-polygon nearest-point量化边界。让Revolute模仿该欠修正、放宽全字段容差或改contact/
  narrowphase均违反冻结边界，因此S5-SOLVER-4永久STOP且未commit。
- 完整16条与broad未运行，fmt已知FAIL；不得把“已运行子集的唯一physics failure”扩写成
  “所有未跑门无其他问题”。
- 只读code review还发现保留patch让optional velocity phase复用contact前mandatory plan；HEAD
  原行为会在contact后按latest wake state重建active rows。该差异可能漏掉本帧被contact唤醒的
  sleeping Distance island，尚待RED-5实跑验真。

### 2026-07-15 User-approved REPLAN-3 Decision

- 保持ADR-S5-5、2x2/COM、public surface、phase/stream、one-row/no-stats和既有阈值不变；
  RED-5只把WorldAnchor shadow等价门收窄到能改变`P_contact`的CCD/role、residual position
  correction与对应StepStats，velocity speed/bias/impulse保留finite/positive真实性门与诊断。
- 现有第16条`post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows`扩展
  contact-wake Distance active-plan子case，不新增第17条；clean分类仍须`12 RED / 4 GREEN`。
- comparator必须有独立negative control；SOLVER-5还须用reviewer触发unit闭合sub-EPSILON
  actual-change/wake语义。禁止production test hook、public config或contact/CCD/integrate改动。

### Unknown

Plan Gate无阻塞未知项；以下仍必须由RED-5/SOLVER-5实跑，不能提前写PASS：clean baseline能否
严格保持unit批准symbol RED与integration`12 RED / 4 GREEN`；第16条Distance子case能否在HEAD
GREEN且在保留patch上稳定暴露plan复用；pose-effect comparator negative control与两条唯一
signature能否成立；修复后能否unit全部GREEN、integration`16/16`及broad/fmt/scope全GREEN。
任一未知被证伪时按§15 STOP，不得扩大算法/API范围。

## 3. 执行不变量与 subagent 合同

历史批准执行链保留如下；它已在S5-SOLVER终止，不授权沿旧链进入S5-LAB-RED：

```text
S5-D -> S5-API-RED -> S5-API -> S5-BEHAVIOR-RED -> S5-SOLVER
     -> S5-LAB-RED -> S5-LAB -> S5-V -> S5-C
```

用户批准后的RED-2/SOLVER-2链已在test-contract conflict处终止：

```text
S5-REPLAN -> S5-BEHAVIOR-RED-2 -> S5-SOLVER-2 [STOPPED]
```

RED-3/SOLVER-3链已在第二个test-contract conflict处终止：

```text
S5-BEHAVIOR-RED-3 -> S5-SOLVER-3 [STOPPED]
```

RED-4/SOLVER-4链已在第三个test-contract conflict处终止：

```text
S5-SOLVER-3 [STOPPED] -> S5-REPLAN-2 -> S5-BEHAVIOR-RED-4 -> S5-SOLVER-4 [STOPPED]
```

用户本轮批准后的当前唯一执行链为：

```text
S5-SOLVER-4 [STOPPED] -> S5-REPLAN-3 -> S5-BEHAVIOR-RED-5 -> S5-SOLVER-5
```

本次OpenSpec change在S5-SOLVER-5 final commit与只读复核后hard-stop。历史future链
`S5-LAB-RED -> S5-LAB -> S5-V -> S5-C`仍保留在本文，但必须由用户另行授权，不能由本次
“继续完成三节点”自动进入。

通用规则：

- 每个 node 都分派独立叶子 `worker`、`reviewer`、`verifier`；prompt 必须包含
  “你是叶子 agent，不允许再启动 subagent。”
- `worker` 只写该 node ownership 表列出的文件；不得 stage、commit、push、merge、
  branch、clean 或修改别的 node 文件。
- `reviewer`、`verifier` 只读；发现问题不修。reviewer findings-first，按
  High/Medium/Low 给出文件与行号。
- 主 Codex 是 supervisor，只做范围裁决、角色交接、High/Medium 闭环、固定 allowlist
  stage、commit 和 commit 后复核，不亲自实现已委派内容。
- 新链的task checkbox与完成进度只由OpenSpec `tasks.md`拥有；本文§16只注册immutable Start HEAD、
  前序node commit identity和可执行receipt，scope verifier不得把PENDING/SHA解释为task完成度。
  final S5-SOLVER-5 SHA只进入commit后的外部只读receipt，不回写本文。
- 当前工作区最多一个写入 worker。每个 commit 前 reviewer 必须无未闭合
  High/Medium，independent verifier 必须完成该 node 的 exact gate。
- 所有 Cargo/Git/Rust/Web 验证使用 `rtk proxy`。不得把 reviewer判断代替 executable
  receipt。
- S5-API-RED、S5-BEHAVIOR-RED、S5-BEHAVIOR-RED-2、S5-BEHAVIOR-RED-3、
  S5-BEHAVIOR-RED-4、S5-BEHAVIOR-RED-5、S5-LAB-RED必须先提交acceptance artifacts并记录
  真实 RED，再允许后续 implementation worker 开始。
- 每个node由supervisor在第一处文件改动前运行`rtk proxy git rev-parse HEAD`，把40位
  immutable full start HEAD写入§16该node receipt；未来node保持`PENDING`，不得预填
  猜测hash。该字段一经写入不得在node内修改。
- 每个worker/reviewer/verifier及每个新shell都必须通过§4.1 `receipt-head`模式从living
  spec重新读取并显式赋值`S5_NODE_START_HEAD`，不能继承上一个shell的环境变量。
  node结束时用§4.1 binary scope verifier汇总`git diff --name-only <start HEAD>`（含
  staged/unstaged tracked）与`git ls-files --others --exclude-standard`，拒绝任何未列路径。
  `git status`只用于展示，不能替代binary scope gate。Commit前supervisor还要对cached
  paths做exact allowlist比较。
- RED 不能通过删除、ignore、放宽断言、改批准阈值、伪造 Rust facts 或把失败改成
  snapshot update 来消除。
- S5-REPLAN-2因scope script尚不识别new nodes，只使用§9F inline exact-path gate；
  S5-BEHAVIOR-RED-4在exact-3内提交scope state transition后，RED-4 reviewer/verifier与
  S5-SOLVER-4才统一使用`receipt-head`/binary scope入口。
- S5-REPLAN-3在clean detached worktree使用inline exact-8 gate；S5-BEHAVIOR-RED-5在exact-5
  内提交scope state transition，之后SOLVER-5统一使用`receipt-head`/binary scope入口。

角色交付合同：

| 角色 | 必须交付 | 禁止事项 |
| --- | --- | --- |
| worker | changed files、实现/测试范围、运行命令与exit、未覆盖风险 | 扩 scope、stage/commit/push、修别的 node |
| reviewer | Findings first、severity、证据行号、scope/contract结论 | 修改文件、把未知写成通过 |
| verifier | exact command、exit code、关键输出、生成文件和git-visible检查 | 修改源文件、用聚合结论掩盖失败 |
| supervisor | gate裁决、修复分派、allowlist stage/commit、commit scope复核 | 跳过RED、在review前commit |

## 4. Milestone 总表与文件 ownership

| Node | Primary worker ownership | 明确范围外 | Runnable acceptance | Commit gate / message |
| --- | --- | --- | --- | --- |
| S5-D | 本design、本文、parent、handoff、AI routing、design index | `crates/**`、tests、Cargo、Web | §5 docs gates | architecture/spec/routing review + docs verifier；`docs: design revolute joint milestone` |
| S5-API-RED | external temp-crate fixture/public verifier、binary scope verifier、`world_step_review_regressions.rs` existing-kind lifecycle locks、本文 RED receipt | production/API implementation、workspace compile failure | §6 surface RED + six exact runtime wake RED + workspace compile GREEN | test/spec review + RED verifier；`test: lock revolute public api` |
| S5-API | public/storage/lifecycle/debug/fixture/type skeleton；`pipeline/island.rs` explicit skip；compiler-required lab public re-export链；external exhaustive compiler adapters；focused API tests；经review后由独立bounded worker一次性同步scope script required set | solve-plan/solver row/math、scenario、artifact/server UI behavior；implementation worker不得修改scope script逻辑 | §7 targeted + workspace compile green；row count 0；16-path binary scope | scope amendment reviewer先批准，scope-sync再独立执行；随后API/lifecycle/compat reviewer + verifier；`feat: add revolute joint api` |
| S5-BEHAVIOR-RED | `physics_realism_acceptance.rs`、new `pipeline/joints/tests.rs`、`pipeline/joints.rs`唯一`#[cfg(test)] mod tests;`、本文 RED receipt | solver production code、lab/Web | §8 exact RED set + binary scope | test/spec review + RED verifier；`test: lock revolute joint behavior` |
| S5-SOLVER（historical STOPPED） | 曾批准`pipeline/island.rs`、`pipeline/joints.rs`、minimal `solver/body_state.rs`、`pipeline.rs` public doc/config lock、必要 focused unit tests；production diff已恢复 | stream merge/reorder、contact solver、lab/Web、StepConfig字段/default变化 | §9 targeted未全GREEN；保留conflict证据 | 不得commit、不得改写为PASS |
| S5-REPLAN | exact 2：design与本文 | production/tests/scope script/Cargo/Web | §9A exact-path docs gate | docs review + verifier；`docs: replan revolute post-contact reconciliation` |
| S5-BEHAVIOR-RED-2 | exact 4：`physics_realism_acceptance.rs`、`world_step_review_regressions.rs`、`verify_revolute_scope.rb`、本文 | production solver/lab/Web/design | §9B原13历史分类+3条新增exact+scope self-tests | test/spec review + RED verifier；`test: lock revolute post-contact reconciliation` |
| S5-SOLVER-2（historical STOPPED） | 曾批准required exact 6与optional unit-only path；production patch保留在主worktree作冲突证据 | tests/scope script、contact solver/CCD algorithm、lab/Web、StepConfig字段/default变化 | §9C implementation使new latest GREEN，但两条stale absolute-angle oracle FAIL | `STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`；不得续写旧node |
| S5-BEHAVIOR-RED-3 | exact 3：`physics_realism_acceptance.rs`、`verify_revolute_scope.rb`、本文 | production/design/其他tests/lab/Web/Cargo | §9D两条superseding exact + 16条integration分类 + scope/hygiene | test/spec review + RED verifier；`test: supersede stale revolute final-angle oracles` |
| S5-SOLVER-3（historical STOPPED） | 曾继承§9C required exact 6；production patch保留在主worktree作冲突证据 | tests/scope script、contact solver/CCD algorithm、lab/Web、StepConfig字段/default变化 | unit 8/8、16 exact仅15/16；§9E保留conflict evidence | `STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`；不得续写旧node |
| S5-REPLAN-2 | exact 1：本文 | design、tests/scope script、production、lab/Web、Cargo | §9F inline exact-path docs gate | docs review + verifier；`docs: replan revolute contact-state oracle` |
| S5-BEHAVIOR-RED-4 | exact 3：`physics_realism_acceptance.rs`、`verify_revolute_scope.rb`、本文 | production/design/其他tests/lab/Web/Cargo | §9G两条shadow-control oracle、16条integration分类、scope/hygiene | test/spec review + RED verifier；`test: lock revolute contact-state oracle` |
| S5-SOLVER-4（historical STOPPED） | 曾继承§9C required exact 6；保留production patch作证据 | tests/scope script、contact/CCD/`integrate.rs`、lab/Web | 已运行子集唯一physics assertion FAIL；§9H保留STOP receipt | `STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`；不得续写 |
| S5-REPLAN-3 | exact 8：design、本文、OpenSpec config与本change `.openspec.yaml`/proposal/design/spec/tasks | production/tests/scope script/Cargo/lab/Web | §9I OpenSpec/doc exact gates | spec review + docs verifier；`docs: replan revolute pose-state oracle` |
| S5-BEHAVIOR-RED-5 | exact 5：`physics_realism_acceptance.rs`、`world_step_review_regressions.rs`、scope script、本文、OpenSpec tasks | production/design/OpenSpec其他artifacts/Cargo/lab/Web | §9J comparator negative control、两条pose-effect RED、第16条Distance GREEN、16条`12 RED / 4 GREEN` | test/spec review + RED verifier；`test: lock revolute pose-state oracle` |
| S5-SOLVER-5 | required exact 7：五个production文件、本文、OpenSpec tasks；optional `pipeline/joints/tests.rs`由reviewer触发 | committed integration/scope/design、contact/CCD/`integrate.rs`、lab/Web、StepConfig字段/default | §9K既有8+新增sub-EPS unit、16/16、broad/fmt/scope/hygiene | spec/code review + verifier；`feat: reconcile revolute constraints after contacts` |
| S5-LAB-RED | artifact/server/UI/i18n contract tests、本文 RED receipt | lab/Web production implementation、core solver | §10 exact RED set | test/spec review + RED verifier；`test: lock revolute lab contracts` |
| S5-LAB | scenario/fixture completion、artifact/server evidence、Web type/label/consumers；生成§11固定external browser prompt | live patch、motor/limit UI、Web physics、CLI调用Chrome | §11 Rust/Web/server GREEN + external browser receipt | CLI review/verifier通过后可commit candidate；browser未回填仍是`BROWSER PENDING`且不得进入S5-V；`feat: expose revolute joint in lab` |
| S5-V | leaf worker只读运行CLI preflight；leaf reviewer只读审查；independent leaf verifier只读复跑；生成§12固定external browser prompt | 三角色均不写文件、不修问题、不调用Chrome；禁止新功能/阈值变更 | §12 full CLI gates + milestone binary scope + external browser receipt | reviewer无High/Medium、CLI verifier PASS且browser receipt PASS；否则不得称full acceptance；不单独commit |
| S5-C | 本文、design、parent、handoff、routing、ordering contract | production/tests | §13 docs/scope gates | docs review + verifier；`docs: close revolute joint milestone` |

ownership 补充：

- S5-API-RED 可为 public contract 增加 `crates/picea/tests/verify_revolute_public_api.rb`
  及其嵌套 fixture，但 fixture 必须复制到批准的 temp root 后运行，不能在 repo 生成
  `Cargo.lock` / `target`。本node还必须在
  `crates/picea/tests/world_step_review_regressions.rs`提交六个baseline-compilable的
  existing Distance/WorldAnchor lifecycle exact locks；这些runtime tests允许按§6合同RED，
  但`rtk proxy cargo check --workspace --all-targets`必须GREEN。
- S5-API只建立API/lifecycle/debug/fixture/type skeleton和最小compile adapters。
  `pipeline/island.rs`必须对Revolute显式skip且不得创建任何row；执行前用`rtk proxy rg`
  查找所有external exhaustive consumers，当前至少迁移`scene_lattice.rs`与
  `fixture/tests.rs`，只允许wildcard/compat改动。该checkpoint用test锁定row count 0。
  此外，`scenario/mod.rs`和lab crate-root `lib.rs`必须显式re-export
  `SceneRevoluteJointFixture`；这是committed external fixture已经证明的public export链，
  属于required implementation，不得归入compiler-discovered optional exhaustive adapters。
- exact-16 scope reviewer已二元批准，bounded scope-sync worker已只在
  `SCOPES["S5-API"][:required]`加入`scenario/mod.rs`、lab `lib.rs`和scope script自身。
  已确认既有live `future-receipt-pending`断言与已冻结S5-API current receipt冲突；唯一允许的
  state-transition exception已获review并由独立bounded worker执行：删除
  `expect_contract_error("future-receipt-pending") { receipt_sha("S5-API") }`，替换为
  `validate_cli_sha!("S5-API", "6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68")`和PASS tag
  `S5_SCOPE_SELF_TEST=current-s5-api-receipt:PASS`。`PENDING`负向合同继续由既有synthetic
  `parser-future-pending`与generic `pending`覆盖；禁止绑定`S5-BEHAVIOR-RED`或任何其他真实
  future node。除上述唯一self-test state transition和已批准required三行外，parser、CLI、
  helpers、milestone union、其他self-test逻辑必须zero-diff。首次independent scope verifier
  技术门全PASS，仅因本文仍写未同步状态而最终`FAIL`；status remediation后independent
  re-verifier=`S5-API SCOPE RE-VERIFIER PASS`，明确解除scope implementation gate。
  S5-API implementation worker已完成且worker gates=`GREEN`；independent implementation reviewer
  首轮=`0 High / 1 Medium / 0 Low`、裁决`FAIL`（顶部仍误报implementation未开始），第一次bounded
  remediation已完成；第一次复审=`0 High / 1 Medium / 0 Low`、裁决`FAIL`（文档仍误报第一次状态修复未完成），
  第二次status-only bounded remediation已完成；最终独立复审=`0 High / 0 Medium / 0 Low`、
  裁决`PASS`；independent verifier=`S5-API VERIFIER PASS`；已commit为
  `c5a47ed4252363ebf914802c8189cf2bcc9e3563`。S5-BEHAVIOR-RED immutable Start HEAD固定为
  同一commit；worker首轮完成后independent reviewer=`1 High / 3 Medium / 1 Low`、裁决
  `FAIL`，针对CCD final-angle oracle、free-rotation solvable mass、nonzero-COM lever、living
  spec状态和早期sleep fixture记录的bounded remediation已完成；final reviewer=
  `0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=
  `S5-BEHAVIOR-RED VERIFIER PASS`；已commit为
  `385c4c350ceee98947c65da5e3f63384804c69e1`。历史S5-SOLVER随后命中frozen-contract
  conflict并STOPPED、production diff已恢复；当时的next S5-REPLAN按§9A执行。Chrome保持
  `NOT RUN / BROWSER PENDING`。
  implementation worker未继续修改scope script。
- S5-BEHAVIOR-RED保留直接2x2/singular/sign unit contract。若采用
  `pipeline/joints/tests.rs`，`pipeline/joints.rs`除`#[cfg(test)] mod tests;`外production
  必须zero-diff；reviewer逐行确认。历史S5-SOLVER、STOPPED S5-SOLVER-2/3/4都不得修改
  committed integration阈值/断言；current S5-SOLVER-5只按§9K消费committed RED-5。
- S5-SOLVER-5继承S5-SOLVER-2/3/4对`crates/picea/src/pipeline.rs`的窄边界：只允许把
  `StepConfig::joint_velocity_projection` public doc从distance-only改为joint
  point-velocity semantics，并同步既有配置default/serde行为锁；不得增加字段、改default
  或借机重构pipeline。
- S5-REPLAN-3先在clean detached `29bafce...`按exact allowlist导入current living blob
  `6bc4dc8...`及6个OpenSpec paths；design导入blob必须仍为`f945b0f...`，五个主worktree
  production blobs必须与§9I receipt一致。任何复制缺失/额外路径均FAIL。
- S5-BEHAVIOR-RED-5在现有第16条no-repeat exact中增加Distance contact-wake projection子case，
  不增加第17条；enabled radial`<=1e-4`、disabled`>1e-3`。两条CCD helper必须执行
  `position_correction_input_max_depth + 1e-3` negative control并输出固定self-test signature。
- S5-SOLVER-5恢复HEAD的contact后active velocity-plan rebuild，mandatory plan继续独占stats与
  Revolute post rows；恢复WorldAnchor原表达。Reviewer已触发optional `pipeline/joints/tests.rs`
  增加sub-EPSILON actual-change/wake locks，先RED再修exact-change判定。
- S5-REPLAN只允许修改design与本文；当前scope script尚不识别新node，因此本node由
  supervisor使用§9A inline Ruby exact-path gate，不得提前修改script。
- S5-BEHAVIOR-RED-2拥有scope script state transition与两类core behavior locks：必须加入
  `S5-REPLAN`/`S5-BEHAVIOR-RED-2`/`S5-SOLVER-2` nodes、scopes、self-tests与milestone union，
  同时保持SHA/changed-path/cached判定语义不变。
- S5-SOLVER-2/3/4均因test-contract conflict永久STOPPED。现有production patch只留在
  主worktree作reviewer证据，不能带入REPLAN-3或RED-5、不能提交到旧node、不能在旧node继续修
  测试或production；只有S5-SOLVER-5可在RED-5 commit后复用该patch。
- S5-BEHAVIOR-RED-3只拥有exact 3；两条测试保留fixture、test name、CCD/contact identity、
  impulse、latest velocity mutation、warning、row与drift阈值，只把旧absolute-angle equality
  替换为独立full pose oracle。不得修改new latest/would-wake tests、其他integration tests、
  production、design、lab/Web或Cargo。
- S5-REPLAN-2严格exact-1且不改scope script；本node只把已验证conflict、用户Plan Gate与未来
  executable contracts写入本文，不把RED-4/SOLVER-4写成已验证。
- S5-BEHAVIOR-RED-4严格exact-3；两条CCD tests统一改用test-side WorldAnchor shadow恢复
  `P_contact`，并在scope script中完成new-node state transition。不得改production、其他tests、
  design、lab/Web或Cargo。
- S5-SOLVER-4继承§9C required exact 6与optional unit-only边界；production worker不得修改
  committed tests或scope script。`pipeline/joints/tests.rs`仍只在reviewer提出unit coverage缺口、
  supervisor另派bounded test-only worker时可增强；不得改integration阈值/断言。
- S5-LAB 只能消费 `DebugSnapshot` / `StepStats` / Rust artifact/server facts；Web 不得
  重新计算 anchor、row count 或 physics 状态。

### 4.1 Binary scope verifier contract

S5-API-RED必须提交`crates/picea/tests/verify_revolute_scope.rb`。脚本输入node id和
start HEAD；必须调用`rtk proxy git diff --name-only <start>`与
`rtk proxy git ls-files --others --exclude-standard`，对下表逐路径required/optional
集合做binary判定：missing required或unexpected path均exit nonzero，并输出actual set。
禁止用directory glob、`git status`或只检查production目录代替。

| Node | Required paths | Explicit optional paths |
| --- | --- | --- |
| S5-API-RED | public verifier、scope verifier、fixture `Cargo.toml`/`src/lib.rs`/`src/bin/exhaustive.rs`、`crates/picea/tests/world_step_review_regressions.rs`、living spec receipt | 无 |
| S5-API | `crates/picea/src/joint.rs`, `crates/picea/src/lib.rs`, `crates/picea/src/world/api.rs`, `crates/picea/src/recipe.rs`, `crates/picea/src/debug.rs`, `crates/picea/src/pipeline/island.rs`, `crates/picea/tests/core_model_world.rs`, `crates/picea/tests/world_step_review_regressions.rs`, `crates/picea-lab/src/scenario/fixture.rs`, `crates/picea-lab/src/scenario/fixture/tests.rs`, `crates/picea-lab/src/scenario/scene_lattice.rs`, `crates/picea-lab/web/src/types.ts`, `docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md`, `crates/picea-lab/src/scenario/mod.rs`, `crates/picea-lab/src/lib.rs`, `crates/picea/tests/verify_revolute_scope.rb`（exact 16） | compiler指出的其他external exhaustive consumer；加入前必须由reviewer确认只做wildcard/compat迁移。该optional机制不适用于`scenario/mod.rs`与lab `lib.rs`的public re-export；两者已是required |
| S5-BEHAVIOR-RED | `pipeline/joints.rs`, new `pipeline/joints/tests.rs`, `physics_realism_acceptance.rs`, living spec | 无 |
| S5-SOLVER | `pipeline.rs`, `pipeline/island.rs`, `pipeline/joints.rs`, `solver/body_state.rs`, living spec | `pipeline/joints/tests.rs`仅增强unit coverage，不得改integration阈值；`pipeline.rs`仅public doc与既有config lock |
| S5-REPLAN | design、living spec（exact 2） | 无；在scope script支持前由§9A supervisor inline Ruby gate验收 |
| S5-BEHAVIOR-RED-2 | `crates/picea/tests/physics_realism_acceptance.rs`, `crates/picea/tests/world_step_review_regressions.rs`, `crates/picea/tests/verify_revolute_scope.rb`, living spec（exact 4） | 无 |
| S5-SOLVER-2 | `crates/picea/src/pipeline.rs`, `crates/picea/src/pipeline/step.rs`, `crates/picea/src/pipeline/island.rs`, `crates/picea/src/pipeline/joints.rs`, `crates/picea/src/solver/body_state.rs`, living spec（required exact 6） | `crates/picea/src/pipeline/joints/tests.rs`仅unit增强；production worker不得修改该optional或任何committed integration/lifecycle test |
| S5-BEHAVIOR-RED-3 | `crates/picea/tests/physics_realism_acceptance.rs`, `crates/picea/tests/verify_revolute_scope.rb`, living spec（exact 3） | 无 |
| S5-SOLVER-3 | 与S5-SOLVER-2相同的required exact 6 | 与S5-SOLVER-2相同的optional `crates/picea/src/pipeline/joints/tests.rs`，只允许reviewer触发的unit增强 |
| S5-REPLAN-2 | living spec（exact 1） | 无；本node不改scope script，使用§9F inline exact-path verifier |
| S5-BEHAVIOR-RED-4 | `crates/picea/tests/physics_realism_acceptance.rs`, `crates/picea/tests/verify_revolute_scope.rb`, living spec（exact 3） | 无 |
| S5-SOLVER-4 | 与S5-SOLVER-2/3相同的required exact 6 | 与S5-SOLVER-2/3相同的optional `crates/picea/src/pipeline/joints/tests.rs`，只允许reviewer触发的unit增强 |
| S5-REPLAN-3 | `docs/design/2026-07-14-revolute-joint-v1-design.md`、living spec、`openspec/config.yaml`、`openspec/changes/complete-s5-revolute-solver-5/{.openspec.yaml,proposal.md,design.md,specs/revolute-joint-solver/spec.md,tasks.md}`（exact 8） | 无；本node不改scope script，使用§9I inline exact-path gate |
| S5-BEHAVIOR-RED-5 | `crates/picea/tests/physics_realism_acceptance.rs`、`crates/picea/tests/world_step_review_regressions.rs`、`crates/picea/tests/verify_revolute_scope.rb`、living spec、OpenSpec `tasks.md`（exact 5） | 无 |
| S5-SOLVER-5 | `crates/picea/src/pipeline.rs`、`crates/picea/src/pipeline/step.rs`、`crates/picea/src/pipeline/island.rs`、`crates/picea/src/pipeline/joints.rs`、`crates/picea/src/solver/body_state.rs`、living spec、OpenSpec `tasks.md`（required exact 7） | `crates/picea/src/pipeline/joints/tests.rs`仅用于reviewer已触发的sub-EPSILON unit行为锁；不得修改committed integration/scope |
| S5-LAB-RED | `artifact_run.rs`, `server_routes.rs`, Web `ui-contract.mjs`, `i18n-contract.mjs`, living spec | fixture compatibility test file仅当API checkpoint尚未锁定对应case |
| S5-LAB | `scenario/mod.rs`, `scenario/scene_dispatch.rs`, new `scenario/scene_revolute.rs`, Web `i18n.ts`, `types.ts`, `SceneHierarchy.tsx`, `Inspector.tsx`, `Timeline.tsx`, living spec | `artifact.rs`/`server.rs`仅在RED证明generic passthrough不足时；`components/workbench/types.ts`仅在compiler/contract要求时 |
| S5-C | design、living spec、parent、handoff、AI index/repo-map/catalog、design index、`solver-island-ordering-contract.md` | 无 |

State-transition contract：S5-REPLAN期间live script尚不识别三个新node，故只使用§9A的
supervisor exact-path Ruby gate，不能把script当前`unknown node`误报为replan失败。
S5-BEHAVIOR-RED-2必须在自己的exact-4范围内一次性完成以下acceptance-as-code更新：

- `SCOPES`加入上述三个node；required/optional必须与本表逐路径完全一致；
- `PRE_V_NODES`改为包含`S5-API-RED`、`S5-API`、`S5-BEHAVIOR-RED`、`S5-REPLAN`、
  `S5-BEHAVIOR-RED-2`、`S5-SOLVER-2`、`S5-LAB-RED`、`S5-LAB`，并明确排除历史
  `STOPPED`的`S5-SOLVER`，避免milestone union要求一个从未提交的旧node；
- 加入新node receipt/parser/scope的正负self-tests，覆盖PENDING、mismatch、missing、
  unexpected与optional；S5-REPLAN current receipt必须验证immutable Start HEAD
  `385c4c350ceee98947c65da5e3f63384804c69e1`，S5-BEHAVIOR-RED-2的Start HEAD才在
  S5-REPLAN commit后由supervisor写成该实际40位commit；
- 保持既有SHA解析、`git cat-file`可解析校验、`changed_paths(base)`的tracked+untracked
  union、`cached`仅看staged paths、required/optional binary判定和milestone base语义不变。

该state transition本身必须先经test/spec reviewer逐行批准；RED-2 worker不得借更新node集合
改写通用parser、SHA、changed-path或cached semantics。

S5-BEHAVIOR-RED-3在自己的exact-3范围内做第二次state transition：

- `PROGRESS_NODES`保留可解析的historical `S5-SOLVER-2`，并加入
  `S5-BEHAVIOR-RED-3`、`S5-SOLVER-3`；
- `SCOPES`增加上述RED-3 exact 3与SOLVER-3 required exact 6/optional unit-only path；
- `PRE_V_NODES`排除两个STOPPED solver node `S5-SOLVER`/`S5-SOLVER-2`，改为消费
  `S5-BEHAVIOR-RED-3`与`S5-SOLVER-3`；
- self-test覆盖new parser、current RED-3 Start HEAD、solver3 PENDING、SHA mismatch、
  missing/unexpected/optional；SOLVER-3从首次提交即采用state-aware PENDING/SHA双分支，未来
  handoff写入immutable SHA时不需要删除PENDING hardcode；
- SHA格式与commit解析、CLI/receipt equality、tracked+untracked path union、cached判定、
  required/optional binary集合与milestone base语义全部保持不变。

S5-REPLAN-2不修改scope script。由于clean `6ffa1e3…`的script尚不认识三个new nodes，本node
必须以§9F inline verifier对`git diff --name-only <start>`与untracked union做exact-1二元判定；
不得把`unknown node S5-REPLAN-2`误报成docs失败，也不得为了让本node走binary入口而提前改script。

S5-BEHAVIOR-RED-4在自己的exact-3范围内做第三次state transition：

- `PROGRESS_NODES`保留三个historical STOPPED solver node可解析，并加入`S5-REPLAN-2`、
  `S5-BEHAVIOR-RED-4`、`S5-SOLVER-4`；
- `SCOPES`增加上述REPLAN-2 exact 1、RED-4 exact 3与SOLVER-4 required exact 6/optional
  unit-only path；
- `PRE_V_NODES`精确包含`S5-API-RED`、`S5-API`、`S5-BEHAVIOR-RED`、`S5-REPLAN`、
  `S5-BEHAVIOR-RED-2`、`S5-BEHAVIOR-RED-3`、`S5-REPLAN-2`、
  `S5-BEHAVIOR-RED-4`、`S5-SOLVER-4`、`S5-LAB-RED`、`S5-LAB`；明确排除
  `S5-SOLVER`、`S5-SOLVER-2`、`S5-SOLVER-3`三个STOPPED/NOT COMMITTED node；
- self-test覆盖new parser/current S5-REPLAN-2 receipt、RED-4与SOLVER-4 PENDING/SHA双分支、
  mismatch、missing、unexpected、optional和三个STOPPED node不进入milestone union；
- SHA格式/commit解析、CLI/receipt equality、tracked+untracked union、cached判定、
  required/optional binary集合与milestone base语义全部保持zero-diff。

S5-BEHAVIOR-RED-4第一个shell在script尚未支持new node时，必须用与S5-API-RED bootstrap同构的
inline Ruby从§16读取`S5-BEHAVIOR-RED-4` immutable Start HEAD；script state transition完成后，
worker后续shell及reviewer/verifier统一切换到：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-BEHAVIOR-RED-4)"
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-BEHAVIOR-RED-4 "$S5_NODE_START_HEAD"
```

S5-REPLAN-3不修改scope script。clean detached `29bafce…`尚不认识new chain，本node必须按§9I
inline verifier对tracked+untracked union执行exact-8判定；不得把`unknown node S5-REPLAN-3`
误报成失败，也不得提前把script带入docs commit。

S5-BEHAVIOR-RED-5在自己的exact-5范围内完成第四次且仅限数据/fixture的state transition：

- `PROGRESS_NODES`保留全部historical nodes并加入`S5-REPLAN-3`、`S5-BEHAVIOR-RED-5`、
  `S5-SOLVER-5`；parser继续要求§16每个node恰有一行；
- `SCOPES`加入上表exact-8、exact-5、required exact-7/optional unit path；缩写在script中展开为
  repository-relative完整路径；
- `PRE_V_NODES`精确包含`S5-API-RED`、`S5-API`、`S5-BEHAVIOR-RED`、`S5-REPLAN`、
  `S5-BEHAVIOR-RED-2`、`S5-BEHAVIOR-RED-3`、`S5-REPLAN-2`、`S5-BEHAVIOR-RED-4`、
  `S5-REPLAN-3`、`S5-BEHAVIOR-RED-5`、`S5-SOLVER-5`、`S5-LAB-RED`、`S5-LAB`；明确排除
  `S5-SOLVER`、`S5-SOLVER-2`、`S5-SOLVER-3`、`S5-SOLVER-4`四个STOPPED/NOT COMMITTED node；
- self-test覆盖new parser、current REPLAN-3 SHA、RED-5与SOLVER-5的PENDING/SHA双分支、receipt/
  CLI mismatch、missing/unexpected/optional，以及四个STOPPED node不进入milestone union；
- SHA格式/commit解析、tracked+untracked union、cached判定、required/optional binary集合与
  milestone base语义保持zero-diff；不得借state transition改通用parser/helper。

RED-5第一个shell在script尚未支持new node时，必须用与上文相同的inline Ruby从§16读取
`S5-BEHAVIOR-RED-5` Start HEAD；script更新并通过self-test后，worker后续shell及reviewer/
verifier统一使用：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-BEHAVIOR-RED-5)"
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-BEHAVIOR-RED-5 "$S5_NODE_START_HEAD"
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb cached S5-BEHAVIOR-RED-5 "$S5_NODE_START_HEAD"
```

SOLVER-5从首次写入起直接使用`receipt-head S5-SOLVER-5`与binary scope。Living §16的PENDING/SHA
只提供node identity与immutable baseline，不代表task完成；进度只读OpenSpec `tasks.md`。

表中缩写在scope script中必须展开为repository-relative完整路径。Scope script还必须从
§16 progress table读取node的full start HEAD：empty/`PENDING`、非40位lowercase hex、
`rtk proxy git cat-file -e <sha>^{commit}`不可解析、CLI参数与receipt不一致都必须nonzero。
每个新shell使用以下可跨shell复跑的入口：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head <NODE>)"
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb <NODE> "$S5_NODE_START_HEAD"
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb cached <NODE> "$S5_NODE_START_HEAD"
```

S5-API-RED worker在scope script尚未创建的第一个shell必须用以下bootstrap直接读同一receipt，
随后所有角色统一切换到`receipt-head`；禁止用live `git rev-parse HEAD`临时替代receipt值：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby -e 'node=ARGV.fetch(0); path="docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md"; row=File.readlines(path).find { |line| cells=line.split("|"); cells[2]&.strip==node }; abort("missing node receipt") unless row; sha=row.split("|")[3].strip.delete("`"); abort("invalid receipt SHA") unless sha.match?(/\A[0-9a-f]{40}\z/); abort("unresolvable receipt SHA") unless system("rtk","proxy","git","cat-file","-e","#{sha}^{commit}"); print sha' S5-API-RED)"
```

`cached`模式必须调用`rtk proxy git diff --cached --name-only`并执行同一required/optional
判定和receipt baseline校验；只能由supervisor在固定allowlist stage后运行。Commit后再用
`git diff-tree`复核commit paths。S5-V使用
`milestone 28867b5c8eeae3ec290ad7dfd5f52cbf3ca69e80`模式对全部S5 commits/worktree做同样的required +
explicit optional union audit，并拒绝任何git-visible temp/target/dist/log。Ignored artifacts
不属于binary scope，按§12 filesystem hygiene单独分类。Scope script的判定逻辑只允许在
S5-API-RED实现。S5-API exact-16 required-set同步已完成；除此三行数据同步外，仅有一个
已review并同步的self-test state-transition exception：删除live S5-API
`future-receipt-pending`负向断言，改为固定current receipt
`6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68`的
`validate_cli_sha!("S5-API", "6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68")`正向断言，并输出
`S5_SCOPE_SELF_TEST=current-s5-api-receipt:PASS`。既有synthetic
`parser-future-pending`和generic `pending`必须保持zero-diff并继续覆盖`PENDING`负向合同；
不得把替代断言绑定到`S5-BEHAVIOR-RED`或任何其他真实future node。parser、CLI、helpers、
milestone union、其他self-test逻辑及required/optional判定均必须zero-diff。以后若compiler
证据要求新增optional exhaustive consumer，仍必须先更新living spec、由reviewer批准，再
单独更新script，不能由implementation worker静默扩表。public re-export缺口不得走optional机制。
`self-test`必须对empty、short SHA、non-hex、unresolvable commit和receipt/argument mismatch
逐项断言nonzero，再证明合法full SHA返回0。

## 5. S5-D 文档 Gate

```text
rtk proxy git status --short --branch
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy ruby -e 'paths=%w[docs/design/2026-07-14-revolute-joint-v1-design.md docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md]; missing=paths.reject { |p| File.file?(p) && File.size(p).positive? }; abort("missing docs: #{missing.join(", ")}") unless missing.empty?; puts "s5 docs ok"'
rtk proxy rg -n 'Software Interface Spec|```mermaid|2x2|non_exhaustive|schema v1|wake|S5-API-RED|S5-BEHAVIOR-RED|S5-LAB-RED|S5-C' docs/design/2026-07-14-revolute-joint-v1-design.md docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
rtk proxy ruby -e 'text=File.read("docs/design/2026-07-14-revolute-joint-v1-design.md"); abort("need >=2 Mermaid blocks") if text.scan(/```mermaid/).length < 2; puts "mermaid ok"'
rtk proxy ruby -e 'expected=%w[docs/ai/doc-catalog.yaml docs/ai/index.md docs/ai/repo-map.md docs/design/README.md docs/design/2026-07-14-revolute-joint-v1-design.md docs/handoff-2026-07-14-vnext-s5-revolute-joint.md docs/plans/2026-06-17-physics-realism-vnext-milestones.md docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md].sort; tracked=IO.popen(["rtk","proxy","git","diff","--name-only","HEAD"], &:read).lines.map(&:strip).reject(&:empty?); untracked=IO.popen(["rtk","proxy","git","ls-files","--others","--exclude-standard"], &:read).lines.map(&:strip).reject(&:empty?); actual=(tracked+untracked).uniq.sort; abort("s5-d scope mismatch: #{actual.inspect}") unless actual==expected; puts "s5-d scope ok"'
rtk proxy git diff --check
rtk proxy git diff --exit-code -- crates Cargo.toml Cargo.lock
rtk proxy git status --short
```

S5-D allowlist：

```text
docs/design/2026-07-14-revolute-joint-v1-design.md
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
docs/plans/2026-06-17-physics-realism-vnext-milestones.md
docs/handoff-2026-07-14-vnext-s5-revolute-joint.md
docs/ai/index.md
docs/ai/repo-map.md
docs/ai/doc-catalog.yaml
docs/design/README.md
```

架构 reviewer 必须逐项确认：Software Interface Spec 完整、两张 Mermaid 有 ownership
含义、2x2 算法和 angular sign一致、compat/schema ADR 明确、wake/transaction无泄漏、
acceptance matrix 可执行、handoff mandatory item 均有 owner。Spec reviewer 必须确认
节点顺序、RED/commit gate、文件 ownership、receipt 和 stop condition 没有缺口。
Routing reviewer 必须确认 AI source 路径对应 live tree，且没有把未实现能力写成现状。

## 6. S5-API-RED：public acceptance-as-code

提交前新增一个不会破坏 workspace compile 的 external fixture：

- repository 只保存 fixture source/manifest 与
  `crates/picea/tests/verify_revolute_public_api.rb`，并提交§4.1规定的
  `verify_revolute_scope.rb`；fixture路径固定为
  `crates/picea/tests/fixtures/revolute_public_api/{Cargo.toml,src/lib.rs,src/bin/exhaustive.rs}`。
- script 每次清空并重建
  `/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s5-revolute-api`，
  复制 fixture，改为当前 repository path dependency，然后在 temp cwd 运行
  `rtk proxy cargo check` / `cargo test`。
- positive fixture在S5-API GREEN时编译并运行批准surface/default/create/view/recipe/debug/
  schema与Revolute wake smoke；S5-API-RED基线因missing Revolute surface无法执行这些
  runtime assertions，只能分类为surface RED，绝不能记录为独立wake RED。
- negative fixture 对批准的六个 `#[non_exhaustive]` enums 做 external exhaustive match；
  GREEN 状态必须得到预期 compile-fail，不能把任意编译错误当通过。
- script 支持 `expect-red` 和 `expect-green`；RED 只接受“缺少批准 revolute surface”
  的诊断，路径/依赖/fixture语法错误必须判为 harness failure。
- `world_step_review_regressions.rs`新增以下六个exact tests。每个test先执行适用的
  Distance/WorldAnchor cases并打印稳定sentinel；cascade只适用于有surviving counterpart
  的Distance。`user_data`与rejected-transaction tests先证明“不wake/不泄漏”boundary，
  再运行同test内的positive wake control，所以基线仍必须因指定positive assertion RED：

| Exact test | Sentinel | 基线指定failure |
| --- | --- | --- |
| `joint_lifecycle_wake_create_contract` | `S5_WAKE_CASE:create` | sleeping dynamic endpoint未以`UserPatch` wake |
| `joint_lifecycle_wake_constraint_patch_contract` | `S5_WAKE_CASE:constraint_patch` | constraint patch后endpoint仍sleeping |
| `joint_lifecycle_wake_user_data_only_contract` | `S5_WAKE_CASE:user_data_only` | metadata-only不wake先通过；随后constraint positive control未wake |
| `joint_lifecycle_wake_remove_contract` | `S5_WAKE_CASE:remove` | remove后仍live endpoint未wake |
| `joint_lifecycle_wake_body_cascade_contract` | `S5_WAKE_CASE:body_cascade` | body cascade后surviving counterpart未wake |
| `joint_lifecycle_wake_rejected_transaction_contract` | `S5_WAKE_CASE:rejected_transaction` | rejected transaction零泄漏先通过；随后successful transaction positive control未wake |

RED 命令：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-API-RED)"
rtk proxy ruby crates/picea/tests/verify_revolute_public_api.rb expect-red /var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s5-revolute-api
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_create_contract -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_constraint_patch_contract -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_user_data_only_contract -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_remove_contract -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_body_cascade_contract -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_rejected_transaction_contract -- --exact --nocapture
rtk proxy cargo check --workspace --all-targets
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb self-test
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-API-RED "$S5_NODE_START_HEAD"
rtk proxy git diff --check
rtk proxy git diff --exit-code -- crates/picea/src crates/picea-lab/src crates/picea-lab/web Cargo.toml Cargo.lock
rtk proxy git status --short
```

通过标准：public verifier exit `0` 且内部positive compile是预期nonzero、诊断明确指向
missing `Revolute*`/variant；这只证明surface RED。六个runtime exact commands必须各自输出
`running 1 test`和对应sentinel，并exit nonzero且failure text命中表中指定positive wake
assertion；zero-test、compile error或negative boundary assertion失败都是harness/contract
failure。`cargo check --workspace --all-targets`必须exit `0`，证明committed RED tests可编译。
Repository内不得生成fixture-local `Cargo.lock`/`target`或repo内temp copy。

S5-API-RED receipt：

| Acceptance | Baseline expectation | 实际结果 |
| --- | --- | --- |
| Public desc/patch/prelude | RED: missing approved types | RED有效：external verifier连续两次wrapper exit `0`，内部positive `cargo check --lib`均exit `101`；仅出现批准的`E0432/E0599`，明确缺少prelude `RevoluteJointDesc/RevoluteJointPatch`及对应variants；future GREEN fixture已精确锁定desc default的两个invalid handles、zero anchors、`user_data=0`与patch default全`None`，当前因surface缺失`NOT RUN` |
| Six enum variants/attributes | RED: missing variant；attribute diagnosis分类记录 | 六个external feature probe均仅以`E0599`缺少`Revolute`失败：`JointKind/JointDesc/JointPatch/JointBundle/DebugJointKind/SceneJointFixture`；当前closed enums尚不能产生future `#[non_exhaustive]`的`E0004`，expect-green已逐enum锁定届时必须仅得到non-exhaustive wildcard `E0004` |
| Recipe/debug/fixture surface | RED: missing approved surface | RED有效：诊断明确覆盖`JointBundle::Revolute`/`JointBundle::revolute`、`DebugJointKind::Revolute`、`SceneRevoluteJointFixture`/`SceneJointFixture::Revolute`缺口；无manifest/path/network/fixture syntax或其他compiler error；future schema-v1 GREEN lock使用两组不同非零anchors并精确断言body indices、两anchors、user_data、version/tag及old-reader reject，当前`NOT RUN` |
| Existing Distance/WorldAnchor lifecycle runtime | RED：六个exact tests均`running 1 test`+sentinel+指定positive wake assertion failure | RED有效：六条exact命令均`running 1 test`、正确`S5_WAKE_CASE:*`、exit `101`；create/constraint patch/user-data positive control/remove均在最终positive failure前输出`S5_WAKE_OBSERVATION:*`并保存Distance与WorldAnchor两类state/event observation；body cascade在surviving Distance counterpart失败；rejected transaction先完整drain setup events，再通过revision/joint/body descriptor/完整empty event receipt以及untouched-control handle/descriptor一致性boundary，最后在successful WorldAnchor transaction positive control失败；无`S5_HARNESS_BOUNDARY`失败 |
| Revolute lifecycle runtime | NOT RUN：baseline missing public surface；只能记录external surface RED | `NOT RUN`：positive fixture因public surface缺失不能执行；verifier稳定输出`S5_PUBLIC_API_RUNTIME=NOT_RUN`，未把surface RED冒充runtime wake RED |
| Workspace all-target compile | GREEN：RED tests baseline-compilable | GREEN：`rtk proxy cargo check --workspace --all-targets` exit `0` |
| Temp isolation | GREEN: repo无lock/target | GREEN：仅在批准root`/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s5-revolute-api`生成isolated `Cargo.lock/target`；repository fixture无local `Cargo.lock`/`target`且repo内无temp copy |
| Binary node scope | GREEN: exact required set、无unexpected path；negative self-test非零 | GREEN：receipt parser只读取§16严格Markdown data rows并要求target node唯一；self-test证明正文/代码块node字符串不能覆盖真实row、duplicate row拒绝，并覆盖empty/PENDING/short/nonhex/unresolvable/noncommit/mismatch均拒绝且valid通过；S5-API-RED验收时future S5-API为`raw=PENDING`并按合同失败；S5-API-RED commit后已将S5-API immutable Start HEAD固定为`6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68`；node scope actual严格等于7 required paths，missing/unexpected均空；`git diff --check`与production/lab/Web/root Cargo zero-diff通过 |
| Independent review | PASS: 无未闭合finding | final independent reviewer=`0 High / 0 Medium / 0 Low`，裁决`PASS` |
| Independent verification | PASS: 独立复跑完整RED gate | independent verifier=`S5-API-RED VERIFIER PASS`；public expect-red连续两次wrapper `0`/internal `101`且仅approved diagnostics，六条wake exact均`running 1 test`、正确sentinel、approved exit `101`；workspace/scope/temp/diff均GREEN，7-path exact；Revolute runtime与Chrome均`NOT RUN` |
| Commit | reviewer/verifier通过后由supervisor执行 | 已commit：`6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68`（`test: lock revolute public api`）；S5-API/solver implementation未开始 |

## 7. S5-API：public surface、lifecycle 与 authoring

当前状态：scope spec/script sync已完成并reviewed，scope reviewer=`0 High / 0 Medium / 0 Low`、
裁决`PASS`，independent re-verifier=`S5-API SCOPE RE-VERIFIER PASS`；首次independent scope
verifier仅因living spec stale-status mismatch最终`FAIL`的历史保留。scope implementation gate
已解除；implementation worker已完成批准实现，worker targeted/full/scope gates=`GREEN`；
independent implementation reviewer首轮=`0 High / 1 Medium / 0 Low`、裁决`FAIL`（顶部仍误报
implementation未开始），第一次bounded remediation已完成；第一次复审=
`0 High / 1 Medium / 0 Low`、裁决`FAIL`（文档仍误报第一次状态修复未完成），第二次status-only bounded
remediation已完成；最终独立复审=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=
`S5-API VERIFIER PASS`。immutable Start HEAD=
`6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68`；S5-API已commit为
`c5a47ed4252363ebf914802c8189cf2bcc9e3563`；Chrome
`NOT RUN / BROWSER PENDING`。implementation worker未修改scope script。

最小实现必须严格匹配 design §7/§10：

- 增加 `RevoluteJointDesc/Patch`、批准 enums variants 与批准的六处
  `#[non_exhaustive]`；prelude 只增加 desc/patch。
- `JointRecord::body_handles()`、validation、patch、view/debug 支持两端 body；不增加
  typed getter 或 debug payload field。
- create先完整验证再allocate/attach/event/wake/revision；patch先完整验证并区分
  constraint fields vs `user_data`；remove/cascade先保存 endpoints再detach/wake surviving
  dynamic endpoints。
- lifecycle wake 使用 `UserPatch`；already-awake、static/kinematic 不产生 synthetic wake。
- recipe resolve 只替换 handles，保留 anchors/user_data；scratch rejection不泄漏wake。
- fixture enum/type skeleton与TS `DebugJoint.kind` union允许后续 LAB-RED 编译，但不在本节点
  实现 scenario/artifact/server label/consumer behavior。
- `pipeline/island.rs`为`JointDesc::Revolute`增加显式skip，不创建
  `JointSolvePlanRow`/`JointSolverRow`；focused test必须证明API checkpoint的revolute
  `joint_row_count == 0`。
- worker开始前运行下列`rg`，并迁移所有live compiler指出的external exhaustive matches；
  当前至少`scenario/scene_lattice.rs`与`scenario/fixture/tests.rs`。迁移只加
  wildcard/compat分支，不改变existing scenario/test语义。
- worker还必须完成`scenario/mod.rs` -> lab `lib.rs`的显式public re-export链，使committed
  fixture能继续从`picea_lab` crate root导入`SceneRevoluteJointFixture`。这两处是required，
  不是上述external exhaustive optional adapters。

Targeted GREEN：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-API)"
rtk proxy ruby crates/picea/tests/verify_revolute_public_api.rb expect-green /var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s5-revolute-api
rtk proxy rg -n 'match .*JointDesc|JointDesc::Distance|JointDesc::WorldAnchor' crates/picea-lab -g '*.rs'
rtk proxy cargo test -p picea --test core_model_world revolute_joint_ -- --nocapture
rtk proxy cargo test -p picea --test core_model_world revolute_joint_api_checkpoint_has_no_solver_row -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_create_contract -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_constraint_patch_contract -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_user_data_only_contract -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_remove_contract -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_body_cascade_contract -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_rejected_transaction_contract -- --exact --nocapture
rtk proxy cargo test -p picea-lab scene_fixture_revolute -- --nocapture
rtk proxy npm --prefix crates/picea-lab/web run build
rtk proxy cargo test -p picea --test core_model_world
rtk proxy cargo test -p picea --test world_step_review_regressions
rtk proxy cargo test -p picea-lab --lib
rtk proxy cargo check --workspace --all-targets
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-API "$S5_NODE_START_HEAD"
rtk proxy git diff --check
```

reviewer 必须重点检查 external source break 是否只限批准列表、struct literals仍可用、
wrong-kind/stale/same-body/NaN失败原子性、`user_data`-only不wake、direct与scratch transaction
语义一致、body cascade只wake surviving counterpart。Compat reviewer必须确认
`pipeline/island.rs`只有explicit skip、没有row/math，external consumer改动只有
wildcard/compat，并确认两个public re-export文件只扩展批准的fixture export chain。Scope
reviewer必须先对本文exact 16 required set给出二元批准；scope-sync复核必须证明script只把
这16条同步为S5-API required set。self-test exception reviewer还必须二元确认只删除live
`future-receipt-pending`断言并换成固定S5-API current receipt的`validate_cli_sha!`与指定PASS
tag，synthetic `parser-future-pending`、generic `pending`及其他self-test逻辑zero-diff，且未绑定
任何其他真实future node。Verifier还必须确认workspace compile、
row count 0、16-path binary scope和
`git diff --exit-code -- Cargo.toml Cargo.lock`。六个lifecycle commands必须各自exit `0`、
输出`running 1 test`和对应sentinel，并在适用case输出Distance/WorldAnchor/Revolute覆盖；
任何zero-test都不是GREEN。

### S5-API 验收报告（2026-07-14）

**成功标准**：在exact-16批准范围内交付Revolute public/lifecycle/authoring API，保持solver
row/math为零改动且Revolute `joint_row_count == 0`，并让targeted、full、scope与hygiene门全部
GREEN。

**检查结果**

- Public/targeted surface：

  ```text
  rtk proxy ruby crates/picea/tests/verify_revolute_public_api.rb expect-green /var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s5-revolute-api
  rtk proxy cargo test -p picea --test core_model_world revolute_joint_ -- --nocapture
  rtk proxy cargo test -p picea --test core_model_world revolute_joint_api_checkpoint_has_no_solver_row -- --exact --nocapture
  rtk proxy cargo test -p picea-lab scene_fixture_revolute -- --nocapture
  ```

  全部exit `0`。external verifier输出`S5_PUBLIC_API_POSITIVE_GREEN`与
  `S5_PUBLIC_API_EXPECTED_GREEN`，六enum probe均只得到批准的non-exhaustive wildcard
  `E0004`；core focused=`6 passed`；row-zero exact显示`running 1 test`与
  `S5_API_CHECKPOINT:joint_count=1;joint_row_count=0`；fixture focused=`3 passed`，覆盖
  roundtrip、core defaults与old-reader unknown-variant reject。

- 六条lifecycle exact：

  ```text
  rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_create_contract -- --exact --nocapture
  rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_constraint_patch_contract -- --exact --nocapture
  rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_user_data_only_contract -- --exact --nocapture
  rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_remove_contract -- --exact --nocapture
  rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_body_cascade_contract -- --exact --nocapture
  rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_rejected_transaction_contract -- --exact --nocapture
  ```

  六条均exit `0`、各自`running 1 test`并输出对应`S5_WAKE_CASE:*`。create/constraint
  patch/user-data-only positive control/remove覆盖Distance、WorldAnchor、Revolute；cascade覆盖
  Distance/Revolute surviving counterpart；rejected transaction证明scratch rejection零泄漏并
  覆盖WorldAnchor/Revolute positive controls；empty/user-data-only patch不wake。

- Full/build gates：

  ```text
  rtk proxy npm --prefix crates/picea-lab/web run build
  rtk proxy cargo test -p picea --test core_model_world
  rtk proxy cargo test -p picea --test world_step_review_regressions
  rtk proxy cargo test -p picea-lab --lib
  rtk proxy cargo check --workspace --all-targets
  rtk proxy cargo fmt --all --check
  ```

  全部exit `0`；full suites分别`24 passed`、`18 passed`、`38 passed`；Web build完成且仅有
  既有500 kB chunk warning；workspace all targets与fmt均GREEN。

- Scope/hygiene gates：

  ```text
  rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb self-test
  rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-API 6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68
  rtk proxy git diff --check
  rtk proxy git diff --exit-code -- Cargo.toml Cargo.lock
  rtk proxy git diff --cached --exit-code
  rtk proxy git diff --exit-code 6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68 -- crates/picea/src/pipeline/joints.rs crates/picea/src/solver
  rtk proxy git diff --unified=0 6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68 -- crates/picea/src/joint.rs crates/picea/src/debug.rs crates/picea/src/recipe.rs crates/picea-lab/src/scenario/fixture.rs | rg -xF '+#[non_exhaustive]' | awk 'END { print "S5_ENUM_NON_EXHAUSTIVE_ADDITIONS=" NR; exit NR == 6 ? 0 : 1 }'
  find crates/picea/tests -maxdepth 3 \( -name Cargo.lock -o -name target -o -name '*.tmp' \) -print
  ```

  全部exit `0`；`S5_SCOPE_SELF_TEST_PASS`；actual严格等于16 paths且missing/unexpected为空；
  `S5_SCOPE_PASS`；root Cargo、cached与solver paths均zero-diff；enum audit输出
  `S5_ENUM_NON_EXHAUSTIVE_ADDITIONS=6`；repository fixture/temp搜索为空。`pipeline/island.rs`
  相对Start HEAD只增加4行Revolute explicit skip/comment，不创建slot/row/math。

- Independent review/verifier receipt：final independent reviewer=`0 High / 0 Medium / 0 Low`、
  裁决`PASS`，前两轮Medium均已通过两次bounded status remediation闭环。independent verifier
  首行=`S5-API VERIFIER PASS`，Start/HEAD均为
  `6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68`；独立串行复跑§7 mandatory全部exit `0`：
  public positive GREEN且六enum均只有批准的`E0004`，public runtime=`2 passed`，core focused=
  `6 passed`，row-zero为`running 1 test`并输出
  `S5_API_CHECKPOINT:joint_count=1;joint_row_count=0`；六条lifecycle均各自`running 1 test`、
  对应sentinel与批准kind覆盖；fixture=`3 passed`；Web PASS且仅有既有500 kB warning；full
  core/world/lab=`24/18/38 passed`；workspace check、scope self-test/exact-16、fmt、diff、Cargo、
  solver-zero-diff、cached/untracked与fixture/temp hygiene均PASS。首次public harness运行因verifier
  与另一进程重叠使用固定temp root而出现一次`ENOENT`；停止重叠后串行执行同一命令及runtime
  补跑均PASS，且期间源文件zero-diff，因此分类为verifier orchestration noise，不是产品失败。

**复跑方式**：checkout `c5a47ed4252363ebf914802c8189cf2bcc9e3563`，以
`6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68`作为immutable Start HEAD，按上方四组命令顺序
复跑；scope必须继续得到exact 16，row-zero必须继续输出指定checkpoint sentinel，六条lifecycle
必须各自显示`running 1 test`与对应`S5_WAKE_CASE:*`。

**范围外**：未实现或修改S5-BEHAVIOR-RED、S5-SOLVER row/math、`pipeline/joints.rs`、
`solver/*`、scenario/artifact/server/UI behavior；未push；Chrome验收未在CLI执行，
保持`NOT RUN / BROWSER PENDING`，由ChatGPT App在后续S5-LAB/S5-V节点按本文prompt验收。

**残余风险**：Revolute当前被明确保留但不进入solver，行为能力
必须等待后续独立S5-BEHAVIOR-RED/S5-SOLVER节点；六个`#[non_exhaustive]`是批准的external
source break；Web build仍有既有500 kB chunk warning；Chrome仍为
`NOT RUN / BROWSER PENDING`。

## 8. S5-BEHAVIOR-RED：runtime 行为锁

在 S5-API commit 上新增以下 acceptance tests，不改 solver production：

1. `revolute_joint_two_dynamic_preserves_anchor_coincidence`：240帧，记录全窗/final drift
   和 numeric warnings。
2. `revolute_joint_leaves_relative_rotation_free`：60帧相对角变化至少 `1.0 rad`。
3. `revolute_joint_static_dynamic_preserves_static_pose`：static pose bit-exact。
4. `revolute_joint_off_center_anchor_uses_rotational_inertia`：dynamic rotation至少
   `0.5 rad`，与 translation-only错误实现可区分。
5. `revolute_joint_is_deterministic_and_finite`：同场景双跑逐帧hash相同，全部 facts finite。
6. `revolute_joint_mixed_contact_island_keeps_separate_logical_rows`：contact rows > 0、
   `joint_row_count == 1`。
7. `revolute_joint_connected_bodies_still_contact`：connected pair仍产生contact。
8. `revolute_joint_nonzero_local_center_of_mass_preserves_pivot`：translated/off-center
   collider产生nonzero local COM且endpoint具有非零angular velocity，击穿把world COM
   delta直接加Pose origin或只按current pose重建的错误实现。
9. `revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle`：命中帧保留clamped
   translation但使用`angle + angular_velocity*dt`；不扩展CCD算法。
10. `revolute_joint_awake_sleeping_pair_uses_current_pose_and_wakes_on_correction`：同一joint
    island另一endpoint awake时，sleeping dynamic使用current pose参与solve；只有finite
    nonzero correction才以`JointCorrection` wake，zero/singular/skip保持sleeping。
11. `revolute_joint_contact_full_step_preserves_pivot_with_projection_enabled`与
    `revolute_joint_contact_full_step_preserves_pivot_with_projection_disabled`：contact-rich
    240帧full-step在projection两种配置下都满足既有全窗`<=0.03`、final`<=0.01`，并证明
    contact实际发生；不得靠stream reorder通过。
12. `revolute_joint_ccd_clamped_contact_full_step_preserves_pivot`：CCD clamp与contact velocity
    mutation同帧发生，linear final integration仍skip clamped body，angular final integration
    使用contact/projection后的最新angular velocity，并满足同一drift阈值。
13. focused unit tests 覆盖2x2 off-diagonal、clockwise-positive cross sign、static-static、
    singular/non-finite fail closed、COM pose rebuild和atomic apply。Test-only ownership固定为
    new `crates/picea/src/pipeline/joints/tests.rs`，`pipeline/joints.rs`只加
    `#[cfg(test)] mod tests;`。

RED 命令：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-BEHAVIOR-RED)"
rtk proxy cargo test -p picea --lib pipeline::joints::tests::revolute_point_constraint_ -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_two_dynamic_preserves_anchor_coincidence -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_leaves_relative_rotation_free -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_static_dynamic_preserves_static_pose -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_off_center_anchor_uses_rotational_inertia -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_is_deterministic_and_finite -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_mixed_contact_island_keeps_separate_logical_rows -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_connected_bodies_still_contact -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_nonzero_local_center_of_mass_preserves_pivot -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_awake_sleeping_pair_uses_current_pose_and_wakes_on_correction -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_contact_full_step_preserves_pivot_with_projection_enabled -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_contact_full_step_preserves_pivot_with_projection_disabled -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_ccd_clamped_contact_full_step_preserves_pivot -- --exact --nocapture
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-BEHAVIOR-RED "$S5_NODE_START_HEAD"
rtk proxy git diff --check
rtk proxy git diff --exit-code -- crates/picea-lab crates/picea/tests/core_model_world.rs crates/picea/tests/world_step_review_regressions.rs Cargo.toml Cargo.lock
```

期望：API create/lifecycle边界保持green。每个integration exact命令必须输出
`running 1 test`；下表expected RED项必须exit nonzero且命中指定solver assertion，expected
boundary GREEN项必须exit `0`。zero matched tests不算RED或GREEN。Unit command若因planned helper尚不存在compile RED，只接受
明确E0425/E0599且诊断指向批准的2x2/COM helper；其他compile failure是harness failure。
每条命令单独记录exit/关键断言，不得只用一个filter聚合替代分类。Reviewer必须确认
`pipeline/joints.rs`除test module声明外production zero-diff。

| Acceptance | Baseline expectation | 实际结果 |
| --- | --- | --- |
| 2x2 unit contract | RED：批准assertion；或仅E0425/E0599指向planned helper | expected RED有效：exit `101`，仅8个`E0425`，全部指向`invert_revolute_effective_mass`、`revolute_position_deltas`、`rebuild_revolute_current_pose_from_world_com`、`apply_revolute_pose_pair_atomically`四个批准的2x2/COM/atomic planned helper；无其他诊断 |
| two-dynamic drift | RED | expected RED有效：`running 1 test`，exit `101`；`max_drift=8.000021`、`final_drift=8.000021`、numeric warning `0`，命中`S5_REVOLUTE_SOLVER_ASSERT:two_dynamic_max_drift` |
| free rotation | boundary GREEN：relative rotation `>=1.0 rad`；pivot preservation由two-dynamic RED锁定 | boundary GREEN：`running 1 test`、exit `0`；两个dynamic endpoint均有真实矩形collider且禁用pair contact，inverse mass=`(2.0,2.0)`、inverse inertia=`(19.200001,19.200001)`；`rA=rB=0`时`K=(inv_mass_a+inv_mass_b)I`、determinant=`16.0`，证明row可逆；60帧relative change=`1.999999 rad`，baseline每帧`joint_row_count=0`，不把deterministic skip写成solver能力 |
| static/dynamic + inertia | RED | 两条expected RED均有效且各自`running 1 test`、exit `101`：static pose先bit-exact，pivot drift=`1.999998`后命中static/dynamic solver assertion；off-center rotation=`0.000000`、warning `0`，命中`rotation>=0.5` solver assertion |
| awake/sleeping current pose + correction wake | RED；zero/singular/skip子案例必须保持sleeping | expected RED有效：`running 1 test`、exit `101`；zero/singular/inactive-skip三子案例均先以`body_a/body_b sleeping=true`、无wake event、row `0` GREEN；positive case facts为finite correction=`0`、sleeping endpoint仅以`Unknown`离睡且无`JointCorrection`，命中批准wake assertion。早期fixture曾因复用固定`can_sleep=false`的`create_body` helper而先在sleeping前置断言失败；该失败属于fixture错误而非产品RED，改为直接创建`BodyDesc { can_sleep: true, .. }`后exact得到上述有效分类 |
| nonzero local COM pose rebuild | RED，且exact实际运行1 test | expected RED有效：`running 1 test`、exit `101`；local COM=`(0.75,0.25)`、local anchor=`(1.25,0.25)`，因此anchor不等于COM、lever=`(0.5,0.0)`且length=`0.5`；inverse mass/inertia=`2.666667/39.384617`、endpoint angular velocity=`3.0`、max/final drift=`2.549487/0.359781`，命中nonzero-COM max-drift solver assertion |
| CCD-clamped final angle | RED，且exact实际运行1 test | expected RED有效：`running 1 test`、exit `101`；真实clamp/contact=`1/1`，从trace重建clamped translation=`(-0.09899998,0)`，sampled eval angle=`0.1`；沿eval-pose lever的径向error使显式2x2 `K`求得position angular correction=`0`，latest omega=`2.054233`，actual/expected angle均=`0.034237`；actual-world handle/collider identity与control CCD geometry/decision facts分别通过，最终仅以pivot drift=`0.052117`命中solver assertion |
| contact full-step projection enabled/disabled | 两条均RED；沿用`0.03/0.01`阈值 | 两条expected RED均有效且各自`running 1 test`、exit `101`；两配置均有211个contact frames、warning `0`，max/final drift均=`1.275484/1.275482`，分别命中固定`0.03` max-drift assertion |
| CCD-clamped contact full-step | RED；linear skip + updated angular advance | expected RED有效：`running 1 test`、exit `101`；clamp/contact=`1/1`、normal impulse=`1.570796`，contact把omega从`6.0`改为`2.054233`，径向oracle证明position angular correction=`0`，final/expected angle均=`0.034237=latest omega*dt`；actual-world identity与control trace语义边界通过，最终仅以drift=`0.024294`命中solver assertion |
| determinism/finite | boundary GREEN；不把deterministic skip冒充solver能力 | boundary GREEN：`running 1 test`、exit `0`；双跑120帧snapshot hash逐帧相同、body/mass/pose/velocity全部finite、warning `0`，baseline row facts逐帧均`0` |
| mixed island logical row | RED | expected RED有效：`running 1 test`、exit `101`；pair contact=`1`、contact row=`1`先GREEN，joint row=`0`后命中`one logical joint row` assertion |
| connected contact | boundary GREEN，必须exit 0；若RED则contract/harness失败 | boundary GREEN：`running 1 test`、exit `0`；connected pair contact=`1`、contact row=`1`，baseline joint row事实=`0` |

### S5-BEHAVIOR-RED 验收报告（2026-07-14，worker receipt）

**Start HEAD**：`c5a47ed4252363ebf914802c8189cf2bcc9e3563`。

**成功标准**：unit仅以批准的2x2/COM helper compile contract变红；13条integration exact
逐条真实运行1 test并严格得到10条expected RED与3条boundary GREEN；solver production
zero-diff、node scope exact 4。

**检查结果**：首轮reviewer=`1 High / 3 Medium / 1 Low`、裁决
`FAIL`。High指出CCD final-angle没有排除position phase角修正；三个Medium分别指出
free-rotation缺少positive mass/inertia与可逆`K`证明、nonzero-COM anchor等于COM导致lever为零、
living spec仍误报`READY / NOT STARTED`；Low要求记录awake/sleep fixture早期
`can_sleep=false` harness failure。bounded remediation已逐项闭环：加入径向zero-angular
correction 2x2 oracle、真实mass/inertia与可逆`K`前置证据、非零lever fixture和本状态回填；
CCD跨world trace改为actual identity与几何/decision facts分层比较，避免world-local handle误报。
final reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`，五项finding均已闭环；首轮
`1 High / 3 Medium / 1 Low`及其bounded remediation历史完整保留。independent verifier=
`S5-BEHAVIOR-RED VERIFIER PASS`。

§8列出的unit与13条integration exact命令已逐条运行；每条integration均输出
`running 1 test`。unit exit `101`且仅批准的`E0425`；two-dynamic、static/dynamic、off-center、
mixed-row、nonzero-COM、CCD-final-angle、awake/sleep、projection enabled、projection disabled、
CCD-contact十条均exit `101`并命中上表指定`S5_REVOLUTE_SOLVER_ASSERT`；free-rotation、
determinism/finite、connected-contact三条均exit `0`；remediation后无warning或harness failure。
awake/sleep的最早失败为fixture复用`can_sleep=false` helper导致sleeping前置断言先失败，不是
产品RED；改为直接使用`can_sleep=true`的`BodyDesc`后，zero/singular/inactive保持sleeping，
positive case仅缺`JointCorrection`。额外API边界命令
`rtk proxy cargo test -p picea --test core_model_world revolute_joint_ -- --nocapture`为
`6 passed`并保留`S5_API_CHECKPOINT:joint_count=1;joint_row_count=0`；lifecycle边界命令
`rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_ -- --nocapture`
为`6 passed`，六类sentinel与Revolute/既有joint-kind observation均保留。`cargo fmt --all --check`
与scope self-test均PASS；`pipeline/joints.rs`相对Start HEAD唯一diff为
`#[cfg(test)] mod tests;`；scope self-test为`S5_SCOPE_SELF_TEST_PASS`，node scope为exact 4、
missing/unexpected均空；`git diff --check`
与lab/core API tests/world regression/root Cargo/Cargo.lock范围外zero-diff均PASS。

independent verifier在Start/HEAD均为
`c5a47ed4252363ebf914802c8189cf2bcc9e3563`的同一worktree从头复跑并给出
`S5-BEHAVIOR-RED VERIFIER PASS`：unit exit `101`且只有8个`E0425`，严格落在四个批准的
planned helpers；13条integration均为`running 1 test`，其中10条expected RED只命中批准的
solver assertion，3条boundary GREEN均exit `0`。关键metric与上表一致：two-dynamic
max/final drift=`8.000021/8.000021`、warning=`0`；free-rotation relative change=`1.999999`、
`K` determinant=`16.0`；static/dynamic drift=`1.999998`、off-center rotation=`0.000000`；
nonzero-COM lever length=`0.5`、max/final drift=`2.549487/0.359781`；CCD final-angle
clamp/contact=`1/1`、drift=`0.052117`；projection enabled/disabled均有211 contact frames且
max/final drift=`1.275484/1.275482`；CCD-contact clamp/contact=`1/1`、normal impulse=
`1.570796`、drift=`0.024294`；mixed/connected contact row均为`1`，baseline joint row均为
`0`。API focused=`6 passed`并保留row-zero checkpoint，lifecycle=`6 passed`；fmt、scope
self-test、exact-4、diff-check、solver/lab/Cargo zero-diff、cached empty、唯一untracked
`crates/picea/src/pipeline/joints/tests.rs`及repo-local generated-artifact hygiene均PASS。
verifier有一次只读status `rg`因pattern中的反引号被shell展开而输出`command not found`；随即
以单引号安全复跑同一检查并PASS，未修改文件、未改变任何验收结果，分类为verifier命令编排
噪声，不是产品失败。

**复跑方式**：该段最初记录的是未提交worktree快照；当前应checkout
`385c4c350ceee98947c65da5e3f63384804c69e1`，并从receipt读取
`S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-BEHAVIOR-RED)"`，
按§8 `RED 命令`代码块原顺序逐条运行unit、13条integration exact、scope、diff与zero-diff
命令；禁止用aggregate filter替代逐条分类。

**范围外**：未实现2x2 helper、`JointSolverRow::Revolute`、joint solve batch/math、COM apply、
phase reorder或任何solver production；未修改lab/Web、API/lifecycle tests、scope script、Cargo
文件或integration阈值；未stage/commit/push；Chrome保持`NOT RUN / BROWSER PENDING`。

**残余风险（历史快照 + current supersede）**：本段最初写下的“commit `PENDING`、
S5-SOLVER尚未开始、下一步提交RED”是当时worker receipt，现已被§16后续事实supersede：
S5-BEHAVIOR-RED已commit为`385c4c350ceee98947c65da5e3f63384804c69e1`；历史S5-SOLVER
随后执行但因frozen-contract conflict永久STOPPED且`NOT COMMITTED`；当时next node是
S5-REPLAN。该链又被§9D记录的RED-3/SOLVER-3 current chain supersede；十条historical
expected RED与原阈值/断言继续冻结。Chrome仍为`NOT RUN / BROWSER PENDING`。

## 9. S5-SOLVER：2x2 point constraint

当前状态：worker与独立reviewer均确认`FROZEN CONTRACT CONFLICT`，S5-SOLVER未GREEN、未提交。
严格按本节2x2/COM/atomic合同实现后，13条integration中7条GREEN、6条FAIL：two-dynamic
首帧drift=`0.033333`，nonzero-COM max drift=`0.056089`，两条CCD drift分别为
`0.021701`/`0.019774`；projection on/off contact fixture在pivot正确对齐后，按live
full-dimensions语义仍有`1.25` vertical gap，故`contact_frames=0`；不是切触。独立reviewer
裁决=`2 High / 0 Medium / 0 Low`，确认不是K、符号、COM
重建、atomic apply、wake、row或stream实现遗漏。命中§15 stop condition，等待用户在
post-contact pose reconciliation、phase-lag验收语义或velocity/CCD语义重设计之间裁决；
不得在裁决前修改冻结测试、阈值、phase或contact路径。Chrome保持
`NOT RUN / BROWSER PENDING`。

上述“等待用户裁决”是2026-07-14的历史状态。用户已于2026-07-15选择post-contact pose
reconciliation + positive-penetration fixture；旧S5-SOLVER仍保持STOPPED且production diff已
恢复，不在本节继续实现。当前先完成§9A复审/verifier，再按§9B-§9C执行，之后回到
§10-§13；不得回到旧S5-SOLVER续写。

实现必须遵守 design §9：

- 一个 `JointSolverRow::Revolute` 保存两个 dense slots 和 descriptor；一个 row计数为1。
- position/optional velocity phases都构造同一对称 2x2 `K`，包含 inverse mass、
  `rA/rB` 与 inverse inertia；禁止两个串行 scalar corrections。
- position phase遵守 existing CCD-clamped vs predicted pose选择；optional velocity
  projection只在 existing config启用时执行。
- `lambda`平移是world COM delta；对dynamic endpoint先用corrected eval world COM/
  corrected eval angle反推corrected eval origin，再减去`eval_pose - current_pose`对应的
  sampled linear/angular advance，得到要写回的authoritative current pose。不能直接写回
  predicted pose造成final integration double-advance，也不能只用current COM/current angle
  重建而丢失本帧angular advance。A/B new current poses全部finite后原子apply，
  static/kinematic不动。
- endpoint pose是mandatory joint position phase的sampling pose，不无条件等于final pose：
  static current；kinematic预测translation/angle；awake dynamic预测translation/angle；同一
  active joint island里的sleeping dynamic使用current pose；CCD-clamped awake dynamic保留
  current translation但按该sampling时点的angular velocity预测angle。其后contact solve与
  optional joint velocity projection可能改变linear/angular velocity；final integration使用
  更新后的velocity，且CCD-clamped body继续skip linear advance但保留更新后的angular
  advance。不得为消除这一区别而重排stream。
- sleeping dynamic只有收到finite nonzero correction才以`JointCorrection` wake；row skip、
  singular/non-finite fail-closed或zero correction都不得产生synthetic wake。
- singular、static-static、non-finite和non-finite delta整行/整phase fail closed；所有
  body deltas预计算并验证后原子apply，不新增 numeric warning。
- 不保留 accumulated impulse，不改变 relative-angle target，不改 stream或phase order。
- `pipeline.rs`只把`StepConfig::joint_velocity_projection` public doc同步为joint
  point-velocity projection语义，并保持现有字段、default与serde配置行为锁不变。

Targeted GREEN：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-SOLVER)"
rtk proxy cargo test -p picea --lib pipeline::joints::tests::revolute_point_constraint_ -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_two_dynamic_preserves_anchor_coincidence -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_leaves_relative_rotation_free -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_static_dynamic_preserves_static_pose -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_off_center_anchor_uses_rotational_inertia -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_is_deterministic_and_finite -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_mixed_contact_island_keeps_separate_logical_rows -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_connected_bodies_still_contact -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_nonzero_local_center_of_mass_preserves_pivot -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_awake_sleeping_pair_uses_current_pose_and_wakes_on_correction -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_contact_full_step_preserves_pivot_with_projection_enabled -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_contact_full_step_preserves_pivot_with_projection_disabled -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_ccd_clamped_contact_full_step_preserves_pivot -- --exact --nocapture
rtk proxy cargo test -p picea --lib pipeline::tests::default_step_config_matches_single_step_contract -- --exact --nocapture
rtk proxy cargo test -p picea --lib pipeline::tests::step_config_deserializes_new_solver_policy_with_defaults -- --exact --nocapture
rtk proxy cargo test -p picea --lib pipeline::island -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4 -- --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-SOLVER "$S5_NODE_START_HEAD"
rtk proxy git diff --check
```

reviewer 必须逐式核对 `K11/K12/K22`、clockwise-positive point velocity与angular apply
符号、A/B impulse/correction方向、finite/atomic gate、one-row/separate-stream不变量和
COM-to-origin rebuild、sampling/current/final pose separation、sleeping endpoint current pose、
CCD pose selection以及contact/projection后velocity mutation。Verifier必须报告drift、relative angle、
rotation、sleep/wake reason、projection on/off、nonzero COM、clamped translation/updated final
angle、hash、row counts、warning count，
而不只报告`passed`。

## 9A. S5-REPLAN：superseding design/spec contract

当前状态：Start HEAD=`385c4c350ceee98947c65da5e3f63384804c69e1`；用户Plan Gate已于
2026-07-15通过；docs worker完成；final reviewer=`0 High / 0 Medium / 1 Low`、裁决`PASS`，
唯一Low `A05-A10d -> A05-A10e`已闭合；independent verifier首轮因stale reviewer status
裁决`FAIL`，status-only remediation已完成，re-verifier=`S5-REPLAN RE-VERIFIER PASS`；
supervisor已commit=`d987d1f7a85964f8c26121bc70b2a96b7f406f6e`。Chrome保持
`NOT RUN / BROWSER PENDING`。

Ownership严格exact 2：

```text
docs/design/2026-07-14-revolute-joint-v1-design.md
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
```

本node只记录verified conflict、用户Decision、ADR-S5-5、new execution chain、RED-2/SOLVER-2
ownership/gates与STOP边界；不修改production、tests、scope script、Cargo或Web。因为live
`verify_revolute_scope.rb`尚不识别新node，使用以下supervisor inline Ruby exact-path gate；
script的正式state transition归S5-BEHAVIOR-RED-2：

```text
S5_REPLAN_START_HEAD=385c4c350ceee98947c65da5e3f63384804c69e1
rtk proxy git diff --check
rtk proxy ruby -e 'start=ARGV.fetch(0); expected=%w[docs/design/2026-07-14-revolute-joint-v1-design.md docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md].sort; tracked=IO.popen(["rtk","proxy","git","diff","--name-only",start], &:read).lines.map(&:strip).reject(&:empty?); untracked=IO.popen(["rtk","proxy","git","ls-files","--others","--exclude-standard"], &:read).lines.map(&:strip).reject(&:empty?); actual=(tracked+untracked).uniq.sort; puts "S5_REPLAN_SCOPE_ACTUAL=#{actual.join(",")}"; abort("S5-REPLAN exact-path mismatch: #{actual.inspect}") unless actual==expected; puts "S5_REPLAN_SCOPE=PASS"' "$S5_REPLAN_START_HEAD"
rtk proxy git diff --exit-code 385c4c350ceee98947c65da5e3f63384804c69e1 -- crates Cargo.toml Cargo.lock
rtk proxy rg -n 'S5-REPLAN|S5-BEHAVIOR-RED-2|S5-SOLVER-2|ADR-S5-5|post-contact|positive penetration|0.125' docs/design/2026-07-14-revolute-joint-v1-design.md docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
rtk proxy rg -n 'would-wake|would_wake_resamples_latest_velocity|SharedShape::rect|derived half extents|12条expected RED / 4条boundary GREEN|NOT COMMITTED|2 High / 3 Medium / 1 Low' docs/design/2026-07-14-revolute-joint-v1-design.md docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
rtk proxy ruby -e 'paths=ARGV; paths.each { |path| text=File.read(path); opens=text.scan(/^```mermaid\s*$/).length; closes=text.scan(/^```\s*$/).length; abort("missing Mermaid in #{path}") if path.include?("design/") && opens < 2; abort("unclosed fence in #{path}") if text.scan(/^```/).length.odd? }; puts "S5_REPLAN_MERMAID_STRUCTURE=PASS"' docs/design/2026-07-14-revolute-joint-v1-design.md docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
rtk proxy git diff --cached --exit-code
```

### S5-REPLAN docs worker验收报告（2026-07-15）

**Start HEAD**：`385c4c350ceee98947c65da5e3f63384804c69e1`（immutable）。

**成功标准**：在exact-2 docs范围内保留旧S5-SOLVER STOPPED证据，并把用户批准的
ADR-S5-5、新chain、RED-2/SOLVER-2 ownership、acceptance与STOP合同写成可复跑规范；
production/tests/script/Cargo/Web zero-diff。

**检查结果**：docs worker已在同一worktree运行§9A全部命令：

- `rtk proxy git diff --check` exit `0`；无whitespace error。
- inline exact-path Ruby gate exit `0`，输出
  `S5_REPLAN_SCOPE_ACTUAL=docs/design/2026-07-14-revolute-joint-v1-design.md,docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md`
  与`S5_REPLAN_SCOPE=PASS`，actual严格exact 2。
- `rtk proxy git diff --exit-code 385c... -- crates Cargo.toml Cargo.lock` exit `0`，证明
  production/tests/script/Cargo均zero-diff。
- required-term `rg` exit `0`，两份docs均命中三个new nodes、ADR-S5-5、post-contact、
  positive penetration和`0.125`合同。
- Mermaid结构checker exit `0`并输出`S5_REPLAN_MERMAID_STRUCTURE=PASS`；人工检查design两张
  图的fence/节点/边闭合，新增flow只表达existing contact -> optional projection ->
  Revolute-only reconciliation，不把pass连成second stats source。
- `rtk proxy git diff --cached --exit-code` exit `0`；无staged paths。

以上为docs worker receipt；final reviewer=`0 High / 0 Medium / 1 Low`、裁决`PASS`，唯一Low
`A05-A10d -> A05-A10e`已由bounded修复闭合；independent verifier首轮因stale reviewer
status裁决`FAIL`，status-only remediation已完成；re-verifier=`S5-REPLAN RE-VERIFIER PASS`。

**复跑方式**：从Start HEAD读取当前worktree，按§9A代码块顺序运行全部命令；scope actual
必须严格等于上述两条路径。

**范围外**：不实现solver，不修改任何test或scope script，不stage/commit/push/merge；
不执行Chrome，保持`NOT RUN / BROWSER PENDING`。

**残余风险**：ADR-S5-5尚未由RED-2 acceptance-as-code或SOLVER-2 GREEN证明；final reviewer
已PASS且唯一Low已闭合，re-verifier=`S5-REPLAN RE-VERIFIER PASS`；supervisor随后已提交
S5-REPLAN=`d987d1f7a85964f8c26121bc70b2a96b7f406f6e`。

### S5-REPLAN docs review history

- 首轮docs reviewer=`2 High / 3 Medium / 1 Low`、裁决`FAIL`；该失败记录永久保留，不能被
  后续复审覆盖或改写。
- High 1：RED-2把baseline explicit-skip Revolute误写成前置`joint_row_count==1` GREEN。
  bounded remediation已改为先记录baseline row `0`，row `1`是批准solver assertion RED，并
  分开记录原13历史分类与RED-2总计`12 RED / 4 GREEN`的新分类。
- High 2：post-contact sleeper只写current-pose correction会在wake后被final integration用其
  nonzero stored velocity再次带偏。bounded remediation已加入current-pose probe -> would-wake
  latest-velocity整行重采样、finite/atomic commit与专用exact lock。
- Medium 1：错误把`SharedShape::rect(width,height)`参数称为half extents，并把旧fixture误判为
  切触。bounded remediation已改为full dimensions、derived half extents/ranges/`0.125`
  penetration，并把旧证据修正为`1.25` gap + `contact_frames=0`。
- Medium 2：downstream仍引用S5-SOLVER commit且S5-C required-term gate缺new nodes/ADR。
  bounded remediation已统一路由到S5-SOLVER-2并扩充可执行closeout `rg`。
- Medium 3：receipt/历史状态仍有PENDING/下一步旧S5-SOLVER漂移。bounded remediation已补
  explicit Start HEAD、把旧RED receipt标为historical snapshot、永久STOPPED commit写成
  `NOT COMMITTED`并清理陈旧next-step文本。
- Low 1：缺少首轮review失败历史。bounded remediation已在顶部、§16与本节显式保留。
- Final reviewer：`0 High / 0 Medium / 1 Low`、裁决`PASS`；唯一Low指出S5-SOLVER-2 handoff
  mapping漏掉A10e，`A05-A10d`已由bounded修复精确改为`A05-A10e`，该Low已闭合。
- Independent verifier：首轮因stale reviewer status裁决`FAIL`；status-only remediation完成后，
  re-verifier=`S5-REPLAN RE-VERIFIER PASS`。

当前声明final reviewer PASS、唯一Low已闭合且re-verifier=`S5-REPLAN RE-VERIFIER PASS`；
supervisor commit=`d987d1f7a85964f8c26121bc70b2a96b7f406f6e`。

## 9B. S5-BEHAVIOR-RED-2：post-contact acceptance-as-code

Immutable Start HEAD=`d987d1f7a85964f8c26121bc70b2a96b7f406f6e`，已由supervisor在
S5-REPLAN commit后写入§16。Ownership exact 4：

```text
crates/picea/tests/physics_realism_acceptance.rs
crates/picea/tests/world_step_review_regressions.rs
crates/picea/tests/verify_revolute_scope.rb
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
```

必须保留§8原13条integration tests、测试名、断言与全窗`<=0.03`/final`<=0.01`阈值。
在不写production的前提下只做以下superseding locks：

1. projection enabled/disabled contact fixture固定为解析正穿透。`SharedShape::rect(width,height)`
   参数是full dimensions：floor constructor args=`(8.0,0.5)`、derived half extents=
   `(4.0,0.25)`、center `y=1.125`、y range=`[0.875,1.375]`、local anchor=
   `(0,-2.125)`；pendulum constructor args=`(1.0,2.0)`、derived half extents=
   `(0.5,1.0)`、center `y=0`、y range=`[-1.0,1.0]`、anchor=`(0,-1.0)`。两端initial
   pivot均为`y=-1.0`，解析y penetration=`1.0-0.875=0.125 > 1e-4`。两条test在baseline
   必须先证明initial pivot一致、首帧`contact_count > 0`、`contact_row_count > 0`、normal
   impulse `> 0`、所有facts finite、numeric warning `0`，并记录explicit-skip
   `joint_row_count == 0`。随后以`joint_row_count == 1`命中批准的
   `S5_REVOLUTE_SOLVER_ASSERT:positive_penetration_joint_row` expected RED；只有SOLVER-2
   GREEN后才继续检查原drift阈值。不得用`contact_frames > 0`掩盖gap/non-contact或跳过row锁。
2. 新增exact `revolute_joint_post_contact_reconciliation_uses_latest_velocity`：fixture必须先
   证明contact/projection实际修改linear/angular velocity，再用独立pose oracle锁定post pass
   按latest velocity构造eval pose、撤回同一latest sampled advance，final integration无double
   advance；CCD-clamped awake dynamic只预测latest angle、不预测translation。
3. 新增exact
   `revolute_joint_post_contact_reconciliation_would_wake_resamples_latest_velocity`：mandatory
   correction必须为0，contact/projection后首次产生post correction；sleeping endpoint保留
   nonzero linear/angular velocity。Test锁定current-pose probe产生finite nonzero
   would-wake demand后，整行按latest stored velocity重采样；最终以`JointCorrection` wake、
   final drift `<=0.01`且final pose与latest velocity一致，zero/singular controls不wake。
4. 新增expected boundary GREEN exact
   `post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows`：锁定Distance
   position/stiffness与WorldAnchor correction/damping不会被第二次通用solve；该test不得依赖
   Revolute production已实现。
5. scope script按§4.1加入三个new nodes/scopes/self-tests；`PRE_V_NODES`包含new chain并排除
   historical STOPPED `S5-SOLVER`；SHA/changed-path/cached/milestone-base semantics保持不变。

RED gate：先完整逐条复跑§8的unit与原13条integration exact命令，再运行：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-BEHAVIOR-RED-2)"
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_post_contact_reconciliation_uses_latest_velocity -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_post_contact_reconciliation_would_wake_resamples_latest_velocity -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows -- --exact --nocapture
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb self-test
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-BEHAVIOR-RED-2 "$S5_NODE_START_HEAD"
rtk proxy git diff --check
rtk proxy git diff --exit-code "$S5_NODE_START_HEAD" -- crates/picea/src crates/picea-lab Cargo.toml Cargo.lock
```

二元分类：

- §8中原13条在已提交S5-BEHAVIOR-RED的**历史分类**保持10条expected RED / 3条boundary
  GREEN。本RED-2不伪称沿用相同failure signature：两条projection fixture重写后单独分类为
  positive-contact preconditions GREEN + baseline `joint_row_count==0` observation +
  `positive_penetration_joint_row` expected RED。原13条在RED-2仍各自`running 1 test`，总分类
  仍为10 RED / 3 GREEN，但上述两条RED的失败点已明确rebase到row assertion。
- latest-velocity exact必须`running 1 test`并exit nonzero，只接受最终
  `S5_REVOLUTE_REPLAN_ASSERT:latest_velocity_post_contact`；velocity mutation、oracle identity、
  finite与harness前置失败均不算有效RED。
- would-wake resample exact必须`running 1 test`并exit nonzero，只接受最终
  `S5_REVOLUTE_REPLAN_ASSERT:would_wake_resamples_latest_velocity`；mandatory-zero、首次post
  demand、nonzero stored velocities、zero/singular no-wake controls任一失败都是harness/
  contract failure。
- existing-joint no-repeat exact必须`running 1 test`、exit `0`并打印Distance/WorldAnchor两类
  observation；zero-test或因Revolute缺失而跳过不算GREEN。
- scope self-test、exact-4、diff-check与production/lab/Cargo zero-diff必须全部GREEN。

因此RED-2 integration exact总计16条：12条expected RED / 4条boundary GREEN；unit compile
contract仍按§8单独分类，不计入16条integration数量。任何zero-test、compile/path error、
非批准failure signature或前置oracle失败都属于harness failure。

Reviewer必须逐行确认原阈值/测试名未弱化、历史分类与RED-2新分类清楚分层、正穿透解析
几何无误、sleeping两阶段oracle完整、new exact不是snapshot-only，并审查scope script只有
批准state transition。Verifier逐条报告16条integration exact的
exit、`running 1 test`、failure signature/metrics、contact/impulse/row与scope actual；不得只跑
aggregate filter。

### S5-BEHAVIOR-RED-2 worker验收报告（2026-07-15）

**Start HEAD**：`d987d1f7a85964f8c26121bc70b2a96b7f406f6e`。

**成功标准**：exact-4内完成scope state transition与三条新增behavior locks；unit保持仅8个
批准`E0425`，16条integration exact逐条真实运行并严格得到12 RED / 4 GREEN，production、
design、lab与Cargo均zero-diff。

**检查结果**：

- Scope script只增加`S5-REPLAN`、`S5-BEHAVIOR-RED-2`、`S5-SOLVER-2`的数据、scope与
  self-tests；historical `S5-SOLVER`仍可解析但已从`PRE_V_NODES`排除。`self-test` exit `0`，
  current S5-REPLAN/RED-2 receipt、SOLVER-2 PENDING拒绝、mismatch/missing/unexpected/optional
  均PASS。`receipt-head S5-BEHAVIOR-RED-2` exit `0`并输出上述Start HEAD；
  `receipt-head S5-SOLVER-2`按合同exit `1`并输出`empty or PENDING SHA raw=PENDING`。
- Unit命令
  `rtk proxy cargo test -p picea --lib pipeline::joints::tests::revolute_point_constraint_ -- --nocapture`
  exit `101`：仅8个`E0425`，仍只指向四个批准helper
  `invert_revolute_effective_mass`、`revolute_position_deltas`、
  `rebuild_revolute_current_pose_from_world_com`、`apply_revolute_pose_pair_atomically`。
- 下表中前15条均使用
  `rtk proxy cargo test -p picea --test physics_realism_acceptance <test> -- --exact --nocapture`；
  第16条使用同形命令但test target为`world_step_review_regressions`。每条均输出
  `running 1 test`：

| # | Exact test | Exit / 分类 | 关键事实与唯一失败点 |
| --- | --- | --- | --- |
| 1 | `revolute_joint_two_dynamic_preserves_anchor_coincidence` | `101` / RED | max/final=`8.000021/8.000021`、warning=`0`；`two_dynamic_max_drift` |
| 2 | `revolute_joint_leaves_relative_rotation_free` | `0` / GREEN | inverse mass=`2/2`、inverse inertia=`19.200001/19.200001`、`det(K)=16`、relative change=`1.999999` |
| 3 | `revolute_joint_static_dynamic_preserves_static_pose` | `101` / RED | static bit-exact；drift=`1.999998`；`static_dynamic_pivot_drift` |
| 4 | `revolute_joint_off_center_anchor_uses_rotational_inertia` | `101` / RED | rotation=`0`；`off_center_requires_rotational_inertia` |
| 5 | `revolute_joint_is_deterministic_and_finite` | `0` / GREEN | 两次120帧hash/row一致、finite、warning=`0` |
| 6 | `revolute_joint_mixed_contact_island_keeps_separate_logical_rows` | `101` / RED | contact/row=`1/1`先GREEN，joint row=`0`；`mixed_island_requires_one_logical_joint_row` |
| 7 | `revolute_joint_connected_bodies_still_contact` | `0` / GREEN | pair contact=`1`、contact row=`1`、baseline joint row=`0` |
| 8 | `revolute_joint_nonzero_local_center_of_mass_preserves_pivot` | `101` / RED | COM=`(0.75,0.25)`、lever=`0.5`、max/final=`2.549487/0.359781`；`nonzero_com_max_pivot_drift` |
| 9 | `revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle` | `101` / RED | clamp/contact=`1/1`、latest omega=`2.054233`、angle oracle一致、drift=`0.052117`；`ccd_final_angle_pivot_drift` |
| 10 | `revolute_joint_awake_sleeping_pair_uses_current_pose_and_wakes_on_correction` | `101` / RED | zero/singular/inactive均不wake；positive缺`JointCorrection`；`finite_nonzero_correction_must_wake_with_joint_correction` |
| 11 | `revolute_joint_contact_full_step_preserves_pivot_with_projection_enabled` | `101` / RED | penetration=`0.125`；首帧contact/rows=`2/2`、normal impulse=`0.465877`、warning=`0`、baseline joint row=`0`；`positive_penetration_joint_row` |
| 12 | `revolute_joint_contact_full_step_preserves_pivot_with_projection_disabled` | `101` / RED | 与enabled同样首帧facts；`positive_penetration_joint_row` |
| 13 | `revolute_joint_ccd_clamped_contact_full_step_preserves_pivot` | `101` / RED | clamp/contact=`1/1`、normal impulse=`1.570796`、latest angle oracle一致、drift=`0.024294`；`ccd_contact_full_step_pivot_drift` |
| 14 | `revolute_joint_post_contact_reconciliation_uses_latest_velocity` | `101` / RED | 同一Revolute CCD/contact fixture在projection disabled/enabled两配置均有normal impulse=`1.570796`并把omega `6 -> 2.054233`；baseline point speed=`0.422997/0.422997`、projection velocity delta=`0`、joint rows=`0/0`、post demand=`0.013150/0.013150`、oracle drift=`0.000406/0.000406`、actual=`0.024294/0.024294`；仅`S5_REVOLUTE_REPLAN_ASSERT:latest_velocity_post_contact` |
| 15 | `revolute_joint_post_contact_reconciliation_would_wake_resamples_latest_velocity` | `101` / RED | mandatory=`0`、probe demand=`0.013150`、recompute demand=`0.018604`、stored/post velocity均为`(0.3,-0.2)/0.7`且bit-exact preserved、oracle drift=`0.000771`、actual=`0.013150`、wake=`false`；recompute-failure control另证probe=`0.013150`、latest eval nonfinite、sleeper pose/velocity exact preserved、无`JointCorrection`、末尾仅sleep refresh `Unknown`、spinner pose error=`0`、numeric warning=`0`；仅`S5_REVOLUTE_REPLAN_ASSERT:would_wake_resamples_latest_velocity` |
| 16 | `post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows` | `0` / GREEN | Distance actual/single/repeated=`0.1/0.1/0.196667`；WorldAnchor correction=`0.058056/0.058056/0.102921`、damped velocity=`0.5/0.5/0.25`；各row=`1` |

- `rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-BEHAVIOR-RED-2 "$S5_NODE_START_HEAD"`
  exit `0`，actual严格为批准的4条路径；`rtk proxy git diff --check`、
  `rtk proxy cargo fmt --all --check`与Start HEAD到当前的production/design/lab/Cargo zero-diff均
  exit `0`。
- API/lifecycle回归：`rtk proxy cargo test -p picea --test core_model_world revolute_joint_ -- --nocapture`
  exit `0`、`6 passed`；正确row-zero exact输出`running 1 test`、
  `joint_count=1;joint_row_count=0`；
  `rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_ -- --nocapture`
  exit `0`、`6 passed`；同test target全量复跑exit `0`、`19 passed`。
- Harness remediation history：latest exact首次编译因两个untyped float `.abs()`触发`E0689`，
  明确标注`f32`后才得到批准RED；no-repeat首次编译因测试文件未导入`Point`触发`E0433`，改用
  已有`Into<Point>`写法后GREEN；首次fmt check exit `1`只报告本轮测试格式diff，手工按rustfmt
  输出修正后复跑exit `0`。一次额外row-zero命令误写不存在的filter而`running 0 tests`，不计入
  验收；随后使用正确`revolute_joint_api_checkpoint_has_no_solver_row` exact复跑为
  `running 1 test`、exit `0`。这些早期结果均不作为RED/GREEN授权。

### S5-BEHAVIOR-RED-2 test/spec review history

- 独立reviewer首轮=`1 High / 1 Medium / 0 Low`、裁决`FAIL`。High指出旧latest-velocity
  projection observation借用了Distance row，不能证明Revolute projection；Medium指出
  would-wake缺少latest-velocity recompute失败的atomic no-apply/no-wake control，且成功路径没有
  显式锁住sleeper stored velocity保存。
- Bounded remediation将latest exact改为同一个Revolute CCD/contact fixture的
  projection disabled/enabled对照。两次运行都先证明真实CCD clamp、positive finite contact
  impulse、velocity mutation、finite state与warning 0；最终批准signature同时要求两边各一个
  logical Revolute row、disabled point speed `>0.05`、enabled point speed `<=1e-4`、两次velocity
  差异`>1e-4`、final drift `<=0.01`及pose oracle error `<=1e-4`。当前baseline仅因row=`0/0`、
  point speed相同且velocity delta=`0`命中原批准最终signature，没有新增harness failure。
- Would-wake成功路径现将post-step sleeper linear/angular velocity与输入
  `(0.3,-0.2)/0.7`做bit-exact比较，并合入原批准最终signature。新增finite-extreme control使用
  sleeping endpoint `pose.angle=f32::MAX`、stored `angular_velocity=f32::MAX`与circle mass；mandatory
  correction为0，contact后current-pose probe finite/nonzero，但latest advance产生nonfinite angle，
  因而recompute必须fail closed。full-step观测证明sleeper pose/velocity exact preserved且没有
  `JointCorrection`；末尾`Unknown`明确来自post pass之后的既有sleep refresh，不记为pass wake。
- Bounded remediation复跑中，latest与would-wake exact均为`running 1 test`、exit `101`且只命中
  各自批准最终signature；两条positive-penetration exact均为`running 1 test`、exit `101`且只命中
  `positive_penetration_joint_row`；no-repeat为`running 1 test`、exit `0`。Unit仍只报8个批准
  `E0425`；API=`6 passed`、lifecycle=`6 passed`、world regression=`19 passed`，正确row-zero exact
  为`running 1 test`并输出`joint_count=1;joint_row_count=0`。scope self-test、exact-4、fmt、
  diff-check、production/design/lab/Cargo zero-diff均exit `0`，dirty仍严格exact-4。
- Remediation复跑曾把row-zero filter误投到`physics_realism_acceptance`而得到一次
  `running 0 tests`；该结果不计入验收，随后立即改用正确`core_model_world` target并得到上述
  `running 1 test` GREEN。该harness噪声未修改文件或合同。
- 第二轮reviewer确认首轮High/Medium主体闭合，但给出`0 High / 1 Medium / 0 Low`、裁决
  `FAIL`：atomic recompute-failure control没有打印并断言`numeric_warnings == 0`，未把frozen
  design §9.3/§9.7及本文§9C的“recompute fail-closed不新增warning”写成可执行合同。
- 第二轮bounded remediation只在该control的boundary fact增加`numeric_warnings`，并在最终
  harness gate要求其等于0；would-wake exact复跑仍为`running 1 test`、exit `101`，control输出
  `numeric_warnings=0`并自身GREEN，最终仍只命中批准的
  `S5_REVOLUTE_REPLAN_ASSERT:would_wake_resamples_latest_velocity`。
- Final reviewer确认第二轮Medium闭合，结论=`0 High / 0 Medium / 0 Low`、裁决`PASS`；前两轮
  `FAIL`及两次bounded remediation历史继续保留，不以final PASS覆盖。
- Independent verifier正式给出`S5-BEHAVIOR-RED-2 VERIFIER PASS`：unit仅8个批准`E0425`且
  只指向四个planned helper；16条integration exact逐条均`running 1 test`并严格分类为
  `12 RED / 4 GREEN`；API=`6 passed`、row-zero exact、lifecycle=`6 passed`、world regression=
  `19 passed`、scope self-test/exact-4、fmt、diff、production/design/lab/Cargo zero-diff、cached
  与untracked hygiene均PASS。Chrome保持`NOT RUN / BROWSER PENDING`。
- 本段当时的next-step状态已被§16后续事实supersede：RED-2已commit，scope state-aware follow-up
  后full HEAD=`07b2bd6cd3ff4b80d1b6c124a4752e36e8e8d88f`；S5-SOLVER-2已执行并因
  test-contract conflict STOPPED，当前转入RED-3。

**复跑方式**：先按§8代码块逐条运行unit与原13条integration；再按§9B RED gate顺序运行
三条新增exact、scope self-test、receipt-head/exact-4、diff-check和zero-diff gate；API/lifecycle
使用上方三条命令复跑。不得用aggregate filter替代16条exact分类。

**范围外**：未修改production、design、lab、Web、Cargo或已冻结API/lifecycle合同；未实现
Revolute row、post-contact pass、warm-start、motor、limits、damping或phase reorder；未stage、
commit、push、merge或运行Chrome。

**残余风险（historical receipt）**：RED-2当时的12条expected RED不能单独证明solver能力；
后续S5-SOLVER-2已证明new latest/would-wake方向，但暴露两条旧absolute-angle oracle冲突并
STOPPED。当前由§9D RED-3 superseding tests与§9E SOLVER-3重新闭合；Chrome保持
`NOT RUN / BROWSER PENDING`。

## 9C. S5-SOLVER-2：2x2 + latest-velocity reconciliation

Historical immutable Start HEAD=`07b2bd6cd3ff4b80d1b6c124a4752e36e8e8d88f`。本node已冻结为
`STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`；以下implementation合同与GREEN gates保留给
§9E S5-SOLVER-3继承，不授权继续写S5-SOLVER-2。原required ownership exact 6：

```text
crates/picea/src/pipeline.rs
crates/picea/src/pipeline/step.rs
crates/picea/src/pipeline/island.rs
crates/picea/src/pipeline/joints.rs
crates/picea/src/solver/body_state.rs
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
```

Optional仅`crates/picea/src/pipeline/joints/tests.rs`的unit增强，且production worker禁止修改；
只有reviewer发现unit coverage缺口、supervisor另派bounded test-only worker时才允许进入scope。
不得修改committed integration/lifecycle tests或scope script。

精确phase顺序：

```text
velocity integration -> CCD -> mandatory joint position -> contact
-> optional joint velocity projection -> Revolute-only post-contact reconciliation
-> final position integration -> finalize/sleep
```

这是用户批准的局部插入，不是既有phase reorder。实现必须满足：

- 一个`JointSolverRow::Revolute`、一个logical row；mandatory plan只计数一次，post pass不创建
  row、不返回/累计stats，`joint_row_count == 1`。
- mandatory position与optional velocity projection仍使用同一对称2x2 `K`、COM-to-origin、
  finite/fail-closed、atomic apply与wake helpers；禁止两个串行scalar corrections。
- post pass只匹配existing Revolute row；不得再次应用Distance position/stiffness或
  WorldAnchor correction/damping。
- post pass第一阶段只读probe：static/sleeping dynamic=current；awake dynamic=
  current+latest stored velocity*dt；CCD-clamped awake dynamic=current translation + predicted
  latest angle；kinematic可预测但不接收correction。Probe只判断current-pose row是否对
  sleeping endpoint存在finite nonzero correction demand；inactive/zero/singular/non-finite
  probe不apply、不wake。
- 若probe没有would-wake sleeper，finite/atomic probe row可直接commit。若任一sleeping
  endpoint would-wake，则任何write/wake前必须重构整行：该sleeper按wake后final integration
  会消费的latest stored linear/angular velocity预测；CCD-clamped sleeper仍current
  translation + latest predicted angle；其他endpoint保持latest规则。重新计算geometry/K/
  lambda/A+B deltas并finite gate；recompute失败不apply、不wake。
- 最终采用的eval row以COM/angle反推origin，写回current时减去**该row本次latest sampled
  advance**，final integration加回同一advance。不得减mandatory旧advance、直接写eval pose、
  冻结旧velocity、skip/freeze final integration或修改`integrate.rs`语义。
- Would-wake sleeper以`JointCorrection` wake，资格来自first probe的finite nonzero demand；
  即使recomputed pose delta因stored velocity恰好闭合误差而为zero也必须wake，不归类为
  zero-row synthetic wake。Probe zero/singular/non-finite或recompute失败仍不wake；最终A/B
  writes与wake必须atomic，不新增warning。
- `pipeline.rs`只允许public doc/config lock，不增加字段、不改default/serde；不做warm-start
  cache、motor、limits、damping、contact/CCD修改或stream merge。

Targeted GREEN必须包含§9原代码块的unit、原13条integration exact、两个StepConfig、island、
stack与world regression全部功能门；其中legacy `receipt-head S5-SOLVER`和legacy node scope
两行由下方`S5-SOLVER-2`等价命令取代，不能反过来读取STOPPED node receipt。随后追加：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-SOLVER-2)"
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_post_contact_reconciliation_uses_latest_velocity -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_post_contact_reconciliation_would_wake_resamples_latest_velocity -- --exact --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows -- --exact --nocapture
rtk proxy cargo test -p picea --test core_model_world revolute_joint_ -- --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_ -- --nocapture
rtk proxy cargo fmt --all --check
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb self-test
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-SOLVER-2 "$S5_NODE_START_HEAD"
rtk proxy git diff --check
rtk proxy git diff --exit-code "$S5_NODE_START_HEAD" -- crates/picea/tests crates/picea-lab Cargo.toml Cargo.lock
```

原13条与三条新增exact都必须`running 1 test`且GREEN；projection enabled/disabled必须首帧
报告解析penetration=`0.125`、contact/contact-row/normal-impulse正值和joint row=`1`，并保留
原drift阈值；would-wake exact必须报告mandatory-zero、首次post demand、nonzero stored
linear/angular velocity、probe/recompute pose、`JointCorrection`与zero/singular controls。
Reviewer findings-first逐式审查K/sign/COM/latest-advance/atomic/wake、one-row/no-stats/no-repeat、
sleeping两阶段重采样与phase/stream/integrate边界。Independent verifier必须从头报告drift、
relative angle、contact/penetration/normal impulse、nonzero COM、CCD clamp/latest angle、row counts、
wake reason、deterministic hash、numeric warnings，以及scope required/optional actual；只写
`passed`不构成receipt。

## 9D. S5-BEHAVIOR-RED-3：superseding full-pose oracle

Immutable Start HEAD=`07b2bd6cd3ff4b80d1b6c124a4752e36e8e8d88f`。Ownership exact 3：

```text
crates/picea/tests/physics_realism_acceptance.rs
crates/picea/tests/verify_revolute_scope.rb
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
```

本node只修复以下两条stale oracle，保留测试名、fixture、CCD/contact identity、positive normal
impulse、contact对velocity的真实mutation、numeric warning、一个logical row、`drift <= 0.01`
及完整pose精度`<=1e-4`：

1. `revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle`
2. `revolute_joint_ccd_clamped_contact_full_step_preserves_pivot`

两条test都必须使用test-side `s5_revolute_pose_oracle`的symmetric 2x2/COM-to-origin实现：先从
initial mandatory eval pose得到mandatory corrected eval；CCD-clamped endpoint撤回initial sampled
angle、撤回translation advance `0`，重建mandatory authoritative current pose。随后读取被测step
后的latest linear/angular velocity；post eval明确记录latest linear velocity但CCD-clamped sampled
translation advance仍为`0`，angle advance=`latest omega * dt`。对post eval再次求2x2 corrected
eval；post write撤回、final integration加回同一latest advance，因此expected final完整pose严格
等于corrected post eval。

Oracle必须构造并打印两个negative candidates：`old_no_post_candidate=post eval`与
`double_advance_candidate=corrected post eval + latest advance`；二者相对expected full pose error
都必须`>1e-4`。Post demand必须finite/nonzero、expected drift必须`<=0.01`。这些是test oracle
自证；被测能力只在末尾二元signature统一要求`joint_row_count == 1`、`numeric_warnings == 0`、
actual drift `<=0.01`与actual full pose error `<=1e-4`，防止clean baseline先在row-zero或旧angle
harness断言失败。批准的唯一RED signatures为：

```text
S5_REVOLUTE_REPLAN_ASSERT:ccd_post_contact_final_pose
S5_REVOLUTE_REPLAN_ASSERT:ccd_contact_post_contact_final_pose
```

不得修改new latest/would-wake tests、其他integration tests、production、design、lab/Web、Cargo
或阈值。RED gate为§8的unit与原13条integration exact逐条运行，再逐条运行§9B新增latest、
would-wake与no-repeat exact；总计16条integration，每条必须`running 1 test`。Clean
`07b2bd6…` baseline的期望分类固定为`12 RED / 4 GREEN`：本节两条只能各自命中上述final-pose
signature；new latest/would-wake保持原批准signature，positive/no-repeat及其余分类保持不变。

Scope与hygiene gate：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-BEHAVIOR-RED-3)"
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb self-test
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-BEHAVIOR-RED-3 "$S5_NODE_START_HEAD"
rtk proxy cargo fmt --all --check
rtk proxy git diff --check
rtk proxy git diff --exit-code "$S5_NODE_START_HEAD" -- crates/picea/src crates/picea/tests/world_step_review_regressions.rs crates/picea/tests/core_model_world.rs crates/picea-lab docs/design Cargo.toml Cargo.lock
rtk proxy git diff --cached --exit-code
rtk proxy git ls-files --others --exclude-standard
```

### S5-BEHAVIOR-RED-3 worker验收报告（2026-07-15）

**Start HEAD**：`07b2bd6cd3ff4b80d1b6c124a4752e36e8e8d88f`。

**成功标准**：exact-3内将两条stale absolute-angle oracle替换为可区分no-post/double-advance的
full-pose oracle；clean baseline的unit只保留8个批准`E0425`，16条integration逐条
`running 1 test`并严格得到`12 RED / 4 GREEN`，两条superseding exact只落在各自批准最终
signature；scope/self-test/fmt/diff/zero-diff/cached/untracked全部闭合。

**检查结果**：

- Unit command exit `101`且只包含8个批准的`E0425`，逐项指向
  `invert_revolute_effective_mass`、`revolute_position_deltas`、
  `rebuild_revolute_current_pose_from_world_com`、`apply_revolute_pose_pair_atomically`四个helper；
  无其他compiler diagnostic。
- 16条integration exact全部真实输出`running 1 test`。12条RED为two-dynamic、static/dynamic、
  off-center、mixed-row、nonzero-COM、两条本节superseding CCD exact、awake/sleeping、projection
  enabled、projection disabled、new latest、would-wake；4条GREEN为free-rotation、
  determinism/finite、connected-contact、no-repeat existing rows。每条exit/signature保持§8/§9B
  批准分类，总计严格`12 RED / 4 GREEN`，没有zero-test。
- `ccd_post_contact_final_pose` clean baseline：clamp/contact=`1/1`、normal impulse=`1.570796`、
  latest linear=`(0,0.100657366)`、latest omega=`2.054233`、post demand=`0.013150`、expected/
  actual drift=`0.000406/0.052117`、full pose error=`0.113321`、old-no-post/double-advance error=
  `0.064312/0.034237`、row/warning=`0/0`。全部oracle/identity/impulse/velocity mutation前置与两个
  negative candidates先GREEN，最终只命中
  `S5_REVOLUTE_REPLAN_ASSERT:ccd_post_contact_final_pose`。
- `ccd_contact_post_contact_final_pose` clean baseline：clamp/contact=`1/1`、normal impulse=
  `1.570796`、latest linear=`(0,0.100657366)`、latest omega=`2.054233`、post demand=`0.013150`、
  expected/actual drift=`0.000406/0.024294`、full pose error=`0.083323`、old-no-post/
  double-advance error=`0.064312/0.034237`、row/warning=`0/0`；最终只命中
  `S5_REVOLUTE_REPLAN_ASSERT:ccd_contact_post_contact_final_pose`。
- New latest/would-wake保持原signature：latest clean baseline rows=`0/0`、post demand=
  `0.013150/0.013150`、pose error=`0.083323/0.083323`；would-wake mandatory/probe/recompute demand=
  `0/0.013150/0.018604`、stored velocity=`(0.3,-0.2)/0.7` bit-exact preserved、recompute-failure
  atomic control warning=`0`，最终分别只命中原批准
  `latest_velocity_post_contact`/`would_wake_resamples_latest_velocity`。Positive penetration两条仍
  先证明penetration=`0.125`、首帧contact/contact-row=`2/2`、impulse=`0.465877`、warning=`0`，
  再以row=`0`命中原批准signature；no-repeat GREEN保持Distance/WorldAnchor单次事实。
- `receipt-head`返回immutable Start HEAD；scope self-test覆盖RED-3/SOLVER-3 parser、current、
  pending/SHA、mismatch、missing/unexpected/optional且输出`S5_SCOPE_SELF_TEST_PASS`；node binary
  scope actual严格为上述exact 3、missing/unexpected均空。`cargo fmt --all --check`、
  `git diff --check`、production/design/other-tests/lab/Cargo zero-diff、cached empty、untracked empty
  全部exit `0`；dirty恰为exact 3。
- Harness history：首次批量验证的JS输出筛选regex自身语法错误，命令在启动前即终止，未运行
  测试、未改文件，不计RED/GREEN；改为无regex的逐行筛选后完整复跑unit+16。两次fmt check
  只报告本轮Rust测试共5处格式diff，按rustfmt输出修正并复跑PASS；两条exact初跑还因旧oracle
  facts暂时不再读取产生一次dead-code warning，改为显式打印legacy lever/radial/angular facts后
  完整16条复跑无warning。这些均不冒充产品RED。

**复跑方式**：checkout本node最终commit，从`receipt-head S5-BEHAVIOR-RED-3`读取Start HEAD，
按§8/§9B逐条运行unit与16条integration exact，再运行本节scope/hygiene代码块。

**范围外**：production、design、其他tests、lab/Web、Cargo、Chrome；本node不stage、commit、
push或运行Chrome。

**残余风险**：test/spec reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier
首轮因§16 stale review状态以`0 High / 1 Medium / 0 Low`裁决`FAIL`的历史保留。Status-only
remediation后independent re-verifier=`0 High / 0 Medium / 0 Low`，结论
`S5-BEHAVIOR-RED-3 RE-VERIFIER PASS`，并确认physics/scope blobs未变化及全部receipt/scope/
hygiene门PASS。当时剩余风险只在下一节点：只有S5-SOLVER-3可证明accepted production对两条
superseding full-pose oracle GREEN；后续§9E已证实该node因contact-state oracle conflict STOPPED。
Chrome保持`NOT RUN / BROWSER PENDING`。

### S5-BEHAVIOR-RED-3 test/spec review history

- Independent test/spec reviewer：`0 High / 0 Medium / 0 Low`、裁决`PASS`；确认两条full-pose
  oracle、no-post/double-advance negative candidates、一个logical row、warning/drift/pose阈值、
  scope state transition与exact-3 ownership均符合§9D合同。
- Independent verifier首轮：`0 High / 1 Medium / 0 Low`、裁决`FAIL`。唯一Medium是§16
  S5-BEHAVIOR-RED-3行仍写“test/spec review待派”，与已经完成的reviewer PASS事实冲突；其余
  behavior/oracle、unit仅8个批准`E0425`、16条integration逐条`running 1 test`且严格
  `12 RED / 4 GREEN`、scope self-test/exact-3、fmt/diff与全部hygiene门均PASS。
- Status-only remediation：只同步顶部、本节review history、§16 RED-3状态与残余风险；tests、
  scope script、metrics、阈值、production/design/lab/Cargo全部zero-diff；未提前推进
  S5-SOLVER-3。
- Independent re-verifier：`0 High / 0 Medium / 0 Low`，结论
  `S5-BEHAVIOR-RED-3 RE-VERIFIER PASS`。确认physics/scope blobs相对首轮verifier未变化；
  `receipt-head`、Ruby syntax、scope self-test、RED-3 exact-3、`git diff --check`、范围外zero-diff、
  cached/untracked hygiene全部PASS。首轮`0 High / 1 Medium / 0 Low` FAIL历史继续保留。

## 9E. S5-SOLVER-3：消费RED-3 superseding tests

Immutable Start HEAD=`6ffa1e3f29e902b68708b62e11d5fae160db97ec`。本node已冻结为
`STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`；以下继承合同只作历史证据，不授权续写。
本node完整继承
§9C的physics implementation、required exact 6、optional unit-only path、phase顺序、2x2/COM/
latest-advance/atomic/wake/one-row/no-stats/no-repeat合同、GREEN gates与stop conditions。唯一变化是
消费§9D已经提交的两条superseding full-pose tests；S5-SOLVER-3 worker不得修改任何tests或
scope script。

§9C命令中的`receipt-head S5-SOLVER-2`与node scope调用替换为`S5-SOLVER-3`；其余unit、16条
integration exact、StepConfig/island/stack/world regression、fmt/diff/zero-diff gate逐项不变。
若accepted production仍不能让两条superseding full-pose oracle与new latest/would-wake同时
GREEN，或需要修改contact solver/CCD算法、existing joint kinds、row/stats/schema/API、冻结旧
velocity、重排既有phase/stream，立即STOP并回填证据，不得修改tests。目标commit message为
`feat: reconcile revolute constraints after contacts`。

### S5-SOLVER-3 worker停机验收报告（2026-07-15）

**Start HEAD**：`6ffa1e3f29e902b68708b62e11d5fae160db97ec`。

**成功标准**：required exact 6 production使unit 8条、16条integration exact、§9C broad gates、
scope与hygiene全部GREEN，同时保持contact后current、latest-advance、one-row/no-stats/no-repeat。

**检查结果**：

- Unit exit `0`，真实输出`running 8 tests`、`8 passed`。16条integration exact全部逐条真实输出
  `running 1 test`，结果=`15 GREEN / 1 FAIL`；two-dynamic/static-dynamic/off-center/free-angle/
  nonzero-COM、determinism、mixed rows、awake/sleeping、positive penetration、latest、would-wake、
  no-repeat均GREEN。
- 唯一FAIL为`revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle`，只命中
  `S5_REVOLUTE_REPLAN_ASSERT:ccd_post_contact_final_pose`：full-pose error=`0.011336`；同时
  clamp/contact=`1/1`、normal impulse=`1.570796`、row/warning=`1/0`、expected/actual drift=
  `0.000915/0.001104`均满足。另一条CCD full-pose error=`0`。
- 诊断与review确认失败fixture的contact residual correction translation=`0.00194835861`、depth=
  `0.0487089641`、position-correction input contact/body count=`1/1`；GREEN对照residual=`0`。
  Mandatory后spinner current x=`-0.094008304`，post pass前x=`-0.09595666`。RED-3 oracle遗漏
  这次authoritative pose mutation，而production按ADR-S5-5重新读取contact后的current。
- Independent reviewer=`1 High / 1 Medium / 0 Low`、裁决`CONFLICT CONFIRMED`；production不是
  root cause。`cargo fmt --all --check`另因dirty `pipeline/joints.rs` rustfmt diff exit `1`，属于
  独立hygiene failure。Scope actual仍为required exact 6，tests/lab/Cargo zero-diff、cached/
  untracked empty；未进入后续broad gates或S5-LAB。

**复跑方式**：复用主worktree现有production patch，逐条运行§9/§9B的unit与16条exact；最小
复现为`rtk proxy cargo test -p picea --test physics_realism_acceptance revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle -- --exact --nocapture`。

**范围外**：committed tests/scope script、contact solver、CCD、`integrate.rs`、existing joint
kinds、API/schema/stats/default、Lab/Web/Cargo、Chrome；未stage、commit、push。

**残余风险**：S5-SOLVER-3永久保持`STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`；只有
用户批准的S5-REPLAN-2 -> RED-4 -> SOLVER-4可修正oracle并重新取证。Chrome保持
`NOT RUN / BROWSER PENDING`。

## 9F. S5-REPLAN-2：contact-state oracle docs contract

Immutable Start HEAD=`6ffa1e3f29e902b68708b62e11d5fae160db97ec`。用户已明确批准本次Plan Gate；
ADR-S5-5与design保持不变。本node ownership exact 1：

```text
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
```

本node只把§2已验证conflict、新chain、RED-4 shadow oracle、SOLVER-4 gates、scope transition、
STOP conditions与receipt写成可执行spec；不修改scope script，也不把RED-4/SOLVER-4写成已验证。

Docs gate：

```text
S5_NODE_START_HEAD=6ffa1e3f29e902b68708b62e11d5fae160db97ec
rtk proxy git diff --check
rtk proxy ruby -e 'base=ARGV.fetch(0); expected=[ARGV.fetch(1)]; tracked=IO.popen(["rtk","proxy","git","diff","--name-only",base], &:read).lines.map(&:strip).reject(&:empty?); untracked=IO.popen(["rtk","proxy","git","ls-files","--others","--exclude-standard"], &:read).lines.map(&:strip).reject(&:empty?); actual=(tracked+untracked).uniq.sort; abort("S5-REPLAN-2 scope mismatch: #{actual.inspect}") unless actual==expected; puts "S5_REPLAN_2_SCOPE_PASS=#{actual.join(",")}"' "$S5_NODE_START_HEAD" docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
rtk proxy git diff --exit-code "$S5_NODE_START_HEAD" -- crates Cargo.toml Cargo.lock docs/design
rtk proxy rg -n 'S5-REPLAN-2|S5-BEHAVIOR-RED-4|S5-SOLVER-4|WorldAnchor shadow|contact residual|P_contact|no_post|double_advance|12 RED / 4 GREEN|TEST CONTRACT CONFLICT|NOT COMMITTED|BROWSER PENDING' docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
rtk proxy ruby -e 'path=ARGV.fetch(0); fences=File.readlines(path).count { |line| line.start_with?("```") }; abort("odd Markdown fence count=#{fences}") unless fences.even?; puts "S5_REPLAN_2_FENCES_PASS=#{fences}"' docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
rtk proxy git diff --cached --exit-code
rtk proxy git ls-files --others --exclude-standard
```

### S5-REPLAN-2 docs worker验收报告（2026-07-15）

**Start HEAD**：`6ffa1e3f29e902b68708b62e11d5fae160db97ec`。

**成功标准**：exact-1内完整记录verified conflict、Fact/Inference/Unknown、用户Plan Gate、new
chain/scopes、WorldAnchor shadow oracle、metrics/gates/STOP和review chain，且所有docs/scope/
zero-diff/hygiene gates GREEN，不提前声称RED-4/SOLVER-4已验证。

**检查结果**：初始branch=`feat/vnext-s5-revolute-red4`、HEAD为本40位Start HEAD、worktree clean。
Worker修改后逐条实跑：`git diff --check` exit `0`且无输出；inline exact-path exit `0`并输出
`S5_REPLAN_2_SCOPE_PASS=docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md`；crates/
Cargo/design zero-diff exit `0`且无输出；required-term `rg` exit `0`，12个列名全部实际命中；
Markdown gate exit `0`并输出`S5_REPLAN_2_FENCES_PASS=80`；cached gate exit `0`且无输出；
untracked gate exit `0`且无输出。本node未调用尚不识别new nodes的scope script，后续仍由
independent docs verifier复跑确认。

**Independent review**：`0 High / 0 Medium / 0 Low`，裁决`PASS`、允许派verifier。Reviewer确认
exact-1与范围外zero-diff、WorldAnchor shadow及observable fields合同、new chain/PRE_V、STOP与
progress状态均无finding；该PASS只批准docs contract，不代表RED-4 shadow或SOLVER-4已经验证。

**Independent verifier**：`0 High / 0 Medium / 0 Low`，结论
`S5-REPLAN-2 VERIFIER PASS`。Verifier独立复跑§9F exact-1、zero-diff、required terms、fence与
cached/untracked gates并确认状态一致；commit仍为`PENDING`，由supervisor提交后另行回填。

**复跑方式**：在`/tmp/picea-s5-red4`按本节Docs gate逐条运行；reviewer findings-first核对
shadow可证伪性、baseline分类、唯一signature、scope transition和STOP边界，verifier独立复跑。

**范围外**：design、tests/scope script、production、lab/Web、Cargo、Chrome；未stage、commit、
push、merge。

**残余风险（S5-REPLAN-2验收时点）**：RED-4与SOLVER-4当时均为`PENDING / NOT STARTED`；
后续事实以§16为准。shadow contact equivalence、
clean baseline `12 RED / 4 GREEN`和最终16/16只能由后续committed acceptance与production gates
证明。Worker结论：`S5-REPLAN-2 WORKER COMPLETE`；reviewer与verifier均PASS，commit待supervisor。

## 9G. S5-BEHAVIOR-RED-4：WorldAnchor shadow contact-state oracle

Immutable Start HEAD=`4e8bd4616e1f43e330d75212a74c459e8e693ca7`，review/verifier均PASS；
已由supervisor提交为`29bafce59a0913be7b4d37055ab57ed5e5233116`。
Ownership exact 3：

```text
crates/picea/tests/physics_realism_acceptance.rs
crates/picea/tests/verify_revolute_scope.rb
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
```

本node统一更新以下两条CCD full-pose tests；测试名、fixture几何、CCD/contact identity、positive
impulse、warning、logical row、drift/pose阈值保持不变：

1. `revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle`
2. `revolute_joint_ccd_clamped_contact_full_step_preserves_pivot`

### 9G.1 Oracle构造

两条test必须使用同一test-side helper，按以下顺序恢复contact-post authoritative pose；禁止从
subject final pose反推或引入production test hook：

1. 先用existing independent symmetric 2x2/COM-to-origin oracle，从initial mandatory eval得到
   `mandatory_corrected_eval`，撤回mandatory sampled advance得到`mandatory_current`。
2. 构建test-side `WorldAnchor shadow` world。wall/target/spinner的创建顺序、handles对应关系、
   colliders、shape/material、初始linear/angular velocity、CCD设置必须与subject一致；不创建
   Revolute。为spinner创建WorldAnchor：`local_anchor=Point::default()`、`world_anchor=
   mandatory_current.point()`、`stiffness=1.0/DT`、`damping=0`；spinner initial angle改为
   `mandatory_current.angle()`。Shadow必须真实跑完整一个step，不得手调contact facts。
3. Shadow step后读取final pose与latest velocity。CCD-clamped sampled translation advance固定为
   `Vector::default()`，sampled angle advance=`shadow_latest_omega * DT`；从shadow final pose撤回
   这份advance，恢复contact solve/residual correction完成、final integration开始前的
   authoritative pose `P_contact`。Shadow latest linear velocity仍必须finite并打印，但不进入
   CCD-clamped translation advance。
4. Subject post eval使用同一个`P_contact`与subject latest advance：CCD-clamped translation
   advance=`0`，angle advance=`subject_latest_omega * DT`。在target current pose与该post eval上
   再跑同一test-side symmetric 2x2/COM-to-origin solve；post write撤回subject latest advance，
   final integration加回同一advance，所以`expected_final=corrected_post_eval`。

必须构造并打印：

```text
no_post = P_contact + subject_latest_advance
double_advance = expected_final + subject_latest_advance
```

`P_contact`、shadow/subject velocities、post demand、expected final、expected drift和两份negative
error都必须finite；post demand必须`>1e-4`、expected drift必须`<=0.01`，`no_post`与
`double_advance`相对expected full-pose error都必须`>1e-4`。

### 9G.2 Subject/shadow contact等价锁

Shadow不是snapshot替身。两world必须逐项比较以下可观察solver facts：

- CCD trace的`moving_body`/`static_body`/`moving_collider`/`static_collider` identity对应关系、
  `target_kind`、`swept_start`/`swept_end`、`target_swept_start`/`target_swept_end`、`toi`、
  `advancement`、`clamp`/`target_clamp`、`slop`、`toi_point`；
- `solver_initial_normal_speed`、`solver_initial_tangent_speed`、`solver_final_normal_speed`、
  `solver_final_tangent_speed`、`solver_position_bias`、`solver_restitution_bias`、
  `solver_normal_impulse`、`solver_tangent_impulse`；
- `solver_position_correction_depth`与body A/B correction translation；
- StepStats的`position_correction_input_contact_count`、`position_correction_input_max_depth`、
  `position_correction_input_total_depth`、`position_correction_body_count`、
  `position_correction_max_translation`、`position_correction_total_translation`。

Identity、`target_kind`与两个`usize` count使用exact equality。全部float先分别通过finite gate，
再显式使用以下scaled comparator，禁止`derive PartialEq`产生bit-exact float合同：

```text
abs(subject - shadow) <= 1e-6 * max(1, abs(subject), abs(shadow))
```

WorldAnchor shadow与Revolute mandatory路径代数等价，但允许因合法的运算顺序差异产生f32
roundoff；`1e-6`比既有`1e-4` pose/negative gate小两个数量级。每条CONTACT_STATE fact必须打印
`tolerance_scale`、`discrete_equivalent`、最大float field/delta/tolerance/ratio及最终
`contact_equivalent`，使容差是否遮蔽真实差异可直接审计。

不得比较final exported contact point/normal：两world在final integration后的展示几何允许因
Revolute post correction不同。Shadow自身必须`joint_row_count==1`、`numeric_warnings==0`，并
证明真实CCD/contact/positive normal impulse；recovered `P_contact`及全部shadow facts必须finite。

Clean `6ffa1e3…`没有mandatory Revolute row，subject/shadow的`contact_equivalent`预计为`false`。
因此该bool不得作为提前harness assert；它必须与subject `joint_row_count==1`、warning=`0`、
drift/pose error一起进入每条test唯一最终RED signature。Oracle recovery、shadow validity、
negative candidates或finite checks若提前失败，都属于harness failure而非有效RED。批准的新signature：

```text
S5_REVOLUTE_CONTACT_STATE_ASSERT:ccd_post_contact_final_pose
S5_REVOLUTE_CONTACT_STATE_ASSERT:ccd_contact_post_contact_final_pose
```

### 9G.3 RED与scope gates

RED worker必须先用以下inline bootstrap读取本node Start HEAD，再更新scope script；禁止用live
`git rev-parse HEAD`代替receipt：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby -e 'node=ARGV.fetch(0); path="docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md"; row=File.readlines(path).find { |line| cells=line.split("|"); cells[2]&.strip==node }; abort("missing node receipt") unless row; sha=row.split("|")[3].strip.delete("`"); abort("invalid receipt SHA") unless sha.match?(/\A[0-9a-f]{40}\z/); abort("unresolvable receipt SHA") unless system("rtk","proxy","git","cat-file","-e","#{sha}^{commit}"); print sha' S5-BEHAVIOR-RED-4)"
```

随后按§8/§9B逐条运行unit和16条integration exact，禁止aggregate filter或zero-test：

- Unit仍只能有8个planned helper `E0425`，无其他compiler diagnostic；
- 16条integration clean baseline严格保持`12 RED / 4 GREEN`；两条本节test只能各自命中上述
  新signature，latest/would-wake保持既有批准signature，free/determinism/connected/no-repeat
  四条boundary GREEN保持exit `0`；
- 两条test都必须先打印并证明shadow row=`1`、warning=`0`、真实CCD/contact/positive impulse、
  recovered `P_contact`、post demand、subject/shadow contact-equivalence facts、`no_post`/
  `double_advance` errors、actual pose error；clean baseline只允许最终row/contact-equivalent/
  pose组合assert失败。

Scope/hygiene：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-BEHAVIOR-RED-4)"
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb self-test
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-BEHAVIOR-RED-4 "$S5_NODE_START_HEAD"
rtk proxy cargo fmt --all --check
rtk proxy git diff --check
rtk proxy git diff --exit-code "$S5_NODE_START_HEAD" -- crates/picea/src crates/picea/tests/world_step_review_regressions.rs crates/picea/tests/core_model_world.rs crates/picea-lab docs/design Cargo.toml Cargo.lock
rtk proxy git diff --cached --exit-code
rtk proxy git ls-files --others --exclude-standard
```

RED-4 receipt必须逐条记录16 exact的exit/`running 1 test`/signature或metrics，以及两条shadow的
全部关键事实和scope actual。若shadow不能证明contact等价、clean RED不只命中批准signature、
需要production test开关/public config或任何production改动，立即STOP。本node目标commit=
`test: lock revolute contact-state oracle`；reviewer/verifier PASS并提交前不得进入SOLVER-4。

### S5-BEHAVIOR-RED-4 worker验收报告（2026-07-15）

**Start HEAD**：`4e8bd4616e1f43e330d75212a74c459e8e693ca7`。

**成功标准**：exact-3内让两条CCD tests统一使用真实WorldAnchor shadow恢复`P_contact`并锁定
subject/shadow contact-state等价；clean baseline的unit只保留8个批准`E0425`，16条integration
exact逐条`running 1 test`且严格`12 RED / 4 GREEN`，scope state transition与全部hygiene门PASS。

**检查结果**：

- Unit exact exit `101`，共8个diagnostic且全部为`E0425`；只指向
  `invert_revolute_effective_mass`、`revolute_position_deltas`、
  `rebuild_revolute_current_pose_from_world_com`、`apply_revolute_pose_pair_atomically`四个批准
  helper，无其他compiler diagnostic。
- 两条CCD test共用`s5_revolute_ccd_contact_state_oracle`。Shadow按wall/target/spinner与collider
  创建顺序真实建world，使用origin WorldAnchor、`stiffness=1/DT`、`damping=0`跑完整step；
  subject/shadow分别校验role-local CCD identity，再只比较§9G批准的CCD、solver与StepStats facts，
  不比较final contact point/normal。三个离散字段exact；全部30个float先通过finite gate，再按
  `abs_delta <= 1e-6 * max(1, |a|, |b|)`逐字段比较并记录最大delta/tolerance/ratio。
- 16条integration exact逐条结果如下；每条都真实输出`running 1 test`，无zero-test：

| # | Exact | Exit / 分类 | Signature / 关键事实 |
| --- | --- | --- | --- |
| 1 | `revolute_joint_two_dynamic_preserves_anchor_coincidence` | `101` / RED | `S5_REVOLUTE_SOLVER_ASSERT:two_dynamic_max_drift`；max/final=`8.000021/8.000021`，warning=`0` |
| 2 | `revolute_joint_leaves_relative_rotation_free` | `0` / GREEN | relative change=`1.999999`，K determinant=`16.0`，baseline rows均`0` |
| 3 | `revolute_joint_static_dynamic_preserves_static_pose` | `101` / RED | `S5_REVOLUTE_SOLVER_ASSERT:static_dynamic_pivot_drift`；drift=`1.999998` |
| 4 | `revolute_joint_off_center_anchor_uses_rotational_inertia` | `101` / RED | `S5_REVOLUTE_SOLVER_ASSERT:off_center_requires_rotational_inertia`；rotation=`0` |
| 5 | `revolute_joint_is_deterministic_and_finite` | `0` / GREEN | 120帧双跑hash/facts deterministic finite，baseline rows均`0` |
| 6 | `revolute_joint_mixed_contact_island_keeps_separate_logical_rows` | `101` / RED | `S5_REVOLUTE_SOLVER_ASSERT:mixed_island_requires_one_logical_joint_row`；pair/contact rows=`1/1`、joint row=`0` |
| 7 | `revolute_joint_connected_bodies_still_contact` | `0` / GREEN | pair contact/contact row=`1/1`、baseline joint row=`0` |
| 8 | `revolute_joint_nonzero_local_center_of_mass_preserves_pivot` | `101` / RED | `S5_REVOLUTE_SOLVER_ASSERT:nonzero_com_max_pivot_drift`；lever=`0.5`、max/final=`2.549487/0.359781` |
| 9 | `revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle` | `101` / RED | 只命中`S5_REVOLUTE_CONTACT_STATE_ASSERT:ccd_post_contact_final_pose` |
| 10 | `revolute_joint_awake_sleeping_pair_uses_current_pose_and_wakes_on_correction` | `101` / RED | `S5_REVOLUTE_SOLVER_ASSERT:finite_nonzero_correction_must_wake_with_joint_correction`；zero/singular/inactive controls保持sleeping |
| 11 | `revolute_joint_contact_full_step_preserves_pivot_with_projection_enabled` | `101` / RED | `S5_REVOLUTE_SOLVER_ASSERT:positive_penetration_joint_row:projection_enabled`；首帧contact/rows=`2/2`、impulse=`0.465877`、joint row=`0` |
| 12 | `revolute_joint_contact_full_step_preserves_pivot_with_projection_disabled` | `101` / RED | `S5_REVOLUTE_SOLVER_ASSERT:positive_penetration_joint_row:projection_disabled`；首帧contact/rows=`2/2`、impulse=`0.465877`、joint row=`0` |
| 13 | `revolute_joint_ccd_clamped_contact_full_step_preserves_pivot` | `101` / RED | 只命中`S5_REVOLUTE_CONTACT_STATE_ASSERT:ccd_contact_post_contact_final_pose` |
| 14 | `revolute_joint_post_contact_reconciliation_uses_latest_velocity` | `101` / RED | `S5_REVOLUTE_REPLAN_ASSERT:latest_velocity_post_contact`；latest omega=`2.054233`、post demand=`0.013150`、rows=`0/0` |
| 15 | `revolute_joint_post_contact_reconciliation_would_wake_resamples_latest_velocity` | `101` / RED | `S5_REVOLUTE_REPLAN_ASSERT:would_wake_resamples_latest_velocity`；probe/recompute demand=`0.013150/0.018604`、wake=`false` |
| 16 | `post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows` | `0` / GREEN | Distance actual/single/repeated=`0.1/0.1/0.196667`；WorldAnchor=`0.058056/0.058056/0.102921`，rows各`1` |

- CCD final-angle shadow：shadow clamp/contact/normal impulse=`1/1/1.570796`、row/warning=`1/0`；
  recovered `P_contact=Pose(-0.09595666,0.049750205,~0)`，post demand=`0.015095`；subject/
  shadow residual translation=`0/0.0019483586`、shadow correction depth=`0.048708964`，故clean
  `contact_equivalent=false`；最大float差异字段=`solver_position_correction_depth`，delta/tolerance=
  `0.048708964/0.000001000`、ratio=`48708.965`，且correction counts使
  `discrete_equivalent=false`。actual drift/pose error=`0.052117/0.122657`，no-post/double errors=
  `0.073750/0.034237`。所有前置finite/identity/CCD/contact/positive impulse/negative checks先PASS，
  只在批准的最终组合signature失败。
- CCD contact-full-step shadow：shadow clamp/contact/normal impulse=`1/1/1.570796`、row/warning=
  `1/0`；recovered `P_contact=Pose(-0.09700331,0.019900082,0)`，post demand=`0.013150`；两world
  residual correction均为`0`，且`discrete_equivalent=true`；最大float差异字段=
  `solver_initial_tangent_speed`，delta/tolerance=`0.011979997/0.000001000`、ratio=`11979.997`，
  故clean`contact_equivalent=false`。actual drift/pose error=`0.024294/0.083323`，no-post/double
  errors=`0.064312/0.034237`。同样无提前harness failure，只命中本节第二个批准signature。
- Scope script只新增REPLAN-2/RED-4/SOLVER-4的`PROGRESS_NODES`、`SCOPES`、`PRE_V_NODES`数据
  与对应self-tests；通用parser、SHA/cat-file、tracked+untracked、cached和milestone-base函数
  zero-diff。`ruby -c`=`Syntax OK`，`receipt-head`返回本40位Start HEAD，self-test=
  `S5_SCOPE_SELF_TEST_PASS`；node actual严格为physics test、scope script、living spec exact-3，
  missing/unexpected均空，三个STOPPED solver均未进入PRE_V。
- `cargo fmt --all --check`、`git diff --check`、production/其他tests/lab/design/Cargo zero-diff、
  cached empty、untracked empty全部exit `0`。

**复跑方式**：在`/tmp/picea-s5-red4`读取
`S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-BEHAVIOR-RED-4)"`，
按§8原顺序逐条运行unit与13条integration exact，再按§9B逐条运行latest、would-wake、no-repeat；
最后运行§9G.3 scope/hygiene命令。不得用aggregate filter替代逐条分类。

**范围外**：未修改production、design、其他tests、lab/Web、Cargo、阈值或测试名；未增加test
hook/public config，未stage、commit、push、merge或运行Chrome；未进入S5-SOLVER-4。

**残余风险**：clean baseline只能证明RED-4 acceptance合同有效；`contact_equivalent=false`与
subject row=`0`是预期RED，不是solver能力结论。当时只有S5-SOLVER-4在不改committed tests/scope的
前提下证明两条`contact_equivalent=true`及16/16 GREEN后，才能把shadow推断升级为production
验收事实。Worker bounded remediation已完成；final re-review=`0 High / 0 Medium / 0 Low`、
裁决`PASS`；independent verifier=`0 High / 0 Medium / 0 Low`、
`S5-BEHAVIOR-RED-4 VERIFIER PASS`。Commit仍为`PENDING`，由supervisor执行；Chrome保持
`NOT RUN / BROWSER PENDING`。

### S5-BEHAVIOR-RED-4 review / verifier history

- 首轮independent reviewer=`0 High / 1 Medium / 0 Low`、裁决`FAIL`。唯一Medium指出
  `S5ContactStateFacts`及nested CCD facts用derived `PartialEq`比较，使全部f32变成bit-exact；
  合法sub-ULP roundoff可能让正确的S5-SOLVER-4永久得到`contact_equivalent=false`。
- Bounded remediation移除两个facts struct的`PartialEq`，加入上述显式scaled comparator；
  enum/count/identity保持exact，30个float均finite后比较，mismatch仍只进入每条test唯一最终组合
  signature。两条clean baseline最大差异分别超过tolerance约`48708.965x`与`11979.997x`，因此
  `contact_equivalent=false`仍由真实solver/contact差异支撑，不是浮点抖动。
- Remediation后unit仍只报8个批准`E0425`；16条integration exact逐条均`running 1 test`并严格
  `12 RED / 4 GREEN`；两条CCD test只命中各自批准CONTACT_STATE signature，无harness failure。
  Scope script blob保持`fb78d83911f3ac208ced37a3c881a52bb9286abc`不变。
- Final independent re-reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`；确认首轮Medium已闭合、
  首轮FAIL历史保留、comparator覆盖与baseline margin可审计，允许派independent verifier。
  该PASS本身不代表verifier PASS、commit完成或SOLVER-4授权。
- Independent verifier=`0 High / 0 Medium / 0 Low`，结论
  `S5-BEHAVIOR-RED-4 VERIFIER PASS`；确认unit仅8个批准`E0425`、16条integration exact严格
  `12 RED / 4 GREEN`、两条CCD只命中批准CONTACT_STATE signature，并确认scope exact-3、fmt、
  diff/zero-diff、cached/untracked与physics/scope blob不变量全部PASS。Commit仍由supervisor执行，
  S5-SOLVER-4在commit前保持`PENDING / NOT STARTED`。

## 9H. S5-SOLVER-4：消费committed RED-4

Immutable Start HEAD=`29bafce59a0913be7b4d37055ab57ed5e5233116`，已由supervisor在
S5-BEHAVIOR-RED-4 commit后写入§16。本node完整继承
§9C的required exact 6、optional unit-only path、phase顺序、2x2/COM/latest-advance/atomic/wake/
one-row/no-stats/no-repeat合同；required exact 6为：

```text
crates/picea/src/pipeline.rs
crates/picea/src/pipeline/step.rs
crates/picea/src/pipeline/island.rs
crates/picea/src/pipeline/joints.rs
crates/picea/src/solver/body_state.rs
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
```

Optional仍仅`crates/picea/src/pipeline/joints/tests.rs`，且production worker禁止修改；只有reviewer
发现unit coverage缺口、supervisor另派bounded test-only worker时才允许。S5-SOLVER-4只消费
committed RED-4 tests，并复用主worktree保留的production patch；不得修改committed tests/scope
script、contact solver、CCD、`integrate.rs`、existing joint kinds、API/schema/stats、StepConfig
字段/default或phase/stream顺序。

### S5-SOLVER-4 STOP receipt（2026-07-15）

状态：`STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`。Worker与supervisor均停止在§15，
未把局部数值接近改写为GREEN：

- receipt、exact-6 scope与unit `8/8`均PASS；第一条RED-4 CCD shadow exact PASS，
  `contact_equivalent=true`，最大float delta约`8.1e-8 <= 1e-6`；
- 第二条exact真实输出`running 1 test`，row/warning=`1/0`、pose error=`0`、drift=`0.000917`、
  residual correction=`0`均满足合同，但唯一最终signature仍为
  `contact_equivalent=false`；subject/shadow tangent impulse分别为`0.0007952799`与
  `0.00081559457`，delta=`0.000020315 > 0.000001`；
- worker用三次LLDB row观察定位首个分叉：Revolute mandatory body center y=
  `0.0199000835`，与冻结2x2/test-side oracle一致；WorldAnchor shadow center y=
  `0.0199000817`，低1 ULP。该1 ULP跨过analytic circle-polygon nearest-point量化边界，
  contact point y从`0.019900322`分叉为`0.0198993683`，随后放大为上述tangent impulse差；
- WorldAnchor的1 ULP来自shadow correction中的`axis * (1 / DT) * DT` f32舍入。让Revolute
  模仿该欠修正会引入hidden under-relaxation，违反§9.4完整2x2 correction与no-stiffness合同；
  其余路径需要修改committed shadow/test tolerance、narrowphase或contact，均属禁止范围；
- 唯一raw-K算术实验已撤销，主worktree保留进入worker前的accepted production patch字节形态；
  exact-6、`git diff --check`、forbidden zero-diff、cached empty、untracked empty均保持PASS。
  因已STOP，未运行完整16条/broad gates；已知`cargo fmt --all --check`仍因既有
  `pipeline/joints.rs`格式差异FAIL；
- 未派reviewer/verifier，未stage/commit/push/merge，未运行Chrome，未进入S5-LAB-RED。

新的docs/test replan必须由用户另行批准；当前不得把S5-SOLVER-4写成可继续的in-progress node。

GREEN gate必须逐条执行§9原unit/13条integration exact与§9B latest/would-wake/no-repeat exact，
结果必须unit=`8/8`、integration=`16/16`；两条shadow-control exact必须证明
`contact_equivalent=true`、shadow/subject row=`1/1`、warning=`0/0`、recovered `P_contact` finite、
expected/actual full pose error`<=1e-4`、drift`<=0.01`、negative errors`>1e-4`。随后运行：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-SOLVER-4)"
rtk proxy cargo test -p picea --lib pipeline::tests::default_step_config_matches_single_step_contract -- --exact --nocapture
rtk proxy cargo test -p picea --lib pipeline::tests::step_config_deserializes_new_solver_policy_with_defaults -- --exact --nocapture
rtk proxy cargo test -p picea --lib pipeline::island -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4 -- --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions
rtk proxy cargo test -p picea --test core_model_world revolute_joint_ -- --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_ -- --nocapture
rtk proxy cargo fmt --all --check
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb self-test
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-SOLVER-4 "$S5_NODE_START_HEAD"
rtk proxy git diff --check
rtk proxy git diff --exit-code "$S5_NODE_START_HEAD" -- crates/picea/tests crates/picea-lab docs/design Cargo.toml Cargo.lock
rtk proxy git diff --cached --exit-code
rtk proxy git ls-files --others --exclude-standard
```

`cargo fmt --all --check`必须GREEN；S5-SOLVER-3遗留的`joints.rs` rustfmt diff不能豁免。Reviewer
findings-first逐式审查K/sign/COM、contact后current、shadow等价、latest advance、atomic/wake、
one-row/no-stats/no-repeat与phase边界；verifier必须报告全部drift/rotation/contact/impulse/residual/
`P_contact`/row/warning/hash/scope facts。若production必须忽略或撤回contact residual correction，
或需要修改禁止范围才能16/16，立即STOP。该历史node已按上方receipt停止且不得commit；以下只由
用户批准的S5-REPLAN-3/RED-5/SOLVER-5新链继续。

## 9I. S5-REPLAN-3：pose-effect oracle与OpenSpec执行合同

Immutable Start HEAD=`29bafce59a0913be7b4d37055ab57ed5e5233116`。用户已明确批准本次
Plan Gate；本node在clean detached worktree执行，ownership exact 8：

```text
docs/design/2026-07-14-revolute-joint-v1-design.md
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
openspec/config.yaml
openspec/changes/complete-s5-revolute-solver-5/.openspec.yaml
openspec/changes/complete-s5-revolute-solver-5/proposal.md
openspec/changes/complete-s5-revolute-solver-5/design.md
openspec/changes/complete-s5-revolute-solver-5/specs/revolute-joint-solver/spec.md
openspec/changes/complete-s5-revolute-solver-5/tasks.md
```

导入receipt固定为：living source blob=
`6bc4dc8ff9f506c5e13bb048afd1da6b0db3b56f`，Start HEAD design blob=
`f945b0f530191b4c1de0921aa930146210e723ce`。主worktree保留production blobs依次为：

```text
pipeline.rs         46e5b6a55bd2aa503361c9334a74336a65891a47
pipeline/step.rs    33b60d027bbddc9818de7c8b7f881c94ff85c46c
pipeline/island.rs  ea4388e8a942a2199e636dd653d37e5737b965fa
pipeline/joints.rs  def54c87469a21519c65149057dfcd2e929f6e68
solver/body_state.rs 573f9f71e4cb2b8b8001930c756a2ebfef037d56
```

本node只同步S5-SOLVER-4 STOP事实、pose-effect comparator职责、新三节点ownership/scope/receipt、
OpenSpec单向进度指针和可执行gate；§7 public surface、§9.1-§9.8 math/phase、§10 wake以及
ADR-S5-1..5保持冻结。OpenSpec `tasks.md`是新链唯一checkbox/完成进度源；living只保存immutable
Start HEAD、前序commit identity和实际验收证据，不从PENDING/SHA推导任务完成度。

Docs/OpenSpec gate必须逐条执行：

```text
S5_NODE_START_HEAD=29bafce59a0913be7b4d37055ab57ed5e5233116
rtk openspec --version
rtk openspec status --change complete-s5-revolute-solver-5
rtk openspec validate complete-s5-revolute-solver-5 --strict
rtk openspec instructions apply --change complete-s5-revolute-solver-5
rtk proxy ruby -e 'require "yaml"; ARGV.each { |path| YAML.load_file(path) }; puts "S5_REPLAN_3_YAML_PASS"' openspec/config.yaml openspec/changes/complete-s5-revolute-solver-5/.openspec.yaml docs/ai/doc-catalog.yaml
rtk proxy ruby -e 'base=ARGV.shift; expected=ARGV.sort; tracked=IO.popen(["rtk","proxy","git","diff","--name-only",base], &:read).lines.map(&:strip).reject(&:empty?); untracked=IO.popen(["rtk","proxy","git","ls-files","--others","--exclude-standard"], &:read).lines.map(&:strip).reject(&:empty?); actual=(tracked+untracked).uniq.sort; abort("S5-REPLAN-3 scope mismatch: #{actual.inspect}") unless actual==expected; puts "S5_REPLAN_3_SCOPE_PASS=#{actual.join(",")}"' "$S5_NODE_START_HEAD" docs/design/2026-07-14-revolute-joint-v1-design.md docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md openspec/config.yaml openspec/changes/complete-s5-revolute-solver-5/.openspec.yaml openspec/changes/complete-s5-revolute-solver-5/proposal.md openspec/changes/complete-s5-revolute-solver-5/design.md openspec/changes/complete-s5-revolute-solver-5/specs/revolute-joint-solver/spec.md openspec/changes/complete-s5-revolute-solver-5/tasks.md
rtk proxy git diff --exit-code "$S5_NODE_START_HEAD" -- crates Cargo.toml Cargo.lock docs/ai
rtk proxy rg -n 'S5-REPLAN-3|S5-BEHAVIOR-RED-5|S5-SOLVER-5|pose-effect|S5_CONTACT_POSE_EFFECT_SELF_TEST|contact_woken_distance_projection|12 RED / 4 GREEN|16/16|24/24|NOT COMMITTED|BROWSER PENDING' docs/design/2026-07-14-revolute-joint-v1-design.md docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md openspec/changes/complete-s5-revolute-solver-5
rtk proxy ruby -e 'ARGV.each { |path| n=File.readlines(path).count { |line| line.start_with?("```") }; abort("odd fences #{path}=#{n}") unless n.even? }; puts "S5_REPLAN_3_FENCES_PASS"' docs/design/2026-07-14-revolute-joint-v1-design.md docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
rtk proxy git diff --check
rtk proxy git diff --cached --exit-code
```

Supervisor还必须从main worktree只读复核上述五个production blob与living import blob；任一变化、
exact-8缺失/多出路径、OpenSpec非4/4、strict validate失败、fence/YAML/link target缺失均FAIL。
Reviewer findings-first核对Fact/Inference/Unknown、comparator字段职责、scope/STOP和final commit无
自引用；docs verifier独立复跑全部命令。两者闭合High/Medium后，才允许把OpenSpec tasks 1.1-1.6
勾完、stage exact-8并提交`docs: replan revolute pose-state oracle`。Commit前cached必须exact-8；
commit后只读`git diff-tree`复核，不在同commit中记录自己的SHA。

Worker完成：初始OpenSpec=`4/4 artifacts complete`、strict validate PASS、apply=`0/24`；首次
independent spec review因缺§9I/J/K、scope/receipt与closeout自引用以
`4 High / 4 Medium / 1 Low`裁决FAIL。Bounded remediation后re-review先收敛到
`0 High / 2 Medium / 0 Low`，最终两项status/normal-impulse合同闭合，结论
`0 High / 0 Medium`、PASS。Independent docs verifier复跑OpenSpec 1.6.0、4/4、strict validate、
apply、exact-8、YAML/fences/required terms、outside zero-diff、main blobs、diff/cached gates，给出
`0 High / 0 Medium / 0 Low`与`S5-REPLAN-3 DOCS VERIFIER PASS`。OpenSpec tasks 1.1-1.6完成；
supervisor提交门已解锁，commit SHA不在本commit中自引用。

## 9J. S5-BEHAVIOR-RED-5：pose-effect acceptance-as-code

Immutable Start HEAD只在S5-REPLAN-3提交后由supervisor写入§16；当前为`PENDING`。Ownership
exact 5：

```text
crates/picea/tests/physics_realism_acceptance.rs
crates/picea/tests/world_step_review_regressions.rs
crates/picea/tests/verify_revolute_scope.rb
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
openspec/changes/complete-s5-revolute-solver-5/tasks.md
```

本node不得修改production/design/OpenSpec其他artifacts/lab/Web/Cargo。它先完成§4.1第四次scope
state transition，再只修改两条CCD test共用的shadow comparator与现有第16条no-repeat exact；
不得新增第17条或修改测试名、fixture、`1e-6`、full-pose`1e-4`、drift`0.01`及其他阈值。

### 9J.1 Pose-effect comparator

WorldAnchor shadow继续真实跑完整step并恢复`P_contact`。Cross-world等价只比较能改变该pose的：

- CCD role/identity/target kind及swept start/end、target sweep、TOI、advancement、clamp、slop、
  TOI point等finite geometry facts；identity/kind exact，float仍用scaled `1e-6`；
- `solver_position_correction_depth`与body A/B correction translation；
- StepStats的position-correction input contact/body counts（exact）、input max/total depth、
  max/total translation（scaled `1e-6`）。

normal/tangent initial/final speed、position/restitution bias、normal/tangent impulse全部继续先过finite
gate并完整打印；subject normal impulse必须`>0`，subject latest velocity mutation仍是独立hard gate。
这些velocity-response数值不进入cross-world pose-effect bool。两条helper invocation都必须复制一组
finite pose-effect facts，仅把`position_correction_input_max_depth += 1e-3`，证明comparator=false，
并输出：

```text
S5_CONTACT_POSE_EFFECT_SELF_TEST
max_field=<field> delta=<finite> tolerance=<finite> equivalent=false
```

Clean baseline的两条唯一批准最终signature改为：

```text
S5_REVOLUTE_POSE_EFFECT_ASSERT:ccd_post_contact_final_pose
S5_REVOLUTE_POSE_EFFECT_ASSERT:ccd_contact_post_contact_final_pose
```

不得用subject row=`0`提前退出；shadow validity、finite、positive subject impulse、negative candidates、
self-test与recovered `P_contact`都必须先GREEN，最终组合assert才构成expected RED。

### 9J.2 Contact-woken Distance boundary

扩展现有第16条
`post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows`，增加独立子case：初始让
Distance-connected island sleeping，由真实contact在本帧唤醒；optional velocity projection enabled
必须在contact后按最新active membership执行，radial speed `<=1e-4`；disabled control必须
`>1e-3`。两边均验证真实contact/wake、finite、existing Distance logical stats不重复，并输出：

```text
S5_REVOLUTE_REPLAN_BOUNDARY:contact_woken_distance_projection
```

该子case在clean HEAD应GREEN；临时带入旧S5-SOLVER-4 patch时必须因复用contact前plan而失败，
从而证明lock有检出力。不得为它修改production、增加test hook或新public config。

### 9J.3 RED分类、compatibility witness与提交门

Unit继续运行：

```text
rtk proxy cargo test -p picea --lib pipeline::joints::tests::revolute_point_constraint_ -- --nocapture
```

clean baseline只能出现既有4个planned helper各两处、共8个批准`E0425`，不得出现其他compiler
diagnostic。Integration必须按下表逐条用`--exact --nocapture`运行，禁止aggregate filter或zero-test：

| # | Target | Exact test | Clean分类 |
| --- | --- | --- | --- |
| 1 | `physics_realism_acceptance` | `revolute_joint_two_dynamic_preserves_anchor_coincidence` | RED |
| 2 | `physics_realism_acceptance` | `revolute_joint_leaves_relative_rotation_free` | GREEN |
| 3 | `physics_realism_acceptance` | `revolute_joint_static_dynamic_preserves_static_pose` | RED |
| 4 | `physics_realism_acceptance` | `revolute_joint_off_center_anchor_uses_rotational_inertia` | RED |
| 5 | `physics_realism_acceptance` | `revolute_joint_is_deterministic_and_finite` | GREEN |
| 6 | `physics_realism_acceptance` | `revolute_joint_mixed_contact_island_keeps_separate_logical_rows` | RED |
| 7 | `physics_realism_acceptance` | `revolute_joint_connected_bodies_still_contact` | GREEN |
| 8 | `physics_realism_acceptance` | `revolute_joint_nonzero_local_center_of_mass_preserves_pivot` | RED |
| 9 | `physics_realism_acceptance` | `revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle` | RED，仅本节pose-effect signature |
| 10 | `physics_realism_acceptance` | `revolute_joint_awake_sleeping_pair_uses_current_pose_and_wakes_on_correction` | RED |
| 11 | `physics_realism_acceptance` | `revolute_joint_contact_full_step_preserves_pivot_with_projection_enabled` | RED |
| 12 | `physics_realism_acceptance` | `revolute_joint_contact_full_step_preserves_pivot_with_projection_disabled` | RED |
| 13 | `physics_realism_acceptance` | `revolute_joint_ccd_clamped_contact_full_step_preserves_pivot` | RED，仅本节pose-effect signature |
| 14 | `physics_realism_acceptance` | `revolute_joint_post_contact_reconciliation_uses_latest_velocity` | RED |
| 15 | `physics_realism_acceptance` | `revolute_joint_post_contact_reconciliation_would_wake_resamples_latest_velocity` | RED |
| 16 | `world_step_review_regressions` | `post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows` | GREEN，含Distance新子case |

每条命令形态固定为：

```text
rtk proxy cargo test -p picea --test <Target> <Exact test> -- --exact --nocapture
```

必须得到`running 1 test`与严格`12 RED / 4 GREEN`。两条CCD只命中新pose-effect signature且均先
打印`S5_CONTACT_POSE_EFFECT_SELF_TEST`；第16条打印Distance boundary signature，总数仍为16。

提交RED前必须在同一detached worktree做临时production compatibility witness：从main worktree
生成五文件binary diff并应用；两条CCD exact必须GREEN，证明收窄oracle能消费accepted 2x2 pose；
第16条必须只在contact-woken Distance enabled边界失败，证明旧plan复用可被检出。随后立即用
Start HEAD恢复五个production文件。Witness前后exact-5 acceptance blobs必须bit-identical，恢复后
scope actual必须重新严格exact-5；临时production不得stage/commit。若目标两条仍FAIL、Distance
case不失败或出现其他failure，按§15 STOP，不得直接进入SOLVER-5。

Scope/hygiene gate：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-BEHAVIOR-RED-5)"
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb self-test
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-BEHAVIOR-RED-5 "$S5_NODE_START_HEAD"
rtk proxy cargo fmt --all --check
rtk openspec validate complete-s5-revolute-solver-5 --strict
rtk openspec instructions apply --change complete-s5-revolute-solver-5
rtk proxy git diff --check
rtk proxy git diff --exit-code "$S5_NODE_START_HEAD" -- crates/picea/src crates/picea/tests/core_model_world.rs crates/picea-lab docs/design openspec/config.yaml openspec/changes/complete-s5-revolute-solver-5/.openspec.yaml openspec/changes/complete-s5-revolute-solver-5/proposal.md openspec/changes/complete-s5-revolute-solver-5/design.md openspec/changes/complete-s5-revolute-solver-5/specs Cargo.toml Cargo.lock
rtk proxy git diff --cached --exit-code
rtk proxy git ls-files --others --exclude-standard
```

Reviewer逐项审查comparator字段所有权、negative control、12/4分类、Distance fixture与scope transition；
independent RED verifier从clean baseline重跑unit+16 exact、witness和全部scope/hygiene。闭合High/Medium
后把OpenSpec tasks 2.1-2.8勾完，stage exact-5并提交`test: lock revolute pose-state oracle`；commit
后只读复核paths，不在该commit内记录自身SHA。

## 9K. S5-SOLVER-5：消费committed RED-5

Immutable Start HEAD只在S5-BEHAVIOR-RED-5提交后由supervisor写入§16；当前为`PENDING`。
Required exact 7：

```text
crates/picea/src/pipeline.rs
crates/picea/src/pipeline/step.rs
crates/picea/src/pipeline/island.rs
crates/picea/src/pipeline/joints.rs
crates/picea/src/solver/body_state.rs
docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
openspec/changes/complete-s5-revolute-solver-5/tasks.md
```

Reviewer已触发唯一optional path=`crates/picea/src/pipeline/joints/tests.rs`，只允许新增sub-EPSILON
unit locks。SOLVER-5不得修改committed integration/scope/design/OpenSpec其他artifacts、contact/
narrowphase/CCD/`integrate.rs`、lab/Web、API/schema、StepConfig字段/default或批准阈值。

### 9K.1 最小production修复

- mandatory joint phase建立`MandatoryJointPlan`，只拥有mandatory batches、一次stats与Revolute-only
  post rows；`joint_row_count`仍每个Revolute为1；
- contact后optional velocity phase重新调用active-island builder，按latest wake state构造临时
  velocity batches；这些rows不累计任何mandatory stats，也不替换post plan；
- post-contact reconciliation仍只遍历mandatory plan的Revolute carriers，不再次处理Distance/
  WorldAnchor position/stiffness/damping；phase相对顺序保持§9C；
- WorldAnchor damping恢复HEAD原内联
  `Vector::new(omega * r.y(), -omega * r.x())`，Revolute helper不重构existing kind；
- 保留冻结2x2 `K`、clockwise sign、COM-to-origin、CCD translation skip/angle advance、finite/atomic
  A/B apply、would-wake two-stage recompute和static/kinematic write边界。

Optional unit必须先证明旧patch为RED，再修实际状态差异：probe的finite correction component只要
`!=0.0`即是demand；pose wake比较最终committed/current的x、y、angle；velocity wake比较实际
`next/current` linear/angular components，而不是原始delta或`>EPSILON`。可表示但小于EPSILON的真实
mutation必须wake；加法舍入后bitwise未改变的状态不得虚假wake。至少覆盖pose、velocity与post
would-wake三类；zero/singular/non-finite controls保持不apply、不wake。

### 9K.2 GREEN与broader gates

先逐条重跑§9J同一unit及16条integration exact，结果必须unit既有`8/8`加新增sub-EPSILON unit
全部GREEN、integration=`16/16`。两条CCD必须pose-effect equivalent、row/warning=`1/0`、full-pose
error`<=1e-4`、drift`<=0.01`；Distance contact-wake enabled/disabled与self-test/negative signatures
全部满足。随后逐条运行：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-SOLVER-5)"
rtk proxy cargo test -p picea --lib pipeline::tests::default_step_config_matches_single_step_contract -- --exact --nocapture
rtk proxy cargo test -p picea --lib pipeline::tests::step_config_deserializes_new_solver_policy_with_defaults -- --exact --nocapture
rtk proxy cargo test -p picea --lib pipeline::island -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4 -- --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions
rtk proxy cargo test -p picea --test core_model_world revolute_joint_ -- --nocapture
rtk proxy cargo test -p picea --test world_step_review_regressions joint_lifecycle_wake_ -- --nocapture
rtk proxy cargo test -p picea --lib
rtk proxy cargo test -p picea --tests
rtk proxy cargo test -p picea --examples --no-run
rtk proxy cargo bench -p picea --no-run
rtk proxy cargo test --workspace --all-targets
rtk proxy cargo check --workspace --all-targets
rtk proxy cargo clippy --workspace --all-targets -- -D warnings
rtk proxy cargo fmt --all --check
rtk openspec status --change complete-s5-revolute-solver-5
rtk openspec validate complete-s5-revolute-solver-5 --strict
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb self-test
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-SOLVER-5 "$S5_NODE_START_HEAD"
rtk proxy git diff --check
rtk proxy git diff --exit-code "$S5_NODE_START_HEAD" -- crates/picea/tests crates/picea-lab docs/design openspec/config.yaml openspec/changes/complete-s5-revolute-solver-5/.openspec.yaml openspec/changes/complete-s5-revolute-solver-5/proposal.md openspec/changes/complete-s5-revolute-solver-5/design.md openspec/changes/complete-s5-revolute-solver-5/specs Cargo.toml Cargo.lock
rtk proxy git ls-files --others --exclude-standard
```

Solver spec reviewer核对RED-5与§9K合同；code reviewer findings-first审panic/NaN/stale handles、plan
ownership、actual-change wake和existing-kind zero-diff；independent verifier从头重跑unit+16、targeted、
broad、fmt、OpenSpec、scope/hygiene。任一High/Medium、16/16/broad失败或需要禁止范围修改都按§15
STOP。

全部GREEN后，先把实际命令输出、未运行项与残余风险写入本节/§16，并把OpenSpec tasks 3.1-4.2
全部勾完；§16的S5-SOLVER-5 Commit cell在该candidate中保持`PENDING`，避免自引用SHA。仅stage
required exact 7加已批准optional并让cached binary scope PASS。至此repo tasks为24/24；随后由
supervisor执行不属于checkbox的外部closeout动作，创建单一
`feat: reconcile revolute constraints after contacts` commit。Commit后只读复核`diff-tree`、HEAD、
clean worktree、OpenSpec 4/4、strict validate与`instructions apply=24/24`；最终SHA只写外部报告，
不amend、不新增receipt commit、不archive/sync/deploy/push/merge/PR，也不进入S5-LAB-RED。

## 10. S5-LAB-RED：跨层 acceptance-as-code

本次OpenSpec change在S5-SOLVER-5完成后hard-stop，不授权进入本node。只有用户后续明确授权，
且S5-SOLVER-5 final commit与外部只读receipt均PASS后，才在该commit之上新增：

- `artifact_run` exact test：按字符串选择 `revolute_pendulum`，要求 debug joint kind、
  两个anchors、logical row count、finite/deterministic hash。
- `server_routes` exact test：创建/step该scenario，响应保留 `revolute` 和anchors。
- UI contract：Inspector/Hierarchy/Timeline存在显式 revolute label/consumer，不接受
  fallback为distance/world_anchor/raw unknown。
- i18n contract：en-US/zh-CN 都有显式 revolute label。
- fixture compatibility negative：old-reader fixture模拟对new variant明确unknown reject；
  schema v1 old fixtures继续green。
- debug compatibility negative：模拟old closed `DebugJointKind` serde consumer，读取
  artifact/server的`"revolute"`时必须明确unknown-variant reject；Rust facts path不得降级kind。

RED 命令：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-LAB-RED)"
rtk proxy cargo test -p picea-lab --test artifact_run revolute_pendulum -- --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run revolute_debug_kind_old_consumer_rejects_unknown_variant -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test server_routes revolute_pendulum -- --nocapture
rtk proxy cargo test -p picea-lab scene_fixture_revolute -- --nocapture
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
rtk proxy npm --prefix crates/picea-lab/web run build
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-LAB-RED "$S5_NODE_START_HEAD"
rtk proxy git diff --check
rtk proxy git diff --exit-code -- crates/picea/src crates/picea/tests Cargo.toml Cargo.lock
```

期望：core behavior继续green；artifact/server/UI/i18n中至少一个真实 capability 缺口稳定
RED。TypeScript语法错误、缺依赖、server未启动等 harness failure 不算有效 RED。

| Acceptance | Baseline expectation | 实际结果 |
| --- | --- | --- |
| artifact scenario/facts | RED | 待填 |
| server passthrough | RED | 待填 |
| old debug consumer unknown-variant reject | RED until authoritative revolute JSON exists；不得kind fallback | 待填 |
| fixture compatibility | existing old v1 green；new capability分类记录 | 待填 |
| UI explicit consumer | RED | 待填 |
| i18n two locales | RED | 待填 |
| production type build | GREEN或明确missing capability RED，不得syntax fail | 待填 |

## 11. S5-LAB：scenario、facts、Web 与 browser

最小实现：

- `ScenarioId::RevolutePendulum`、descriptor/catalog/dispatch和单独
  `scene_revolute.rs`；场景固定dt、static/dynamic hinge、无motor/limits/damping。
- fixture v1完整转换与roundtrip；不升级 envelope/version。
- artifact/server沿 generic `DebugSnapshot` / `StepStats` projection保留 facts；只有
  behavior lock证明 generic path不足时才允许最小 production special case。
- old closed debug consumer的unknown-variant reject必须保持可观察；new artifact/server
  原样传输`revolute`，不通过fallback伪造forward compatibility。
- Web type union、i18n、Inspector/Hierarchy/Timeline consumer显式支持 `revolute`；不增加
  physics计算、live patch和advanced control。

Targeted GREEN：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-LAB)"
rtk proxy cargo test -p picea-lab scene_fixture_revolute -- --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run revolute_pendulum -- --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run revolute_debug_kind_old_consumer_rejects_unknown_variant -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test server_routes revolute_pendulum -- --nocapture
rtk proxy cargo test -p picea-lab --lib
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
rtk proxy npm --prefix crates/picea-lab/web run test:profile
rtk proxy npm --prefix crates/picea-lab/web run build
rtk proxy just picea-lab-web-check
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-LAB "$S5_NODE_START_HEAD"
rtk proxy git diff --check
```

### 11.1 External browser gate（ChatGPT App）

当前CLI worker/reviewer/verifier不启动或控制Chrome，也不得把Rust/Web/server GREEN写成
browser PASS。CLI gate通过后，supervisor可先commit可识别的S5-LAB candidate，再把以下
固定prompt交给ChatGPT App；external receipt必须针对该candidate的40位full commit SHA。
未回填时S5-LAB状态只能是`BROWSER PENDING / NOT RUN`，不得进入S5-V。若FAIL，回到
supervisor分派bounded remediation，产生新commit后必须用新SHA重跑。

```text
你在 ChatGPT App 中执行 Picea S5-LAB 外部 Chrome 验收。只验收，不修改任何仓库文件，
不运行 formatter，不 commit、不 push、不 merge。

固定输入：
- repo: /Users/asyncrustacean/projects/picea
- expected branch: feat/vnext-s5-revolute-joint
- tested full commit SHA: <S5_LAB_COMMIT_FULL_SHA，由CLI supervisor回填，必须40位>
- preferred URL: <CLI supervisor回填；若为空则自行查询>
- viewport: 1440x900
- scenario: revolute_pendulum
- screenshot root: /tmp/picea-s5-browser/<S5_LAB_COMMIT_FULL_SHA>/s5-lab

步骤：
1. 在repo运行 `rtk proxy git status --short --branch`、`rtk proxy git rev-parse HEAD`；branch和
   full SHA必须与固定输入完全一致。若worktree有除已知browser运行产物外的源码/docs改动，
   立即FAIL并报告，不要修。
2. 若preferred URL可访问则复用；否则运行`rtk proxy just picea-lab-web-status`，仍不可用时
   运行`rtk proxy just picea-lab-web-start`，再以status输出的实际URL为准。记录是否复用或启动。
3. 用Chrome打开实际URL并加`?picea-profile=1`，设置viewport 1440x900，记录Chrome版本。
4. 选择并运行`revolute_pendulum`，至少观察到frame index >= 240。确认source badge明确是
   Rust live/artifact source，不是offline/demo/fallback。
5. 记录visible/DOM facts：场景名；Inspector/Hierarchy/Timeline中的显式revolute label；
   两个body；两个world anchors；画面中relative rotation自由且pivot无可见分离或跳变。
6. 从UI所消费的network response/artifact/debug facts核对：至少一个joint的
   `kind == "revolute"`、`anchors.length == 2`、两个anchor均finite、该active revolute的
   `joint_row_count == 1`。禁止根据画面自行推算或把unknown kind降级。
7. 检查Chrome console和network：记录全部error/warning、failed request、非2xx API响应；
   PASS要求新增console error/warning为0、关键API失败为0。
8. 截图至少三张：运行中全景、Inspector/Hierarchy joint facts、Timeline/source badge；保存到
   screenshot root之外不得写repo。若本次启动了服务，结束后运行
   `rtk proxy just picea-lab-web-stop`；复用既有服务则不要擅自停止，并在receipt说明。

严格输出一个receipt：
- status: PASS | FAIL | BLOCKED
- tested_full_commit_sha / branch / URL / tested_at(含timezone)
- Chrome version / viewport
- service: reused | started-and-stopped
- checklist逐项PASS/FAIL与观察值
- debug facts: kind / anchors / joint_row_count / frame index / source badge
- screenshot absolute paths
- console warnings/errors与network异常明细（没有则写none）
- failure或blocked时：第一失败步骤、可复现动作、最小下一步；不得自行修代码。
```

External receipt缺少full SHA、URL、时间、Chrome版本/viewport、逐项PASS/FAIL、截图绝对路径、
console/network明细中的任一项，都按不完整证据处理，状态保持PENDING而非PASS。

## 12. S5-V：完整独立验收

S5-V仍执行三个独立叶子角色，且三者都只读、不修问题：

1. leaf worker：记录S5-V start HEAD，运行全部preflight gates并在消息中整理五段receipt；
   不写living spec或任何文件。
2. leaf reviewer：审查`28867b5c8eeae3ec290ad7dfd5f52cbf3ca69e80..HEAD` final diff、各node RED/GREEN evidence、public/COM/
   CCD/stream/schema scope；findings-first，不运行修复。
3. independent leaf verifier：在review High/Medium闭环后从头独立复跑本节全部CLI命令和
   milestone binary scope；不得复用worker“已通过”结论，也不得调用Chrome。

任何角色发现失败都回到supervisor triage并另派bounded worker；S5-V角色本身不改文件。

Core/API/solver：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-V)"
rtk proxy ruby crates/picea/tests/verify_revolute_public_api.rb expect-green /var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s5-revolute-api
rtk proxy cargo fmt --all --check
rtk proxy cargo test -p picea --lib
rtk proxy cargo test -p picea --tests
rtk proxy cargo test -p picea --examples --no-run
rtk proxy cargo bench -p picea --no-run
```

Workspace/lab：

```text
rtk proxy cargo test -p picea-lab
rtk proxy cargo test -p picea-lab --test artifact_run
rtk proxy cargo test -p picea-lab --test server_routes
rtk proxy cargo test --workspace --all-targets
rtk proxy cargo check --workspace --all-targets
rtk proxy cargo clippy --workspace --all-targets -- -D warnings
```

Web/docs/scope：

```text
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract
rtk proxy npm --prefix crates/picea-lab/web run test:i18n
rtk proxy npm --prefix crates/picea-lab/web run test:profile
rtk proxy npm --prefix crates/picea-lab/web run build
rtk proxy just picea-lab-web-check
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb milestone 28867b5c8eeae3ec290ad7dfd5f52cbf3ca69e80
rtk proxy ruby -e 'forbidden=%w[crates/picea/tests/fixtures/revolute_public_api/Cargo.lock crates/picea/tests/fixtures/revolute_public_api/target picea-s5-revolute-api]; found=forbidden.select { |path| File.exist?(path) }; abort("repo-local generated artifact: #{found.join(", ")}") unless found.empty?; puts "s5 filesystem hygiene ok"'
rtk proxy git status --short --ignored
rtk proxy git diff --check
rtk proxy git status --short --branch
```

Binary scope只判断committed、staged/unstaged tracked与untracked，也就是git-visible路径；
不得再宣称所有ignored `target/dist/log`都不存在。Filesystem hygiene额外拒绝fixture-local
`Cargo.lock`/`target`和repo内temp copy。批准temp root仅为
`/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s5-revolute-api`，browser截图仅写
`/tmp/picea-s5-browser/<full-sha>/...`。既有或本轮正常生成的ignored `target/`、
`crates/picea-lab/web/{node_modules,dist}/`、`.playwright-cli/`、`logs/`可存在，但必须在
`git status --short --ignored` receipt中分类，不能作为clean或PASS证据；新S5命名产物若落在
批准边界外则FAIL，不删除现场。

S5-V CLI verifier 还必须检查：

- `JointKind`等只有批准的六个 enums 增加`#[non_exhaustive]`；public structs没有该attribute。
- prelude 只增加 `RevoluteJointDesc/Patch`。
- fixture仍是schema v1；没有motor/limits/damping/`collide_connected`/cache。
- contact/joint rows仍分离，live phase order没有重排，一个revolute row count为1。
- `verify_revolute_scope.rb milestone 28867b5c8eeae3ec290ad7dfd5f52cbf3ca69e80`
  必须binary PASS，匹配§4.1 milestone union；ignored artifacts按上述filesystem边界另行报告。

### 12.1 Independent external browser gate（ChatGPT App）

S5-V的external browser receipt必须针对S5-V开始时的最终40位HEAD独立生成，不得复用
S5-LAB receipt。CLI三角色全部PASS但external receipt未回填时，只能写
`CLI PASS / BROWSER PENDING (NOT RUN)`，不能宣称full browser acceptance或S5-V完成。

```text
你在 ChatGPT App 中执行 Picea S5-V 独立最终 Chrome 验收。不要复用任何S5-LAB浏览器
结论；从头执行并独立取证。只验收，不修改repo文件，不运行formatter，不commit、不push、
不merge。

固定输入：
- repo: /Users/asyncrustacean/projects/picea
- expected branch: feat/vnext-s5-revolute-joint
- tested full commit SHA: <S5_V_FULL_SHA，由CLI supervisor回填，必须40位>
- preferred URL: <CLI supervisor回填；可为空>
- viewport: 1440x900
- scenario: revolute_pendulum
- screenshot root: /tmp/picea-s5-browser/<S5_V_FULL_SHA>/s5-v

步骤：
1. 运行`rtk proxy git status --short --branch`和`rtk proxy git rev-parse HEAD`；branch/full SHA
   必须与输入完全一致。发现源码/docs dirty则FAIL并停止，不要修。
2. 优先复用可访问的preferred URL；否则用`rtk proxy just picea-lab-web-status`查询，必要时
   `rtk proxy just picea-lab-web-start`。记录actual URL与reused/started。
3. 在Chrome记录版本并设置1440x900，打开actual URL + `?picea-profile=1`，选择并运行
   `revolute_pendulum`到frame index >= 240。
4. 独立核对visible/DOM：真实Rust live/artifact source badge、显式revolute label、两个body、
   两个world anchors、relative rotation自由、pivot无可见分离/跳变。
5. 独立核对network response/artifact/debug facts：`kind == "revolute"`、
   `anchors.length == 2`且finite、active revolute `joint_row_count == 1`、frame index和source
   badge一致；禁止浏览器端重算facts或kind fallback。
6. 检查并完整记录console error/warning、failed request和非2xx关键API。新增console
   error/warning和关键network failure都必须为0。
7. 保存至少三张截图到screenshot root：全景、Inspector/Hierarchy facts、Timeline/source
   badge。若本次启动服务则执行`rtk proxy just picea-lab-web-stop`；复用服务则不停止。

严格输出receipt：
- status: PASS | FAIL | BLOCKED
- tested_full_commit_sha / branch / actual URL / tested_at(含timezone)
- Chrome version / viewport / service reused或started-and-stopped
- 每个步骤PASS/FAIL及实际观察值
- debug facts: kind / anchors / joint_row_count / frame index / source badge
- screenshot absolute paths
- console/network异常明细（无则none）
- failure/blocked时给第一失败步骤、复现动作、最小下一步；不得自行修代码。
```

receipt缺少full SHA、URL、时间、Chrome版本/viewport、逐项结果、截图路径或
console/network明细时，一律视为不完整并保持PENDING。

## 13. S5-C：closeout、routing 与最终 receipt

只在 S5-V PASS 且 final reviewer 无 High/Medium 后：

- 本文状态改为完成，逐节点记录 commit、RED/GREEN receipt和残余风险。
- design状态改为implemented/verified，但保留 ADR 与deferred scope。
- parent/handoff只声明 handoff §5独立完成，不冒充父 E5/E6完成。
- 同步 `docs/design/solver-island-ordering-contract.md` 到 live phase order，但明确
  contact/joint streams仍分离；不得借closeout改solver。
- 最小刷新 AI routing/doc catalog/design index。

Closeout gates：

```text
S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-C)"
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy rg -n 'S5-D|S5-API-RED|S5-API|S5-BEHAVIOR-RED|S5-SOLVER|S5-REPLAN|S5-BEHAVIOR-RED-2|S5-SOLVER-2|S5-BEHAVIOR-RED-3|S5-SOLVER-3|S5-REPLAN-2|S5-BEHAVIOR-RED-4|S5-SOLVER-4|S5-REPLAN-3|S5-BEHAVIOR-RED-5|S5-SOLVER-5|ADR-S5-5|S5-LAB-RED|S5-LAB|S5-V|S5-C' docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md docs/design/2026-07-14-revolute-joint-v1-design.md
rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb S5-C "$S5_NODE_START_HEAD"
rtk proxy git diff --check
rtk proxy git diff --exit-code -- crates Cargo.toml Cargo.lock
rtk proxy git status --short --branch
```

S5-C commit后，supervisor 必须用 `rtk proxy git show --stat --oneline HEAD` 和
`rtk proxy git diff-tree --no-commit-id --name-only -r HEAD` 复核docs-only scope。不要push。

## 14. 验收报告格式

每个 GREEN/RED node 的 receipt 严格按以下顺序写入进度日志：

1. Start HEAD：supervisor在第一处改动前记录的40位immutable full commit SHA；未来node只能
   写`PENDING`。worker/reviewer/verifier及新shell均用`receipt-head`读取，不手填短SHA。
2. 成功标准：预先批准的二元 bar，一行。
3. 检查结果：每条 exact command、exit code、关键输出/断言或artifact链接。
4. 复跑方式：用户不依赖agent即可执行的命令/步骤。
5. 范围外：本node刻意未做的内容。
6. 残余风险：已知缺口、deferred项和下一最小动作。

RED receipt额外记录 failure signature，并区分：

- expected RED：批准能力缺失导致断言/compile contract失败；
- expected boundary GREEN：实现前就成立的边界锁；
- harness failure：路径、依赖、语法、环境错误，不能授权下一node。

## 15. Stop conditions

立即停止并报告，不得自行裁决：

- 需要 motor、limits、damping、`reference_angle`、`collide_connected`、break force、
  stiffness或persistent joint warm-start cache；
- 需要修改/合并/reorder contact/joint solver stream、既有phase或contact solver；
  ADR-S5-5明确批准的optional projection之后、final integration之前的Revolute-only局部插入
  不算reorder，但扩大到其他phase/kind立即STOP；
- contact/projection enabled/disabled或CCD-clamped full-step gate只能通过修改contact solver/
  CCD algorithm、重排phase、冻结mandatory旧velocity、skip/freeze final integration、修改
  `integrate.rs`语义或绕过positive contact才能满足；
- post-contact pass需要再次solve Distance/WorldAnchor、创建第二logical row、public/schema/stats
  surface或改变`joint_row_count`累计语义；
- 需要升级 fixture schema version；
- 需要修改冻结字段、default、`#[non_exhaustive]`列表、prelude边界、wake reason或
  `user_data` wake规则；
- 需要把一个 revolute 计为两个logical rows；
- RED只能通过删除/ignore/放宽断言、改阈值或伪造Web facts消除；
- RED-4的WorldAnchor shadow无法用§9G列出的可观察solver facts证明subject/contact等价，或
  recovered `P_contact`/post demand/negative candidates存在non-finite、循环依赖或提前harness
  failure；
- Clean `6ffa1e3…`的16条integration不是严格`12 RED / 4 GREEN`，两条新CCD exact没有各自只
  命中批准的`S5_REVOLUTE_CONTACT_STATE_ASSERT:*` signature，或需要把contact mismatch/
  row-zero改成提前assert才能得到RED；
- RED-4需要production test hook、public config、fixture-only runtime branch，或SOLVER-4需要
  production忽略/撤回contact residual correction、冻结mandatory旧pose/velocity才能通过；
- REPLAN-3 exact-8出现额外/缺失路径、OpenSpec非4/4、strict validate失败，或冻结§7/§9.1-§9.8/
  §10/ADR-S5-1..5产生语义变化；
- RED-5 clean unit不是仅8个批准symbol RED、16条integration不是严格`12 RED / 4 GREEN`、两条
  CCD没有各自只命中`S5_REVOLUTE_POSE_EFFECT_ASSERT:*`、comparator negative control不为false，
  或第16条Distance子case在clean不GREEN；
- 临时production witness不能让两条CCD GREEN、不能让旧plan复用在Distance边界被检出，或恢复后
  acceptance blobs/scope发生变化；
- SOLVER-5需要复用contact前mandatory plan做optional velocity、重复累计stats/处理existing
  position rows、保留WorldAnchor无关重构、用`>EPSILON`掩盖真实sub-EPSILON mutation，或任一
  unit/16 exact/broad/fmt/scope gate失败；
- SOLVER-5完成后未经新用户授权进入S5-LAB-RED，或试图通过amend/第二receipt commit把final commit
  SHA自引用写回同一change；
- external ChatGPT App browser receipt不能证明真实Rust facts path、被测full SHA不一致或
  返回FAIL；未回填只保持PENDING，不得写PASS；
- 发现不属于当前node的dirty写入且无法确认所有权。

## 16. 进度日志

| 日期 | Node | Start HEAD (full, immutable) | 状态 | Commit | Receipt / 备注 |
| --- | --- | --- | --- | --- | --- |
| 2026-07-14 | S5-D | `28867b5c8eeae3ec290ad7dfd5f52cbf3ca69e80` | 已commit | `eecbc331a36bfb694676c979bffb03f5a47202dd` | 第三轮reviewer=`0 High / 0 Medium / 1 Low`，允许进入verifier；independent docs verifier于`2026-07-14 17:16:23 CST`给出`S5-D VERIFIER PASS`；8-file scope/YAML/Mermaid/required terms/diff-check/crates/Cargo/Web zero-diff/cached empty均PASS；唯一Low留到S5-C；Chrome `NOT RUN / BROWSER PENDING`；implementation/API/solver未开始 |
| 2026-07-14 | S5-API-RED | `eecbc331a36bfb694676c979bffb03f5a47202dd` | final reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=`S5-API-RED VERIFIER PASS`；已commit | `6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68` | 4M remediation complete；public expect-red连续两次wrapper `0`/internal `101`且仅approved diagnostics；六enum missing-variant分类与六条existing-kind lifecycle exact RED均有效，wake exact各自`running 1 test`、正确sentinel、approved exit `101`；workspace/scope/temp/diff GREEN，7-path exact；Revolute runtime `NOT RUN`；API/solver未开始；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-14 | S5-API | `6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68` | scope reviewer/re-verifier已PASS；implementation worker完成；worker targeted/full/scope/hygiene gates=`GREEN`；independent implementation reviewer首轮=`0 High / 1 Medium / 0 Low`、裁决`FAIL`（顶部仍误报implementation未开始），第一次bounded remediation已完成；第一次复审=`0 High / 1 Medium / 0 Low`、裁决`FAIL`（文档仍误报第一次状态修复未完成），第二次status-only bounded remediation已完成；最终独立复审=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=`S5-API VERIFIER PASS`；已commit；Chrome `NOT RUN / BROWSER PENDING` | `c5a47ed4252363ebf914802c8189cf2bcc9e3563` | exact-16内已实现Revolute public/lifecycle/authoring API、schema-v1 fixture/re-export/TS kind compatibility与island explicit skip；public positive GREEN+六enum仅批准`E0004`，runtime `2 passed`，core focused `6 passed`，row-zero为`running 1 test`+`joint_count=1;joint_row_count=0`，六条lifecycle各自`running 1 test`+sentinel/kind覆盖，fixture `3 passed`；full core/world/lab=`24/18/38 passed`，Web/workspace/fmt/scope self-test/exact-16/diff/Cargo/solver/cached/untracked/hygiene均PASS。首次public harness因重叠使用固定temp root出现一次`ENOENT`，串行同命令及runtime补跑PASS且源文件zero-diff，分类为verifier orchestration noise；solver、scenario behavior与Chrome未开始；未push；scope script保持implementation前approved diff，worker未修改 |
| 2026-07-14 | S5-BEHAVIOR-RED | `c5a47ed4252363ebf914802c8189cf2bcc9e3563` | worker首轮完成；independent reviewer首轮=`1 High / 3 Medium / 1 Low`、裁决`FAIL`；bounded remediation完成；final reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=`S5-BEHAVIOR-RED VERIFIER PASS`；已commit | `385c4c350ceee98947c65da5e3f63384804c69e1` | 五项finding全部闭环；CCD final-angle zero-position-angular-correction oracle、free-rotation positive mass/inertia与可逆`K`、nonzero-COM非零lever均已补强；早期`can_sleep=false` fixture harness failure已记录并修复；unit仅8个批准`E0425`/四个planned helpers，13条integration均`running 1 test`且保持10 RED/3 GREEN；API focused=`6 passed`、row-zero checkpoint=`joint_count=1;joint_row_count=0`、lifecycle=`6 passed`；fmt/scope self-test/exact-4/diff/solver/lab/Cargo/cached/untracked/generated hygiene均PASS；一次只读status `rg`反引号shell展开噪声已用单引号安全复跑PASS且无文件修改；solver未提前开始；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-14 | S5-SOLVER | `385c4c350ceee98947c65da5e3f63384804c69e1` | `STOPPED / FROZEN CONTRACT CONFLICT`；worker与独立reviewer均确认命中§15 stop condition；2026-07-15用户已选择superseding replan，旧node仍永久STOPPED | `NOT COMMITTED` | focused unit=`8 passed`、13条integration=`7 GREEN / 6 FAIL`；two-dynamic=`0.033333`、nonzero-COM=`0.056089`、CCD drift=`0.021701/0.019774`、projection on/off contact fixture按full dimensions存在`1.25` gap且均`contact_frames=0`；reviewer=`2 High / 0 Medium / 0 Low`；production diff已恢复，未改测试/阈值/contact/phase，未stage/commit/push；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-15 | S5-REPLAN | `385c4c350ceee98947c65da5e3f63384804c69e1` | 首轮docs reviewer=`2 High / 3 Medium / 1 Low`、裁决`FAIL`；bounded remediation已完成；final reviewer=`0 High / 0 Medium / 1 Low`、裁决`PASS`，唯一Low `A05-A10d -> A05-A10e`已闭合；independent verifier首轮因stale reviewer status裁决`FAIL`，status-only remediation已完成；re-verifier=`S5-REPLAN RE-VERIFIER PASS`；已commit | `d987d1f7a85964f8c26121bc70b2a96b7f406f6e` | exact-2 design/living spec；逐finding与verifier历史见§9A；保留旧STOPPED历史，新增ADR-S5-5、唯一new chain、RED-2/SOLVER-2 ownership/scope/gates/STOP合同；worker receipt见§9A；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-15 | S5-BEHAVIOR-RED-2 | `d987d1f7a85964f8c26121bc70b2a96b7f406f6e` | 保留两轮reviewer `FAIL`与bounded remediation历史；final reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=`S5-BEHAVIOR-RED-2 VERIFIER PASS`；已commit | `2bb9f90efdc373344d1b618b5b1a295a56de54a8`；scope state-aware follow-up=`07b2bd6cd3ff4b80d1b6c124a4752e36e8e8d88f` | exact-4 acceptance与scope follow-up均已落地；unit仅8个批准`E0425`/四helper；16条exact逐条`running 1 test`且`12 RED / 4 GREEN`；API6、row-zero、lifecycle6、world19、scope/fmt/diff/zero-diff/cached/untracked均PASS；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-15 | S5-SOLVER-2 | `07b2bd6cd3ff4b80d1b6c124a4752e36e8e8d88f` | `STOPPED / TEST CONTRACT CONFLICT`；independent reviewer=`CONFLICT CONFIRMED` | `NOT COMMITTED` | accepted full post 2x2 patch使new latest exact pose error=`0/0`、row=`1/1`；两条旧absolute-angle oracle分别angle error=`0.105111/0.095758`但drift=`0.001104/0.000917 <= 0.01`，只败旧angle equality；production patch保留主worktree作证据、不属于RED-3；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-15 | S5-BEHAVIOR-RED-3 | `07b2bd6cd3ff4b80d1b6c124a4752e36e8e8d88f` | test/spec reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier首轮因stale progress review状态以`0 High / 1 Medium / 0 Low`裁决`FAIL`；status-only remediation后re-verifier=`0 High / 0 Medium / 0 Low`，结论`S5-BEHAVIOR-RED-3 RE-VERIFIER PASS`；已commit | `6ffa1e3f29e902b68708b62e11d5fae160db97ec` | exact-3；首轮及re-verifier确认unit仅8个批准`E0425`、16条integration逐条`running 1 test`且严格`12 RED / 4 GREEN`、两条superseding exact signature、scope self-test/exact-3/fmt/diff/全部zero-diff与hygiene均PASS；physics/scope blobs未变化；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-15 | S5-SOLVER-3 | `6ffa1e3f29e902b68708b62e11d5fae160db97ec` | `STOPPED / TEST CONTRACT CONFLICT`；independent reviewer=`1 High / 1 Medium / 0 Low`、`CONFLICT CONFIRMED` | `NOT COMMITTED` | unit=`8/8`、16 exact=`15 GREEN / 1 FAIL`；唯一FAIL full-pose error=`0.011336`，row/warning=`1/0`、drift=`0.001104`；contact residual translation/depth=`0.00194835861/0.0487089641`、input contact/body=`1/1`，RED-3 oracle遗漏authoritative current mutation；另有`joints.rs` fmt hygiene FAIL；production patch仅留主worktree；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-15 | S5-REPLAN-2 | `6ffa1e3f29e902b68708b62e11d5fae160db97ec` | docs worker=`S5-REPLAN-2 WORKER COMPLETE`；independent reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=`0 High / 0 Medium / 0 Low`、`S5-REPLAN-2 VERIFIER PASS`；已commit | `4e8bd4616e1f43e330d75212a74c459e8e693ca7` | exact-1 living spec；保持ADR-S5-5/design不变，记录new chain、WorldAnchor shadow oracle、RED-4/SOLVER-4 scopes/gates/STOP与worker receipt；review/verifier确认exact-1/zero-diff、shadow observable fields、new chain/PRE_V、STOP/progress与全部docs gates均无finding；不声称future nodes已验证；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-15 | S5-BEHAVIOR-RED-4 | `4e8bd4616e1f43e330d75212a74c459e8e693ca7` | 首轮reviewer=`0 High / 1 Medium / 0 Low`、裁决`FAIL`；bit-exact float equality bounded remediation已完成；final re-reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=`0 High / 0 Medium / 0 Low`、`S5-BEHAVIOR-RED-4 VERIFIER PASS`；已commit | `29bafce59a0913be7b4d37055ab57ed5e5233116` | exact-3；显式float comparator=`1e-6 * max(1,abs(a),abs(b))`；两条baseline最大delta/tolerance=`0.048708964/0.000001000`与`0.011979997/0.000001000`；unit仅8个批准`E0425`；16条integration逐条`running 1 test`并严格`12 RED / 4 GREEN`；两条WorldAnchor shadow只命中各自批准CONTACT_STATE signature，scope/self-test/fmt/zero-diff/hygiene均PASS；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-15 | S5-SOLVER-4 | `29bafce59a0913be7b4d37055ab57ed5e5233116` | `STOPPED / TEST CONTRACT CONFLICT`；worker命中§15，supervisor fresh复跑同一唯一FAIL；未派reviewer/verifier | `NOT COMMITTED` | exact-6保留accepted production patch；receipt/scope与unit 8/8 PASS；第一条shadow GREEN，第二条因WorldAnchor shadow 1 ULP under-correction导致tangent impulse delta=`0.000020315 > 0.000001`、`contact_equivalent=false`；row/warning=`1/0`、pose error=`0`、drift=`0.000917`；实验已撤销；完整16条/broad未运行，fmt仍FAIL；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-15 | S5-REPLAN-3 | `29bafce59a0913be7b4d37055ab57ed5e5233116` | 首轮independent spec reviewer=`4 High / 4 Medium / 1 Low`、FAIL；bounded remediation后final re-review=`0 High / 0 Medium`、PASS；independent docs verifier=`0 High / 0 Medium / 0 Low`、`S5-REPLAN-3 DOCS VERIFIER PASS`；supervisor commit gate unlocked | `PENDING` | clean detached exact-8；导入living blob=`6bc4dc8ff9f506c5e13bb048afd1da6b0db3b56f`、design baseline blob=`f945b0f530191b4c1de0921aa930146210e723ce`及6个OpenSpec paths；OpenSpec 4/4、strict validate、apply 4/24 pre-status、YAML/fences/terms/exact-8/outside-zero-diff/main blobs/diff/cached均PASS；commit identity由RED-5 receipt登记；Chrome `NOT RUN / BROWSER PENDING` |
| - | S5-BEHAVIOR-RED-5 | `PENDING` | 未开始；等待S5-REPLAN-3 commit | `PENDING` | exact-5；clean unit 8 approved symbol RED、16 integration=`12 RED / 4 GREEN`、pose-effect negative control、Distance contact-wake boundary与temporary production witness均待执行 |
| - | S5-SOLVER-5 | `PENDING` | 未开始；等待committed RED-5 | `PENDING` | required exact-7 + reviewer-triggered optional unit path；unit+sub-EPS、16/16、targeted/broad/fmt/scope/OpenSpec gates待执行；完成后本change hard-stop，不进入S5-LAB-RED |
| - | S5-LAB-RED | `PENDING` | 未开始 | - | 必须单独提交cross-layer contracts |
| - | S5-LAB | `PENDING` | 未开始 | - | CLI candidate commit后external browser receipt未回填则`BROWSER PENDING` |
| - | S5-V | `PENDING` | 未开始 | - | CLI三角色只读；独立external browser receipt另行回填 |
| - | S5-C | `PENDING` | 未开始 | - | docs-only closeout |

## 17. 当前残余风险

- 单次 linearized position correction 对极端大初始 anchor 误差不作额外收敛承诺；
  acceptance 只采用批准的240帧窗口。
- ADR-S5-5已完成用户批准、S5-REPLAN与RED-2 acceptance-as-code；S5-SOLVER-2 production
  behavior本身已使new latest oracle GREEN，但两条旧absolute-angle oracle与accepted full post
  corrected-eval合同互斥，因此该node是`STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`，
  不是production capability失败或可续写的in-progress node。
- RED-3修正了absolute-angle问题，但其中一条full-pose oracle又遗漏contact residual correction
  对authoritative current的真实mutation。S5-SOLVER-3因此同样是
  `STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`；三个历史solver node都不得进入
  current PRE_V chain或被改写为PASS。
- RED-4已commit；S5-SOLVER-4证明第一条shadow `contact_equivalent=true`，但第二条WorldAnchor
  shadow因`(1 / DT) * DT` f32舍入比冻结2x2 oracle低1 ULP，跨过nearest-point量化边界并把
  tangent impulse差放大到`0.000020315 > 0.000001`。production pose本身与冻结oracle一致，
  pose error=`0`、drift=`0.000917`。因此S5-SOLVER-4已按§15停止为
  `STOPPED / TEST CONTRACT CONFLICT / NOT COMMITTED`；不得增加hidden under-relaxation、修改
  committed tests/contact/narrowphase或放宽assert。当前只走用户批准的REPLAN-3/RED-5/SOLVER-5。
- 主worktree保留的`pipeline/joints.rs`仍有rustfmt diff。S5-SOLVER-5必须在owned Rust范围内使
  `cargo fmt --all --check`真正GREEN，不能以physics gate为由豁免独立hygiene失败。
- RED-5尚未实跑，pose-effect clean分类、comparator negative control与contact-woken Distance
  boundary仍是未知；尤其旧plan复用能否被第16条稳定检出必须由temporary witness证明。
- post-contact reconciliation增加一次Revolute 2x2 pose evaluation；当前无性能阈值变化，
  但S5-SOLVER-5 verifier仍需报告warning、row/stats/no-repeat、contact residual、`P_contact`、
  active-plan rebuild与determinism事实。
- sub-EPSILON实际状态变化目前可能被`>EPSILON`误判为zero；optional unit必须先锁RED，再以
  committed/current和next/current的真实差异修复，不能把所有tiny delta无条件当wake。
- 本change只完成到S5-SOLVER-5并hard-stop；S5-LAB-RED及后续browser链仍为
  `PENDING / NOT AUTHORIZED IN THIS CHANGE`。
- singular row fail closed 会跳过该phase；这是防止world污染，不是约束成功保证。
- 新 `ScenarioId` variant 对仓库外 exhaustive match 有source影响，但本次不改变其
  attribute policy。
- old schema-v1 reader不能读取new `revolute` capability；已明确记录为forward
  incompatibility。
- old closed-enum debug serde consumer不能读取`DebugJointKind::Revolute`；artifact/server
  保持authoritative kind，consumer必须升级。
- Web production bundle既有体积warning不因本milestone扩大scope；只有新增
  error/warning才阻塞browser gate。
