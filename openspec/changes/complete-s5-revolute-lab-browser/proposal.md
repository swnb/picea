## Why

当前 `feat/vnext-s5-revolute-joint` 已完成 Revolute public API 与 solver，但真实 Chrome 验收在第一步失败：Rust 场景目录与 Web 选择器都没有 `revolute_pendulum`，因此用户无法观察或核对 authoritative revolute facts。用户已明确授权修正该缺口并复测，本 change 接续 living spec 的 `S5-LAB-RED -> S5-LAB`，不进入后续 S5-V/S5-C。

## What Changes

- 先增加 artifact、server、UI 与 i18n 跨层行为锁，证明缺失的是 capability 而不是构建或服务 harness。
- 增加固定步长、可复现的 `ScenarioId::RevolutePendulum` / `revolute_pendulum`，使用 schema-v1 fixture 创建 static/dynamic pin-only hinge。
- 通过既有 `DebugSnapshot`、artifact 与 live server 原样暴露 `kind = "revolute"`、两个 world anchors 与一个 logical joint row；保留旧 closed-enum consumer 的明确 reject。
- 让 Web 场景分组、Inspector、Hierarchy、Timeline 与中英文标签显式消费 revolute facts，不在浏览器重算 physics，也不 fallback 到 existing joint kind。
- 在 targeted/broader gates 全绿后，使用真实 Chrome 对同一工作区 HEAD 运行到至少第 240 帧并记录 DOM、截图、console 与 network 证据。

## Capabilities

### New Capabilities

- `revolute-lab-workbench`: 定义 revolute pendulum 场景、authoritative artifact/server facts、显式 Web 消费及真实浏览器关键旅程。

### Modified Capabilities

无。仓库当前没有归档到 `openspec/specs/` 的主规格；冻结的 Revolute core API/solver 合同不变。

## Impact

- Rust Lab：`crates/picea-lab/src/scenario/*`、必要的 artifact/server contract tests。
- Web：`crates/picea-lab/web/src/i18n.ts` 与既有 joint consumers、`scripts/ui-contract.mjs`、`scripts/i18n-contract.mjs`。
- 文档/进度：本 change artifacts；按新增场景模块同步最小 AI 路由，不生成易过期的状态快照。
- 无新依赖，不升级 fixture schema，不修改 core public API、solver/contact/CCD/integration、motor/limit/damping 或 live joint patch 协议。
