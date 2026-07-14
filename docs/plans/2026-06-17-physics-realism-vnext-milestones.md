# Picea Physics Realism vNext 里程碑计划

状态：已批准
计划文档：docs/plans/2026-06-17-physics-realism-vnext-milestones.md
最后更新：2026-07-14
工作目录：/Users/asyncrustacean/projects/picea
工作区状态：§1-§3 integration记录保留历史base；§4已冻结；handoff §5 S5-D review/verifier已通过、commit待完成，API/solver未开始，Chrome `NOT RUN / BROWSER PENDING`；执行时仍以live `git status`为准
计划重量：heavy
Goal 协调：planning/execution goal active；用户已明确预授权 Plan Gate
提交策略：源实现提交已进入本次集成分支；本区块由后续独立 docs closeout commit 承载
执行策略：完整计划预授权后连续执行；遇高风险门、现实冲突、验证阻塞时停
高风险门：Public API/compatibility break；`SharedShape::ConcavePolygon` hard reject；`Material` public schema；deformable public surface；删除/迁移/部署
SpecFlow：不使用
Change root：none
Profiles：architecture-heavy, api-contract, ui-browser
完成状态：D0/V0 已验证；handoff §1 velocity-first 正式化、§2 WorldAnchor damping 正确性子切片、§3 Point/Vector equality contract 已实现、复审并完成对应 Rust/lab/web/browser 端到端验收；§4已完成并冻结；§5仅进入独立S5-D文档节点；这不代表整个 E1/E2/E5/E6 或其余执行里程碑已完成；已归档 no；可发布 no

## 本次集成收尾

### 范围与提交链

- 本次只收口 handoff §1-§3：§1 velocity-first 正式化、§2 WorldAnchor damping 正确性子切片、§3 Point/Vector equality contract；不代表整个 E1/E2/E5、V8、E6 或 C9 完成。
- integration base 为 `main=247fbda`；source history 包含 `a83e3d9`、`e7d906f`、正式化提交 `44931fe`，以及 equality cherry-pick `5b9ba37`；reviewer 修复提交为 `b1f1515`。
- 文档提交前 source HEAD=`b1f1515`；本区块随 docs closeout commit 落地，避免把提交前 source HEAD 误写为最终文档 HEAD。

### Reviewer 闭环

- 首轮 reviewer 有 1 个 Medium：`integrate_body_positions` 对 CCD-clamped body 整体跳过 final position phase，而 predictive CCD 只提前推进平移，导致命中帧丢失整步角度积分。
- 行为锁 `ccd_clamped_dynamic_circle_preserves_full_step_angular_integration` 先 RED：exit `101`，`angle=0`，`expected_angle=0.10000001`；最小修复只跳过 clamped dynamic body 的二次线性平移，保留 `angular_velocity * dt` 与既有 invalid-pose containment，随后 exact test 为 `1 passed`。
- 复审无 High/Medium。保留 1 个 Low：`artifact_run` 的 final-geometry candidate 诊断命名可能被误读为 solver-start eligibility；这是诊断解释风险，本任务不扩 scope 修改。

### Rust 与 Lab 验收

- Targeted 12/12 均 exit `0`：physics `71 passed / 0 failed`；artifact `27 passed / 5 ignored`；math algebra `3 passed`；geometry cache `1 passed`；math lib `4 passed`；math compile-fail harness `1 passed`；picea lib `109 passed / 1 ignored`；server routes `20 passed`；Clippy clean；`crates/picea/src/lib.rs` zero diff。
- Full 15/15 均 exit `0`：workspace `339 passed / 6 ignored / 0 failed`；trybuild 38 个内部 case 均通过；Criterion 9 个 scenario 均 Success；picea-lab aggregate `82 passed / 5 ignored`；workspace check/Clippy、examples no-run 与 bench no-run 均通过。

### Web 与浏览器验收

- `npm ci` added 188 packages、audited 189 packages、0 vulnerabilities；UI contract、i18n、profile、production build 与 `just picea-lab-web-check` 均通过。Vite 只有既有的非阻塞 `>500 kB` chunk warning。
- 真实 API `http://127.0.0.1:8080` / Web `http://127.0.0.1:5173`：pause/single-step 从 715 到 716；WorldAnchor live 运行超过 5486 step，保持 1 dynamic / 1 joint；soft-spring pointer drag 将物体从原点拉到约 `(2.9, -1.3)`，拖拽时出现第二个 grab WorldAnchor joint，释放后正确回收。
- Matrix full frame 1345 / step 1346 显示 1 static + 48 dynamic、48 sleeping / 0 awake、max linear/angular 均为 0、outside floor 为 0；artifact header 的 mode/source 均为“生成产物并回放”。
- clean unusable-API fallback session 显示“演示回放”和唯一“离线演示数据 · 非真实模拟”watermark；已验收 online/offline sessions 的 console 均为 0 error / 0 warning。所有 browser sessions、API 和 Web 服务均已停止。

### Hygiene 与剩余边界

- Git-visible 仅三份 docs；playwright、`target`、`dist`、`node_modules` 均不 visible；`crates/picea/src/lib.rs` 相对 `247fbda` zero diff。
- §1-§3 integration closeout与handoff §4均已完成；§4最终实现为`91698b3`，clippy remediation为`57cdb19`，full S4-V通过。§5的S5-D design/living spec已通过review/verifier但commit待完成，全部API/solver/lab节点仍未开始，Chrome `NOT RUN / BROWSER PENDING`；§6未开始。E1其余contact/sleep、E2的body damping与`DistanceJointDesc.damping`、父E5的chain/bridge diagnostics，以及E3/E4/E6/D7/V8/C9仍未整体完成；下文范围外与残余风险继续有效。

### Handoff §4 addendum（已完成）

- Handoff §4 由独立 living spec `docs/plans/2026-07-13-vnext-s4-manifold-persistence-milestone.md` 路由；其 `S4-*` 前缀不重命名、替代或推进本计划 E4 complex-shape milestone。
- 用户已明确批准 raw SAT feature 保持 final-geometry 语义，并由 history-aware `ContactId` / `ManifoldId` 双射保证跨 reference/incident swap persistence；该裁决 supersede handoff 的 raw-id 字面目标。
- 用户已批准把shape/local-pose geometry revision invalidation纳入S4。设计、RED、oracle强化、implementation、full verification与closeout均已完成；提交链与完整证据以冻结living spec为准。最终core实现=`91698b3`，clippy remediation=`57cdb19`。

### Handoff §5 addendum（S5-D commit前）

- Handoff §5 使用独立 architecture package `docs/design/2026-07-14-revolute-joint-v1-design.md` 与 living spec `docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md`；其 `S5-*` 前缀不重命名、替代或推进本计划 E5/E6。
- 用户已批准完整 Pin-only V1 public字段、六个 `#[non_exhaustive]` enum边界、lifecycle wake、fixture schema v1、lab/browser范围和延期项。S5-D第三轮reviewer结论为`0 High / 0 Medium / 1 Low`，independent docs verifier于`2026-07-14 17:16:23 CST`给出`S5-D VERIFIER PASS`；唯一Low留到S5-C，S5-D commit仍为`PENDING`，S5-API-RED及后续implementation尚未开始，Chrome `NOT RUN / BROWSER PENDING`。
- 执行顺序固定为 `S5-D -> S5-API-RED -> S5-API -> S5-BEHAVIOR-RED -> S5-SOLVER -> S5-LAB-RED -> S5-LAB -> S5-V -> S5-C`。完整验收、stop conditions和提交边界以living spec为准。
- S5-API-RED同时提交baseline-compilable的existing Distance/WorldAnchor lifecycle runtime
  locks；external missing Revolute只算surface RED。S5 solver acceptance还覆盖active island中的
  sleeping endpoint、contact/projection更新velocity后的full-step drift和CCD-clamped final
  integration，不允许借此重排solver streams。
