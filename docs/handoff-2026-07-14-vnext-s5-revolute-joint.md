# 交接：vNext §5 Revolute Joint（2026-07-14）

## 1. 当前现场

- 仓库：`/Users/asyncrustacean/projects/picea`
- 当前分支：`feat/vnext-s5-revolute-joint`
- 当前HEAD：`7fc5d32 docs: close sat manifold persistence milestone`
- 分支基线：`main=origin/main=9427a17`
- Worktree：创建本交接文件前clean；现场仍以`rtk proxy git status --short --branch`为准。
- Push：本轮没有push；用户授权了通过gate后的commit，没有授权push。
- 进程：没有需要继承的server、web dev server或后台命令。
- §5状态：只完成public API范围确认和只读证据探索；尚未创建S5 design/living spec，尚未修改任何§5 production/test代码。

## 2. 已完成的§4

Handoff §4 SAT manifold persistence已经完整实现、复审、验证并冻结，不要重新开启或重新实现。

提交链：

```text
6045bd2 docs: design sat manifold persistence
c4298ae test: lock manifold persistence matching
4a0c865 docs: design manifold attribution oracle
4a32ddb test: attribute persistent manifold candidates
cca2475 docs: record manifold attribution evidence
d9d96b0 test: distinguish absorbed manifold candidates
91698b3 fix: persist sat manifold point identity
57cdb19 chore: satisfy manifold matching clippy
7fc5d32 docs: close sat manifold persistence milestone
```

冻结事实：

- Raw `ContactFeatureId`继续表达当帧authoritative final geometry；跨帧identity由`ContactId`/`ManifoldId`承担。
- Shape/local-pose private `u64` geometry revision会使warm-start、lifecycle和source-row三类history consumer失效。
- Matching为pair-scoped exact-hard reservation，再对residual做maximum-cardinality一对一匹配；warm/lifecycle/source-row独立消费。
- Previous sensor、source contention、symmetric edge index、单侧NaN、edge-swap distractor、5x5和deterministic tie-break均有行为锁。
- S4-V：picea lib `128/128`；physics realism `79/79`；artifact `30 passed / 5 ignored`；workspace harness `369 passed / 5 ignored`；9个Criterion场景成功；clippy 0 warnings；examples和bench no-run通过。
- Matrix180：penetration max/sum `0.027232/0.927301`，0 awake/48 sleeping，quiet `0/0`，无ejection，hash `0efffe6f80f71d72`。
- Aligned1200：`0.003365/0.054014`，0/12，quiet `0/0`，无ejection，hash `34902715d547abc5`。
- Forced600：`0.027232/0.927301`，0/48，quiet `0/0`，无ejection，hash `9f5998a3db236c97`。
- Source-freeze replacement digest：`ec463132197639a58bf7b1287dcdb8d9c91059839d263d149a23530ed643a0df`。

冻结文档：

- `docs/design/2026-07-13-sat-manifold-persistence-design.md`
- `docs/plans/2026-07-13-vnext-s4-manifold-persistence-milestone.md`

§4保留Low residual，不阻塞§5：

- A08/A09没有单独锁住nonzero revision经过WorldCommands scratch clone后继续递增，也没有单独锁住revision mismatch旧IDs对应的`ContactEnded`。
- `u64` revision理论上会wrap；实际world lifetime假设不会发生。
- Residual matcher按当前每pair最多4点枚举；未来扩大manifold点数时需重新评估组合复杂度。

## 3. 用户工作规则

- 剩余任务按顺序推进：§5 revolute joint，随后§6独立design gates，再做body damping、`DistanceJointDesc.damping`和grab手感。
- 每个节点必须使用叶子subagent执行worker、reviewer和verifier职责；subagent不得再创建subagent。
- 先提交可执行行为锁并确认RED，再写最小实现；不得删除、放宽或重写失败断言来制造GREEN。
- 每个节点只有在spec/code/test reviewer无High/Medium且独立verifier通过后才能commit。
- 所有Cargo、Git和验证命令使用`rtk proxy`。
- 遇到dirty改动先确认来源；不得revert、覆盖或格式化不属于当前节点的内容。
- 可以按上述gate自主commit；不要push。
- Public API、schema、阈值或不可逆行为与批准设计冲突时必须停下，不得静默扩张。

## 4. 已批准的§5 Public API Gate

用户在2026-07-14明确选择：**完整 Pin-only V1**。

