# Picea Physics Realism vNext 里程碑计划

状态：已批准
计划文档：docs/plans/2026-06-17-physics-realism-vnext-milestones.md
最后更新：2026-06-17
工作目录：/Users/asyncrustacean/projects/picea
工作区状态：创建前 clean；`main...origin/main`，`HEAD=ac2941d`；执行时以 live `git status` 为准
计划重量：heavy
Goal 协调：planning/execution goal active；用户已明确预授权 Plan Gate
提交策略：不提交
执行策略：完整计划预授权后连续执行；遇高风险门、现实冲突、验证阻塞时停
高风险门：Public API/compatibility break；`SharedShape::ConcavePolygon` hard reject；`Material` public schema；deformable public surface；删除/迁移/部署
SpecFlow：不使用
Change root：none
Profiles：architecture-heavy, api-contract, ui-browser
完成状态：D0/V0 已验证；execution milestones 未实现；AI 路由已同步；已归档 no；可发布 no

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
- 主要未知：fresh full workspace tests 是否全绿；browser automation 是否可用；future deformable V1 representation 仍需 RFC 冻结。

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
| E1 | 执行 | 计划中 | 堆叠/接触稳定性 acceptance-as-code 与最小实现 | contact lifecycle / position-row / sleep | worker gpt-5.4 |
| E2 | 执行 | 计划中 | 材质/阻尼语义债，先兑现 joint damping | material / joint damping | worker gpt-5.4 |
| E3 | 执行 | 计划中 | CCD dynamic circle -> stationary dynamic convex target | CCD trace | worker gpt-5.4 |
| E4 | 执行 | 计划中 | compound/concave provenance 与 boundary evidence | complex shape | worker gpt-5.4 |
| E5 | 执行 | 计划中 | joint ordering 合同锁与 chain/bridge diagnostics | joint / solver ordering | worker gpt-5.4 |
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
状态：计划中
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
状态：计划中
来源设计：D0

目标：先保护已有 body damping 行为，再兑现已有 `joint.damping` 语义；不先扩 `Material`。

执行输入：
- `Material` 不新增字段。
- `restitution > 1` 只记录/测试当前 clamp 语义，不改 validation。
- `linear_damping` / `angular_damping` 是 body 既有行为，E2 至少补 regression 或确认已有 gate 覆盖。
- `joint.damping` 优先进入 joint solver 或明确文档化不支持；推荐实现。

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
状态：计划中
来源设计：D0

目标：锁定当前真实 step 顺序，补 joint damping/chain/bridge diagnostics，不合并 row stream。

执行输入：
- 当前代码顺序权威：joint solve -> CCD -> contact phases -> optional joint velocity projection -> sleep。
- M28 文档若不一致，应同步文档而不是按旧文档改代码。
- `joint_velocity_projection` 是 contact phases 之后、sleep 之前的可选阶段；E5 reviewer 必须审查它没有被遗漏或提前。
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
- 下一步：复跑 V0 文档验证；必要时运行 targeted smoke。