- 当前CLI不执行Chrome。S5-LAB与S5-V分别由living spec中的固定prompt交给外部ChatGPT App
  针对40位full commit SHA回填browser receipt；未回填只能PENDING/NOT RUN，不能称full
  browser acceptance。

## 目标

把“物理引擎真实模拟效果优化”拆成 7 条可由 subagent 执行、可验收、可回滚的路线：

1. 堆叠与接触稳定性。
2. 材质、摩擦、反弹与阻尼。
3. CCD 覆盖扩展。
4. 复杂形状、compound、concave authoring。
5. 关节、约束、solver ordering。
6. 观测、lab artifact、browser/lab-web 验收。
7. deformable / soft-body / particle/PBD 未来路线。

## 约束

- 所有验证命令使用 `rtk proxy`。
- 计划/设计阶段只改 docs，不改 `crates/**` 生产实现。
- 每个执行 milestone 先写 acceptance-as-code 或可复现验收步骤，再实现。
- 当前工作区最多一个写代码 `worker` 同时运行。
- 所有 subagent 必须是叶子 agent，不允许再启动 subagent。
- reviewer/verifier 不修代码；发现问题后由主 Codex triage，再派 bounded worker。
- lab/web 不能重算 physics；browser 验收必须明确 URL/步骤/证据。
- 不自动 commit；若后续用户明确授权提交，仍由主 Codex 在所有 gates 通过后提交。

## Subagent 输出合同

- worker：列出 changed files、实现范围、行为锁/测试、未覆盖风险；不得 stage/commit/push/branch/clean。
- reviewer：Findings first，按 High/Medium/Low 排序，带文件和行号；若无 High，明确写出。
- verifier：列出 exact command、exit code、关键输出、生成/修改文件；失败时说明 blocker 与最小复现命令。
- explorer：只读列出事实、推断、未知项和建议路线；不得把推断写成已实现事实。

## 待确认问题

### 必须确认

无。用户已明确“无需批准 gate，完全自主”。但高风险门仍按计划硬停，因为它们涉及 public compatibility 或不可逆行为。

### 可带假设推进

- 假设 `restitution > 1` 暂不改 validation，只记录 solver clamp 语义。早期验证：material milestone 只补文档/测试，不改 public API。
- 假设 direct concave hard reject 需要单独 public/API gate。早期验证：shape milestone 只补 boundary test/provenance，若要 hard reject 则停。
- 假设 deformable 本轮只做 design gate。早期验证：D7 不触碰 production code。
- 假设 browser automation 可能不可用。早期验证：E6 写明 fallback manual steps 和最小可重跑 server/web checks。

## 规划依据

- explorer：已运行。7 个方向均已返回；deformable 结论为 RFC/design gate，不进入本轮 production implementation。
- architecture design：已产出：`docs/design/2026-06-17-physics-realism-vnext-architecture.md`。
- 关键证据：`docs/ai/repo-map.md`、`docs/ai/index.md`、`docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`、`docs/design/*stability*`、`docs/design/deformable-body-roadmap.md`、`crates/picea/tests/physics_realism_acceptance.rs`、`crates/picea-lab/tests/artifact_run.rs`、`crates/picea-lab/tests/server_routes.rs`。
- 剩余未知与开放边界：future deformable V1 representation 仍需 RFC 冻结；handoff §5已进入独立S5-D文档门但实现未开始，§6与其余未完成 milestone 仍须各自进入设计、实现和验收门。§4已完成，不再作为开放项。

## 架构设计输入

- software-architecture-design：已产出：`docs/design/2026-06-17-physics-realism-vnext-architecture.md`
- 关键接口 / 架构图 / 关键取舍：
  - CCD 保持 explicit pose-clamp phase。
  - lab/web 只消费 Rust facts。
  - material public API 暂不扩。
  - joint damping 先兑现现有 field。
  - direct concave solver 不进入本轮。
  - deformable 只进入 RFC/design gate。

## 里程碑

| ID | 类型 | 状态 | 目标 | 架构归属 | 主角色 / 后续角色 |
| --- | --- | --- | --- | --- | --- |
| D0 | 设计 | 已完成 | 产出本架构设计和计划 | 全部 | main + explorer |
| V0 | 验证 | 已完成 | 跑当前基线和 doc routing 检查 | 全部 | verifier |
| E1 | 执行 | 部分完成（§1 已完成） | 堆叠/接触稳定性 acceptance-as-code 与最小实现 | contact lifecycle / position-row / sleep | worker gpt-5.4 |
| E2 | 执行 | 部分完成（§2 子切片已完成） | 材质/阻尼语义债，先兑现 joint damping | material / joint damping | worker gpt-5.4 |
| E3 | 执行 | 计划中 | CCD dynamic circle -> stationary dynamic convex target | CCD trace | worker gpt-5.4 |
| E4 | 执行 | 计划中 | compound/concave provenance 与 boundary evidence | complex shape | worker gpt-5.4 |
| E5 | 执行 | 部分完成（§1 已完成） | joint ordering 合同锁与 chain/bridge diagnostics | joint / solver ordering | worker gpt-5.4 |
| E6 | 执行 | 计划中 | lab/live/browser evidence hardening | observability | worker gpt-5.4 |
| D7 | 设计 | 计划中 | deformable / PBD RFC gate | deformable | explorer/reviewer |
| V8 | 验证 | 计划中 | 全路线 regression + review | 全部 | verifier，然后 reviewer |
| C9 | 收尾 | 计划中 | docs/AI routing 同步和验收报告 | docs | main |

### D0：架构设计与计划

类型：设计
状态：已完成

目标：把 7 个方向拆成可执行、可验收、可由 subagent 分派的路线。

输出交付物：
- `docs/design/2026-06-17-physics-realism-vnext-architecture.md`
- `docs/plans/2026-06-17-physics-realism-vnext-milestones.md`

设计验收：
- 7 个方向均有 owner、非目标、验收输入和风险。
- Plan Gate 已由用户预授权。

### V0：当前基线与文档路由验证

类型：验证
状态：已完成
来源设计：D0

目标：确认文档可解析、新增文档真实存在且被检查、当前 repo baseline 和最小 targeted gates 可运行。

验收标准：
- 新增 architecture/plan 文档必须可被命令直接读取，不能只依赖 `git diff --check`。
- `docs/ai/doc-catalog.yaml` YAML parse 通过。
- `git diff --check` 通过。
- 至少跑一组 core/lab targeted smoke，失败则记录 blocker。

验收检查：
- `rtk proxy git status --short --branch`
- `rtk proxy git rev-parse --short HEAD`
- `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- `rtk proxy ruby -e 'paths=%w[docs/design/2026-06-17-physics-realism-vnext-architecture.md docs/plans/2026-06-17-physics-realism-vnext-milestones.md]; missing=paths.reject { |p| File.file?(p) && File.size(p).positive? }; abort("missing docs: #{missing.join(", ")}") unless missing.empty?; puts "new docs ok"'`
- `rtk proxy rg -n "^# Picea Physics Realism vNext|^# Picea Physics Realism vNext 里程碑计划" docs/design/2026-06-17-physics-realism-vnext-architecture.md docs/plans/2026-06-17-physics-realism-vnext-milestones.md`
- `rtk proxy git diff --check`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run stack_artifacts_capture_lab_frame_diagnostics_with_marker_sources -- --exact`

Subagent 执行计划：
- verifier：叶子 agent；运行上述命令，报告 exact outcome、exit code、任何生成/修改文件。

### E1：堆叠与接触稳定性

类型：执行
状态：部分完成；handoff §1 velocity-first 正式化切片已完成，E1 其余范围仍开放
来源设计：D0

