# Picea Debug Playbook

这份 playbook 只定义 Picea 的调试标准流程，不定义实现策略本身。目标是把 bug 调试变成可复现、可裁决、可归档的工作流。

## 1. 基本原则

1. 先确认仓库事实，再动手推理。
2. 先复现，再修复；先锁行为，再改实现。
3. 先最小测试/最小失败用例，再做最小修复。
4. 固定 `dt` 和输入序列，避免“看起来好了”的随机结果。
5. 路由先分清：scene、collision、constraints、shape、wasm，各自有自己的边界和验证点。
6. 任何 workaround-first、跳过失败、吞掉 panic、扩大默认容错的做法，默认都不接受，除非它被明确写进 spec 并被 reviewer 批准。

## 2. 开始前必须确认的事实

每次 debug 开始前都先确认以下内容，写进 `repro.md` 或 `verification.md`：

- 当前 git branch。
- 当前 `HEAD`。
- 当前工作区是否 clean，是否存在他人改动。
- 当前 repo status / HEAD / 验证输出是否与计划文档记录一致。
- 相关 milestone 文档是否已经给出范围、硬边界和验收门。

如果这些事实不清楚，不要直接进入修复；先补事实再下结论。

## 3. 标准调试流程

### Step 1: 锁定问题描述

- 把 bug 描述成“某个输入下，某个可观察结果不符合预期”。
- 明确是编译失败、测试失败、运行时 panic、数值漂移、行为退化，还是 wasm 路由问题。
- 定义最小复现路径，避免一上来就用整套场景代替单点问题。

### Step 2: 先复现

- 先跑最接近失败点的命令。
- 先保留原始失败输出，不要先修代码再猜。
- 如果失败依赖 frame 序列、时间步或随机种子，固定这些输入。

### Step 3: 最小行为锁

- 先补一个会失败的测试，或者一个最小 repro。
- 行为锁必须直接抓住 bug 观察点，不要只测“相关区域”。
- 先让红灯稳定，再改代码。

### Step 4: 分路由定位

按下面顺序排查，除非证据已经明确指向某一路由：

1. `scene` 路由：tick 顺序、帧步进、sleep/wakeup、状态刷新。
2. `collision` 路由：broadphase、narrowphase、manifold 生命周期、contact key transfer/drop reason。
3. `constraints` 路由：warm start、effective mass、lambda、position/velocity solve、restitution/friction。
4. `shape` 路由：local/world 变换、AABB/support/mass cache、分解结果是否被重复构造。
5. `wasm` 路由：API 参数合法性、导出语义、JS/Rust 边界错误是否被正确返回。

常见症状的第一跳路由如下：

| 症状 | 优先模块 | 首选命令 | 预期先确认的失败点 |
| --- | --- | --- | --- |
| examples 编译失败 | `scene/context`、examples | `rtk proxy cargo test -p picea --examples --no-run` | public API 是否被 examples 误用、context 配置入口是否越过封装 |
| 凹多边形 panic 或拆分异常 | `shape/utils.rs`、`shape/concave.rs` | `rtk proxy cargo test -p picea shape::utils --lib` 或 `rtk proxy cargo test -p picea shape::concave --lib` | 退化点、重复点、方向、local/world transform 是否被锁住 |
| fixed dt / frame split 行为不一致 | `scene/mod.rs` | `rtk proxy cargo test -p picea scene::tests --lib` | accumulator、substep 上限、skipped duration、fractional remainder |
| stale id / 悬挂引用 / remove 后崩溃 | `element/store.rs`、`scene/mod.rs` | `rtk proxy cargo test -p picea element::store::tests --lib` | store map/cache、pair mut、contact manifold invalidation |
| broadphase 候选异常 | `collision/mod.rs` | `rtk proxy cargo test -p picea rough_collision_detection --lib` | AABB 是否有限、range 是否归一化、SAP 候选是否过宽或漏掉 |
| contact warm-start 或 lambda 继承错误 | `constraints/contact.rs`、`scene/mod.rs`、`collision/mod.rs` | `rtk proxy cargo test -p picea contact_identity --lib` | contact key、pending refresh、active/inactive pass、lambda transfer/drop reason |
| solver 出现 NaN 或无效质量行为 | `constraints/contact.rs`、`constraints/join.rs`、`constraints/point.rs` | `rtk proxy cargo test -p picea contact::tests --lib` 或 `rtk proxy cargo test -p picea point::tests --lib` | effective mass、inverse mass、zero/non-finite mass 是否 no-op 或受控 |
| wasm 调用失败但 core 通过 | `crates/picea-web/src/common.rs`、`wasm.rs` | `rtk proxy cargo test -p picea-web --lib` | JS/Rust 参数校验、callback error isolation、wasm-bindgen 版本/runner 是否一致 |

### Step 5: 最小修复

- 只修复触发该 bug 的最小原因。
- 不顺手重构邻近模块。
- 不靠更大的默认值、更宽松的容错、或额外的隐藏分支掩盖根因。

### Step 6: 回归验证

- 重新跑最小 repro。
- 再跑受影响模块的定向测试。
- 最后跑 milestone 要求的验证门。
- 如果修复依赖固定 `dt`，确认不同 frame split 下结果一致或误差在明确范围内。

## 4. milestone 方式的评审链

Picea 的 debug 归档必须按下面顺序过一遍：

1. **Implementer**
   - 负责补行为锁、最小修复、最小验证。
   - 只对实现负责，不替 reviewer 做结论。
2. **Spec Reviewer**
   - 只看是否真的满足 milestone 目标、范围和硬边界。
   - 重点找偷跑到下一 milestone 的内容。
3. **Code Reviewer**
   - findings-first。
   - 重点看 panic、NaN、悬挂引用、错误路由、错误的缓存转移、以及测试是否真的锁住行为。
4. **Supervisor Acceptance**
   - 只有在验证门和 review 都闭环后，才允许把这轮结论写成“已完成”。

任何一层没过，都不能把结果写成“已解决”。

## 5. Picea 专用红线

- 不把暂时跑通当成正确。
- 不把局部单测通过当成系统正确。
- 不把 `unwrap` / `panic` 的消失当成行为正确。
- 不在不明确的情况下改 solver、collision 算法或 wasm 公开契约。
- 不用“继续推进”替代“先把 bug 修对”。

## 6. 最低验收口径

一轮 debug 至少要留下这些东西：

- 最小 repro。
- 失败前后的行为差异。
- 相关测试或行为锁。
- 最终验证命令和结果。
- 这次修复没有碰的边界。
