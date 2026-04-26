# Picea Physics Engine TODO

## 未完成任务清单

### 1. Broadphase 生产化

- [ ] 将当前每步重建的 dynamic AABB tree 改为持久 proxy tree。
- [ ] 为 collider 保存 broadphase proxy id。
- [ ] 实现 fat AABB，减少小位移时的 tree update。
- [ ] 支持 proxy move / remove / reinsert。
- [ ] 增加 tree balance / rebuild 策略和指标。
- [ ] 复用 broadphase tree 做 ray cast、AABB query、region query。
- [ ] 暴露 candidate count、drop reason、tree depth 等 debug facts。

### 2. SAT + Clipping Manifold

- [ ] 实现 rect / convex polygon 的 SAT 最小穿透轴。
- [ ] 实现 reference edge / incident edge 选择。
- [ ] 实现 clipping，生成 1-2 点 contact manifold。
- [ ] 为 contact point 生成 stable feature id。
- [ ] 实现 contact reduction，避免过多或抖动接触点。
- [ ] 补 circle-polygon、circle-segment 的解析 narrowphase。

### 3. GJK / EPA / Generic Convex Fallback

- [ ] 为 convex shape 增加 support mapping。
- [ ] 实现 GJK distance / intersection。
- [ ] 实现 EPA 或 closest-feature fallback。
- [ ] 用 GJK 支撑复杂 convex fallback、distance query 和 CCD TOI fallback。

### 4. 完整 Sequential Impulse Solver

- [ ] 引入 effective mass 和 inverse inertia。
- [ ] 实现 normal impulse 和 tangent impulse。
- [ ] 实现 warm-start。
- [ ] 实现 Coulomb friction clamp。
- [ ] 加入 restitution threshold，避免低速抖动反弹。
- [ ] 接入 velocity iterations / position iterations。
- [ ] 加入 angular contact response。
- [ ] 保留 position correction，但避免覆盖速度求解结果。

### 5. 质量 / 惯量模型

- [ ] 根据 shape density 计算 mass。
- [ ] 计算 center of mass。
- [ ] 计算 moment of inertia。
- [ ] 明确 static / kinematic / dynamic 的 inverse mass 和 inverse inertia 语义。
- [ ] 增加非法 mass / inertia 输入校验。

### 6. Persistent Contact / Warm Start

- [ ] 建立稳定 contact key。
- [ ] 持久化 manifold cache。
- [ ] 继承上一帧 normal / tangent impulse。
- [ ] 处理法线翻转、A/B 交换和 feature drift。
- [ ] 对接触点漂移做保守失配处理，避免错误继承 impulse。

### 7. Island Sleep

- [ ] 基于 contact / joint graph 构建 island。
- [ ] 实现 island 级 sleep，而不是只看单个 body。
- [ ] 实现 wake-on-impact。
- [ ] 实现 wake reason：contact、joint、user patch、transform edit、velocity edit。
- [ ] 增加 resting stack sleep / wake 回归测试。

### 8. CCD TOI

- [ ] 实现 swept AABB broadphase。
- [ ] 先支持 fast circle vs static thin wall TOI。
- [ ] 扩展到 circle / convex static TOI。
- [ ] 引入 conservative advancement。
- [ ] 明确 substep 和 contact event 语义。
- [ ] 将当前 ignored CCD known-red 测试推进为正式通过项。

### 9. API Bundles / Recipes

- [ ] 增加 `BodyBundle`。
- [ ] 增加 `ColliderBundle`。
- [ ] 增加 `SceneRecipe`，用于声明式测试场景和示例。
- [ ] 增加 `WorldCommands` 批处理 create / destroy / patch。
- [ ] 增加 material presets。
- [ ] 增加 collision layer presets。
- [ ] 返回结构化创建结果：handles、events、validation errors。

### 10. Observability / Debug Facts

- [ ] 暴露 broadphase candidate 和 drop reason。
- [ ] 暴露 narrowphase manifold facts。
- [ ] 暴露 solver impulse、warm-start 命中、clamp 状态。
- [ ] 暴露 sleep / wake reason。
- [ ] 暴露 CCD TOI trace。
- [ ] 让 debug snapshot 能解释每一步物理决策。

### 11. 性能与 Benchmarks

- [ ] 减少 per-step allocation。
- [ ] 缓存 world-space vertices / support data。
- [ ] 建立 active island compact arrays。
- [ ] 增加 sparse broadphase benchmark。
- [ ] 增加 dense broadphase benchmark。
- [ ] 增加 stack stability benchmark。
- [ ] 增加 CCD bullet benchmark。
- [ ] 增加 API batch creation benchmark。

### 12. 更完整验收测试

- [ ] SAT polygon contact。
- [ ] Stack stability。
- [ ] Ramp friction。
- [ ] Elastic restitution threshold。
- [ ] Wake-on-impact。
- [ ] CCD bullet。
- [ ] API recipe / batch 场景测试。