目标：围绕 contact lifecycle、position-row eligibility、dense pressure、sleep convergence 补行为锁和最小实现。

执行输入：
- 以 `contacts.rs` / `solver/contact.rs` / `sleep.rs` 为 owner。
- 不改 public API、row ordering、lab-web physics。

验收标准：
- 新增或更新的 acceptance-as-code 先红后绿。
- `stack_4` 保持 deterministic/quiet/support/sleep。
- `matrix_stack` artifact 仍可解释 first-bad-frame 或通过既定 acceptance。

验收检查：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4 -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface -- --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --nocapture`

Subagent 执行计划：
- worker：gpt-5.4，单写代码 worker。Ownership：`crates/picea/src/pipeline/contacts.rs`、`crates/picea/src/solver/contact.rs`、`crates/picea/src/pipeline/sleep.rs`、相关 tests。
- reviewer：只读，审查是否扩大 warm-start truth、是否改 public API、是否弱化 existing gates。
- verifier：运行 targeted gates。

### E2：材质、摩擦、反弹与阻尼

类型：执行
状态：部分完成；handoff §2 WorldAnchor damping 正确性子切片已完成，整个 E2 未完成
来源设计：D0

目标：已兑现 `WorldAnchorJointDesc.damping` 语义；后续仍需保护 body damping 行为并单独处理 Distance joint damping，不先扩 `Material`。

执行输入：
- `Material` 不新增字段。
- `restitution > 1` 只记录/测试当前 clamp 语义，不改 validation。
- `linear_damping` / `angular_damping` 是 body 既有行为，E2 至少补 regression 或确认已有 gate 覆盖。
- `WorldAnchorJointDesc.damping` 已进入 joint solver；`DistanceJointDesc.damping` 仍未实现，必须作为独立子切片推进。

验收检查：
- 新增或确认 `body_damping_reduces_linear_and_angular_velocity` 等价行为锁。
- 新增 `joint_damping_reduces_peak_radial_speed_vs_zero_damping`。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance friction`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance restitution`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance body_damping -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance joint_damping -- --nocapture`

Subagent 执行计划：
- worker：gpt-5.4。Ownership：`crates/picea/src/pipeline/integrate.rs`、`crates/picea/src/pipeline/joints.rs`、`crates/picea/tests/physics_realism_acceptance.rs`、必要 docs。
- reviewer：审查 public API drift、energy injection、damping formula 注释。
- verifier：运行 friction/restitution/body damping/joint gates。

### E3：CCD dynamic circle-vs-dynamic 窄片

类型：执行
状态：计划中
来源设计：D0

目标：新增 `dynamic circle -> stationary dynamic convex target` CCD slice，保持 explicit trace。

范围 / 不做：
- 可能触及：`crates/picea/src/pipeline/ccd.rs`、`events.rs` serde defaults（仅 additive 时）、`physics_realism_acceptance.rs`、`picea-lab/src/scenario.rs`、`artifact_run.rs`、bench no-run。
- 不做：rotational CCD、all-shape CCD、dynamic concave/compound support-map rewrite。

验收检查：
- 新增 positive / missed sweep / earliest dynamic hit tests。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance ccd`
- `rtk proxy cargo test -p picea-lab --test artifact_run ccd_dynamic -- --nocapture`
- `rtk proxy cargo bench -p picea --no-run`

Subagent 执行计划：
- worker：gpt-5.4。Ownership：CCD files/tests/artifact scenario。
- reviewer：重点审查 false positive、trace compatibility、scope creep。
- verifier：运行 CCD + bench no-run。

### E4：复杂形状 provenance 与 concave boundary

类型：执行
状态：计划中
来源设计：D0

目标：强化 compound/concave generated piece provenance、mass/inertia evidence，并锁住 direct concave boundary。

高风险门：
- 若要把 low-level `SharedShape::ConcavePolygon` 从可构造改成 create-collider hard reject，必须停。

验收检查：
- `rtk proxy cargo test -p picea-lab concave -- --nocapture`
- `rtk proxy cargo test -p picea-lab compound_provenance -- --nocapture`
- `rtk proxy cargo test -p picea concave -- --nocapture`
- `rtk proxy cargo test -p picea --test query_debug_contract query_shape_rejects_direct_concave_polygon_input -- --exact`
- artifact backward compatibility tests for missing provenance.

Subagent 执行计划：
- worker：gpt-5.4。Ownership：`crates/picea-lab/src/scenario.rs`、`artifact.rs`、lab/core tests；不改 core public rejection without gate。
- reviewer：审查 direct concave 边界和 provenance 兼容性。
- verifier：运行 concave/compound gates。

### E5：关节、约束与 solver ordering

类型：执行
状态：部分完成；handoff §1 solver-ordering 正式化切片已完成，E5 其余范围仍开放
来源设计：D0

目标：锁定当前真实 step 顺序，补 joint damping/chain/bridge diagnostics，不合并 row stream。

执行输入：
- 当前代码顺序权威：速度积分 -> CCD `pose_clamp` -> joint solve -> current/predicted contact discovery 与 velocity solve -> optional joint velocity projection -> 最终位置积分 -> final authoritative contact finalize -> sleep。
- M28 文档若不一致，应同步文档而不是按旧文档改代码。
- `joint_velocity_projection` 是 contact velocity solve 之后、最终位置积分之前的可选阶段；E5 reviewer 必须审查它没有被遗漏或提前。
- 不允许把 contact rows 和 joint rows 合并成单一 public/diagnostic stream。

验收检查：
- 新增整步 phase-order contract test。
- 新增 mixed contact+joint same-island runtime test。
- 新增 chain/bridge stability test。
- `rtk proxy cargo test -p picea --lib pipeline::island`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance phase_order -- --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance joint`
- `rtk proxy cargo test -p picea --test world_step_review_regressions joint`

Subagent 执行计划：
- worker：gpt-5.4。Ownership：`pipeline/joints.rs`、`pipeline/island.rs`、joint tests、solver ordering doc if needed。
- reviewer：审查 doc/code order consistency and no unified stream.
- verifier：运行 joint/order gates。

### E6：观测、lab artifact、browser/lab-web 验收

类型：执行
状态：计划中
来源设计：D0

目标：固化 live summary/full evidence contract、missing/not hydrated UI、profile/browser acceptance，并覆盖 stack/CCD/compound/lattice 的能力展示入口。

验收检查：
- `rtk proxy cargo test -p picea-lab --test server_routes live_session_step_defaults_to_summary_and_omits_full_only_payload -- --exact`
- `rtk proxy cargo test -p picea-lab --test server_routes live_session_frame_lookup_returns_full_frame_without_mutating_session -- --exact`
- `rtk proxy cargo test -p picea-lab --test artifact_run stack_artifacts_capture_lab_frame_diagnostics_with_marker_sources -- --exact`
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run test:profile`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- Browser acceptance：`rtk proxy just picea-lab-web-start`，打开 `http://127.0.0.1:5173/?picea-profile=1`，使用 Rust live session 而不是 artifact replay，先执行 summary step，再通过 paused inspection/full-frame hydrate 路径取完整 frame；记录截图/DOM/console/profile 证据，验证 summary not hydrated、full hydrate、source label 和 profile marker。
- Browser showcase coverage：至少覆盖 `stack_4` 或 `matrix_stack` 的 stack/diagnostics；`ccd_dynamic_convex_pair` 或 `ccd_dynamic_compound_wall` 的 CCD trace / trajectory；`compound_provenance` 或 `concave_decomposition` 的 provenance；`lattice_grid` 的 rigid-body proxy / not-soft-body 文案。
- Browser cleanup/check：`rtk proxy just picea-lab-web-check`；结束后 `rtk proxy just picea-lab-web-stop`。

