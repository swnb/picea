# Repo Map

当前路由优先级：先看实时仓库事实（`git status`、Cargo manifests、`crates/picea/src/lib.rs`、最新验证输出），再用本文定位模块。

`docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`
是当前生产化 milestone 路线；M11-M22 在当前路线已按 2026-04-28
验收完成。`docs/design/physics-engine-upgrade-technical-plan.md`
解释 Post-M20 baseline、M21 public query 和 M22 authoring boundary
之后的系统深化方向。M15 已落地为 Performance Data Path：
`QueryPipeline` 通过内部 broadphase-style spatial index 复用做候选遍历，
collider 派生几何使用 transform/revision-backed cache，contact/CCD/GJK 路径
减少重复几何重建并做保守 pre-sizing。M16 Dense Island Execution 已
把 contact/joint active-island batching 收敛为 deterministic island-local
body slots / contact row indices / joint rows。M17 Performance Evidence 已把
query traversal/candidate/filter/hit counters、broadphase traversal/prune
counters、solver island/row/slot counters、`picea-lab` perf counter summaries
和 query/island Criterion 场景接到证据层。M18 已把 broadphase
candidate-pair 路径从 per-leaf root scan 收敛为 internal child-pair
traversal，同时保持 candidate/query 的 public ordering 和语义不变。M19
已完成所选 translational dynamic-vs-dynamic convex CCD slice，并通过
`CcdTrace.target_kind` / target sweep facts 暴露动态目标语义。M20 已把
`picea-lab` scene fixture 稳定到 versioned v1 schema：老 fixture
缺省 `schema_version` 会回落到 v1，非 v1 版本会在 world 实例化前直接报 clear
scene-schema error，fixture 还能通过 recipe-indexed `distance` /
`world_anchor` joints 走既有 `JointBundle` + `WorldRecipe::with_joint` 路径。
M21 已把 public distance / shape query API 落到 `QueryShape`、
`ShapeHit`、`QueryShapeError` 以及 `QueryPipeline::intersect_shape` /
`closest_shape`。M22 已把 lab scene fixture 的 compound / concave
authoring boundary 固定为 validated convex pieces 或稳定 authoring error；
direct concave contact solving 仍不属于 core solver。
`docs/plans/2026-04-18-picea-physics-engine-milestones.md`
只用于历史归档；其中旧 `Scene` / `Context` / `picea-web` / wasm 叙述不代表当前默认路由。

仓库是一个 Rust workspace，当前三类 crate / 工具入口：

- `crates/picea`：核心 2D physics engine
- `crates/picea-lab`：本地 C/S 模拟器、场景 runner、artifact capture、HTTP/SSE server，以及 `web/` React Canvas workbench
- `crates/macro-tools`：独立 proc-macro crate；在 workspace 内单独验证，当前不在 `crates/picea` 的直接依赖图上

## `crates/picea`

### `crates/picea/src/lib.rs`
- owns：当前 public crate-root surface 和 `prelude` 重导出。
- does_not_own：不放具体物理逻辑，不放 wasm 接口，不放文档状态。
- entrypoints：`algo`、`body`、`collider`、`debug`、`events`、`handles`、`joint`、`math`、`pipeline`、`query`、`recipe`、`world`、`prelude`。
- tests：`rtk proxy cargo test -p picea --lib`。

### Current `crates/picea` module map

