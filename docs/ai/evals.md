# Picea AI Evals

这份文档收录一组真实仓库评估题，用来检查 AI 是否理解当前 Picea，而不是只会背通用物理引擎术语或旧 milestone 记忆。

## 1. 评估原则

- 题目必须依赖当前仓库资料，而不是泛化常识。
- 每题都要写明 expected sources。
- 每题都要有明确 pass criteria。
- 题目要覆盖 repo understanding、routing、debug、implementation readiness、freshness/conflict。
- 如果答案需要依赖当前 HEAD、工作区状态或验证结果，必须把 freshness / conflict 算进去。

## 2. Questions

### Repo Understanding

1. **当前 Picea workspace 有哪些 crate？哪个 crate 是物理核心？**
   - Expected sources: `docs/ai/repo-map.md`, root `Cargo.toml`, `crates/*/Cargo.toml`
   - Pass criteria: 正确指出当前 workspace 是 `crates/picea` 和 `crates/macro-tools`；`crates/picea` 是 World-centric core；不要把旧 `picea-web` 当成当前 crate。

2. **当前核心运行入口是什么？**
   - Expected sources: `docs/ai/repo-map.md`, `crates/picea/src/lib.rs`, `crates/picea/src/pipeline.rs`
   - Pass criteria: 指出 `World` + `SimulationPipeline::step`，不是旧 `Scene::tick`。

3. **当前 pipeline 的核心模块如何分工？**
   - Expected sources: `docs/ai/repo-map.md`, `crates/picea/src/pipeline/*`
   - Pass criteria: 至少区分 integrate、joints、contacts、broadphase、narrowphase、sleep、step orchestration。

4. **当前文档中哪些旧内容只能当归档看？**
   - Expected sources: `docs/ai/index.md`, `docs/ai/repo-map.md`, `docs/plans/2026-04-18-picea-physics-engine-milestones.md`
   - Pass criteria: 明确旧 `Scene` / `Context` / `picea-web` / wasm narrative 不作为当前默认路由。

### Routing

5. **一个“接触法线方向翻转”的 bug，优先查哪里？**
   - Expected sources: `crates/picea/src/pipeline/contacts.rs`, `crates/picea/src/pipeline/narrowphase.rs`, `crates/picea/tests/world_step_review_regressions.rs`
   - Pass criteria: 优先指向 contact observation ordering、narrowphase normal orientation、recycled handle ordering测试，而不是先改 solver 参数。

6. **一个“圆形 AABB 重叠但几何未接触仍报 contact”的 bug，优先查哪里？**
   - Expected sources: `crates/picea/src/pipeline/narrowphase.rs`, `crates/picea/tests/physics_realism_acceptance.rs`
   - Pass criteria: 指向 circle-circle narrowphase，而不是只看 broadphase。

7. **一个“静止一帧就睡眠”的 bug，优先查哪里？**
   - Expected sources: `crates/picea/src/pipeline/sleep.rs`, `crates/picea/src/body.rs`, `crates/picea/tests/physics_realism_acceptance.rs`
   - Pass criteria: 能说明 sleep idle timer/stability window、awake body reset、BodyPatch wake reset。

### Debug

8. **如果某个碰撞 bug 只在某个 step 序列下复现，第一步应该记录什么？**
   - Expected sources: `docs/ai/debug-playbook.md`
   - Pass criteria: 必须提到固定 `StepConfig::dt`、输入序列、最小 repro、行为锁、当前 git/HEAD/status。

9. **为什么 debug 时不能只看“最终看起来没问题”的截图？**
   - Expected sources: `docs/ai/debug-playbook.md`, `docs/design/debug-observability-design.md`
   - Pass criteria: 能说明需要 StepReport、WorldEvent、DebugSnapshot、candidate/contact/sleep 过程证据。

10. **如果一个 material response bug 会吃掉切向速度，应该如何锁行为？**
    - Expected sources: `crates/picea/tests/physics_realism_acceptance.rs`, `crates/picea/src/pipeline/contacts.rs`
    - Pass criteria: 能提出 separating-overlap friction 或 restitution/friction acceptance，先红灯再修。

11. **为什么 debug playbook 强制先确认 git/HEAD/repo status？**
    - Expected sources: `docs/ai/debug-playbook.md`, `AGENTS.md`
    - Pass criteria: 能说明避免把旧状态、他人改动或错位 HEAD 当成当前事实。

### Implementation Readiness

12. **一个 bug 修复是否已经“准备好实施”，需要哪些最小信号？**
    - Expected sources: `docs/ai/debug-playbook.md`
    - Pass criteria: 至少包括最小 repro、失败行为锁、明确路由、影响面和验证门。

13. **如果要改 broadphase，什么时候不应该现在动手？**
    - Expected sources: `docs/design/physics-engine-upgrade-technical-plan.md`, `docs/ai/debug-playbook.md`
    - Pass criteria: 能指出问题其实在 narrowphase/solver/sleep，或没有候选对行为锁和验证门时，不应越界改 broadphase。

14. **什么 artifact 最适合区分 broadphase 问题和 narrowphase 问题？**
    - Expected sources: `docs/design/debug-observability-design.md`, `docs/design/picea-lab-observability-architecture.md`
    - Pass criteria: 至少包含 broadphase candidate、narrowphase contact、contact normal/depth、reject reason 或 counters。

### Freshness / Conflict

15. **当用户给出“当前 branch、HEAD、验证结果”这类事实时，AI 该怎么处理它们和本地仓库状态的关系？**
    - Expected sources: 用户任务说明、`docs/ai/debug-playbook.md`
    - Pass criteria: 应优先核对当前工作区事实，并把差异写进 verification，而不是无条件相信旧记忆。

16. **如果设计文档和当前实现不一致，应该信哪个？**
    - Expected sources: `docs/ai/index.md`, `docs/ai/repo-map.md`
    - Pass criteria: 当前代码和验证输出优先；设计文档负责方向，不能覆盖 live facts。

17. **在 Picea 这种多阶段仓库里，为什么“实现完成”不等于“可合并”？**
    - Expected sources: `docs/ai/debug-playbook.md`, AGENTS work rules
    - Pass criteria: 必须提到行为锁、验证门、review chain、dirty worktree 保护，而不是只看单测。

## 3. 使用建议

- 这组题适合做口试、文档阅读评估、或者给 agent 做 repo comprehension benchmark。
- 评估时不要只看答案是否像对的，要看它是否引用正确来源、是否尊重当前 World-centric 路由、是否能把问题路由到正确模块。
- 如果答案需要猜 HEAD、工作区状态或最新验证结果，必须判为 freshness 不足。