Subagent 执行计划：
- worker：gpt-5.4。Ownership：`crates/picea-lab/src/artifact.rs`、`server.rs`、`web/src/*`、tests。
- reviewer：审查 Rust authoritative facts vs Web-derived display。
- verifier：运行 server/web/browser gates；报告 generated files。

### D7：Deformable / Soft-body RFC Gate

类型：设计
状态：计划中
来源设计：D0

目标：把 future XPBD/PBD / particle-grid / deformable handles / artifact schema 写成 RFC，不进入实现。

已确认事实：
- 当前 core public surface 仍是 rigid route，没有 deformable 专属 handle、desc、query hit 或 debug schema。
- `lattice_grid` / M38 是 rigid-body lattice proxy，不是真 soft-body solver。
- 真实 deformable 的第一步是 RFC：representation、solver phase、rigid coupling、query boundary、artifact/debug facts 和 deterministic scene lanes。
- Query/selection 是 public compatibility gate；RFC 必须先定义 whole-deformable、particle、element/constraint、collision-proxy 哪些能成为 hit/selection 语义，以及 revision-aware cache 和 lab-web selection 红线。

不做：
- 不改 `BodyDesc` / `ColliderDesc` / `JointDesc`。
- 不把 `lattice_grid` 改名为 soft-body。
- 不新增 production code。

设计验收：
- 明确 V1 representation、handles、solver phase、rigid coupling、query/selection boundary、artifact fields、browser acceptance、非目标。
- 所有 soft-body/particle 术语都与 rigid proxy 区分。

Subagent 执行计划：
- explorer：只读补证据。
- reviewer：审查是否有 false capability claim。

### V8：全路线验证与审查

类型：验证
状态：计划中

目标：每个 execution milestone 完成后跑 targeted + shared gates，审查没有 scope drift。

验收检查：
- `rtk proxy cargo fmt --all --check`
- `rtk proxy cargo test -p picea --lib`
- `rtk proxy cargo test -p picea --tests`
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy cargo test -p picea --test query_debug_contract query_shape_rejects_direct_concave_polygon_input -- --exact`
- `rtk proxy cargo test -p picea --examples --no-run`
- `rtk proxy cargo bench -p picea --no-run`
- `rtk proxy cargo test -p picea-lab --test server_routes live_session_step_defaults_to_summary_and_omits_full_only_payload -- --exact`
- `rtk proxy cargo test -p picea-lab --test server_routes live_session_frame_lookup_returns_full_frame_without_mutating_session -- --exact`
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run test:profile`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- Browser acceptance：按 E6 的 live summary -> full hydrate 和 stack/CCD/compound/lattice showcase coverage 执行；如浏览器不可用，记录 blocker 和最小人工复现步骤。
- `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- `rtk proxy ruby -e 'paths=%w[docs/design/2026-06-17-physics-realism-vnext-architecture.md docs/plans/2026-06-17-physics-realism-vnext-milestones.md]; missing=paths.reject { |p| File.file?(p) && File.size(p).positive? }; abort("missing docs: #{missing.join(", ")}") unless missing.empty?; puts "new docs ok"'`
- `rtk proxy git diff --check`

Subagent 执行计划：
- reviewer：只读 diff review。
- verifier：运行 gates。

### C9：文档同步与验收报告

类型：收尾
状态：计划中

目标：同步 design README、doc-catalog、必要的 repo-map/index，并生成验收报告。

验收检查：
- `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- `rtk proxy ruby -e 'paths=%w[docs/design/2026-06-17-physics-realism-vnext-architecture.md docs/plans/2026-06-17-physics-realism-vnext-milestones.md]; missing=paths.reject { |p| File.file?(p) && File.size(p).positive? }; abort("missing docs: #{missing.join(", ")}") unless missing.empty?; puts "new docs ok"'`
- `rtk proxy rg -n "physics-realism-vnext" docs/ai/index.md docs/ai/repo-map.md docs/ai/doc-catalog.yaml docs/design/README.md`
- `rtk proxy rg -n "场景更新|能力呈现|scenario|parameter-schema|browser-acceptance|summary-full|lattice-proxy|scene-update" docs/ai/index.md docs/ai/repo-map.md docs/ai/doc-catalog.yaml docs/design/2026-06-17-physics-realism-vnext-architecture.md`
- `rtk proxy git diff --check`
- docs contain no stale route for completed new docs.

## 计划验收

- 状态：已批准
- Plan Gate：已由用户明确预授权
- 规划审查：已运行四位只读 reviewer；第二轮文档/产品展示 findings 已修订进能力展示矩阵、E4、E6、V8、C9 和 AI 路由。
- reviewer（Codex/gpt，只读，仅作建议性预筛，不替代验收与用户确认）：reviewer-1 已运行；reviewer-2 已运行。
- 终审可执行信号：V0 已完成；后续 execution closeout 仍需 V8 全量 gates 和 E6 web/browser gates。
- review fail 说明：已补齐 V8 漏 E6、V0/C9 漏未跟踪新文档、E5 phase-order 交接不足、D7 query/selection gate 不足、E2 body damping gate 不足、E4 direct-concave query gate、E6 live summary/full browser path、`test:i18n`、以及场景能力展示矩阵。
- 用户确认：已确认预授权

## 进度记录

### 2026-06-17 - D0

- 状态：已完成
- Commit：none
- 验证：V0 已完成：
  - `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'` -> pass。
  - 新增 design/plan 文档存在性检查 -> pass。
  - 新文档 heading 检查与 AI 路由 `physics-realism-vnext` 检查 -> pass。
  - `git diff --check` -> pass。
  - `rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling -- --nocapture` -> 1 passed。
  - `rtk proxy cargo test -p picea-lab --test artifact_run stack_artifacts_capture_lab_frame_diagnostics_with_marker_sources -- --exact` -> 1 passed。
- 意外与发现：M28 文档与 live code step ordering 存在不一致；计划以 live code 为权威并安排 phase-order 合同锁。
- 风险 / 后续：deformable 只能作为 RFC/design gate；query/selection contract 是 public compatibility gate。
- 当时下一步：复跑 V0 文档验证；必要时运行 targeted smoke。

### 2026-07-11 - Spike：步进重排（velocity-first + CCD 前置）

- 状态：历史 spike 已完成并收口；其结论随后完成正式化，正式实现现已通过 `44931fe` 进入当前集成分支（关联 E1 堆叠稳定性 / E5 solver ordering）。
- 分支：`spike/step-reorder-integrate-after-solve`；worktree `.claude/worktrees/step-reorder-spike`；2 commit（`ca339f7` variant A 先解速度后积分；`a51e801` scheme G' 预测性 CCD keeps traces in-step）。
- 假设：`integrate→solve` 顺序是塔倾覆（塔身不 sleep 的 integrate-then-correct 穿透极限环）的疑似根因。
- 重排后 step 顺序：速度积分 → CCD `pose_clamp`（用 `dt` 预测推进快速物体到 time-of-impact）→ 关节求解 → 接触碰撞+求解 → 位置积分（用解出的速度）→ sleep。对比当时 E5 文档记录的旧顺序（`joint solve -> CCD -> contact phases -> ... -> sleep`），本 spike 把速度积分提到最前、位置积分挪到接触之后，CCD 从"回看 `previous pose`"改为"用 `dt` 预测"，`StepContext.previous_body_poses` 字段随之删除。
- 结论（假设证实）：塔 6/6 sleep、倾角 0.21°（此前数度倾覆）；`physics_realism_acceptance` 红锁 17→2、`picea-lab` 6→1（scheme G' 修掉 14 个）。核心门全绿：core lib 103、`world_step_review_regressions` 12、`core_model_world` 18、clippy 净、`artifact_run` 主体 35 绿。
- 代价（剩 3 red，同一根因）：velocity-first 下 contact 阶段看到未做位置积分的 pose（CCD 只预测推进被判定穿隧的快速物体），慢速接触检测 / warm-start 滞后一步。
  - `sleeping_body_wakes_on_contact_solver_impact`（`physics_realism_acceptance.rs:2717`）：10m/s 子弹撞击单步内无 `ContactStarted/Persisted` 事实，冲击响应滞后一步。
  - `warm_start_cache_transfers_tangent_impulse_across_small_tangential_slip`（`:1172`）：`Hit`→`DroppedPointDrift`，重排偏移了接触点相对锚点的 drift 判定基点，sub-threshold slip 的 warm-start 缓存被误丢。
  - `matrix_stack_artifacts_capture_nxm_grid_stack_facts`（`artifact_run.rs:4283`）：`source_row_continuity_candidate` 计数为 0，帧间接触不连续的下游后果。