### 4.1 Core public surface

```rust
pub struct RevoluteJointDesc {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub local_anchor_a: Point,
    pub local_anchor_b: Point,
    pub user_data: u64,
}

pub struct RevoluteJointPatch {
    pub local_anchor_a: Option<Point>,
    pub local_anchor_b: Option<Point>,
    pub user_data: Option<u64>,
}
```

批准的同步surface：

- `JointKind::Revolute`
- `JointDesc::Revolute(RevoluteJointDesc)`
- `JointPatch::Revolute(RevoluteJointPatch)`
- `JointBundle::Revolute`与`JointBundle::revolute(...)`
- `DebugJointKind::Revolute`
- Lab `SceneJointFixture`的`type: "revolute"`变体
- Web joint kind union、label、i18n和UI contract
- Prelude只新增`RevoluteJointDesc`和`RevoluteJointPatch`；`JointKind`继续module-qualified，不额外扩大prelude。

Default与validation：

- Default沿用现有风格：invalid/default handles、zero local anchors、`user_data=0`。
- 两个local anchors每个分量必须finite。
- 两个body handle必须live且不同；same-body、missing或stale handle拒绝。
- Patch必须先完整验证再原子应用；失败时descriptor、revision、wake状态和row facts都不变。
- 不静默clamp NaN、Infinity或拓扑错误。

### 4.2 Compatibility decision

本次给所有相关public joint enums增加`#[non_exhaustive]`：

- Core：`JointKind`、`JointDesc`、`JointPatch`、`JointBundle`、`DebugJointKind`
- Lab：`SceneJointFixture`

现有和新增public structs不加`#[non_exhaustive]`，继续支持项目现有struct-literal风格。新增enum variant本身已经会使外部exhaustive match需要修改，因此本次是建立未来variant扩展边界的唯一合理时点。设计文档必须把这个源码兼容破坏记录为ADR。

### 4.3 Physics semantics

- Revolute V1只约束两个local anchors在world space重合。
- 相对旋转完全自由，不投影为固定角度。
- Connected bodies保持Picea现有行为，仍可产生contact；本次不增加`collide_connected`。
- 首版不公开motor、angle limits、`reference_angle`、stiffness、damping、break force或persistent joint warm-start cache。
- 不把两个scalar distance correction顺序执行来模拟pivot；必须使用包含偏心anchor和转动惯量的2x2 point constraint。
- 一个revolute仍计为一个logical joint row carrier，不能把debug/artifact的`joint_row_count`静默变成x/y两个rows。
- Position phase维持anchor coincidence；现有optional joint velocity projection启用时处理relative point velocity。不要借§5合并contact/joint row streams。
- Singular或non-finite effective mass必须fail closed，不产生NaN impulse或numeric warning。

### 4.4 Lifecycle wake semantics

作为§5 prerequisite correctness，对所有joint kinds统一：

- 成功create joint后唤醒受影响的dynamic endpoints。
- 成功patch约束字段后唤醒受影响的dynamic endpoints；只改`user_data`不唤醒。
- 成功remove joint后唤醒受影响的dynamic endpoints；body cascade删除joint时也要处理仍存活的dynamic counterpart。
- `WorldCommands`只在整个transaction成功commit后产生wake副作用；rejected transaction不得泄漏wake或部分patch。

### 4.5 Lab scope

- 完整V1包含lab fixture、一个可复现的revolute pendulum/hinge场景、artifact/debug facts、server读取路径、Web kind/label和真实browser acceptance。
- Fixture envelope保持现有schema v1，因为文档结构没有变化，只增加tagged capability。旧reader遇到`"revolute"`会明确拒绝unknown variant；不要伪造旧reader兼容。
- 本轮不增加generic live joint patch protocol，也不增加motor/limit UI。

## 5. 当前代码证据与路由

三个只读explorer/research任务已经完成，但其session不能由下一Codex继承；关键结论已固化在本文件。

### 5.1 Core API和存储

- `crates/picea/src/joint.rs`：现有`DistanceJointDesc`、`WorldAnchorJointDesc`、public enums、private `JointRecord`、validation和patch exhaustive matches。
- `crates/picea/src/lib.rs`：prelude当前只重导出Distance/WorldAnchor desc/patch/view。
- `crates/picea/src/world/api.rs`：create/patch/remove joint，same-body与live handle topology validation。
- `crates/picea/src/recipe.rs`：public `JointBundle`、recipe body-index resolution、transactional commands。
- Public enums目前都没有`#[non_exhaustive]`；新增variant和该attribute均是明确public source compatibility gate。

