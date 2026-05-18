# AI 路由索引

本文只回答“先读哪里”。具体设计、计划状态、实现细节分别下沉到
`docs/design/*`、`docs/plans/*` 和 `crates/*`。

## 渐进式读取顺序

1. **现场事实**：`rtk proxy git status --short --branch`、`rtk proxy git rev-parse --short HEAD`、相关验证输出、`Cargo.toml` / `crates/*/Cargo.toml`、`crates/picea/src/lib.rs`。
2. **根规则**：`AGENTS.md`，只用于命令约束、dirty 保护和权威顺序。
3. **模块定位**：`docs/ai/repo-map.md`，用于找到 owner、entrypoint、常用验证门。
4. **文档选择**：`docs/ai/doc-catalog.yaml`，按 `kind` / `authority` / `status` / `applies_to` 过滤。
5. **领域细节**：只读被路由命中的设计、计划、runbook 或代码文件。

冲突时，当前工作区事实和验证输出优先；当前生产化计划其次；AI 路由文档只负责定位，不覆盖 live code。

## 常见问题路由

| 问题类型 | 先读 | 再读 | 常用验证 |
| --- | --- | --- | --- |
| 当前 crate / module 边界 | `docs/ai/repo-map.md`, `docs/architecture/system-overview.md` | `Cargo.toml`, `crates/*/Cargo.toml`, `crates/picea/src/lib.rs` | `rtk proxy cargo test -p picea --lib` |
| Public beta / 用户入口 | `docs/public-beta.md`, `README.md`, `crates/picea/README.md` | `crates/picea/examples/public_beta_smoke.rs` | `rtk proxy cargo test -p picea --examples --no-run` |
| 当前生产 milestone | `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md` | 对应设计文档和 `repo-map.md` 中的模块 | milestone 内列出的 targeted gate |
| 旧 milestone 历史 | `docs/plans/2026-04-18-picea-physics-engine-milestones.md` | `docs/architecture/legacy-scene-runtime.md` | 不作为当前默认 gate |
| core bug / physics 行为回归 | `docs/ai/debug-playbook.md`, `docs/ai/repo-map.md` | `crates/picea/src/world/*`, `pipeline/*`, `solver/*`, 相关测试 | 先最小 repro，再跑模块 targeted tests |
| broadphase / query / performance counter | `docs/design/physics-engine-upgrade-technical-plan.md`, `docs/design/performance-threshold-policy.md` | `pipeline/broadphase.rs`, `query.rs`, `debug.rs`, `benches/physics_scenarios.rs` | `rtk proxy cargo test -p picea --test query_debug_contract`; `rtk proxy cargo bench -p picea --no-run` |
| CCD / shape query / compound authoring | `docs/ai/repo-map.md`, `docs/design/physics-engine-upgrade-technical-plan.md` | `pipeline/ccd.rs`, `pipeline/gjk.rs`, `query.rs`, `recipe.rs`, `picea-lab/src/scenario.rs` | 对应 `physics_realism_acceptance` / `query_debug_contract` / `picea-lab` gate |
| stack / matrix stack 稳定性 | `docs/design/matrix-stack-stability-acceptance.md`, `docs/design/matrix-stack-stability-optimization-design.md`, `docs/design/dense-pressure-position-row-architecture.md` | `solver/contact.rs`, `pipeline/contacts.rs`, `pipeline/sleep.rs`, `picea-lab/src/artifact.rs` | stack / matrix 相关 targeted gates |
| picea-lab artifact / diagnostics | `docs/design/performance-stability-diagnostics-contract.md`, `docs/design/stack-stability-repro-diagnostics.md` | `crates/picea-lab/src/artifact.rs`, `tests/artifact_run.rs` | `rtk proxy cargo test -p picea-lab --test artifact_run` |
| picea-lab server / live session | `docs/design/picea-lab-live-session-semantics.md` | `crates/picea-lab/src/server.rs`, `tests/server_routes.rs` | `rtk proxy cargo test -p picea-lab --test server_routes` |
| picea-lab-web UI / browser workflow | `docs/plans/2026-05-04-picea-lab-web-industrial-ux-milestones.md`, `docs/plans/2026-05-07-picea-lab-web-live-performance-milestones.md`, `docs/design/scene-runtime-config-and-parameter-ui-design.md` | `crates/picea-lab/web/src/*` | `cd crates/picea-lab/web && rtk proxy npm run build`; browser acceptance |
| soft-body / particle / cloth / deformable mesh | `docs/design/deformable-body-roadmap.md` | 当前 `body.rs`, `collider.rs`, `joint.rs`, `debug.rs`, lab scenarios | 先 RFC / design gate，不直接改 UI 假装支持 |
| proc macro helper | `docs/ai/repo-map.md`, `crates/macro-tools/README.md` | `crates/macro-tools/src/*` | `rtk proxy cargo test -p picea-macro-tools` |
| 文档/AI 路由治理 | `.agents/skills/picea-doc-routing/SKILL.md`, 本文件, `doc-catalog.yaml` | `AGENTS.md`, `repo-map.md`, `evals.md`, `skill-candidates.md` | AI-context validator + YAML parse + `rtk proxy git diff --check` |

## 状态类文档的读法

- `docs/plans/*` 里的 `工作区状态`、验证输出和执行记录是当时快照，不代表当前工作区事实。
- `status: frozen` 的计划可作为完成记录和验收证据，但不要把它当成下一步默认路线。
- `status: deprecated` 的文档只能用于历史解释；除非用户明确要求追溯旧路径，否则不要先读。
- `docs/ai/doc-catalog.yaml` 是判断这些状态的第一入口。

## 当前默认边界

- 当前 core 路线是 `World + SimulationPipeline`，不是旧 `Scene::tick` / `Context`。
- 当前 web 工具是 `crates/picea-lab/web`，不是旧 `picea-web` / wasm facade。
- `picea-lab-web` 消费 Rust artifact / live session facts，不在浏览器里重算 physics。
- `picea-macro-tools` 是 workspace 内独立 proc-macro crate；它的验证 gate 不代表 core runtime 依赖。
