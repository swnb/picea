# 交接：picea vNext 未完成项（2026-07-11）

## 最新 closeout 状态（本次集成）

- 当前集成分支为 `feat/vnext-s1-s3-integration`，integration base 为 `main=247fbda`；文档提交前 source HEAD=`b1f1515`，本区块随后由 docs closeout commit 承载。现场事实仍以当前 `git status` 和提交历史为准。
- §1 velocity-first + CCD 前置正式化已完成实现、复审及 Rust/lab/web/真实浏览器 E2E，并通过 `44931fe` 进入当前集成分支。
- §2 的 `WorldAnchorJointDesc.damping` 语义 bug 已完成正确性修复、复审及 E2E，提交 `247fbda` 已在当前分支祖先中；grab 主观手感调参、body damping、`DistanceJointDesc.damping` 仍未做。
- §3 Point/Vector equality contract 已完成实现、RED/GREEN、复审及端到端消费者验收，并通过 `5b9ba37` 进入当前集成分支。
- 组合验收已完成：targeted 12/12、full 15/15、Web contracts/build 与真实浏览器 online/offline 路径均通过；服务已全部停止，Git hygiene 与 `lib.rs` zero-diff 边界通过。
- 首轮 reviewer 的 1 个 Medium（CCD-clamped body 丢失整步角度积分）已由行为锁和最小修复 `b1f1515` 闭环，复审无 High/Medium；保留 Low：artifact final-geometry candidate 命名可能被误读为 solver-start eligibility，仅作为诊断解释风险保留。
- 本次仅完成 handoff §1-§3 集成 closeout，不代表整个 E1/E2/E5、V8、E6 或 C9 完成；§4 narrowphase `#[ignore]`、§5 revolute joint、§6 其余独立 design gate 仍开放，范围与风险没有因本次集成而缩小。
- 下方正文是 `HEAD=2178902` 时的历史交接快照，不删除也不改写其证据；发生状态冲突时，以本区块和当前 Git 事实为准。

> 面向接手的 agent（codex）。这份文档自包含：读完即可上手。所有 `file:line` 已在 `HEAD=2178902`（branch `main`，与 `origin/main` 同步）核实。

## 0. 先读什么 / 工作纪律（不可跳过）

权威顺序（来自 `AGENTS.md`）：

1. **当前工作区事实和验证命令输出**——以 `git status` 和实际命令输出为准，文档与代码冲突时以仓库现状为准。
2. `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`——生产 milestone 边界与验收门。
3. `docs/ai/repo-map.md`——每个模块的 owns / does-not-own 边界、入口文件、targeted test。
4. `docs/ai/index.md` + `docs/ai/doc-catalog.yaml`——问题类型路由 + 文档状态清单。
5. 调物理回归前先读 `docs/ai/debug-playbook.md`。

硬约束：

- **所有验证命令加 `rtk proxy` 前缀**：`rtk proxy cargo test -p picea --lib` 等。
- **先锁后写**：先补失败测试 / 行为锁（acceptance-as-code）证明现状为红，再写最小实现变绿。不为让测试通过而放宽或删除既有断言——若必须改既有断言，停下来上报，不要自行决定。
- **public API 零变更**：`crates/picea/src/lib.rs:15-53` 的 `prelude` 是 public beta surface（见 §6 完整清单）。新增/删除/重命名任何 `pub` 字段、类型、签名、serde 字段名都属高风险门，需先确认。
- **dirty 保护**：看到不是自己这轮改出来的 dirty / worktree 内容，先确认来源；不要 revert、覆盖、格式化、删除。
- **声称完成前必须真的跑过验证命令**，以输出为准。

仓库现状锚点：`main=2178902` 与 `origin` 同步；上一批"遗留收尾"（架构安全批次 + grab 修复 + trybuild 漂移 + Dependabot + trajectoryOverlay + step-reorder spike 收口）已全部完成并 push。本文件列的是**其后仍未做**的项。

---

## 1.【P1，有 spike 基础】步进重排正式化（velocity-first + CCD 前置）

**这是最有价值、且已有探索基础的项。** 完整结论已固化在 `docs/plans/2026-06-17-physics-realism-vnext-milestones.md` 的「2026-07-11 - Spike：步进重排」进度记录条，先读它。

- **spike 分支**：`spike/step-reorder-integrate-after-solve`；worktree `.claude/worktrees/step-reorder-spike`；2 commit（`ca339f7` variant A 先解速度后积分；`a51e801` scheme G' 预测性 CCD keeps traces in-step）。
- **已证实**：步进顺序是塔稳定性的真实杠杆——红锁 17→3、塔 6/6 sleep、倾角 0.21°（此前数度倾覆）。重排后顺序：`速度积分 → CCD pose_clamp(用 dt 预测推进) → 关节 → 接触碰撞+求解 → 位置积分 → sleep`（对比当前 main 的 `joint solve -> CCD -> contact phases -> ... -> sleep`）。
- **剩 3 red，同一根因**（velocity-first 下 contact 阶段看到未做位置积分的 pose，慢速接触检测/warm-start 滞后一步）：
  1. `sleeping_body_wakes_on_contact_solver_impact` — `crates/picea/tests/physics_realism_acceptance.rs:2717`（10m/s 子弹撞击单步内无 `ContactStarted/Persisted` 事实）。
  2. `warm_start_cache_transfers_tangent_impulse_across_small_tangential_slip` — 同文件 `:1172`（`Hit`→`DroppedPointDrift`，drift 判定基点偏移）。
  3. `matrix_stack_artifacts_capture_nxm_grid_stack_facts` — `crates/picea-lab/tests/artifact_run.rs:4283`（`source_row_continuity_candidate` 计数为 0）。
