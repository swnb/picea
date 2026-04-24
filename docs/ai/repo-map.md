# Repo Map

当前路由优先级：先看实时仓库事实（`git status`、Cargo manifests、`crates/picea/src/lib.rs`、最新验证输出），再用本文定位模块。

`docs/plans/2026-04-18-picea-physics-engine-milestones.md` 只用于当前仍有效的 milestone 边界或历史归档；其中旧 `Scene` / `Context` / `picea-web` / wasm 叙述不代表当前默认路由。

仓库是一个 Rust workspace，当前两个 crate：

- `crates/picea`：核心 2D physics engine
- `crates/macro-tools`：独立 proc-macro crate；在 workspace 内单独验证，当前不在 `crates/picea` 的直接依赖图上

## `crates/picea`

### `crates/picea/src/lib.rs`
- owns：当前 public crate-root surface 和 `prelude` 重导出。
- does_not_own：不放具体物理逻辑，不放 wasm 接口，不放文档状态。
- entrypoints：`algo`、`body`、`collider`、`debug`、`events`、`handles`、`joint`、`math`、`pipeline`、`query`、`world`、`prelude`。
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
| `pipeline` | `SimulationPipeline`、`StepConfig`、`StepReport`、step orchestration。 | 不拥有 world 持久状态。 | `pipeline.rs`, `pipeline/step.rs`, `pipeline/integrate.rs`, `pipeline/contacts.rs`, `pipeline/joints.rs`, `pipeline/sleep.rs` | `rtk proxy cargo test -p picea --test v1_api_smoke`, `rtk proxy cargo test -p picea --lib --tests` |
| `query` | `QueryPipeline`、`QueryFilter`、`AabbHit`、`PointHit`、`RayHit`。 | 不直接修改 authoritative world state。 | `query.rs` | `rtk proxy cargo test -p picea --test world_step_review_regressions`, `rtk proxy cargo test -p picea --lib --tests` |
| `world` | `World` 状态、lifecycle API、runtime retained facts、error/store/contact state。 | 不承载低层数学兼容或消费者壳。 | `world.rs`, `world/api.rs`, `world/store.rs`, `world/runtime.rs`, `world/error.rs`, `world/contact_state.rs` | `rtk proxy cargo test -p picea --test core_model_world`, `rtk proxy cargo test -p picea --test world_step_review_regressions` |

### `crates/picea/src/solver/*` (internal)
- owns：当前 `World` + `SimulationPipeline` 路径内部求解辅助实现。
- does_not_own：不作为 public crate-root surface 暴露，不承担路由入口职责。
- entrypoints：`solver/mod.rs`、`solver/body_state.rs`。
- tests：跟随 `rtk proxy cargo test -p picea --lib --tests`。

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
- `docs/plans/2026-04-18-picea-physics-engine-milestones.md`：当前仍有效的 milestone 边界与历史归档；不要把旧 `Scene` / `Context` / `picea-web` / wasm 条目当作当前默认路由
