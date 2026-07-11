# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 仓库是什么

Picea 是用 Rust 写的 2D 物理引擎(开发中),workspace 含三个 crate:

- `crates/picea`:核心引擎。公开 beta surface 是 `World` + `SimulationPipeline` + `QueryPipeline` + `DebugSnapshot` + `WorldRecipe`(见 `crates/picea/src/lib.rs` 的 `prelude`)。
- `crates/picea-lab`:本地 C/S 模拟器(确定性场景、artifact 采集、HTTP/SSE server、CLI),`web/` 下是 React + Canvas workbench。
- `crates/macro-tools`:独立 proc-macro crate(`Accessors` / `Builder` / `Deref`),单独验证,**不在** `crates/picea` 的依赖图上。

## 先读什么(权威顺序)

仓库已有一套 AI 路由文档体系,本文件不重复其内容,按此顺序路由:

1. 当前工作区事实和验证命令输出(`git status`、Cargo manifests、`crates/picea/src/lib.rs`)。
2. `AGENTS.md` — 根规则:命令约束、dirty 保护、权威顺序。
3. `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md` — 当前生产 milestone 边界与验收门;物理真实感 vNext 计划在 `docs/plans/2026-06-17-physics-realism-vnext-milestones.md`。
4. `docs/ai/repo-map.md` — 每个模块的 owns / does-not-own 边界、入口文件、targeted test。
5. `docs/ai/index.md` — 问题类型 → 该读哪个文档;`docs/ai/doc-catalog.yaml` 是文档状态(frozen/deprecated)清单。
6. `docs/ai/debug-playbook.md` — 调 bug / 物理回归前先读;仓库内开发规范 skills 在 `.agents/skills/*/SKILL.md`。

文档与代码冲突时,以仓库现状和命令真实输出为准,并同步更新 AI 文档。

## 命令

按仓库约定,cargo / npm / curl 这类验证命令加 `rtk proxy` 前缀。

### 测试

```bash
rtk proxy cargo test -p picea --lib                      # 核心库单元测试
rtk proxy cargo test -p picea-lab                        # lab crate 测试
rtk proxy cargo test -p picea-macro-tools                # proc-macro crate(独立 gate)
rtk proxy cargo test -p picea --examples --no-run        # public beta 示例编译门
rtk proxy cargo test --workspace --all-targets --no-run  # 全 workspace 编译门
```

单个测试 / 模块:

```bash
rtk proxy cargo test -p picea --lib pipeline::sleep                  # 按模块路径过滤单元测试
rtk proxy cargo test -p picea --test physics_realism_acceptance ccd  # 集成测试文件 + 名称过滤
rtk proxy cargo test -p picea-lab --test server_routes               # lab 单个集成测试文件
```

集成测试在 `crates/picea/tests/`(`core_model_world`、`physics_realism_acceptance`、`query_debug_contract`、`world_step_review_regressions`、`v1_api_smoke` 等)和 `crates/picea-lab/tests/`(`artifact_run`、`server_routes`)。每个模块对应的 targeted gate 查 `docs/ai/repo-map.md` 的表格。

### Web workbench(crates/picea-lab/web)

```bash
just picea-lab-web                                                # 前后端一起起(API 默认 127.0.0.1:8080,Vite 5173)
just web-start / web-stop / status / logs                         # 后台生命周期
rtk proxy npm --prefix crates/picea-lab/web run build             # tsc + vite build
rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract  # UI 契约
rtk proxy npm --prefix crates/picea-lab/web run test:i18n         # i18n 契约
```

端口被占用时用 `PICEA_LAB_BIND` / `PICEA_LAB_WEB_PORT` 覆盖。浏览器验收要真实走 dev URL,且三个面都要走:demo fallback、Rust artifact replay、Rust live session。

### 其他

```bash
rtk proxy cargo fmt && rtk proxy cargo clippy                      # 提交前无警告
rtk proxy cargo bench -p picea --no-run                            # benchmark 编译门
rtk proxy cargo run -p picea-lab -- serve --bind 127.0.0.1:18080   # 仅起 API server
```

## 架构大图

数据流单向:**core 算物理事实 → lab 采集/服务事实 → web 展示事实**。web 永远不在浏览器里重算物理。

### crates/picea(core)

- 写路径:`world.rs` + `world/*` 拥有 authoritative 状态和 lifecycle API;`pipeline/*`(step、broadphase、narrowphase、gjk、ccd、island、integrate、contacts、joints、sleep)负责固定步长编排;`solver/*` 是内部求解实现(contact rows、warm-start、writeback),`mod solver` 私有,**不是** public surface。
- 读路径:`query.rs`(`QueryPipeline`:AABB / point / ray / distance / shape 查询)和 `debug.rs`(`DebugSnapshot`)是稳定读模型,不改写 world 状态。
- 组装:`recipe.rs`(`WorldRecipe` / `*Bundle` / `WorldCommands`)供测试、示例、lab 场景做可复现搭建;不改低层 `World::create_*` 语义。

### crates/picea-lab

- `scenario`:内置确定性场景与 `RunConfig`(不碰 artifact,不持有 session 状态)。
- `artifact`:headless runner,写 `manifest.json` / `frames.jsonl` / `debug_render.json` / `final_snapshot.json` / `perf.json`。
- `server`:HTTP + SSE;artifact replay session 与 live session(live 在 Rust 端持有 authoritative `World + SimulationPipeline`)。
- `cli`:`picea-lab list | run | serve`。
- `web/`:React + Canvas workbench,消费 Rust facts;play 是前端连续触发 backend step,没有 server 端 autoplay。

## 工作纪律(AGENTS.md 的核心要求)

- 改动贴着当前 milestone 走:先加失败测试 / 行为锁,再写最小实现,过 targeted gate 才推进;不偷跑下一个 milestone。
- 看到不是自己这轮改出来的 dirty/worktree 内容,先确认来源;不要 revert、覆盖、格式化、删除。
- 声称完成前必须真的跑过验证命令,以输出为准。

## 陷阱

- `build.sh` 引用已移除的 `crates/picea-web`(旧 wasm 路径),是历史遗留,不要用。
- 旧 `Scene` / `Context` runtime 和 `picea-web` / wasm 叙述只存在于归档文档(`docs/plans/2026-04-18-*`、`docs/architecture/legacy-scene-runtime.md`),不要当成当前路由或验证目标。
- `cargo test -p picea-macro-tools` 通过不代表 core 依赖它,它是独立 workspace gate。
- `--no-run` 只是编译门,不保证无警告。
- `docs/plans/*` 里的工作区状态和验证输出是当时快照;`status: frozen` / `deprecated` 的计划不要当成下一步默认路线。
- M38/M40 的 lattice 是 rigid-body proxy,不是真 soft-body;讨论 deformable 先读 `docs/design/deformable-body-roadmap.md`。
