# Repo Map

仓库是一个 Rust workspace，当前三 crate：

- `crates/picea`：核心 2D physics engine
- `crates/picea-web`：wasm public API
- `crates/macro-tools`：proc macro helpers

## `crates/picea`

### `crates/picea/src/lib.rs`
- owns：对外模块总入口和 `prelude` 重导出。
- does_not_own：不放具体物理逻辑，不放 wasm 接口，不放文档状态。
- entrypoints：`pub mod ...`、`pub mod prelude`。
- tests：`rtk proxy cargo test -p picea --lib`。

### `crates/picea/src/scene/*`
- owns：`Scene` 的 tick、pipeline 编排、sleep、callback、状态流转。
- does_not_own：不定义 shape 几何算法，不替代 collision/constraint 公式。
- entrypoints：`scene/mod.rs`、`scene/context.rs`、`scene/hooks.rs`。
- tests：`rtk proxy cargo test -p picea scene::tests --lib`、`rtk proxy cargo test -p picea sleep_mode_ --lib`。

### `crates/picea/src/collision/*`
- owns：broadphase、narrowphase seam、AABB cache、manifold lifecycle、contact pair 发现。
- does_not_own：不调 solver 真实感参数，不管 wasm 类型。
- entrypoints：`collision/mod.rs`。
- tests：`rtk proxy cargo test -p picea rough_collision_detection --lib`、`rtk proxy cargo test -p picea contact_identity --lib`。

### `crates/picea/src/constraints/*`
- owns：contact / join / point 的 solver math、warm start 消费、position/velocity solve。
- does_not_own：不做 broadphase，不做 shape 缓存，不做 UI。
- entrypoints：`constraints/contact.rs`、`constraints/join.rs`、`constraints/point.rs`、`constraints/contact_manifold.rs`。
- tests：`rtk proxy cargo test -p picea contact::tests --lib`、`rtk proxy cargo test -p picea point::tests --lib`。

### `crates/picea/src/shape/*`
- owns：local/world 几何、凸凹分解、support/center/AABB 相关 shape 行为。
- does_not_own：不做 scene step，不做 wasm 解析。
- entrypoints：`shape/mod.rs`、`shape/concave.rs`、`shape/convex.rs`、`shape/polygon.rs`、`shape/utils.rs`。
- tests：`rtk proxy cargo test -p picea shape::concave --lib`、`rtk proxy cargo test -p picea --lib`。

### `crates/picea/src/element/*`
- owns：element 数据、handle/存储、clone 语义、id 查询。
- does_not_own：不管 narrowphase 细节，不管 wasm API。
- entrypoints：`element/mod.rs`、`element/store.rs`。
- tests：`rtk proxy cargo test -p picea element::store::tests --lib`。

### `crates/picea/src/math/*`
- owns：`Point`、`Vector`、`Matrix`、`Segment`、`Edge`、`FloatNum` 等基础数值类型与运算。
- does_not_own：不承载 scene policy，不做 milestone 决策。
- entrypoints：`math/mod.rs`、`math/vector.rs`、`math/point.rs`、`math/segment.rs`。
- tests：`rtk proxy cargo test -p picea --lib`。

### `crates/picea/src/meta/*`
- owns：mass、inertia、kinetic 等 body metadata。
- does_not_own：不负责调用者输入校验边界，不做 wasm wrapper。
- entrypoints：`meta/mod.rs`、`meta/force.rs`。
- tests：`rtk proxy cargo test -p picea --lib`。

### `crates/picea/src/tools/*`
- owns：调试视图、拖拽、中间件、snapshot 之类工具。
- does_not_own：不作为 physics 核心行为来源。
- entrypoints：`tools/mod.rs`。
- tests：跟随 `picea --lib` / examples 验证。

## `crates/picea-web`

### `crates/picea-web/src/lib.rs`
- owns：wasm crate 的公开入口。
- does_not_own：不放核心求解逻辑，不放桌面 UI。
- entrypoints：`common.rs`、`wasm.rs`。
- tests：`rtk proxy cargo test -p picea-web --lib`，需要 wasm smoke 时再跑 `--target wasm32-unknown-unknown`。

### `crates/picea-web/src/common.rs` / `wasm.rs`
- owns：JS/TS 侧可见 API、错误通道、smoke helpers。
- does_not_own：不改 core solver 公式。
- tests：`rtk proxy cargo test -p picea-web --lib`、`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner rtk proxy cargo test -p picea-web --lib --target wasm32-unknown-unknown`。

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