| Module | Owns | Does Not Own | Entry Points | Tests |
| --- | --- | --- | --- | --- |
| `algo` | 排序与 collection ordering helpers。 | 不拥有 world state 或 public physics contracts。 | `algo/mod.rs`, `algo/sort.rs` | `rtk proxy cargo test -p picea --lib --tests` |
| `body` | `BodyDesc`、`BodyPatch`、`BodyView`、`Pose`、`BodyType` 等稳定 body API。 | 不拥有 collider geometry、joint lifecycle 或 pipeline orchestration。 | `body.rs` | `rtk proxy cargo test -p picea --test core_model_world`, `rtk proxy cargo test -p picea --lib --tests` |
| `collider` | `ColliderDesc`、`ColliderPatch`、`ColliderView`、`Material`、`CollisionFilter`、`SharedShape`。 | 不拥有 authoritative world lifecycle。 | `collider.rs` | `rtk proxy cargo test -p picea --test core_model_world`, `rtk proxy cargo test -p picea --lib --tests` |
| `debug` | `DebugSnapshot` 与稳定 read model。 | 不直接修改 authoritative world state。 | `debug.rs` | `rtk proxy cargo test -p picea --test world_step_review_regressions`, `rtk proxy cargo test -p picea --lib --tests` |
| `events` | `WorldEvent`、contact/sleep/numerics payloads。 | 不拥有 world mutation 或 solver state。 | `events.rs` | `rtk proxy cargo test -p picea --lib --tests` |
| `handles` | `BodyHandle`、`ColliderHandle`、`JointHandle`、`ContactId`、`ManifoldId`、`WorldRevision`。 | 不拥有 handle lifecycle 或 store mutation。 | `handles.rs` | `rtk proxy cargo test -p picea --lib --tests` |
| `joint` | `DistanceJoint*`、`WorldAnchorJoint*`、`JointDesc`、`JointPatch`、`JointView`。 | 不拥有 solver iteration internals。 | `joint.rs` | `rtk proxy cargo test -p picea --test core_model_world`, `rtk proxy cargo test -p picea --lib --tests` |
| `math` | `Point`、`Vector`、`Matrix`、`Segment`、`Edge`、`FloatNum` 等基础数值类型与运算。 | 不承载 runtime orchestration，不做 milestone 决策。 | `math/mod.rs`, `math/vector.rs`, `math/point.rs`, `math/segment.rs` | `rtk proxy cargo test -p picea --test math_api_compile_fail`, `rtk proxy cargo test -p picea --lib --tests` |
| `pipeline` | `SimulationPipeline`、`StepConfig`、`StepReport`、`StepContext` transient step facts、broadphase/narrowphase contact gathering、M11 broadphase leaf lookup substrate、M15 query/cache/pre-sizing performance data path、M16 dense island solve-plan routing、M17 broadphase/island/row/slot evidence counters、M18 subtree-pair broadphase traversal tuning、内部 GJK/EPA generic convex fallback、M13 staged dynamic-vs-static convex CCD pose-clamping phase、M19 translational dynamic-vs-dynamic convex CCD pose clamp。 | 不拥有 world 持久状态；M21 public distance-query API 归 `query` public surface，不由 `pipeline` 暴露 proxy/cache internals；不承载最终 contact/joint solver row math；M19 不做 rotational CCD、all-shape CCD、dynamic circle-vs-dynamic CCD 或 public lifecycle API 改动。 | `pipeline.rs`, `pipeline/step.rs`, `pipeline/island.rs`, `pipeline/integrate.rs`, `pipeline/contacts.rs`, `pipeline/broadphase.rs`, `pipeline/narrowphase.rs`, `pipeline/gjk.rs`, `pipeline/ccd.rs`, `pipeline/joints.rs`, `pipeline/sleep.rs` | `rtk proxy cargo test -p picea --lib pipeline::island`, `rtk proxy cargo test -p picea --lib pipeline::sleep`, `rtk proxy cargo test -p picea --lib pipeline::broadphase`, `rtk proxy cargo test -p picea --lib pipeline::ccd`, `rtk proxy cargo test -p picea --lib pipeline::gjk`, `rtk proxy cargo test -p picea --lib pipeline::narrowphase`, `rtk proxy cargo test -p picea --test physics_realism_acceptance ccd`, `rtk proxy cargo test -p picea --lib --tests` |
| `query` | `QueryPipeline`、`QueryFilter`、`QueryStats`、`AabbHit`、`PointHit`、`RayHit`、`QueryShape`、`ShapeHit`、`QueryShapeError`；M15 已通过内部 broadphase-style index 做 AABB/point/ray 候选复用，M17 通过 `QueryPipeline::last_stats()` 暴露最近一次查询的 traversal/candidate/prune/filter-drop/hit counters，同时保持 public hit ordering/filter semantics；M21 已增加 public distance / shape query API。 | 不直接修改 authoritative world state，不暴露 broadphase proxy/leaf id；M21 public surface 不把内部 GJK/cache detail 当 public contract。 | `query.rs` | `rtk proxy cargo test -p picea --test query_debug_contract`, `rtk proxy cargo test -p picea --test world_step_review_regressions`, `rtk proxy cargo test -p picea --lib pipeline::gjk`, `rtk proxy cargo test -p picea --lib --tests` |
| `recipe` | `BodyBundle`、`ColliderBundle`、`JointBundle`、`WorldRecipe::with_joint`、transactional `WorldCommands`、material/collision-layer presets、M14 scene/asset recipe helpers、serializable fixture setup；M22 在 scene authoring 层用已有 bundle/recipe path 表达 compound convex pieces。 | 不改低层 `World::create_body` / `World::create_collider` / `World::create_joint` 语义，不承载 solver 或 pipeline behavior；M22 不表示 core solver 直接支持 arbitrary concave contact。 | `recipe.rs`, `benches/physics_scenarios.rs` | `rtk proxy cargo test -p picea --test v1_api_smoke`, `rtk proxy cargo test -p picea --test core_model_world`, `rtk proxy cargo bench -p picea --no-run` |
| `world` | `World` 状态、lifecycle API、runtime retained facts、error/store/contact state。 | 不承载低层数学兼容或消费者壳。 | `world.rs`, `world/api.rs`, `world/store.rs`, `world/runtime.rs`, `world/error.rs`, `world/contact_state.rs` | `rtk proxy cargo test -p picea --test core_model_world`, `rtk proxy cargo test -p picea --test world_step_review_regressions` |