- 裁决：**不校准**这 3 个锁。#2/#3 指向 velocity-first 引入的接触连续性退化（warm-start 命中率下降、帧间接触断裂），校锁会掩盖真实代价并反噬堆叠收敛。
- 正式化前置（E1/E5 已按此闭环）：① 让 contact 阶段在 velocity-first 下看到预测位置，消除滞后并清掉 3 red；② 在当前 main 基线上保留 `909e386` grab 修复、trajectoryOverlay 与 vite bump；③ 先补 phase-order / 接触检测时点 acceptance 锁定义新语义，再实现（遵循 E5 先锁后写）。具体采用的 A2 语义和验收证据见下方 living spec。
- 历史风险裁决：改 core 接触检测时点属于写路径深层契约改动；正式化已按 vNext 正规流程完成，未继续扩张到下方范围外项，剩余风险仍以 A2 记录为准。

### 2026-07-12 - E2a：grab / WorldAnchor damping 语义

#### 成功标准

- 状态：**已验证（仅 handoff §2 / E2 joint damping 子切片；整个 E2 未完成）**。
- `WorldAnchorJointDesc.damping` 在必跑 joint phase 中消费真实锚点点速度 `v + ω × r`，以 `inverse_mass + inverse_inertia * cross(r, axis)^2` 为有效逆质量施加点冲量，并用 `clamp(damping * dt, 0, 1)` 防止过阻尼翻向注入能量；零/近零约束轴跳过本帧 damping。
- 不改变 public API、step phase order、Distance joint 行为或 lab grab 默认参数；实现提交为 `247fbda`，现为当前集成分支祖先。

#### 检查结果

- TDD RED 第一轮：精确锁 exit 101，`zero_damping_peak=6`、`high_damping_peak=6`，证明字段未被 solver 消费。
- TDD RED 第二轮：`world_anchor_damping` filter exit 101；偏心锚点的 zero/high 均为 `5.9925013`，零长度 x case expected `6` got `0`，精确公式 characterization 为 1 passed。修订后的 spec/code review 均无 High/Medium；唯一 Low 是 non-zero local COM 尚无直接测试锁。
- Final verifier（`HEAD 5c8542e`）9/9 exit 0：
  1. `rtk proxy cargo fmt --all --check` -> pass，empty output。
  2. `rtk proxy cargo test -p picea --test physics_realism_acceptance damping -- --nocapture` -> 4 passed，0 failed，61 filtered。
  3. `rtk proxy cargo test -p picea --test world_step_review_regressions joint -- --nocapture` -> 4 passed，0 failed，8 filtered。
  4. `rtk proxy cargo test -p picea --lib` -> 103 passed，0 failed，1 ignored。
  5. `rtk proxy cargo test -p picea --tests` -> aggregate 231 passed，0 failed，1 ignored。
  6. `rtk proxy cargo test -p picea-lab --test server_routes` -> 20 passed，0 failed。
  7. `rtk proxy cargo clippy -p picea --all-targets` -> pass，无 warning/error output。
  8. `rtk proxy just picea-lab-web-check` -> `test:dev-server` / `dev-server-contract.mjs` pass。
  9. `rtk proxy git diff --check` -> pass，empty output。
- Public surface / workspace hygiene：`git diff -- crates/picea/src/lib.rs` 为空；验证未新增 Git-visible files。
- Browser live 面（`http://127.0.0.1:5173/?picea-profile=1`）：source badge 为实时会话；牛顿摆 spring/direct 各拖一次，spring 下摆球离开队列后仍受绳/碰撞约束，direct 下出现虚线指示且球跟随；两次 console 均 0 warnings/errors。
- Browser artifact 面：切换“生成产物并回放”与 `matrix_stack 8x6` 后产物生成成功；尝试拖动后仍为 frame 0、joint 0、state hash `66a1082f4f691367`，证明回放未被交互改写；console 0。
- Browser offline 面：停止 API 后 reload，source badge 显示演示回放并出现“离线演示数据 · 非真实模拟”水印；console 0。Browser 已 finalized，`rtk proxy just picea-lab-web-stop` exit 0，输出 `stopped`。

#### 复跑方式

- Core 行为与回归：按上方 final verifier 1-7、9 顺序复跑；其中 `physics_realism_acceptance damping` 同时覆盖峰值、偏心锚点点速度、clamp/切向保持和零轴无偏置。
- Browser：运行 Web dev server，打开 `http://127.0.0.1:5173/?picea-profile=1`，依次复跑 live spring/direct 拖拽、artifact `matrix_stack 8x6` 不可变性、停 API 后 offline 水印，记录 source badge、frame/joint/state hash 与 console；最后执行 `rtk proxy just picea-lab-web-check` 和 `rtk proxy just picea-lab-web-stop`。

#### 范围外

- body damping、`DistanceJointDesc.damping`、grab 参数/手感调优仍未进入本子切片，继续按 E2 单独推进；step-reorder 正式化未进入本子切片，但已随后作为独立 §1 切片完成。

#### 残余风险

- non-zero `local_center_of_mass` 已走 `pose.transform_point(mass_properties.local_center_of_mass)` 源码路径，但尚无直接行为锁。
- WorldAnchor 的固定目标没有速度事实，当前无法表达 moving target 的相对点速度。
- damping 是每步一次的非 warm-start impulse；长窗口高刚度/复杂 joint 网络仍需后续稳定性证据。

### 2026-07-12 - E1/E5 step-reorder 正式化：A2 设计门

#### 状态

- **实现、复审与完整 E2E 已完成；实现提交 `44931fe` 已进入当前集成分支**。
- 本节是 E1/E5 step-reorder 正式化的 living spec；实现不得越过下述内部语义、范围和 RED/验收门。

#### 已验证根因与被否决的简单 WIP

