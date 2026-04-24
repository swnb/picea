# Picea Debug Playbook

这份 playbook 只定义 Picea 的调试标准流程，不定义实现策略本身。目标是把 bug 调试变成可复现、可裁决、可归档的工作流。

## 1. 基本原则

1. 先确认当前工作区事实，再动手推理。
2. 先复现，再修复；先锁行为，再改实现。
3. 先最小测试/最小失败用例，再做最小修复。
4. 固定 `StepConfig::dt` 和输入序列，避免“看起来好了”的随机结果。
5. 路由先分清：`world`、`pipeline`、`broadphase`、`narrowphase`、`solver`、`sleep`、`query/debug`。
6. 旧 `Scene` / `Context` / `picea-web` / wasm 记录只作为归档背景，不作为当前默认调试入口。
7. workaround-first、跳过失败、吞掉 panic、扩大默认容错的做法，默认都不接受，除非它被明确写进 spec 并被 review 接受。

## 2. 开始前必须确认的事实

每次 debug 开始前都先确认以下内容，写进 `repro.md` 或 `verification.md`：

- 当前 git branch。
- 当前 `HEAD`。
- 当前工作区是否 clean，是否存在他人改动。
- 当前 repo status / HEAD / 验证输出是否与计划文档记录一致。
- 相关模块的 live code 是否已经和设计文档不同。

如果这些事实不清楚，不要直接进入修复；先补事实再下结论。

## 3. 标准调试流程

### Step 1: 锁定问题描述

- 把 bug 描述成“某个输入下，某个可观察结果不符合预期”。
- 明确是编译失败、测试失败、运行时 panic、数值漂移、行为退化、事件错误，还是 debug/query 投影错误。
- 定义最小复现路径，避免一上来就用整套场景代替单点问题。

### Step 2: 先复现

- 先跑最接近失败点的命令。
- 先保留原始失败输出，不要先修代码再猜。
- 如果失败依赖 step 序列、时间步或随机种子，固定这些输入。

### Step 3: 最小行为锁

- 先补一个会失败的测试，或者一个最小 repro。
- 行为锁必须直接抓住 bug 观察点，不要只测“相关区域”。
- 先让红灯稳定，再改代码。

### Step 4: 分路由定位

按下面顺序排查，除非证据已经明确指向某一路由：

1. `world`：handle 生命周期、create/destroy/patch、retained events、validation。
2. `pipeline`：step 顺序、fixed dt、stats、event aggregation。
3. `broadphase`：AABB 生成、候选对顺序、重复/漏报、candidate count。
4. `narrowphase`：shape pair contact geometry、normal orientation、depth、fallback 路径。
5. `solver` / contact response：位置修正、material response、未来 impulse/warm-start。
6. `sleep`：稳定窗口、awake body set、wake reset、sleep event。
7. `query/debug`：snapshot、draw primitive、query filter、read-side facts。

常见症状的第一跳路由如下：

| 症状 | 优先模块 | 首选命令 | 预期先确认的失败点 |
| --- | --- | --- | --- |
| body/collider/joint lifecycle 错误 | `world/*`, `body.rs`, `collider.rs`, `joint.rs` | `rtk proxy cargo test -p picea --test core_model_world` | handle stale/missing、revision、validation 是否正确 |
| step 事件或 stats 错误 | `pipeline/step.rs`, `pipeline.rs` | `rtk proxy cargo test -p picea --test world_step_review_regressions` | `StepReport`、event ordering、stats 是否和 world state 对齐 |
| broadphase 候选异常 | `pipeline/broadphase.rs`, `pipeline/contacts.rs` | `rtk proxy cargo test -p picea --lib pipeline::broadphase` | AABB overlap、candidate ordering、重复/反向 pair |
| narrowphase 假阳性/法线错误 | `pipeline/narrowphase.rs`, `pipeline/contacts.rs` | `rtk proxy cargo test -p picea --lib pipeline::narrowphase` | shape pair 是否正确过滤、normal 是否指向 ordered body |
| 反弹/摩擦/接触速度异常 | `pipeline/contacts.rs`, `solver/body_state.rs` | `rtk proxy cargo test -p picea --test physics_realism_acceptance` | material response、separating contact friction、角速度副作用 |
| sleep 过早或不醒 | `pipeline/sleep.rs`, `body.rs`, `solver/body_state.rs` | `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep_requires` | idle time、awake body set、patch/wake reset |
| DebugSnapshot / query 不一致 | `debug.rs`, `query.rs` | `rtk proxy cargo test -p picea --test query_debug_contract` | read-side facts、filters、draw primitives 是否和 world 对齐 |
| 数值 NaN 或非有限状态外泄 | `pipeline/integrate.rs`, `events.rs` | `rtk proxy cargo test -p picea --test world_step_review_regressions step_emits_numeric` | warning 是否发出、state 是否被 containment |

### Step 5: 最小修复

- 只修复触发该 bug 的最小原因。
- 不顺手重构邻近模块。
- 不靠更大的默认值、更宽松的容错、或额外的隐藏分支掩盖根因。

### Step 6: 回归验证

- 重新跑最小 repro。
- 再跑受影响模块的定向测试。
- 最后跑 `rtk proxy cargo test -p picea --lib` 或 `rtk proxy cargo test -p picea --tests`。
- 如果修复依赖固定 `dt`，确认不同 step 序列下结果一致或误差在明确范围内。

## 4. Review Chain

Picea 的 debug 归档必须按下面顺序过一遍：

1. **Implementer**：补行为锁、最小修复、最小验证。
2. **Spec Reviewer**：检查目标、范围和硬边界。
3. **Code Reviewer**：findings-first，重点看 panic、NaN、错误路由、错误事件、测试是否真的锁住行为。
4. **Supervisor Acceptance**：只有验证门和 review 都闭环后，才允许写成“已完成”。

任何一层没过，都不能把结果写成“已解决”。

## 5. Picea 专用红线

- 不把暂时跑通当成正确。
- 不把局部单测通过当成系统正确。
- 不把 `unwrap` / `panic` 的消失当成行为正确。
- 不在不明确的情况下改 solver、broadphase、narrowphase 或 public API。
- 不用“继续推进”替代“先把 bug 修对”。

## 6. 最低验收口径

一轮 debug 至少要留下这些东西：

- 最小 repro。
- 失败前后的行为差异。
- 相关测试或行为锁。
- 最终验证命令和结果。
- 这次修复没有碰的边界。
