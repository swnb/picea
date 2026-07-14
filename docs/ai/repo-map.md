# Repo Map

当前路由优先级：先看实时仓库事实（`git status`、Cargo manifests、`crates/picea/src/lib.rs`、最新验证输出），再用本文定位模块。

本文是模块地图，不是 milestone 日志。生产化路线和完成记录请读
`docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`；
旧 `Scene` / `Context` / `picea-web` / wasm 叙述只在
`docs/plans/2026-04-18-picea-physics-engine-milestones.md` 和
`docs/architecture/legacy-scene-runtime.md` 中作为归档背景保留。

当前可稳定假设的主线：

- public beta surface：`World` / `SimulationPipeline` / `QueryPipeline` /
  `DebugSnapshot` / `WorldRecipe`。
- core 事实层：`crates/picea` 拥有 authoritative runtime、pipeline、query、
  debug 和 solver internals。
- lab 事实层：`crates/picea-lab` 负责 scenario、artifact、server/live session
  和证据导出。
- web 展示层：`crates/picea-lab/web` 消费 Rust facts，不重新计算 physics。
- macro 工具层：`crates/macro-tools` 是独立 workspace crate，不是 core runtime
  直接依赖图的一部分。

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
| `pipeline` | `SimulationPipeline`、`StepConfig`、`StepReport`、`StepContext` transient step facts、broadphase/narrowphase contact gathering、M11 broadphase leaf lookup substrate、M15 query/cache/pre-sizing performance data path、M16 dense island solve-plan routing、M17 broadphase/island/row/slot evidence counters、M18 subtree-pair broadphase traversal tuning、内部 GJK/EPA generic convex fallback、M13 staged dynamic-vs-static convex CCD pose-clamping phase、M19 translational dynamic-vs-dynamic convex CCD pose clamp、M26 dynamic compound CCD behavior lock、M28 solver island ordering contract。 | 不拥有 world 持久状态；M21 public distance-query API 归 `query` public surface，不由 `pipeline` 暴露 proxy/cache internals；不承载最终 contact/joint solver row math；M26 不做 rotational CCD、all-shape CCD、dynamic circle-vs-dynamic CCD 或 public lifecycle API 改动；M28 不做 multithreaded solver 或 unified contact/joint row stream。 | `pipeline.rs`, `pipeline/step.rs`, `pipeline/island.rs`, `pipeline/integrate.rs`, `pipeline/contacts.rs`, `pipeline/broadphase.rs`, `pipeline/narrowphase.rs`, `pipeline/gjk.rs`, `pipeline/ccd.rs`, `pipeline/joints.rs`, `pipeline/sleep.rs` | `rtk proxy cargo test -p picea --lib pipeline::island`, `rtk proxy cargo test -p picea --lib pipeline::sleep`, `rtk proxy cargo test -p picea --lib pipeline::broadphase`, `rtk proxy cargo test -p picea --lib pipeline::ccd`, `rtk proxy cargo test -p picea --lib pipeline::gjk`, `rtk proxy cargo test -p picea --lib pipeline::narrowphase`, `rtk proxy cargo test -p picea --test physics_realism_acceptance ccd`, `rtk proxy cargo test -p picea --lib --tests` |
| `query` | `QueryPipeline`、`QueryFilter`、`QueryStats`、`AabbHit`、`PointHit`、`RayHit`、`QueryShape`、`ShapeHit`、`QueryShapeError`；M15 已通过内部 broadphase-style index 做 AABB/point/ray 候选复用，M17 通过 `QueryPipeline::last_stats()` 暴露最近一次查询的 traversal/candidate/prune/filter-drop/hit counters，同时保持 public hit ordering/filter semantics；M21 已增加 public distance / shape query API。 | 不直接修改 authoritative world state，不暴露 broadphase proxy/leaf id；M21 public surface 不把内部 GJK/cache detail 当 public contract。 | `query.rs` | `rtk proxy cargo test -p picea --test query_debug_contract`, `rtk proxy cargo test -p picea --test world_step_review_regressions`, `rtk proxy cargo test -p picea --lib pipeline::gjk`, `rtk proxy cargo test -p picea --lib --tests` |
| `recipe` | `BodyBundle`、`ColliderBundle`、`JointBundle`、`WorldRecipe::with_joint`、transactional `WorldCommands`、material/collision-layer presets、M14 scene/asset recipe helpers、serializable fixture setup；M22 在 scene authoring 层用已有 bundle/recipe path 表达 compound convex pieces；M27 继续只消费 lab 生成的 convex pieces。 | 不改低层 `World::create_body` / `World::create_collider` / `World::create_joint` 语义，不承载 solver 或 pipeline behavior；M27 不表示 core solver 直接支持 arbitrary concave contact。 | `recipe.rs`, `benches/physics_scenarios.rs` | `rtk proxy cargo test -p picea --test v1_api_smoke`, `rtk proxy cargo test -p picea --test core_model_world`, `rtk proxy cargo bench -p picea --no-run` |
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
| `scenario` | 内置 deterministic 场景、reset-time overrides、`RunConfig`；包含 M13 `ccd_fast_circle_wall` / `ccd_fast_convex_walls` CCD evidence、M19 `ccd_dynamic_convex_pair` 动态目标 CCD evidence、M20 versioned `SceneRecipeFixture` / joint authoring / backward-compatible fixture loading；M22 已接入 compound convex piece authoring、direct concave rejection、top-level/piece convex validation 和 stable scene-path errors；M26 增加 `ccd_dynamic_compound_wall` evidence；M27 增加 static `concave_polygon` authoring-time decomposition 和 `concave_decomposition` evidence。 | 不读写 artifacts，不持有 live session 状态，不自行运行物理逻辑；不把 M27 authoring support 解读成 lab 或 core 直接运行 arbitrary concave solver；dynamic concave 仍拒绝。 | `crates/picea-lab/src/scenario.rs` | `rtk proxy cargo test -p picea-lab` |
| `artifact` | headless runner、`manifest.json` / `frames.jsonl` / `debug_render.json` / `final_snapshot.json` / `perf.json` 写入；`frames.jsonl` 与 `final_snapshot.json` 保留 core `StepStats` / `DebugStats` CCD counters 和 contact `ccd_trace`，包括 M19 dynamic-target `target_kind` / target sweep facts 和 M26 dynamic compound selected-piece trace facts；M27 generated concave-decomposition pieces 进入 `compound_provenance`；M17 `perf.json.counter_summary` 汇总 deterministic work counters，debug render frames 携带 broadphase traversal/prune 与 island/solver row counters；M24 计划把 broadphase tree、island lifecycle、compound provenance 做成 lab-web 过程可视化；2026-05-06 performance/stability observability 增加 lab-owned `FrameDiagnostics`，用于 per-frame counter delta、penetration、contact churn、warm-start、impulse、sleep/island summary、diagnostic marker、source label 和 missing evidence。 | 不直接服务 HTTP，不把 target 路径暴露给 UI，不从 lab 侧重新计算 CCD，不把 wall-clock timing 当正确性 oracle；`FrameDiagnostics` 是 artifact / lab diagnostics carrier，不代表 solver 已修复。 | `crates/picea-lab/src/artifact.rs` | `rtk proxy cargo test -p picea-lab --test artifact_run` |
| `server` | 本地 HTTP + SSE protocol、session 状态、artifact 下载；M25-A 下 server 同时拥有 artifact replay session 与 request-driven live session，live session 在 Rust 端持有 authoritative `World + SimulationPipeline`；session 明确暴露 `manifest.json` / `final_snapshot.json` replay provenance，empty SSE 使用 idle event 而不是 failed；M25-B 固定 live patch/transaction 语义文档和拒绝边界；2026-05-07 live session 支持 create-time unbounded stepping、retained ring window，以及窄 `POST /api/sessions/:id/gravity` live gravity apply。 | 不接受 generic live session overrides/patch；除窄 gravity apply 外，body/collider/joint paused transaction 必须另过 milestone；artifact replay reset 仍走 runner；不把空事件队列当成模拟失败；evicted live frames 不由 web 伪造重建。 | `crates/picea-lab/src/server.rs` | `rtk proxy cargo test -p picea-lab --test server_routes` |
| `cli` | `picea-lab list`、`run`、`serve` 命令。 | 不拥有 artifact schema 或 scenario 构建细节。 | `crates/picea-lab/src/cli.rs`, `main.rs` | `rtk proxy cargo test -p picea-lab` |
| `web` | React + Canvas 2D replay workbench, hierarchy, inspector, timeline, overlays；joint rows are selectable；source badge 区分 demo / Rust artifact replay / Rust live session；M25-A 的 play 只是前端连续触发 backend step，请求返回后再追加 live buffer。M40 closeout 还把 scenario grouping、overlay presets、copy debug context、stack/trajectory/lattice/perturbation evidence 收口在前端工作台。2026-05-06 diagnostics surface typed 消费 `FrameRecord.diagnostics`，在 timeline rail、Diagnostics tab、Evidence tab 和 copy-debug-context 中展示 performance/stability source 与 missing evidence。2026-05-07 runtime controls 把 header run 固定为新建/重新运行 session，timeline play 固定为当前 session pause/resume；live buffer 使用 retained window；重力轮盘在 artifact replay 写 pending run config，在 live session 通过 Apply 发送窄 runtime gravity patch，并支持 draft reset / undo。 | 不运行 physics，不替代 Rust artifact schema，不发明 server-side autoplay / long-lived tick contract；不把重力轮盘扩大成 arbitrary running-world editor；M40 的 lattice 仍是 rigid-body proxy，不是 true soft-body；缺失 diagnostics 时必须显示 missing，不伪装成 0 成本或 0 风险。 | `crates/picea-lab/web/src/*` | `rtk proxy just picea-lab-web`; `npm run build` from `crates/picea-lab/web`; `npm run test:ui-contract`; `npm run test:i18n`; browser acceptance 优先 `browser-use:browser` |

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
- `docs/public-beta.md`：M30 public beta surface、迁移说明、示例和最终验收矩阵
- `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`：当前生产化 milestone 边界、M11-M22 完成状态、M23-M30 计划 gate 和 Post-M30 follow-up
- `docs/design/2026-06-17-physics-realism-vnext-architecture.md`：物理真实感 vNext 七方向架构包，覆盖 stack/contact、material/damping、CCD、complex shape、joints、observability、deformable RFC 边界，以及 “路线 -> ScenarioId -> 参数面板 -> lab-web 展示面 -> 验收入口” 能力展示矩阵
- `docs/plans/2026-06-17-physics-realism-vnext-milestones.md`：物理真实感 vNext 执行计划、subagent 分派边界、E4 direct-concave query gate、E6/V8 lab-web/browser 能力展示验收
- `docs/design/2026-07-13-sat-manifold-persistence-design.md`：已冻结的handoff §4架构合同，记录SAT raw feature与persistent lifecycle/warm-start/source-row分层、geometry revision invalidation及exact-hard residual最大基数matcher
- `docs/plans/2026-07-13-vnext-s4-manifold-persistence-milestone.md`：已完成并冻结的handoff §4执行与验收证据；包含full workspace、clippy与matrix long-window gates，不改变父计划E4 complex-shape含义
- `docs/design/physics-engine-upgrade-technical-plan.md`：Post-M20 baseline、M21 public query 和 M22 authoring boundary 之后的系统升级设计方向
- `docs/design/matrix-stack-stability-optimization-design.md` 与 `docs/design/matrix-stack-stability-acceptance.md`：matrix stack 稳定性优化顺序和验收窗口
- `docs/design/dense-pressure-position-row-architecture.md`：dense pressure / pseudo-position / position-row 已落地架构记录
- `docs/design/scene-runtime-config-and-parameter-ui-design.md`：scene-owned runtime config 和参数 UI 设计
- `docs/design/deformable-body-roadmap.md`：future soft-body / particle / cloth / deformable mesh RFC；先读它再讨论 true deformable physics，不要把 M38 rigid-body lattice proxy 当作已实现 soft-body
- `docs/design/performance-threshold-policy.md`：M29 benchmark baseline、warn/fail 和 fallback policy
- `docs/architecture/legacy-scene-runtime.md`：旧 `Scene` / `Context` runtime 和 collision/constraint 归档；不要作为当前默认路由
- `docs/plans/2026-04-18-picea-physics-engine-milestones.md`：历史归档；不要把旧 `Scene` / `Context` / `picea-web` / wasm 条目当作当前默认路由