- velocity-first 顺序下，普通 contact 阶段读取尚未位置积分的旧 pose；只有命中 CCD clamp 的物体被提前推进，因此非 CCD 的同帧接近接触、sleeping impact、warm-start 连续性都会滞后一帧。
- 简单 predicted-pose WIP 虽能提前发现接触，却混用了未来 manifold depth、未来 COM/lever arm、帧初 position-correction depth 与最终 active cache/event geometry，时点不一致。
- 该 WIP 下 `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 仍红：`max_penetration_depth = 0.103906676 > 0.041646`；reviewer 判定为 High correctness finding。不得以调阈值、改断言或把预测 penetration 当作最终事实收口。

#### 业内一手参考与采用点

- Box2D manual（speculative contacts、contact/hit event 语义）：<https://box2d.org/documentation/md_simulation.html>。采用点：允许正 separation 的 speculative contact 进入 solver；公开 hit 事实应以已确认的求解冲量/最终接触事实为依据，而不是把未来 overlap 直接冒充当前 penetration。
- Box2D `contact_solver.c`：<https://github.com/erincatto/box2d/blob/main/src/contact_solver.c>。采用点：`s > 0` 时以 `velocityBias = s / h` 表达 speculative separation 的速度目标；Picea 采用相同的“由 separation/dt 产生速度偏置”原则，并按本引擎法线符号约定写成下述 target normal speed。
- PhysX speculative CCD：<https://nvidia-omniverse.github.io/PhysX/physx/5.4.0/docs/AdvancedCollisionDetection.html#speculative-ccd>。采用点：通过 motion-inflated contact offset 发现未来候选，再交给 solver 约束；同时接受其警告——offset 过大会产生不必要约束，legacy SAT 路径还可能出现伪影，因此 Picea 不扩大为无界 contact offset，也不借此修改 broadphase/narrowphase/CCD 契约。

#### 已批准的 A2 内部语义

- predicted manifold **只用于发现 future feature**，不直接作为当前或最终 penetration 事实。
- contact discovery 必须保留 solver-start authoritative pose 下的 current overlap，并在其上叠加 predicted future features；禁止只看 predicted pose，否则正在分离但帧初仍重叠的既有 contact 会被提前丢失。允许在 `contacts.rs` 内部以 current/predicted AABB 的并集喂给现有 broadphase，再对 current/predicted manifolds 做受控合并并按 contact feature 去重；不得修改 `broadphase.rs`。
- 对 predicted manifold point `p`、法线 `n`、预测深度 `d_pred`，构造两个 surface witness：`p_a = p - n * d_pred / 2`、`p_b = p + n * d_pred / 2`。Picea 的 `n` 指向 body A，因此 `s = dot(p_a - p_b, n) = -d_pred`，与“重叠为负”的约定一致。分别把 witness 转为各自 predicted collider local 坐标，再映回 solver-start 的 authoritative collider pose。
- 在 solver-start 时点计算 signed separation：`s = dot(p_a - p_b, n)`；约定分离为正、重叠为负。solver point、lever arm 与 COM 全部使用该当前时点的 authoritative pose，禁止混入未来 COM/lever arm。
- speculative normal 速度目标为 `-(s - POSITION_CORRECTION_SLOP).max(0) / dt`；由 reconstructed witnesses 得到的 signed separation 只供 velocity row 使用。residual position correction 必须由 solver-start authoritative current narrowphase 明确确认的真实 overlap/depth 提供，predicted-only pair 一律使用 `0`，避免在物体旋转或 contact feature 切换时因冻结 normal/witness 产生假穿透和错误位姿修正。
- restitution 采用两阶段：基础 non-penetration/speculative solve 不混入 restitution，并保存 pre-solve approach speed；仅当基础行累计产生正 normal impulse，且同时达到 restitution threshold 与 material 条件时，第二阶段才施加 restitution，并更新既有 telemetry。未产生法向冲量的预测点不得反弹。
- 最终位置积分后，如 final authoritative narrowphase 确认真实 overlap，则 final manifold 是公开 feature identity 与 geometry 的唯一权威：必须按 final points 重建 key、`feature_id`、normal、`reduction_reason` 与 point。solver impulse/telemetry 从 source rows 确定性一对一映射：先全局保留并消费所有 exact feature 对，再对剩余 final points 按 normal-compatible、最近 point/witness、稳定 key tie-break 的顺序 fallback；禁止逐 final point 贪心时让较早 non-exact point 抢走后续 exact source。每个 source 最多消费一次，每个 final feature 只输出一次，禁止保留 unmatched predicted events。
- 对一对一匹配到 source solver row 的 observation，`ContactEvent.depth` 必须保留 solver-start authoritative position-input depth，不能被 final manifold depth 覆盖；dense position-row 既有行为锁依赖此语义。只有 unmatched final point 使用 final depth；confirmed speculative 且 final 无 overlap 时公开 depth 固定为 `0`。
- solver-start 的 `anchor_a` / `anchor_b`、fixed local witnesses 与 source depth 是本帧 transient solver/telemetry 事实；final authoritative manifold 负责跨帧 identity handoff。`ContactRecord` 必须缓存 final feature 对应的 collider-local witnesses，不能把 source witness 与 final feature key 混写。该裁决由既有 small-tangential-slip 行为锁证实：缓存 source witness 会把上一帧求解后的真实位移重复计入下一帧 drift（`0.1875 + 0.08 = 0.2675`），而 final witness 只比较新增的 `0.08`。公开 event 仍不暴露这些内部字段。
- warm-start drift 必须在每个 collider 自己的 local frame 内比较：world normal 分别逆旋转到 A/B local frame 后，再按既有逐侧 `max` 计算 normal/tangent drift；禁止把 local witness delta 直接与 world normal 点乘，也禁止用两侧相减抵消共同漂移。
- final authoritative narrowphase 无 overlap 时，任何非 sensor source row 只有在本帧实际执行过 base normal solve、且最终累计 normal impulse 为正时，才视为 confirmed solver interaction；current/predicted discovery origin 都通过同一门，不作排他条件。此时使用 final pose 投影后的 source collider-local witnesses 定稿公开 point/cache anchor，公开 depth 固定为 `0`，并保留本帧真实 solver telemetry；下一步若没有 current/predicted contact，再自然发出 `ContactEnded`。禁止以旧 warm-start impulse、row 存在或 predicted feature 存在替代本帧 base solve 事实。
- restitution second pass 同样必须以本帧 base normal solve 已执行且最终累计 normal impulse 为正为门；`velocity_iterations = 0` 时不得用旧 warm-start impulse 触发 restitution 或 confirmed lifecycle。
- final feature 无 exact source feature 时，normal-compatible nearest-source remap 还必须通过现有 `PERSISTENT_MANIFOLD_LOCAL_ANCHOR_DRIFT_THRESHOLD` 几何信任门；超限视为 unmatched final，solver telemetry/impulse 归零，禁止把远处旧 row 映射到新拓扑。
- `picea-lab` 的 feature-churn 诊断必须区分时点：`WarmStartCacheReason::MissFeatureId` 与 `source_row_continuity_*` 描述 solver-start source row；由公开 final point 重算的 collider-local drift 只能命名并解释为 final-geometry identity continuity，不能反推本帧 prepare 阶段存在 warm fallback。原 `close_local_anchor_count == 0` 将这两个时点混为一谈，已由 frame 9 的样本证伪（prepare 无 `< 0.01` 候选，但 final drift 为 `0.002294`）。用户已批准移除该等零断言并把指标重命名为 `final_close_local_anchor_count`；solver-start 连续性仍由既有 `source_row_continuity_candidate/reason` 断言负责，所有物理阈值与其余 matrix gates 保持不变。
- 不新增 public field/event；上述 witness、separation 与时点信息只允许作为 core 内部实现细节。

#### Picea 特有兼容约束

- 既有 `sleeping_body_wakes_on_contact_solver_impact` 锁要求同帧产生 `ContactStarted` 或 `ContactPersisted`；因此 confirmed speculative constraint 保留 contact lifecycle，不把事件整体延迟到下一帧。
- 小 slop 允许最终物体停在接触面前；此时公开 depth 必须为 `0`，不能输出预测 penetration。
- warm-start tangential drift 保持既有逐侧 `max` 判据；禁止改成 `(drift_a - drift_b)` 以抵消共同漂移来放宽 cache。final feature 重建后仍由既有 warm-start 行为锁裁决是否命中。
- `matrix_stack_artifacts_capture_nxm_grid_stack_facts` 的现有 RED 是 finalization 数据完整性缺陷的 acceptance gate，禁止修改断言。
- rebase 后完整 `physics_realism_acceptance` 已证明 3 条既有 WorldAnchor damping 锁全红：旧 main 在 position integration 后执行 joint，而 velocity-first 分支在 final position integration 前执行 joint。兼容修复仅允许让 WorldAnchor damping 的 axis/lever arm 按 post-velocity predicted endpoint 评估，并保持既有 public damping 语义与断言；不得改变 Distance joint、阈值或 API。

#### TDD RED 与既有行为锁

新增并先证明以下 3 条 RED：

1. 默认 `position_iterations` 下，speculative-only constraint 不产生 position correction。
2. contact event geometry 与最终 authoritative circles 的 pose/表面一致，不能沿用预测 point/depth。
3. 第二步 warm-start/active cache 不因 solver-start、predicted、final 三个时点漂移而 drop。

同时保留现有新 phase-order 锁 `velocity_first_contact_uses_preintegrated_pose_before_final_position_integration`，并保留以下 3 条既有红锁：

- `sleeping_body_wakes_on_contact_solver_impact`
- `warm_start_cache_transfers_tangent_impulse_across_small_tangential_slip`
- `matrix_stack_artifacts_capture_nxm_grid_stack_facts`

禁止删除、放宽或改写上述既有断言；若实现要求改变断言，必须重新进入 spec/user gate。

#### 文件范围与禁区

- 允许修改：`crates/picea/src/pipeline/integrate.rs`、`crates/picea/src/pipeline/step.rs`、`crates/picea/src/pipeline/contacts.rs`、`crates/picea/src/pipeline/joints.rs`、`crates/picea/src/solver/contact.rs`、`crates/picea/src/world/contact_state.rs`、`crates/picea/tests/physics_realism_acceptance.rs`、`crates/picea-lab/tests/artifact_run.rs`，以及本计划文档。`contact_state.rs` 仅限 `pub(crate)` final local-witness cache 字段；`artifact_run.rs` 仅限上述 final/solver-start 诊断时点纠正；`joints.rs` 仅限上述 WorldAnchor damping predicted-endpoint 兼容修复。
- 禁止修改：`crates/picea/src/lib.rs` 与任何 public API；`broadphase`、`narrowphase`、`ccd`。禁止修改 Distance joint、阈值或 API。
- 禁止通过调整阈值、校准 artifact 或放宽测试断言消除红灯。

#### Targeted 验收门

以下命令全部必须使用 `rtk proxy`，且 targeted/review 闭环完成后才能进入完整 E2E：

- `rtk proxy cargo test -p picea --test physics_realism_acceptance velocity_first_contact_uses_preintegrated_pose_before_final_position_integration -- --exact --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance sleeping_body_wakes_on_contact_solver_impact -- --exact --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache_transfers_tangent_impulse_across_small_tangential_slip -- --exact --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance world_anchor_damping_reduces_offset_anchor_radial_speed_from_rotation -- --exact --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance world_anchor_damping_uses_clamped_per_step_strength_and_preserves_tangent -- --exact --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance world_anchor_damping_at_zero_length_has_no_axis_bias -- --exact --nocapture`
- `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --exact --nocapture`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance`
- `rtk proxy cargo test -p picea --lib`
- `rtk proxy cargo clippy -p picea --all-targets`
- `rtk proxy git diff --check`

