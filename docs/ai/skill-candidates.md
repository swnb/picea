# Repository Skills

本仓库只保留有明确重复价值的开发规范型 skill。模块细节类 workflow 先不提升为 skill，避免把一次性经验固化成噪音。

## 已落地

### `.agents/skills/picea-doc-routing/SKILL.md`

- 用途：维护 `AGENTS.md`、`docs/ai/index.md`、`docs/ai/repo-map.md`、`docs/ai/doc-catalog.yaml`、`docs/ai/skill-candidates.md` 和仓库内 skills。
- 触发：仓库结构、milestone 范围、验证命令、文档权威顺序或 AI 入口变化。
- 边界：只做路由/文档治理，不碰 `crates/**` 实现。

### `.agents/skills/picea-milestone-runner/SKILL.md`

- 用途：执行、验证或监督一个明确的 Picea milestone。
- 触发：用户要求实现/继续/验证/审查某个 milestone，或要求严格停在某个 milestone。
- 边界：必须先确认 git/HEAD 和 milestone 硬边界，先补行为锁，按验证门和 review chain 收口。

## 暂不提升的候选

模块专项调试、shape 契约、wasm smoke 这类主题暂时不做成 skill，因为目前更像模块知识或单轮调试路线。后续只有当某个 workflow 多次重复、失败代价高、且能写成稳定流程时，才重新提名。
