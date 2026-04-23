# Repo Map

仓库是一个 Rust workspace，当前两个 crate：

- `crates/picea`：核心 2D physics engine
- `crates/macro-tools`：proc macro helpers

## `crates/picea`

### `crates/picea/src/lib.rs`
- owns：对外模块总入口和 `prelude` 重导出。
- does_not_own：不放具体物理逻辑，不放 wasm 接口，不放文档状态。
- entrypoints：`pub mod ...`、`pub mod prelude`。
- tests：`rtk proxy cargo test -p picea --lib`。

### `crates/picea/src/math/*`
- owns：`Point`、`Vector`、`Matrix`、`Segment`、`Edge`、`FloatNum` 等基础数值类型与运算。
- does_not_own：不承载 scene policy，不做 milestone 决策。
- entrypoints：`math/mod.rs`、`math/vector.rs`、`math/point.rs`、`math/segment.rs`。
- tests：`rtk proxy cargo test -p picea --test math_api_compile_fail`、`rtk proxy cargo test -p picea --lib --tests`。

### `crates/picea/src/world/*`
- owns：`World` 状态、lifecycle API、runtime retained facts、error/store/contact state。
- does_not_own：不承载低层数学兼容或消费者壳。
- entrypoints：`world.rs`、`world/api.rs`、`world/store.rs`、`world/runtime.rs`、`world/error.rs`、`world/contact_state.rs`。
- tests：`rtk proxy cargo test -p picea --test core_model_world`、`rtk proxy cargo test -p picea --test world_step_review_regressions`。

### `crates/picea/src/pipeline/*`
- owns：`SimulationPipeline` 与 step orchestration。
- does_not_own：不拥有 world 持久状态。
- entrypoints：`pipeline.rs`、`pipeline/step.rs`、`pipeline/integrate.rs`、`pipeline/contacts.rs`、`pipeline/joints.rs`、`pipeline/sleep.rs`。
- tests：`rtk proxy cargo test -p picea --test v1_api_smoke`、`rtk proxy cargo test -p picea --lib --tests`。

### `crates/picea/src/solver/*`
- owns：新的 world 路径内部求解辅助实现。
- does_not_own：不提供旧 Scene/constraint surface。
- entrypoints：`solver/mod.rs`、`solver/body_state.rs`。
- tests：跟随 `rtk proxy cargo test -p picea --lib --tests`。

## `crates/macro-tools`

### `crates/macro-tools/src/lib.rs`
- owns：derive/attribute macro 入口。
- does_not_own：不关心 physics 运行时状态。
- entrypoints：`builder.rs`、`deref.rs`、`fields.rs`。
- tests：`rtk proxy cargo test -p picea-macro-tools`。

## 文档入口

- `docs/ai/index.md`：问题类型路由
- `docs/ai/doc-catalog.yaml`：文档和关键代码索引
- `docs/plans/2026-04-18-picea-physics-engine-milestones.md`：里程碑与硬边界