完整 workspace、lab/web 与 browser E2E 仅在上述 targeted gates 和 reviewer 闭环后执行。

#### 成功标准

- velocity-first 顺序下，普通非 CCD 接触能在同帧使用 preintegrated pose 发现并先解速度，再以已解速度完成最终位置积分。
- speculative manifold 只发现 future feature；velocity row 使用 fixed local witnesses / signed separation，residual position correction 不消费 predicted depth，restitution 只在本帧 base normal solve 已执行且累计正冲量后进入第二阶段。
- final authoritative manifold 负责公开 geometry 与跨帧 cache identity；source telemetry 全局 exact 优先、fallback 受既有 local-witness 信任门约束且一对一消费。
- WorldAnchor damping 在新顺序下仍按 post-velocity predicted endpoint 消费点速度；不改 Distance joint、public API、既有物理阈值。
- targeted、workspace/lab/web、真实浏览器 live session 全部通过，reviewer 无 High/Medium/Low finding。

#### 检查结果

- Targeted：`physics_realism_acceptance` 70/70；core lib 109 passed / 1 个既有 ignore；matrix exact 通过，最大穿透 `0.026379`、warm-start final `154/0/0`、48 个动态体 sleeping、无 ejection；picea all-targets Clippy 无 warning；fmt/diff/public API 边界通过。
- Review：两轮 correctness review 发现并闭环 final-cache 时点、local basis、confirmed lifecycle、nonexact remap、全局 exact reservation、零迭代旧 warm impulse 等问题；最终 release-candidate review 无 High、Medium 或必要 Low。
- 完整 Rust/lab：`cargo test --workspace --all-targets` 共 336 passed / 6 ignored / 0 failed，9 个 Criterion target Success；workspace check 与 Clippy 全绿无 warning；bench no-run 生成 2 个 executable；server routes 20/20；artifact 27 passed / 5 ignored。
- Web：在 lockfile `npm ci` 后，production build、UI contract、i18n 全部 exit 0；安装审计为 0 vulnerabilities。
- Browser：真实 Rust API + Vite live session 成功创建；暂停/单步从 step 1290 到 1291；WorldAnchor 场景持续运行并导出 1 个 joint；软弹簧拖拽把动态箱体实际拉离地面并稳定响应；8x6 matrix 画布显示 49 个 body（1 static + 48 dynamic）且全部动态体进入 sleeping；console 无 warning/error。live diagnostics 对缺失 facts 明确显示“缺失”，未伪造零来源事实。

#### 复跑方式

- Core/targeted：依次运行本节 Targeted 验收门；矩阵详细报告使用 `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --exact --nocapture`。
- 完整 Rust/lab：`rtk proxy cargo test --workspace --all-targets`、`rtk proxy cargo check --workspace --all-targets`、`rtk proxy cargo clippy --workspace --all-targets`、`rtk proxy cargo bench -p picea --no-run`。
- Web：在 `crates/picea-lab/web` 先执行 `rtk proxy npm ci`，再运行 `rtk proxy npm run build`、`rtk proxy npm run test:ui-contract`、`rtk proxy npm run test:i18n`。
- Browser：启动 `rtk proxy cargo run -p picea-lab -- serve --bind 127.0.0.1:18080`，再以 `VITE_PICEA_LAB_API_BASE=http://127.0.0.1:18080 rtk proxy npm run dev -- --host 127.0.0.1 --port 5173` 启动 web；验收 live 落箱暂停/单步、WorldAnchor、soft-spring grab 和 matrix 8x6 sleeping 画面，并检查 console。

#### 范围外

- 未修改 public prelude/events schema、broadphase、narrowphase、CCD 或 Distance joint；未调整既有 warm-start、position-correction、matrix stability 阈值。
- §3 fuzzy `PartialEq`、revolute joint、narrowphase ignore、其余 vNext design gate 未进入本项；其中§3与narrowphase ignore所属的handoff §4已随后按独立milestone完成，revolute joint public gate也已获批准并进入独立S5-D，但API/solver尚未开始；其他项仍需在各自spec/public API门确认。
- grab stiffness/damping/max-speed 的主观手感调参不在 correctness 修复内；本轮只证明 damping 已接线并通过行为锁与 live soft-spring interaction。

#### 残余风险

- 原`stacked_rectangles_keep_feature_id_when_sat_reference_face_swaps` ignored设计红锁已在handoff §4迁移为raw role-swap characterization；冻结实现通过history-aware identity处理运行时连续性，仍未canonicalize final narrowphase feature。
- live session 的完整 diagnostics summary 仍未 hydrated；web 明确展示 missing，artifact/headless 路径仍是详细诊断权威。
- Web production bundle 当前主 chunk 约 `587.45 kB`，Vite 给出大于 500 kB 的既有性能建议；不影响本轮 correctness/E2E，但后续可单独做 code-splitting。
- 普通非 CCD contact 只比较 solver-start 与固定步末 predicted pose；需要完整 sweep 的高速凸体仍由既有 CCD 路径负责，旋转/曲线中途特征覆盖未在本项扩展。

