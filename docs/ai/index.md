# AI 路由索引

所有问题先看实时仓库事实：当前 `git status` / `HEAD` / 验证命令输出、Cargo manifests，以及 `crates/picea/src/lib.rs`。文档只负责路由。

`docs/plans/2026-04-25-picea-physics-engine-production-milestones.md` 是当前生产化 milestone 执行规划。`docs/plans/2026-04-18-picea-physics-engine-milestones.md` 只在任务明确需要历史背景时再看；其中旧 `Scene` / `Context` / `picea-web` / wasm 叙述是归档，不是当前默认验证目标。

## 问题类型 -> 去哪里看

| 问题类型 | 先看文档 | 再看代码 | 常用命令 |
| --- | --- | --- | --- |
| 当前公共 crate surface、模块入口、测试落点 | `docs/ai/repo-map.md` | `crates/picea/src/lib.rs`, 对应 `crates/picea/src/*` | `rtk proxy cargo test -p picea --lib` |
| 当前进度、最近验证、已知 warning | `README.md` | 当前 `git status` / `git log` / 验证命令输出 | `rtk proxy cargo test -p picea --lib --no-run` |
| 当前 crate 依赖图、`picea-lab` / `macro-tools` 是否仍被 core 直接依赖 | `docs/ai/repo-map.md` | `Cargo.toml`, `crates/*/Cargo.toml` | `rtk proxy cargo tree -p picea` |
| 模块职责、谁负责什么 | `docs/ai/repo-map.md` | 对应 `crates/picea/src/*` | `rtk proxy cargo test -p picea --lib` |
| 架构图、当前 crate/module 边界 | `docs/architecture/system-overview.md` | `crates/picea/src/lib.rs`, `world/*`, `pipeline/*`, `solver/*` | 先读图，再选模块测试 |
| 设计目标、非目标、未来扩展点 | `docs/design/README.md` | 对应设计文档指向的模块 | 先确认 milestone 边界 |
| 稳定 world API、debug snapshot、query | `docs/ai/repo-map.md` | `crates/picea/src/world/*`, `debug.rs`, `query.rs` | `rtk proxy cargo test -p picea --test world_step_review_regressions` |
| Step orchestration / transient step facts / CCD phase boundary | `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`, `docs/ai/repo-map.md` | `crates/picea/src/pipeline/step.rs`, `crates/picea/src/pipeline/ccd.rs`, `crates/picea/src/pipeline/contacts.rs`, `crates/picea/src/solver/contact.rs` | `rtk proxy cargo test -p picea --test physics_realism_acceptance ccd`; `rtk proxy cargo test -p picea --lib` |
| M15 performance data path / query-cache-pre-sizing | `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`, `docs/design/physics-engine-upgrade-technical-plan.md` | `crates/picea/src/pipeline/broadphase.rs`, `crates/picea/src/query.rs`, `crates/picea/src/collider.rs`, `crates/picea/src/pipeline/contacts.rs`, `crates/picea/src/pipeline/gjk.rs`, `crates/picea/src/pipeline/sleep.rs`, `crates/picea/src/solver/contact.rs`, `crates/picea/benches/physics_scenarios.rs` | `rtk proxy cargo test -p picea --lib pipeline::broadphase`; `rtk proxy cargo test -p picea --lib pipeline::gjk`; `rtk proxy cargo test -p picea --test query_debug_contract`; `rtk proxy cargo test -p picea --test world_step_review_regressions`; `rtk proxy cargo bench -p picea --no-run` |
| M16 dense island execution / solver data layout | `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`, `docs/design/physics-engine-upgrade-technical-plan.md` | `crates/picea/src/pipeline/island.rs`, `crates/picea/src/solver/contact.rs`, `crates/picea/src/pipeline/sleep.rs`, `crates/picea/src/pipeline/joints.rs`, `crates/picea/src/pipeline/step.rs` | `rtk proxy cargo test -p picea --test physics_realism_acceptance stack`; `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep`; `rtk proxy cargo test -p picea --test world_step_review_regressions`; `rtk proxy cargo test -p picea --lib pipeline::island`; `rtk proxy cargo test -p picea --lib pipeline::sleep`; `rtk proxy cargo bench -p picea --no-run` |
| M12 active island solver | `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`, `docs/design/physics-engine-upgrade-technical-plan.md` | `crates/picea/src/pipeline/sleep.rs`, `crates/picea/src/pipeline/joints.rs`, `crates/picea/src/solver/contact.rs`, `crates/picea/src/pipeline/step.rs` | `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep`; `rtk proxy cargo test -p picea --test world_step_review_regressions` |
| M13 CCD generalization | `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`, `docs/design/physics-engine-upgrade-technical-plan.md` | `crates/picea/src/pipeline/ccd.rs`, `crates/picea/src/pipeline/gjk.rs`, `crates/picea/src/pipeline/narrowphase.rs` | `rtk proxy cargo test -p picea --test physics_realism_acceptance ccd`; `rtk proxy cargo test -p picea --lib pipeline::ccd` |
| M14 ergonomic API v2 / scene recipes | `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`, `docs/design/physics-engine-upgrade-technical-plan.md` | `crates/picea/src/recipe.rs`, `crates/picea/src/world/api.rs`, `crates/picea-lab/src/scenario.rs` | `rtk proxy cargo test -p picea --test v1_api_smoke`; `rtk proxy cargo test -p picea --test core_model_world` |
| 数学类型与新 algebra API | `docs/ai/repo-map.md` | `crates/picea/src/math/*` | `rtk proxy cargo test -p picea --test math_api_compile_fail` |
| proc macro、`Accessors`/`Builder`/`Deref` helper（独立 workspace crate） | `docs/ai/repo-map.md` | `crates/macro-tools/src/*` | `rtk proxy cargo test -p picea-macro-tools` |
| C/S simulator、artifact schema、HTTP/SSE server、React Canvas replay workbench | `docs/ai/repo-map.md`, `docs/design/picea-lab-observability-architecture.md` | `crates/picea-lab/src/*`, `crates/picea-lab/web/src/*` | `rtk proxy cargo test -p picea-lab`; `npm run build` / `npm run test:ui-contract` / `npm run test:i18n` in `crates/picea-lab/web` |
| 当前生产化 milestone 范围、目标、验收方法 | `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md` | 对应仍存在的代码模块 | 先读计划，再选对应 targeted gate |
| 旧 milestone 执行历史和归档记录 | `docs/plans/2026-04-18-picea-physics-engine-milestones.md` | 仅用于历史背景，不做当前 routing | 不作为当前默认验证目标 |
| 旧 `Scene` / `Context` / `picea-web` / wasm gate 历史（归档） | `docs/plans/2026-04-18-picea-physics-engine-milestones.md` | 仅用于历史背景，不做当前 routing | 不作为当前默认验证目标 |
| 只想确认入口文件 | `docs/ai/repo-map.md` | `crates/picea/src/lib.rs`, `crates/picea-lab/src/lib.rs`, `crates/macro-tools/src/lib.rs` | `rtk rg --files crates docs` |

## 读法

- 先确认自己是在问“状态”、还是在问“模块”、还是在问“怎么验证”。
- 如果是 bug 或回归，先确认当前 git/HEAD/验证输出，再按 `repo-map.md` 找模块和测试。
- 如果只是当前模块、依赖图或验证问题，不要先读 milestone 计划；先看 live repo facts 和 `repo-map.md` / `system-overview.md`。
- 如果是新功能或结构调整，先确认 live repo facts，再在任务明确是 milestone work 时回 milestone 计划确认边界。
- 如果是在确认当前依赖图，先看 `Cargo.toml` / `crates/*/Cargo.toml` 和 `rtk proxy cargo tree -p picea`；不要把 workspace 成员直接读成 core runtime 依赖。
- 如果是新 core 行为问题，优先查 `world/*`、`pipeline/*`、`solver/*`，当前以 `World` + `SimulationPipeline` 路径为准；旧 `Scene` narrative 只在归档里保留。
