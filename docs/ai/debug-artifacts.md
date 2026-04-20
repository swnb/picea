# Picea Debug Artifacts

这份文档定义推荐的 debug 产物形状。它不要求本轮实现代码，只要求后续 debug 和 review 能稳定产出同一种证据。

## 1. 产物清单

推荐每次 debug 至少产出这 5 个文件：

- `repro.md`
- `trace.jsonl`
- `final_snapshot.json`
- `debug_render.json`
- `verification.md`

如果问题只涉及编译或 API 形状，`debug_render.json` 可以为空场景，但文件本身仍建议保留。

## 2. 各产物职责

### `repro.md`

面向人读的最小复现说明。

建议包含：

- 问题摘要。
- 触发条件。
- 最小复现命令。
- 预期结果与实际结果。
- 当前 git branch、`HEAD`、工作区状态。
- 相关 milestone 或 spec 约束。

### `trace.jsonl`

面向机器读的逐步时间线。每一行一个事件，适合做 replay、diff 和聚合。

建议字段：

- `run_id`
- `tick`
- `frame`
- `phase`
- `substep`
- `scene_id`
- `element_ids`
- `collision_pass`
- `broadphase_candidate`
- `narrowphase_contact`
- `manifold_id`
- `manifold_lifecycle`
- `contact_key`
- `contact_key_transfer`
- `contact_drop_reason`
- `normal_lambda`
- `friction_lambda`
- `sleep_state`
- `wakeup_reason`
- `dt`
- `source`

`phase` 建议使用可枚举值，至少覆盖：

- `scene::tick`
- `integrate_velocity`
- `warm_start`
- `collision_detect`
- `pre_solve`
- `velocity_solve`
- `position_integrate`
- `position_fix`
- `post_solve`
- `sleep_check`

### `final_snapshot.json`

调试结束时的状态切片，用于和 repro 前状态对比。

建议字段：

- `run_id`
- `tick`
- `frame`
- `scene_state`
- `elements`
- `contacts`
- `manifolds`
- `sleeping_elements`
- `active_pairs`
- `broadphase_candidates`
- `constraints`

`contacts` 和 `manifolds` 至少要能表达：

- `contact_key`
- `contact_point_count`
- `cached_normal_lambda`
- `cached_friction_lambda`
- `was_active_last_pass`
- `is_active`
- `drop_reason`

### `debug_render.json`

面向可视化/回放工具的轻量渲染描述，不要求高保真，但要能把问题看清楚。

建议字段：

- `camera`
- `world_bounds`
- `shapes`
- `aabbs`
- `broadphase_candidates`
- `contacts`
- `contact_normals`
- `manifold_labels`
- `sleep_labels`
- `overlay_text`

这份文件的重点不是美术，而是把 broadphase / narrowphase / manifold / sleep 的关系在画面里分清。

### `verification.md`

面向 reviewer 的验证记录。

建议包含：

- 复现命令。
- 修复前失败点。
- 修复后通过点。
- 受影响模块的定向测试。
- milestone 验收门。
- 未碰边界。

## 3. 关键字段要求

### Tick / phase

每条 trace 都应该说明它属于哪个 tick、哪个 frame、哪个 phase。没有这些字段，后续就很难判断是时序 bug 还是数值 bug。

### Broadphase candidate

必须能记录候选 pair 为什么进入或没有进入 narrowphase。至少要能表达：

- 轴或策略。
- AABB 是否有效。
- 是否被过滤。
- 是否来自 persistent pair。

### Narrowphase contact

至少要能表达：

- contact 数量。
- contact point 是否重建。
- 接触法线。
- 是否命中已有 manifold。

### Manifold lifecycle

至少要能表达：

- `new`
- `refreshed`
- `pending`
- `active`
- `inactive`
- `dropped`

### Contact key transfer / drop reason

必须明确记录：

- 哪个 contact key 被继承了。
- 是因为 continuing contact、recontact、reorder，还是 feature drift。
- 如果丢弃，丢弃原因是什么。

### Lambda

`normal_lambda` 和 `friction_lambda` 至少要能区分：

- cached
- applied
- clamped
- skipped

### Sleep / wakeup

必须能表达：

- 元素是醒着还是睡着。
- 进入 sleep 的理由。
- 唤醒的理由。
- 进入/唤醒时对应的 kinetic、velocity、threshold 状态。

## 4. 推荐约束

- `trace.jsonl` 要尽量 append-only。
- 事件要能按 `run_id + tick + phase` 排序。
- snapshot 要尽量是结构化对象，不要靠自由文本拼接。
- debug render 只表达事实，不要混进解释。

## 5. 最低可用标准

一组 debug artifacts 至少要回答这四个问题：

1. 问题在哪个 tick、哪个 phase 出现。
2. 它是从 broadphase、narrowphase、manifold 还是 solver 传进来的。
3. contact key 和 lambda 是怎么传、怎么丢、怎么重建的。
4. sleep/wakeup 有没有改变最终可观察结果。

