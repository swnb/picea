# vNext Handoff §5 Revolute Joint Pin-only V1 Milestone

状态：S5-D已commit（`eecbc331a36bfb694676c979bffb03f5a47202dd`）；S5-API-RED已commit（`6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68`）；S5-API final reviewer=`0 High / 0 Medium / 0 Low`、independent verifier=`S5-API VERIFIER PASS`，已commit（`c5a47ed4252363ebf914802c8189cf2bcc9e3563`）；S5-BEHAVIOR-RED immutable Start HEAD=`c5a47ed4252363ebf914802c8189cf2bcc9e3563`，首轮reviewer=`1 High / 3 Medium / 1 Low`、裁决`FAIL`，bounded remediation后final reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`，independent verifier=`S5-BEHAVIOR-RED VERIFIER PASS`，commit=`PENDING`；solver未开始；Chrome `NOT RUN / BROWSER PENDING`
日期：2026-07-14
基线：`feat/vnext-s5-revolute-joint@28867b5`
设计：`docs/design/2026-07-14-revolute-joint-v1-design.md`
交接：`docs/handoff-2026-07-14-vnext-s5-revolute-joint.md`
父计划：`docs/plans/2026-06-17-physics-realism-vnext-milestones.md`
Profiles：architecture-heavy, api-contract, ui-browser

## 1. 批准、目标与边界

用户已批准 handoff §5 的“完整 Pin-only V1”public API、字段、
`#[non_exhaustive]` 列表、wake semantics、lab 范围和延期项。不存在 Plan Gate
阻塞问题；只有本计划列出的 stop condition 可以中断连续执行。

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
- 不合并、重排或重构 contact/joint solver streams，不改 contact solver。
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

### Unknown

当前无阻塞未知项。若 implementation 证明 frozen contract 与 live source 无法同时满足，
必须暂停、记录证据并先修订 design/living spec；不得在 worker 内自行改 API 或范围。

## 3. 执行不变量与 subagent 合同

唯一执行链：

```text
S5-D -> S5-API-RED -> S5-API -> S5-BEHAVIOR-RED -> S5-SOLVER
     -> S5-LAB-RED -> S5-LAB -> S5-V -> S5-C
```

通用规则：

- 每个 node 都分派独立叶子 `worker`、`reviewer`、`verifier`；prompt 必须包含
  “你是叶子 agent，不允许再启动 subagent。”
- `worker` 只写该 node ownership 表列出的文件；不得 stage、commit、push、merge、
  branch、clean 或修改别的 node 文件。
- `reviewer`、`verifier` 只读；发现问题不修。reviewer findings-first，按
  High/Medium/Low 给出文件与行号。
- 主 Codex 是 supervisor，只做范围裁决、角色交接、High/Medium 闭环、固定 allowlist
  stage、commit 和 commit 后复核，不亲自实现已委派内容。
- 当前工作区最多一个写入 worker。每个 commit 前 reviewer 必须无未闭合
  High/Medium，independent verifier 必须完成该 node 的 exact gate。
- 所有 Cargo/Git/Rust/Web 验证使用 `rtk proxy`。不得把 reviewer判断代替 executable
  receipt。
- S5-API-RED、S5-BEHAVIOR-RED、S5-LAB-RED 必须先提交 acceptance artifacts 并记录
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
| S5-SOLVER | `pipeline/island.rs`、`pipeline/joints.rs`、minimal `solver/body_state.rs`、`pipeline.rs` public doc/config lock、必要 focused unit tests | stream merge/reorder、contact solver、lab/Web、StepConfig字段/default变化 | §9 targeted green | solver/code review + verifier；`feat: solve revolute point constraints` |
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
  `S5-BEHAVIOR-RED VERIFIER PASS`；commit仍为`PENDING`，solver未开始，Chrome保持
  `NOT RUN / BROWSER PENDING`。
  implementation worker未继续修改scope script。
- S5-BEHAVIOR-RED保留直接2x2/singular/sign unit contract。若采用
  `pipeline/joints/tests.rs`，`pipeline/joints.rs`除`#[cfg(test)] mod tests;`外production
  必须zero-diff；reviewer逐行确认。S5-SOLVER不得修改committed integration阈值/断言，
  只实现row/math并让locks转绿。