### `crates/picea/src/solver/*` (internal)
- owns：当前 `World` + `SimulationPipeline` 路径内部求解辅助实现。
- does_not_own：不作为 public crate-root surface 暴露，不承担路由入口职责。
- entrypoints：`solver/mod.rs`、`solver/body_state.rs`、`solver/contact.rs`。
- notes：M10 后 contact solver rows、effective mass、warm-start impulse application、velocity writeback、residual contact correction 都在 `solver/contact.rs`；M12 的 active-island batching 已在 contact/joint solve 路径验收完成，但 `pipeline/contacts.rs` 仍保留 gather / warm-start / emit。M16 通过 `pipeline/island.rs` 把 map/set-heavy batching 收敛为 dense island-local execution，并保留现有 separate-phase behavior 和 live step order。
- tests：跟随 `rtk proxy cargo test -p picea --lib --tests`。

## `crates/picea-lab`

### `crates/picea-lab/src/lib.rs`
- owns：lab crate 模块边界和公共重导出；保持 core wrapper，而不是 physics runtime。
- does_not_own：不修改 `crates/picea` core API，不持有浏览器 UI 代码。
- entrypoints：`artifact`、`scenario`、`server`、`cli`、`error`。
- tests：`rtk proxy cargo test -p picea-lab`。

### Current `crates/picea-lab` module map

| Module | Owns | Does Not Own | Entry Points | Tests |
| --- | --- | --- | --- | --- |
| `scenario` | 内置 deterministic 场景、reset-time overrides、`RunConfig`；包含 M13 `ccd_fast_circle_wall` / `ccd_fast_convex_walls` CCD evidence、M19 `ccd_dynamic_convex_pair` 动态目标 CCD evidence、M20 versioned `SceneRecipeFixture` / joint authoring / backward-compatible fixture loading；M22 已接入 compound convex piece authoring、direct concave rejection、top-level/piece convex validation 和 stable scene-path errors。 | 不读写 artifacts，不持有 live session 状态，不自行运行物理逻辑；不把 M22 authoring support 解读成 lab 自己运行 concave solver。 | `crates/picea-lab/src/scenario.rs` | `rtk proxy cargo test -p picea-lab` |
| `artifact` | headless runner、`manifest.json` / `frames.jsonl` / `debug_render.json` / `final_snapshot.json` / `perf.json` 写入；`frames.jsonl` 与 `final_snapshot.json` 保留 core `StepStats` / `DebugStats` CCD counters 和 contact `ccd_trace`，包括 M19 dynamic-target `target_kind` / target sweep facts；M17 `perf.json.counter_summary` 汇总 deterministic work counters，debug render frames 携带 broadphase traversal/prune 与 island/solver row counters；M22 未扩展 artifact schema，compound provenance UI/schema 属于 Post-M22。 | 不直接服务 HTTP，不把 target 路径暴露给 UI，不从 lab 侧重新计算 CCD，不把 wall-clock timing 当正确性 oracle。 | `crates/picea-lab/src/artifact.rs` | `rtk proxy cargo test -p picea-lab --test artifact_run` |
| `server` | 本地 HTTP + SSE protocol、session 状态、artifact 下载；session 明确暴露 `manifest.json` / `final_snapshot.json` replay provenance，empty SSE 使用 idle event 而不是 failed。 | 不热改正在积分的 world；reset 通过 runner 重建；不把空事件队列当成模拟失败。 | `crates/picea-lab/src/server.rs` | `rtk proxy cargo test -p picea-lab --test server_routes` |
| `cli` | `picea-lab list`、`run`、`serve` 命令。 | 不拥有 artifact schema 或 scenario 构建细节。 | `crates/picea-lab/src/cli.rs`, `main.rs` | `rtk proxy cargo test -p picea-lab` |
| `web` | React + Canvas 2D replay workbench, hierarchy, inspector, timeline, overlays；joint rows are selectable, server/demo labels distinguish Rust replay from demo replay, and the contact inspector shows staged M13 CCD trace facts plus M14 fixture-backed replay provenance. | 不运行 physics，不替代 Rust artifact schema，不承诺真正 live simulator 语义。 | `crates/picea-lab/web/src/*` | `npm run build` from `crates/picea-lab/web`; `npm run test:ui-contract`; `npm run test:i18n` |

## `crates/macro-tools`

### `crates/macro-tools/src/lib.rs`
- owns：`Accessors`、`Builder`、`Deref` derive macro 入口。
- does_not_own：不关心 physics 运行时状态。
- dependency_status：作为独立 workspace proc-macro crate 单独验证；不要从历史 milestone gate 推断它仍在 `crates/picea` 当前依赖图上。
- entrypoints：`accessors.rs`、`builder.rs`、`deref.rs`。
- tests：`rtk proxy cargo test -p picea-macro-tools`。

## 文档入口

- `docs/ai/index.md`：问题类型路由
- `docs/ai/doc-catalog.yaml`：文档和关键代码索引
- `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`：当前生产化 milestone 边界、M11-M22 完成状态和 Post-M22 follow-up
- `docs/design/physics-engine-upgrade-technical-plan.md`：Post-M20 baseline、M21 public query 和 M22 authoring boundary 之后的系统升级设计方向
- `docs/plans/2026-04-18-picea-physics-engine-milestones.md`：历史归档；不要把旧 `Scene` / `Context` / `picea-web` / wasm 条目当作当前默认路由
