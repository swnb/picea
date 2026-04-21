# AI 路由索引

先读 `docs/ai/repo-map.md` 和当前 git/验证命令输出，再按下面的问题类型找入口。

## 问题类型 -> 去哪里看

| 问题类型 | 先看文档 | 再看代码 | 常用命令 |
| --- | --- | --- | --- |
| 当前进度、最近验证、已知 warning | `docs/plans/2026-04-18-picea-physics-engine-milestones.md` | 当前 `git status` / `git log` / 验证命令输出 | `rtk proxy cargo test -p picea --lib` |
| 模块职责、谁负责什么 | `docs/ai/repo-map.md` | 对应 `crates/picea/src/*` | `rtk proxy cargo test -p picea --lib` |
| 架构图、运行时流程、模块边界 | `docs/architecture/README.md` | `crates/picea/src/scene/*`, `collision/*`, `constraints/*` | 先读图，再选模块测试 |
| 设计目标、非目标、未来扩展点 | `docs/design/README.md` | 对应设计文档指向的模块 | 先确认 milestone 边界 |
| 可视化、debug artifact、replay、性能观测工具方向 | `docs/design/picea-lab-observability-architecture.md`, `docs/ai/debug-artifacts.md` | `crates/picea/src/tools/*`, `crates/picea-lab/src/main.rs`, `crates/picea-lab/src/viewer.rs`, `crates/picea/benches/physics_scenarios.rs` | `rtk proxy cargo test -p picea tools::observability --lib`、`rtk proxy cargo test -p picea-lab` |
| 几何、shape、拆分、缓存 | `docs/ai/repo-map.md` | `crates/picea/src/shape/*`, `crates/picea/src/math/*` | `rtk proxy cargo test -p picea shape::concave --lib` |
| 碰撞、broadphase、manifold、warm start | `docs/ai/repo-map.md` | `crates/picea/src/collision/*`, `crates/picea/src/constraints/*`, `crates/picea/src/scene/*` | `rtk proxy cargo test -p picea contact_identity --lib` |
| 约束求解、position/velocity solve、sleep | `docs/ai/repo-map.md` | `crates/picea/src/constraints/*`, `crates/picea/src/scene/*`, `crates/picea/src/meta/*` | `rtk proxy cargo test -p picea --lib` |
| wasm public API、TS 类型、smoke 测试 | `docs/ai/repo-map.md` | `crates/picea-web/src/*` | `rtk proxy cargo test -p picea-web --lib` |
| proc macro、derive、builder helper | `docs/ai/repo-map.md` | `crates/macro-tools/src/*` | `rtk proxy cargo test -p picea-macro-tools` |
| milestone 范围、硬边界、subagent 分工 | `docs/plans/2026-04-18-picea-physics-engine-milestones.md` | 对应 milestone 代码段 | 先读计划，再选测试门 |
| 只想确认入口文件 | `docs/ai/repo-map.md` | `crates/picea/src/lib.rs`, `crates/picea-web/src/lib.rs`, `crates/macro-tools/src/lib.rs` | `rg --files crates docs` |

## 读法

- 先确认自己是在问“状态”、还是在问“模块”、还是在问“怎么验证”。
- 如果是 bug 或回归，先确认当前 git/HEAD/验证输出，再按 `repo-map.md` 找模块和测试。
- 如果是新功能或结构调整，先回 milestone 计划，确认没越过边界。
- 如果是 wasm 问题，优先查 `picea-web`，不要先去碰 core 物理实现。