- S5-SOLVER可修改`crates/picea/src/pipeline.rs`，但只允许把
  `StepConfig::joint_velocity_projection` public doc从distance-only改为joint
  point-velocity semantics，并同步既有配置default/serde行为锁；不得增加字段、改default
  或借机重构pipeline。
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
| S5-LAB-RED | `artifact_run.rs`, `server_routes.rs`, Web `ui-contract.mjs`, `i18n-contract.mjs`, living spec | fixture compatibility test file仅当API checkpoint尚未锁定对应case |
| S5-LAB | `scenario/mod.rs`, `scenario/scene_dispatch.rs`, new `scenario/scene_revolute.rs`, Web `i18n.ts`, `types.ts`, `SceneHierarchy.tsx`, `Inspector.tsx`, `Timeline.tsx`, living spec | `artifact.rs`/`server.rs`仅在RED证明generic passthrough不足时；`components/workbench/types.ts`仅在compiler/contract要求时 |
| S5-C | design、living spec、parent、handoff、AI index/repo-map/catalog、design index、`solver-island-ordering-contract.md` | 无 |

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

**复跑方式**：从本node未提交worktree读取
`S5_NODE_START_HEAD="$(rtk proxy ruby crates/picea/tests/verify_revolute_scope.rb receipt-head S5-BEHAVIOR-RED)"`，
按§8 `RED 命令`代码块原顺序逐条运行unit、13条integration exact、scope、diff与zero-diff
命令；禁止用aggregate filter替代逐条分类。

**范围外**：未实现2x2 helper、`JointSolverRow::Revolute`、joint solve batch/math、COM apply、
phase reorder或任何solver production；未修改lab/Web、API/lifecycle tests、scope script、Cargo
文件或integration阈值；未stage/commit/push；Chrome保持`NOT RUN / BROWSER PENDING`。

**残余风险**：final reviewer与independent verifier均已PASS，但本RED节点commit仍为
`PENDING`，S5-SOLVER保持deferred且尚未开始；十条expected RED是已批准的solver缺口合同，
只能由后续S5-SOLVER最小实现转绿，不得修改已冻结integration阈值/断言或stream顺序。
Chrome仍为`NOT RUN / BROWSER PENDING`；下一最小动作是supervisor提交本RED节点，提交前不得
开始solver。

## 9. S5-SOLVER：2x2 point constraint

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

## 10. S5-LAB-RED：跨层 acceptance-as-code

在 S5-SOLVER commit 上新增：

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
rtk proxy rg -n "S5-D|S5-API-RED|S5-API|S5-BEHAVIOR-RED|S5-SOLVER|S5-LAB-RED|S5-LAB|S5-V|S5-C" docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
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
- 需要修改/合并/reorder contact/joint solver stream或contact solver；
- contact/projection enabled/disabled或CCD-clamped full-step gate只能通过重排phase、冻结旧
  velocity或绕过contact才能满足；
- 需要升级 fixture schema version；
- 需要修改冻结字段、default、`#[non_exhaustive]`列表、prelude边界、wake reason或
  `user_data` wake规则；
- 需要把一个 revolute 计为两个logical rows；
- RED只能通过删除/ignore/放宽断言、改阈值或伪造Web facts消除；
- external ChatGPT App browser receipt不能证明真实Rust facts path、被测full SHA不一致或
  返回FAIL；未回填只保持PENDING，不得写PASS；
- 发现不属于当前node的dirty写入且无法确认所有权。

## 16. 进度日志