- **裁决（已定，勿推翻）**：**不校准这 3 个锁**。#2/#3 指向接触连续性退化（warm-start 命中率下降、帧间接触断裂），校锁会掩盖 velocity-first 的真实代价并反噬堆叠收敛。
- **正式化前置**：① 让 contact 阶段在 velocity-first 下看到**预测位置**——把 CCD 预测推进从"仅穿隧物体"扩到所有接近中的接触，或 narrowphase 前做一次预积分快照——消除滞后并清掉 3 red；② **spike 基线落后于 main**（缺 `909e386` grab 修复、trajectoryOverlay、vite bump），合入前必须 rebase，否则回退已 push 修复；③ 先补 phase-order / 接触检测时点 acceptance 锁定义新语义，再实现。
- **归属**：vNext E1（堆叠稳定性）+ E5（solver ordering），见 `docs/plans/2026-06-17-physics-realism-vnext-milestones.md:137` / `:237`。
- **风险级别**：**高风险门**——改 core 接触检测时点是写路径深层契约改动，走 vNext 正规流程（先锁、spec 认可、bounded worker）。
- 验证门：`rtk proxy cargo test -p picea --test physics_realism_acceptance`、`rtk proxy cargo test -p picea-lab --test artifact_run`、`rtk proxy cargo test -p picea --lib`、`rtk proxy cargo clippy -p picea --all-targets`。

---

## 2.【P2】grab spring `damping` 语义 + 手感调优

- **参数默认**：`crates/picea-lab/src/server.rs:272-274`（`DEFAULT_GRAB_STIFFNESS=40.0`、`DEFAULT_GRAB_DAMPING=2.0`、`DEFAULT_GRAB_MAX_SPEED=40.0`）。装配在 `create_grab`（server.rs:671）的 `GrabMode::Spring` 分支（server.rs:720），构造 `JointDesc::WorldAnchor(WorldAnchorJointDesc { stiffness, damping, ... })`（server.rs:723-730）。
- **必须先解决的语义 bug**：`WorldAnchorJointDesc.damping` 被存储和校验（`crates/picea/src/joint.rs:290` 要求 `>=0` 且 finite），但**核心 solver 并不消费它**——WorldAnchor 位置校正 `crates/picea/src/pipeline/joints.rs:113-134` 只用 `desc.stiffness.max(0.0) * dt`，且 WorldAnchor 无速度约束。所以 grab 的 `damping=2.0` 目前对仿真**无实际效果**。
- `max_speed` 不是 joint 字段，是 lab-server 侧概念：仅在 `update_grab` clamp 抓取目标速度（server.rs:1556-1557）。
- **建议路线**：调"手感"之前先定 `damping` 语义——要么在 core 给 WorldAnchor 加速度阻尼约束真正消费 `damping`（改 core solver，需行为锁 + 可能碰 public 语义），要么明确文档化"WorldAnchor 是纯位置弹簧、damping 不支持"并从 grab 移除该参数。**先决定语义，再谈参数调优**，否则调的是没接线的旋钮。
- 验证门：`rtk proxy cargo test -p picea-lab --test server_routes`；浏览器 live session 三面验收（真实 dev URL 拖拽手感）。

---

## 3.【P2，需 spec 决策】fuzzy `PartialEq` 违反 `Eq`/`Hash` 契约

- **证据**：`crates/picea/src/math/point.rs:20-25` 与 `crates/picea/src/math/vector.rs:20-25` 手写 `PartialEq` 用 `abs() < FloatNum::EPSILON`（`FloatNum`=f32）逐分量模糊比较。两个类型 derive 行（point.rs:8 / vector.rs:8）都**不含 `Eq`/`Hash`**，全库无 `impl Eq`/`impl Hash`。
- **影响**：epsilon 比较非传递（a≈b, b≈c ⇏ a≈c），违反 `Eq` 契约；`Point`/`Vector` 因此**不能作 `HashMap`/`HashSet`/`BTreeMap` key**，任何依赖 `Eq` 契约的容器都被此设计排除。
- **参考先例**：上一批次已把 `crates/picea/src/collider.rs:878` 附近的几何缓存 freshness key 从 `Pose == Some(world_pose)`（经 `Point` 模糊比较）改成**位精确**（`to_bits()` 比较）——见 `.superpowers/sdd/fix-plan.md` Task 1e。同样模式可复用。
- **属明确设计门**（`.superpowers/sdd/fix-plan.md:5` 点名排除项）。**需 spec 决策**：是保留 epsilon 语义（几何近似判等），还是引入 bit-exact `Eq`/`Hash`（供容器用，与几何判等分离为两个方法）。
- **public API 触点**：`Point`/`Vector` 在 prelude（`lib.rs:38` math 行）——改 trait impl 属 public 语义变更，高风险门。

