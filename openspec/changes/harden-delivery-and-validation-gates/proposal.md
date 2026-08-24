## Why

Picea 已具备较完整的 core、lab、Web 与长窗口验收，但这些门禁主要依赖人工串行执行，仓库没有持续集成入口，部分已经转绿的 matrix-stack 验收仍被标记为 future/ignored，Web production bundle 也持续产生大 chunk 警告。现在需要把已有事实收敛成可本地复跑、可由 CI 分层执行、失败语义清楚的交付合同，避免继续扩功能时放大验证成本和状态漂移。

## What Changes

- 增加 fast、full、nightly、release 四层本地验证入口，并让 CI 调用同一批脚本。
- 增加 pull request / main CI 与 scheduled/manual nightly workflow；最小权限运行，保存性能证据但不把 wall-clock 当 correctness oracle。
- 将当前实际通过的 matrix-stack support-gap、long-settle 与 sleep-convergence 门接入可持续验证层；把两个已被修复推翻的“已知失败历史” observation 从可执行测试迁回历史文档。
- 为 Criterion 与 deterministic counter evidence 建立 nightly 采集入口；本 change 不凭单机样本设置新的绝对时间阈值。
- 对 picea-lab-web 做低风险 production chunk 拆分，并增加可执行 bundle budget，消除现有 `>500 kB` 警告。
- 将 Web 构建链中审计命中的 dev-only PostCSS 升至安全补丁版本，并让 full profile 对完整依赖树执行 audit hard gate。
- 完善可发布 crate 的 metadata 与 package dry-run 门，但不声明 1.0、不猜测 MSRV、不执行 publish。
- 同步 README、AI 路由与性能/稳定性文档，使门禁边界、运行频率和剩余风险可发现。

## Capabilities

### New Capabilities

- `repository-delivery-gates`: 定义本地与 CI 的分层验证、matrix-stack nightly acceptance、性能证据、Web bundle budget 和 release package 检查。

### Modified Capabilities

无。仓库当前没有归档到 `openspec/specs/` 的主规格；本 change 不修改 physics、public API 或 artifact/schema 行为。

## Impact

- 新增 `.github/workflows/*` 与 `scripts/ci/*`，并更新 `Justfile` 作为人类可发现入口。
- 调整 `crates/picea-lab/tests/artifact_run.rs` 中三条已验证 matrix-stack gate 的调度语义，不改阈值和 solver。
- 调整 `crates/picea/benches/physics_scenarios.rs` 的全窗口确定性计数器证据，不修改生产 solver。
- 调整 `crates/picea-lab/web/vite.config.ts`、`package.json`、`package-lock.json`、`tsconfig.node.json` 及 bundle/audit contract，不改 Rust facts 或 UI 产品行为。
- 更新 `crates/picea/Cargo.toml`、`crates/macro-tools/Cargo.toml` 的发布 metadata，并将 `crates/picea-lab/Cargo.toml` 明确设为不可发布；不新增运行时依赖。
- 更新仓库级 `openspec/config.yaml` 的通用上下文；两个已完成但未 archive 的 S5 change 继续由各自冻结 artifacts 保留边界，不在本 change 中重新 apply 或 archive。
- 更新 README、AI 路由、相关设计/计划文档和本 change artifacts。
- 不执行 OpenSpec archive/sync、commit、push、PR、deploy 或 crates.io publish。