| 日期 | Node | Start HEAD (full, immutable) | 状态 | Commit | Receipt / 备注 |
| --- | --- | --- | --- | --- | --- |
| 2026-07-14 | S5-D | `28867b5c8eeae3ec290ad7dfd5f52cbf3ca69e80` | 已commit | `eecbc331a36bfb694676c979bffb03f5a47202dd` | 第三轮reviewer=`0 High / 0 Medium / 1 Low`，允许进入verifier；independent docs verifier于`2026-07-14 17:16:23 CST`给出`S5-D VERIFIER PASS`；8-file scope/YAML/Mermaid/required terms/diff-check/crates/Cargo/Web zero-diff/cached empty均PASS；唯一Low留到S5-C；Chrome `NOT RUN / BROWSER PENDING`；implementation/API/solver未开始 |
| 2026-07-14 | S5-API-RED | `eecbc331a36bfb694676c979bffb03f5a47202dd` | final reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=`S5-API-RED VERIFIER PASS`；已commit | `6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68` | 4M remediation complete；public expect-red连续两次wrapper `0`/internal `101`且仅approved diagnostics；六enum missing-variant分类与六条existing-kind lifecycle exact RED均有效，wake exact各自`running 1 test`、正确sentinel、approved exit `101`；workspace/scope/temp/diff GREEN，7-path exact；Revolute runtime `NOT RUN`；API/solver未开始；Chrome `NOT RUN / BROWSER PENDING` |
| 2026-07-14 | S5-API | `6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68` | scope reviewer/re-verifier已PASS；implementation worker完成；worker targeted/full/scope/hygiene gates=`GREEN`；independent implementation reviewer首轮=`0 High / 1 Medium / 0 Low`、裁决`FAIL`（顶部仍误报implementation未开始），第一次bounded remediation已完成；第一次复审=`0 High / 1 Medium / 0 Low`、裁决`FAIL`（文档仍误报第一次状态修复未完成），第二次status-only bounded remediation已完成；最终独立复审=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=`S5-API VERIFIER PASS`；已commit；Chrome `NOT RUN / BROWSER PENDING` | `c5a47ed4252363ebf914802c8189cf2bcc9e3563` | exact-16内已实现Revolute public/lifecycle/authoring API、schema-v1 fixture/re-export/TS kind compatibility与island explicit skip；public positive GREEN+六enum仅批准`E0004`，runtime `2 passed`，core focused `6 passed`，row-zero为`running 1 test`+`joint_count=1;joint_row_count=0`，六条lifecycle各自`running 1 test`+sentinel/kind覆盖，fixture `3 passed`；full core/world/lab=`24/18/38 passed`，Web/workspace/fmt/scope self-test/exact-16/diff/Cargo/solver/cached/untracked/hygiene均PASS。首次public harness因重叠使用固定temp root出现一次`ENOENT`，串行同命令及runtime补跑PASS且源文件zero-diff，分类为verifier orchestration noise；solver、scenario behavior与Chrome未开始；未push；scope script保持implementation前approved diff，worker未修改 |
| 2026-07-14 | S5-BEHAVIOR-RED | `c5a47ed4252363ebf914802c8189cf2bcc9e3563` | worker首轮完成；independent reviewer首轮=`1 High / 3 Medium / 1 Low`、裁决`FAIL`；bounded remediation完成；final reviewer=`0 High / 0 Medium / 0 Low`、裁决`PASS`；independent verifier=`S5-BEHAVIOR-RED VERIFIER PASS`；commit=`PENDING` | `PENDING` | 五项finding全部闭环；CCD final-angle zero-position-angular-correction oracle、free-rotation positive mass/inertia与可逆`K`、nonzero-COM非零lever均已补强；早期`can_sleep=false` fixture harness failure已记录并修复；unit仅8个批准`E0425`/四个planned helpers，13条integration均`running 1 test`且保持10 RED/3 GREEN；API focused=`6 passed`、row-zero checkpoint=`joint_count=1;joint_row_count=0`、lifecycle=`6 passed`；fmt/scope self-test/exact-4/diff/solver/lab/Cargo/cached/untracked/generated hygiene均PASS；一次只读status `rg`反引号shell展开噪声已用单引号安全复跑PASS且无文件修改；solver未开始；Chrome `NOT RUN / BROWSER PENDING` |
| - | S5-SOLVER | `PENDING` | 未开始 | - | 2x2 point constraint；streams不变 |
| - | S5-LAB-RED | `PENDING` | 未开始 | - | 必须单独提交cross-layer contracts |
| - | S5-LAB | `PENDING` | 未开始 | - | CLI candidate commit后external browser receipt未回填则`BROWSER PENDING` |
| - | S5-V | `PENDING` | 未开始 | - | CLI三角色只读；独立external browser receipt另行回填 |
| - | S5-C | `PENDING` | 未开始 | - | docs-only closeout |

## 17. 当前残余风险

- 单次 linearized position correction 对极端大初始 anchor 误差不作额外收敛承诺；
  acceptance 只采用批准的240帧窗口。
- singular row fail closed 会跳过该phase；这是防止world污染，不是约束成功保证。
- 新 `ScenarioId` variant 对仓库外 exhaustive match 有source影响，但本次不改变其
  attribute policy。
- old schema-v1 reader不能读取new `revolute` capability；已明确记录为forward
  incompatibility。
- old closed-enum debug serde consumer不能读取`DebugJointKind::Revolute`；artifact/server
  保持authoritative kind，consumer必须升级。
- Web production bundle既有体积warning不因本milestone扩大scope；只有新增
  error/warning才阻塞browser gate。