### 2026-07-13 - §3：Point / Vector 相等性契约

#### 决策与归属

- 状态：**已验证并完成复审与端到端验收（handoff §3 / vNext equality contract 子切片）**；实现已通过 `5b9ba37` 进入当前集成分支。
- 归属：handoff §3 的 vNext 跨切面正确性切片；不扩张为 E1-E6 的新物理能力，shared acceptance 归入 V8。
- `Point` / `Vector` 的 `PartialEq` 改为标准逐分量 `f32 ==`。这恢复 `PartialEq` 对称、传递的标准契约，但仍保留浮点的部分等价语义：`NaN != NaN`，因此不实现 `Eq`。
- 新增显式 `pub fn abs_diff_eq(&self, other: Self, max_abs_diff: FloatNum) -> bool` 方法，供几何算法或测试在确实需要绝对误差容限时调用；名称明确该方法不包含相对误差或 ULP 语义。
- 容器身份与几何近似判等分离。当前不为 `Point` / `Vector` 实现 `Hash`、`Ord` 或公开 key wrapper；内部缓存若需要身份 key，继续采用局部的 `to_bits()` 位精确包装。

#### 成功标准

- 小于 `f32::EPSILON` 但位值不同的有限分量不再通过 `==`；相等关系不再出现旧 epsilon 窗口导致的非传递链。
- 标准浮点边界有行为锁：同号无穷值相等、`+0.0 == -0.0`、任何含 NaN 的点/向量不与自身相等。
- `abs_diff_eq` 逐分量使用有限、非负的绝对容差并包含边界（`difference <= abs_tolerance`）；负值、NaN、正无穷容差返回 `false`。
- `abs_diff_eq` 的每个分量先接受标准精确相等，再检查绝对差，因此同号无穷值和符号零可在合法容差下与另一分量的近似比较组合；含 NaN 的值始终返回 `false`。
- `crates/picea/src/lib.rs` 的 prelude re-export 零变更；不新增依赖，不实现 `Eq` / `Hash`，不改接触、solver、step order 或 lab 行为。

#### TDD 与实现边界

1. RED-A：先锁定 sub-epsilon 有限差异必须 `!=`，在旧 fuzzy `PartialEq` 上看到目标断言失败。
2. GREEN-A：仅把 `Point` / `Vector` 的 `PartialEq` 改为标准分量相等。
3. RED-B：再加入 `abs_diff_eq` public 行为锁，先看到缺少方法的编译失败。
4. GREEN-B：仅实现上述绝对容差契约，并补充解释相等身份与几何近似必须分离的必要注释。
5. 用户已批准迁移两处已知的旧 fuzzy 断言：`collider.rs` 的 sub-epsilon 几何接近前置条件改用 `abs_diff_eq`，但位精确缓存失效主断言必须保留；`math_algebra_regressions.rs` 的旋转近似断言改用 `abs_diff_eq`，且不得放宽原 `f32::EPSILON` 容差。若出现这两处之外的既有 fuzzy 依赖，仍须停止并上报。

文件范围：

- `crates/picea/src/math/point.rs`
- `crates/picea/src/math/vector.rs`
- `crates/picea/src/collider.rs`（仅迁移已批准的缓存测试前置断言与过时注释）
- `crates/picea/tests/math_algebra_regressions.rs`
- 本进度记录

明确非目标：

- `Eq`、`Hash`、`Ord`、公开容器 key wrapper。
- 相对误差、ULP、尺度自适应近似比较。
- 修改 `crates/picea/src/lib.rs`、其他 public 类型或几何/solver 算法调用点；`collider.rs` 只允许上述测试与注释迁移。

#### 验证门

- `rtk proxy cargo test -p picea --test math_algebra_regressions`
- `rtk proxy cargo test -p picea geometry_cache_rejects_sub_epsilon_translation_change --lib`
- `rtk proxy cargo test -p picea --lib math`
- `rtk proxy cargo test -p picea --test math_api_compile_fail`
- `rtk proxy cargo test -p picea --lib`
- `rtk proxy cargo test -p picea --tests`
- `rtk proxy cargo test --workspace --all-targets`
- `rtk proxy cargo test -p picea --examples --no-run`
- `rtk proxy cargo clippy --workspace --all-targets`
- `rtk proxy cargo fmt --all --check`
- `rtk proxy git diff --check`
- `rtk proxy git diff --exit-code -- crates/picea/src/lib.rs`
- `rtk proxy git diff --exit-code main...HEAD -- crates/picea/src/lib.rs`

#### 残余风险

- 这是 public trait 语义变更；仓库外调用方若把 `==` 当几何近似比较，需要迁移到显式 `abs_diff_eq` 或自己的尺度相关容差策略。
- 绝对容差对极大或极小尺度不一定合适；本切片刻意不替调用方选择相对误差或 ULP 策略。
- `Pose` 等包含 `Point` / `Vector` 的派生 `PartialEq` 类型会继承新语义；例如 CCD 的 start-pose 比较现在会对 sub-epsilon 有限位移重算起始顶点。该方向更保守，但尚无专门的 CCD 行为锁。

#### 检查结果

- RED-A：`rtk proxy cargo test -p picea --test math_algebra_regressions point_and_vector_use_standard_float_equality -- --exact` -> exit 101；旧 fuzzy `PartialEq` 把 `0.0` 与 `0.75 * f32::EPSILON` 判等，目标 `assert_ne!` 失败。GREEN-A 同命令 -> 1 passed。
- RED-B：加入 public `abs_diff_eq` 合同后，`rtk proxy cargo test -p picea --test math_algebra_regressions` -> exit 101，17 个预期 `E0599`（方法尚不存在）。GREEN-B -> 3 passed。
- reviewer 首轮发现 whole-value infinity fast path 的 Medium：`(INF, finite)` 与 `(INF, nearby finite)` 会因 `INF - INF = NaN` 假阴性。补 Point / Vector mixed-infinity 行为锁后 exact filter 先 exit 101，再改为逐分量精确相等或绝对差判断；复审 High / Medium / Low 均无 actionable finding。
- 已批准迁移的两处旧断言保持原意：旋转锁仍使用 `f32::EPSILON`；geometry cache 锁改为显式几何近似前置条件，同时保留标准值不等、位模式不等和 cache stale 主断言。
- Targeted：math algebra 3 passed；geometry cache 1 passed；lib math 4 passed；math API trybuild harness 1 passed、3 个 UI case 通过。
- Core / workspace：picea lib 103 passed、1 ignored；picea tests 合计 233 passed、1 ignored；workspace all-targets 327 passed、6 ignored，9 个 benchmark scenario 成功；picea-lab 82 passed、5 ignored；examples 与 bench `--no-run` 编译通过。
- Hygiene / compatibility：workspace all-targets clippy 无 warning；fmt、YAML、`git diff --check` 通过；`crates/picea/src/lib.rs` 的 unstaged、staged、相对 main 三层 diff 均为空。
- Web / end-to-end consumer：新 worktree 首轮因未安装 TypeScript 依赖失败；`rtk proxy npm --prefix crates/picea-lab/web ci` 安装 lockfile 依赖后，UI / i18n / profile contracts、production build、`rtk proxy just picea-lab-web-check` dev-server contract 全部通过。构建成功转换 1680 modules；仅有单 chunk 超过 500 kB 的既有非阻塞 warning。依赖与构建产物均未污染 Git status。

#### 最终范围

- 代码 / 测试：`point.rs`、`vector.rs`、`collider.rs`、`math_algebra_regressions.rs`。
- 文档：本进度记录。
- 未修改 `lib.rs` prelude、Cargo manifests、依赖、solver、step order 或 lab 行为；未实现 `Eq` / `Hash` / `Ord`。
