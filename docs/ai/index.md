# AI 路由索引

先读 `docs/ai/repo-map.md` 和当前 git/验证命令输出，再按下面的问题类型找入口。

## 问题类型 -> 去哪里看

| 问题类型 | 先看文档 | 再看代码 | 常用命令 |
| --- | --- | --- | --- |
| 当前进度、最近验证、已知 warning | `docs/plans/2026-04-18-picea-physics-engine-milestones.md` | 当前 `git status` / `git log` / 验证命令输出 | `rtk proxy cargo test -p picea --lib` |
| 模块职责、谁负责什么 | `docs/ai/repo-map.md` | 对应 `crates/picea/src/*` | `rtk proxy cargo test -p picea --lib` |
| 架构图、运行时流程、模块边界 | `docs/architecture/README.md` | `crates/picea/src/world/*`, `pipeline/*`, `solver/*` | 先读图，再选模块测试 |
| 设计目标、非目标、未来扩展点 | `docs/design/README.md` | 对应设计文档指向的模块 | 先确认 milestone 边界 |
| 稳定 world API、debug snapshot、query | `docs/ai/repo-map.md` | `crates/picea/src/world/*`, `debug.rs`, `query.rs` | `rtk proxy cargo test -p picea --test world_step_review_regressions` |
| 数学类型与新 algebra API | `docs/ai/repo-map.md` | `crates/picea/src/math/*` | `rtk proxy cargo test -p picea --test math_api_compile_fail` |
| proc macro、derive、builder helper | `docs/ai/repo-map.md` | `crates/macro-tools/src/*` | `rtk proxy cargo test -p picea-macro-tools` |
| milestone 范围、硬边界、subagent 分工 | `docs/plans/2026-04-18-picea-physics-engine-milestones.md` | 对应 milestone 代码段 | 先读计划，再选测试门 |
| 只想确认入口文件 | `docs/ai/repo-map.md` | `crates/picea/src/lib.rs`, `crates/macro-tools/src/lib.rs` | `rg --files crates docs` |

## 读法

- 先确认自己是在问“状态”、还是在问“模块”、还是在问“怎么验证”。
- 如果是 bug 或回归，先确认当前 git/HEAD/验证输出，再按 `repo-map.md` 找模块和测试。
- 如果是新功能或结构调整，先回 milestone 计划，确认没越过边界。
- 如果是新 core 行为问题，优先查 `world/*`、`pipeline/*`、`solver/*`，不要假设旧 `Scene` 路径仍然存在。
