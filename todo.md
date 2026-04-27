# Picea Physics Engine TODO

> 当前文件是升级 backlog/archive。实际执行顺序、范围和验收门以
> `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md` 为准。
> M1-M10 的第一生产能力线已经落地；后续重点从“补齐基础算法”转向
> 性能承载层、active island solver、CCD 泛化和更易用的 API。

## 已完成生产基线

### M1 Broadphase 生产化

- [x] 将每步重建的 dynamic AABB tree 改为持久 proxy tree。
- [x] 为 live collider 维护内部 broadphase proxy。
- [x] 实现 fat AABB，减少小位移时的 tree update。
- [x] 支持 proxy move / remove / reinsert / stale cleanup。
- [x] 增加 tree balance / rebuild / compaction 策略和指标。
- [x] 暴露 candidate count、drop reason、tree depth 等 debug facts。

### M2 SAT + Clipping Manifold

- [x] 实现 rect / convex polygon 的 SAT 最小穿透轴。
- [x] 实现 reference edge / incident edge 选择。
- [x] 实现 clipping，生成 1-2 点 contact manifold。
- [x] 为 contact point 生成 stable feature id。
- [x] 实现 contact reduction，避免过多或抖动接触点。
- [x] 补 circle-polygon、circle-segment 的解析 narrowphase。

### M3 质量 / 惯量模型

- [x] 根据 shape density 计算 mass。
- [x] 计算 center of mass。
- [x] 计算 moment of inertia。
- [x] 明确 static / kinematic / dynamic 的 inverse mass 和 inverse inertia 语义。
- [x] 增加非法 mass / inertia 输入校验。

### M4 Persistent Contact / Warm Start

- [x] 建立稳定 contact key。
- [x] 持久化 manifold cache。
- [x] 继承上一帧 normal / tangent impulse。
- [x] 处理法线翻转、A/B 交换和 feature drift。
- [x] 对接触点漂移做保守失配处理，避免错误继承 impulse。

### M5 完整 Sequential Impulse Solver

- [x] 引入 effective mass 和 inverse inertia。
- [x] 实现 normal impulse 和 tangent impulse。
- [x] 实现 warm-start。
- [x] 实现 Coulomb friction clamp。
- [x] 加入 restitution threshold，避免低速抖动反弹。
- [x] 接入 velocity iterations / position iterations。
- [x] 加入 angular contact response。
- [x] 保留 residual position correction，但避免覆盖速度求解结果。

### M6 Island Sleep

- [x] 基于 contact / joint graph 构建 deterministic island。
- [x] 实现 island 级 sleep，而不是只看单个 body。
- [x] 实现 wake-on-impact。
- [x] 实现 wake reason：contact、joint、user patch、transform edit、velocity edit。
- [x] 增加 resting stack sleep / wake 回归测试。

### M7 GJK / EPA / Generic Convex Fallback

- [x] 为 convex shape 增加 support mapping。
- [x] 实现 GJK distance / intersection。
- [x] 实现 EPA 或 closest-feature fallback。
- [x] 用 GJK/EPA 支撑当前复杂 convex fallback。

### M8 CCD TOI

- [x] 实现 swept AABB candidate filtering。
- [x] 先支持 fast circle vs static thin wall TOI。
- [x] 扩展到 dynamic circle / static convex TOI 的第一生产切片。
- [x] 明确当前 CCD 是 pose-clamping phase，contact event 保留 `ccd_trace`。
- [x] 将 ignored CCD known-red 测试推进为正式通过项。

### M9 API Bundles / Recipes / Observability / Benchmarks

- [x] 增加 `BodyBundle`。
- [x] 增加 `ColliderBundle`。
- [x] 增加 `WorldRecipe`，用于声明式测试场景和示例。
- [x] 增加 `JointBundle` / `WorldRecipe::with_joint`。
- [x] 增加 `WorldCommands` 批处理 create / destroy / patch。
- [x] 增加 material presets。
- [x] 增加 collision layer presets。
- [x] 返回结构化创建结果：handles、events、validation errors。
- [x] 暴露 broadphase、narrowphase、solver、sleep、CCD debug facts。
- [x] 增加 sparse broadphase、dense broadphase、stack stability、CCD bullet、API batch creation Criterion baseline。

### M10 Architecture Consolidation / Product Surface Cleanup

- [x] 引入 `StepContext` 管理单步 transient facts。
- [x] 将 contact solver rows / effective mass / warm-start application / residual correction 移到 `solver/contact.rs`。
- [x] 明确 CCD 是命名的 pose-clamping phase。
- [x] 保持 `StepStats`、`DebugSnapshot`、events、lab artifacts 的事实面一致。
- [x] 让 `picea-lab` 明确是 replay/evidence workbench，而不是 live simulator。
- [x] 补齐 final snapshot、joint selection、backend/demo state、empty SSE 语义。

## 后续升级路线

### M10.5 Documentation And Backlog Closeout

- [x] 将生产 milestone、升级技术方案、AI 路由和本文件校准到 M10 后状态。
- [x] 验证 YAML、diff whitespace 和文档范围。

### M11 Performance Substrate

- [ ] 为 broadphase 增加 collider-handle -> leaf 的直接索引。
- [ ] 改进 dynamic AABB tree 插入、平衡和 rebuild 策略。
- [ ] 复用 broadphase-style tree 做 ray cast、AABB query、region query。
- [ ] 缓存 world-space vertices / support data，并用 transform revision 失效。
- [ ] 减少 contact gathering、sleep island、solver setup 的 per-step allocation。
- [ ] 保持 Criterion baseline，以 counter 和 variance 解释性能变化。

### M12 Active Island Solver

- [ ] 建立 active island compact arrays。
- [ ] 将 contact rows 和 joint rows 按 island 统一求解。
- [ ] 让 sleeping islands 退出 hot solver arrays。
- [ ] 保持 contact id、manifold id、warm-start facts、wake reason 与 debug facts 稳定。
- [ ] 增加 stack、ramp friction、jointed island、unrelated island 回归测试。

### M13 CCD Generalization

- [ ] 引入 dynamic-vs-static convex shape cast 或 GJK-backed conservative advancement。
- [ ] 支持非 circle dynamic convex 穿越 thin static geometry 的 TOI。
- [ ] 增加 multi-impact ordering 和 budget 语义。
- [ ] 扩展 `ccd_trace`，记录 selected / ignored TOI hits。
- [ ] 仅在 benchmark 和行为锁支持后再考虑 dynamic-vs-dynamic CCD。

### M14 Ergonomic API V2

- [ ] 增加更高层的 scene / asset recipe helpers。
- [ ] 为 recipe / command validation error 增加 nested path context。
- [ ] 评估 serializable recipe fixtures，用于 examples、benchmarks 和 lab scenarios。
- [ ] 保持低层 `World::create_*` API 稳定。
- [ ] 增加 `v1_api_smoke` 和 lab/example fixture 验收。

## 保留风险

- [ ] Concave polygon decomposition 仍未作为 core solver 能力落地。
- [ ] Public distance query 仍未作为稳定 API 暴露。
- [ ] Dynamic-vs-dynamic CCD 仍需独立 benchmark 和行为锁证明。
- [ ] Absolute performance thresholds 需要多轮 baseline 后再设置。