---

## 4.【P3，依赖 manifold-persistence 设计】narrowphase `#[ignore]` 红锁

- **测试**：`stacked_rectangles_keep_feature_id_when_sat_reference_face_swaps`，`crates/picea/src/pipeline/narrowphase.rs:1232`（`#[ignore]` 属性 :1231，文件内联 `mod tests`，非 `tests/` 集成测试）。全 `crates/picea/` 唯一的 `#[ignore]`。
- **ignore 原因（逐字）**：`diagnostic red lock for future SAT manifold-persistence design; direct canonicalization regresses matrix_stack`。
- **意图**：SAT reference/incident face 角色互换时，stacked rectangle 的 ordered-pair feature id 不应 churn（:1268-1272 feature_indices 相等断言）。
- **姊妹通过测试**（同主题）：`single_end_clipped_incident_edge_keeps_distinct_point_feature_ids`，narrowphase.rs:1202。
- **约束**：ignore 文案明说 direct canonicalization 会回归 `matrix_stack`——**不要直接强行 canonicalize**。这是 manifold-persistence 设计的一部分，需先有持久化设计再动。属探索性正题，非机械修复。

---

## 5.【P3，功能扩展，从零】revolute joint

- **现状**：当前**只有两种 joint，revolute 完全不存在**（`revolute`/`hinge`/`pivot` 全库 grep 零命中，无 stub/TODO）。
  - `enum JointKind`（`crates/picea/src/joint.rs:80`）：`Distance`、`WorldAnchor`。
  - `enum JointDesc`（joint.rs:89）、`struct DistanceJointDesc`（joint.rs:13）、`struct WorldAnchorJointDesc`（joint.rs:49）。
  - solver 入口 `enum JointSolverRow`（`crates/picea/src/pipeline/joints.rs:19`）同样只有 Distance/WorldAnchor；`solve_joint_phase`（:31）位置校正、`solve_joint_velocity_phase`（:47）速度阶段（`apply_joint_velocity_constraints` joints.rs:140 **只处理 Distance 行**）。joint 求解全内联在 `pipeline/joints.rs`，`solver/` 下无独立 joint 文件。
- **需从零新增**：`JointKind`/`JointDesc`/`JointPatch` 变体 + `RevoluteJointDesc`/`Patch` struct + `JointSolverRow` 变体 + 位置/速度约束实现 + island plan 支线 + prelude 扩展。
- **归属**：vNext E5（关节、约束与 solver ordering，`docs/plans/2026-06-17-physics-realism-vnext-milestones.md:237`）。
- **public API 触点**：prelude joint 行（`lib.rs:34-37` 当前只列 `Distance*`/`WorldAnchor*`）+ `joint.rs` 枚举——**突破 prelude 零变更约束**，是主要 public surface 触点。
- **建议**：先补铰链行为锁（约束两 body 共享锚点、放开相对旋转），证明红，再实现。

---

## 6.【各需独立 design gate】其余设计门项

`.superpowers/sdd/fix-plan.md:5` 明确列为"属 public API / 设计门，不在安全批次"的剩余项，各需独立立项：

- **solver 启发式清理**——contact solver（`crates/picea/src/solver/contact.rs`）里的经验参数/条件分支，被历次标为最大技术债；需先有基准行为锁再动。
- **`DebugSnapshotOptions` 字段化**——`DebugSnapshotOptions` 在 prelude（`lib.rs:20` debug 行），public 类型，字段化是 API 变更。
- **`ContactEvent` / `DebugContact` 合并**——两者都在 prelude（events 行 / debug 行），合并是 public schema 变更。
- **live 诊断填充**——vNext E6（观测、lab artifact、browser 验收，`docs/plans/2026-06-17-physics-realism-vnext-milestones.md:265`）的正题。

---

## 7. 建议动手顺序

1. **grab `damping` 语义**（§2）——范围小、边界清晰、直接影响 live 手感，是最快能落地的正确性修复（先定语义再调参）。
2. **step-reorder 正式化**（§1）——收益最大且有 spike 基础，但高风险门，需 vNext E1/E5 正规流程 + spec 认可；从"补接触检测时点 acceptance 锁"起步。
3. **fuzzy PartialEq**（§3）——明确的正确性债，但需 spec 先定容器用法方向。
4. revolute joint（§5）/ narrowphase ignore 锁（§4）/ 其余设计门（§6）——各自独立立项，按 vNext milestone 排期。

每项动手前：`git status` 确认现场 → 读 `docs/ai/repo-map.md` 定位模块 owner → 补失败测试 → 最小实现 → 过 targeted gate。