### 5.2 Solver和island

- `crates/picea/src/pipeline/joints.rs`：private `JointSolverRow`、position phase和optional velocity phase；当前没有retained joint impulse cache。
- `crates/picea/src/pipeline/island.rs`：joint solve-plan ordering和dense body slots。
- `crates/picea/src/pipeline/sleep.rs`：joint graph已通过`body_handles()`泛化，但joint lifecycle wake存在既有缺口。
- `crates/picea/src/solver/body_state.rs`：现有pair position helper只做平移，不足以实现偏心revolute；point impulse helper已经支持inverse inertia。
- `crates/picea/src/pipeline/step.rs`的live顺序是velocity integration、CCD、mandatory joint、contact、optional joint velocity projection、position integration、final contact、sleep。
- `docs/design/solver-island-ordering-contract.md`仍有与live step不一致的旧contact-first叙述。§5必须同步合同，但不能借机unify streams。

### 5.3 Debug、fixture和Web

- `crates/picea/src/debug.rs`：public serde `DebugJointKind`和通用two-anchor `DebugJoint`；通用字段足够表达revolute。
- `crates/picea/src/events.rs`：通用joint create/remove event，无需新增event schema。
- `crates/picea-lab/src/scenario/fixture.rs`：schema v1 tagged joint fixture，目前只有distance/world_anchor。
- `crates/picea-lab/src/artifact.rs`、`server.rs`：直接投影snapshot joints和logical row count。
- `crates/picea-lab/web/src/types.ts`：joint kind union目前只有`"distance" | "world_anchor"`；新增debug kind必须同步TS exhaustive consumers和i18n。

### 5.4 成熟实现先例

- Rapier joint guide：https://rapier.rs/docs/user_guides/rust/joints/
- Rapier revolute source：https://github.com/dimforge/rapier/blob/v0.29.0/src/dynamics/joint/revolute_joint.rs
- Box2D revolute definition：https://github.com/erincatto/box2d/blob/v3.1.1/include/box2d/types.h#L766-L842
- Box2D solver：https://github.com/erincatto/box2d/blob/v3.1.1/src/revolute_joint.c
- Avian revolute API：https://docs.rs/avian2d/0.7.0/avian2d/dynamics/joints/struct.RevoluteJoint.html
- Avian solver：https://github.com/avianphysics/avian/blob/v0.7.0/src/dynamics/solver/xpbd/joints/revolute.rs

共同结论：基础pivot使用两端local anchors；limits、motor、softness/damping和runtime warm-start state都是独立能力，不应混入Picea首版public descriptor。

## 6. 下一Codex的第一项工作

严格只先完成S5-D docs，不写production或tests：

1. 新建设计：`docs/design/2026-07-14-revolute-joint-v1-design.md`
2. 新建living spec：`docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md`
3. 最小同步parent vNext plan、本文handoff、`docs/ai/index.md`、`repo-map.md`、`doc-catalog.yaml`和`docs/design/README.md`。
4. Architecture doc至少包含：Facts/Inference/Decision、Software Interface Spec字段表、default/validation/error/compat示例、Mermaid data flow、Mermaid Body-Joint ER、2x2 effective-mass公式/伪代码、`#[non_exhaustive]` ADR、fixture schema v1 ADR、wake semantics、module ownership、acceptance matrix和里程碑交接合同。
5. S5-D经architecture/spec/routing reviewer无High/Medium、docs verifier通过后单独commit，才可进入S5-API-RED。

推荐执行链：

