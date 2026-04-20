# Picea AI Evals

这份文档收录一组真实仓库评估题，用来检查 AI 是否真的理解 Picea，而不是只会背通用物理引擎术语。

## 1. 评估原则

- 题目必须依赖真实仓库资料，而不是泛化常识。
- 每题都要写明 expected sources。
- 每题都要有明确 pass criteria。
- 题目要覆盖 repo understanding、routing、debug、implementation readiness、freshness/conflict。
- 如果答案需要依赖当前 HEAD、计划文档、或工作区状态，必须把 freshness / conflict 也算进去。

## 2. Questions

### Repo understanding

1. **Picea 的当前 milestone 顺序是什么，为什么不能跳过前置 milestone？**
   - Expected sources: `docs/plans/2026-04-18-picea-physics-engine-milestones.md`
   - Pass criteria: 能说出至少 M0 到 M8 的顺序，并解释“先保基线，再重构”的约束。

2. **core 和 wasm API 分别位于哪个 crate，它们的职责边界是什么？**
   - Expected sources: `docs/plans/2026-04-18-picea-physics-engine-milestones.md`, workspace 目录结构
   - Pass criteria: 正确指出 `crates/picea` 是 core，`crates/picea-web` 是 wasm API/绑定层，不混淆职责。

3. **当前仓库最核心的风险集中在哪几类模块？**
   - Expected sources: milestone 文档中的背景、风险和 residual risk 段落
   - Pass criteria: 至少覆盖 geometry/shape、collision、solver、storage、wasm API 这几类。

4. **为什么计划文档反复强调 deterministic step pipeline 和 fixed dt？**
   - Expected sources: M2 章节及其执行记录
   - Pass criteria: 能解释 frame split 一致性、repro 和误差控制，而不是只说“为了稳定”。

### Routing

5. **一个“接触力不继承”的 bug，优先应该查哪条路由，为什么？**
   - Expected sources: M5、M8 的 collision/manifold/warm-start 记录
   - Pass criteria: 优先指向 collision manifold 生命周期和 warm start transfer，而不是先改 solver 参数。

6. **一个“凹多边形拆分结果异常”的 bug，应该从哪条路由开始排查？**
   - Expected sources: M0、M1、M4 的 shape 相关记录
   - Pass criteria: 能指出 shape/local geometry/decomposition/cache 路由，而不是直接归因到 broadphase。

7. **一个“wasm 侧调用失败但 core 测试通过”的 bug，应该怎么分路由？**
   - Expected sources: M0 和 M7
   - Pass criteria: 能分清 core 行为、公开 API 形状、参数合法性和 JS/Rust 边界。

### Debug

8. **如果某个碰撞 bug 只在某个 frame split 下复现，调试时第一步应该记录什么？**
   - Expected sources: `docs/ai/debug-playbook.md`, M2 记录
   - Pass criteria: 必须提到固定 `dt`、frame 序列、最小 repro 和行为锁。

9. **为什么 debug 时不能只看“最终看起来没问题”的截图？**
   - Expected sources: `docs/ai/debug-playbook.md`, `docs/ai/debug-artifacts.md`
   - Pass criteria: 能说明需要 trace、snapshot、drop reason、lambda 和 sleep/wakeup 的过程证据。

10. **如果一个 contact key 被错误复用，应该通过哪些 artifact 字段识别出来？**
    - Expected sources: `docs/ai/debug-artifacts.md`, M8 record
    - Pass criteria: 至少说出 `contact_key_transfer`、`contact_drop_reason`、`manifold_lifecycle`、`normal_lambda` / `friction_lambda`。

11. **为什么 debug playbook 要强制先确认 git/HEAD/repo status？**
    - Expected sources: `docs/ai/debug-playbook.md`, milestone 文档中的 dirty/worktree 约束
    - Pass criteria: 能说明避免把旧状态、他人改动或错位 HEAD 当成当前事实。

### Implementation readiness

12. **一个 bug 修复是否已经“准备好实施”，需要哪些最小信号？**
    - Expected sources: `docs/ai/debug-playbook.md`
    - Pass criteria: 至少包括最小 repro、失败行为锁、明确路由、影响面和验证门。

13. **如果要对 collision pipeline 做重构，什么情况下不应该现在动手？**
    - Expected sources: milestone 文档的 hard boundary、residual risk
    - Pass criteria: 能指出当问题其实只在 shape 或 solver，或验证门还没稳时，不应该越界改 broad/narrowphase。

14. **什么样的 artifact 形状，最适合让 reviewer 快速判断这是 broadphase 问题还是 manifold 问题？**
    - Expected sources: `docs/ai/debug-artifacts.md`
    - Pass criteria: 能提出 trace 里的 phase/candidate/contact/lifecycle 字段，而不只是截图。

### Freshness / conflict

15. **当用户给出“当前 branch、HEAD、验证结果”这类事实时，AI 该怎么处理它们和本地仓库状态的关系？**
    - Expected sources: 用户任务说明、`docs/ai/debug-playbook.md`
    - Pass criteria: 能说明应优先以当前工作区事实为准，并把差异写进 verification，而不是无条件相信旧记忆。

16. **如果计划文档和当前工作区实现看起来不一致，应该信哪个？**
    - Expected sources: milestone 文档、debug playbook
    - Pass criteria: 能说明要同时核对 HEAD、当前状态和计划文档，冲突必须显式记录，不能默认某一边。

17. **在 Picea 这种多 milestone 仓库里，为什么“实现完成”不等于“可合并”？**
    - Expected sources: milestone 文档里的 Implementer / Spec Reviewer / Code Reviewer / Supervisor Acceptance
    - Pass criteria: 必须提到 review 链和 acceptance gate，而不是只看单测。

## 3. 使用建议

- 这组题适合做口试、文档阅读评估、或者给 agent 做 repo comprehension benchmark。
- 评估时不要只看答案是否像对的，要看它是否引用了正确来源、是否尊重 milestone 边界、是否能把问题路由到正确模块。
- 如果答案需要猜 HEAD、工作区状态或最新验证结果，必须判为 freshness 不足。
