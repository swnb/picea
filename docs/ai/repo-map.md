# Repo Map

仓库是一个 Rust workspace，当前四个 crate：

- `crates/picea`：核心 2D physics engine
- `crates/picea-lab`：headless observability artifact CLI
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
- owns：调试视图、拖拽、中间件、snapshot、observability artifact 之类工具。
- does_not_own：不作为 physics 核心行为来源。
- entrypoints：`tools/mod.rs`。
- tests：`rtk proxy cargo test -p picea tools::observability --lib`，并跟随 `picea --lib` / examples 验证。

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

## `crates/picea-lab`

### `crates/picea-lab/src/main.rs`
- owns：Picea Lab CLI，生成/读取/比较 observability artifacts，并打开 native artifact viewer。
- does_not_own：不实现 physics，不改 core tick 默认行为，不替代 `picea` 测试门。
- entrypoints：`capture-contact <output-dir>`、`replay-contact <output-dir> <run-id> <second-circle-x> <steps>`、`capture-benchmark <output-dir> <run-id> <scenario> <steps>`、`diff <left-dir> <right-dir>`、`view <artifact-dir>`、`export-verification <artifact-dir> <output-md>`。
- tests：`rtk proxy cargo test -p picea-lab`、`rtk proxy cargo run -p picea-lab -- capture-contact target/picea-lab/runs/contact-smoke`、`rtk proxy cargo run -p picea-lab -- replay-contact target/picea-lab/runs/contact-replay replay 1.5 3`、`rtk proxy cargo run -p picea-lab -- capture-benchmark target/picea-lab/runs/bench-contact bench-contact contact_refresh_transfer 2`、`rtk proxy cargo run -p picea-lab -- export-verification target/picea-lab/runs/contact-smoke target/picea-lab/runs/contact-smoke/verification.md`。

### `crates/picea-lab/src/recipes.rs`
- owns：共享 replay / benchmark recipe、场景构造和 artifact capture 入口。
- does_not_own：不做 UI，不做文件写入路径分发，不直接承担 CLI 参数解析。
- entrypoints：`RunRecipe`、`BenchmarkScenario`、`capture_recipe`、`capture_benchmark_artifacts_cli`。
- tests：跟随 `rtk proxy cargo test -p picea-lab` 验证。

### `crates/picea-lab/src/viewer.rs`
- owns：artifact viewer model、filtering、verification summary export、native `egui/eframe` app shell，以及 recipe/regenerate/render controls。
- does_not_own：不运行 physics，不直接读取 private core state，不解释截图为 correctness proof。
- entrypoints：`ViewerModel::load_from_dir`、`ViewerModel::verification_markdown`、`run_viewer`。
- tests：`rtk proxy cargo test -p picea-lab viewer_model_inspects_filters_and_exports_verification_summary`、`rtk proxy cargo test -p picea-lab viewer_model_can_regenerate_after_parameter_change`、`rtk proxy cargo test -p picea-lab viewer_model_supports_basic_view_navigation_and_selection`。

### `crates/picea-lab/web/*`
- owns：browser artifact viewer，读取同一套 `final_snapshot.json` / `debug_render.json` / `trace.jsonl` / `perf.json` fixture 或用户选择的 artifact 文件。
- does_not_own：不运行 physics，不加载 wasm runtime，不复制 core simulation。
- entrypoints：`web/index.html`，默认 fixture 在 `web/fixtures/contact-smoke/`。
- tests：`rtk proxy cargo test -p picea-lab web_viewer_static_assets_and_fixture_are_discoverable`；浏览器验证可用 `python3 -m http.server 4177 --bind 127.0.0.1` 后打开 `http://127.0.0.1:4177/`。

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