| Node | 目标 | 关键gate |
| --- | --- | --- |
| S5-D | Architecture、living spec、routing和父计划addendum | Docs review、YAML、scope、crate zero-diff |
| S5-API-RED | 提交外部temp-crate compile fixture，证明批准surface当前不存在 | Fixture复制到批准temp目录并调用`rtk proxy cargo check`；RED不阻塞workspace编译 |
| S5-API | Public enums/desc/patch/bundle、storage/validation、debug/fixture/TS skeleton、lifecycle wake | Compile fixture GREEN；API/lifecycle reviewer；checkpoint commit |
| S5-BEHAVIOR-RED | 提交runtime行为锁 | Shared anchor、free rotation、off-center inertia、static/dynamic、determinism、contact/island、wake稳定RED |
| S5-SOLVER | 2x2 position/velocity point constraint和island row | Core behavior GREEN；无solver stream合流 |
| S5-LAB-RED | 提交scenario/artifact/server/web contract RED | Rust artifact、TS、i18n、UI contract先RED |
| S5-LAB | Fixture、scenario、artifact/debug、server/Web label和browser体验 | Rust/lab/web/browser GREEN |
| S5-V | Full workspace、clippy、examples、bench、lab、browser、scope | Independent verifier PASS |
| S5-C | Living spec、parent、handoff、routing closeout | Docs reviewer/verifier PASS后commit |

S5-API-RED不能直接提交会破坏workspace编译的integration test。使用嵌套fixture源码和验证工具：工具将fixture复制到`/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode`，在temp中运行`rtk proxy cargo check`，RED/GREEN均不生成repo内`Cargo.lock`或`target`。

## 7. 最低行为验收

- Public prelude compile、create/view/kind/debug projection。
- Atomic create/constraint patch/user-data patch/remove/body cascade与wake semantics。
- NaN/Infinity、same-body、missing/stale handle rejection无副作用。
- Recipe与schema v1 fixture JSON roundtrip，body indices/anchors/user_data不丢失。
- Island ordering、dense slots、one logical joint row。
- Two-dynamic 240帧：全窗anchor drift `<=0.03`，final `<=0.01`，无numeric warning。
- Free relative rotation：60帧相对角变化`>=1.0 rad`，不得投影为零。
- Static/dynamic：static pose bit-exact不变，dynamic旋转`>=0.5 rad`，anchor drift满足同一上限。
- 同场景双跑逐帧snapshot/artifact hash完全相同，所有pose/velocity/anchors finite。
- Mixed contact+revolute island同时有contact rows和`joint_row_count==1`，保持separate streams。
- Connected bodies仍可产生contact。
- Lab scenario artifact保留revolute kind/anchors/row facts；server和Web不把unknown kind降级为distance/world_anchor。
- TS type、i18n、UI contract、production build和真实browser pendulum/hinge展示通过。
- Final：`cargo fmt --all --check`、core/lab/full workspace、clippy 0 warnings、examples no-run、bench no-run、scope/public API审计全部通过。

## 8. 可直接交给下一Codex的Prompt

```text
在 /Users/asyncrustacean/projects/picea 继续工作。先读 AGENTS.md、docs/ai/repo-map.md、docs/ai/index.md，以及 docs/handoff-2026-07-14-vnext-s5-revolute-joint.md。所有回复用简体中文，代码/命令/标识符保持原文。

先用 rtk proxy git status --short --branch、rtk proxy git rev-parse --short HEAD、rtk proxy git log --oneline -12 确认现场。预期 branch=feat/vnext-s5-revolute-joint、HEAD=7fc5d32，handoff commit之后则以live HEAD为准；worktree应clean。不要重新做已冻结的§4，不要push。

必须加载并遵循 picea-milestone-runner、software-architecture-design、spec-driven-development、picea-doc-routing。用户已明确批准§5“完整 Pin-only V1”public API，具体字段、non_exhaustive边界、wake semantics、lab范围和延期项都冻结在handoff中，不要重新提问或擅自扩大。

当前只执行S5-D：创建 docs/design/2026-07-14-revolute-joint-v1-design.md 和 docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md，并最小同步parent/handoff/AI routing/design index。不要修改crates、tests、Cargo或Web代码。Design必须包含接口spec、两张Mermaid、2x2 point constraint算法、compat ADR、fixture v1决策、wake语义、acceptance matrix和S5-D -> API-RED -> API -> BEHAVIOR-RED -> SOLVER -> LAB-RED -> LAB -> V -> C执行链。

每个节点使用叶子worker/reviewer/verifier；从S5-API-RED起先确认RED并commit behavior locks；所有High/Medium闭环且独立verifier通过后才commit。所有Cargo/Git验证走rtk proxy。S5-D完成review、docs verifier和commit后，按living spec进入S5-API-RED；不能跳过RED直接实现revolute solver。

用户授权满足gate后的commit，但没有授权push。遇到与handoff已批准public surface冲突、需要motor/limit/damping/collide_connected、修改solver stream或schema version时立即停下报告。
```
