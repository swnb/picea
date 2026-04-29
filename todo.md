# Picea Physics Engine TODO

> 当前文件是升级 backlog/archive。实际执行顺序、范围和验收门以
> `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md` 为准。
> M1-M10 的第一生产能力线已经落地；后续重点从“补齐基础算法”转向
> Post-M15 的系统质量深化：dense island-local execution、
> 更广的 CCD 覆盖，以及 scene schema / authoring 稳定化。

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

- [x] 为 broadphase 增加 collider-handle -> leaf 的直接索引。

注：M11 当前 milestone scope 以 direct leaf lookup substrate 验收完成；
更深的 query reuse、shape/support cache、allocation reduction 和 perf
threshold 工作转入 Post-M14。

### M12 Active Island Solver

- [x] 建立 active-island batching 第一阶段。
- [x] 将 contact rows 和 joint rows 按 island 分批求解。
- [x] 让 sleeping islands 退出 hot solver arrays。
- [x] 保持 contact id、manifold id、warm-start facts、wake reason 与 debug facts 稳定。
- [x] 增加 stack、friction、jointed island、unrelated island 回归测试。

注：M12 当前 milestone scope 已完成；更 dense 的 per-island arrays、
更强的一体化 island solver 架构，以及 ramp-specific friction 测试转入
Post-M14。

### M13 CCD Generalization

- [x] 引入 dynamic-vs-static convex shape cast 的当前 milestone 切片。
- [x] 支持非 circle dynamic convex 穿越 thin static geometry 的 TOI。
- [x] 增加 multi-impact ordering 和 budget 语义。
- [x] 扩展 `ccd_trace` 相关统计，记录 selected / ignored TOI hits。

注：M13 当前 milestone scope 已完成；rotational、dynamic-vs-dynamic 和
all-shape CCD 覆盖转入 Post-M14。

### M14 Ergonomic API V2

- [x] 增加更高层的 scene / asset recipe helpers。
- [x] 为 recipe / command validation error 增加 nested path context。
- [x] 评估并落地 serializable recipe fixtures，用于 examples、benchmarks 和 lab scenarios。
- [x] 保持低层 `World::create_*` API 稳定。
- [x] 增加 `v1_api_smoke` 和 lab/example fixture 验收。

注：M14 当前 milestone scope 已完成；live lab editing 与公共 scene schema
稳定化转入 Post-M14。

## Post-M15 / 保留风险 / 下一阶段

### M15 Performance Data Path

- [x] 复用 broadphase-style spatial index 做 ray cast、AABB query、region query
  的候选遍历，但不向 public API 泄漏 proxy / leaf id。
- [x] 缓存 world-space vertices / AABB / support data，并用 transform revision
  做失效与 stale-cache 行为锁。
- [x] 减少 contact gathering、CCD、GJK/support-map 路径的重复几何重建，并做
  当前代码能解释的 conservative pre-sizing。
- [x] 为 query ordering、filter semantics、recycled handle、tree rebuild、body
  transform patch 补行为锁。
- [x] 保持 Criterion baseline，以 counter 和 variance 解释性能变化；暂不设置
  absolute perf threshold。

注：M15 当前 milestone scope 已完成；query allocation/perf counters 与更深
solver allocation work 转入 Post-M15。

### M16 Dense Island Execution

- [x] 设计 `IslandSolvePlan`，把 active island 的 body slots、contact row
  indices、joint rows 收敛到 deterministic island-local data。
- [x] 将 contact solver 的 hot path 从 `BTreeMap<BodyHandle, SolverBody>` /
  `BTreeSet<usize>` 查找迁移到 island-local slot index。
- [x] 让 joint rows 复用同一个 island plan，同时保留当前 separate phase /
  live step order。
- [x] 证明 sleeping islands 不构建 hot rows，unrelated islands 不相互影响。
- [x] 保持 warm-start、wake reason、contact/debug facts 和 lab artifact 语义稳定。

### M21 Public Distance And Shape Query API

- [x] 设计稳定的 public distance / shape query API，返回 handles、distance、
  closest points、normal / direction 和 query stats，不暴露 broadphase proxy id。
- [x] 复用 `QueryPipeline`、内部 query index、collider geometry cache 和 GJK
  distance kernel，不新增第二套 query engine。
- [x] 为 ordering、filter semantics、stale sync、recycled handle、degenerate input
  和 no-hit 行为补行为锁。
- [x] 保持 AABB / point / ray query 现有语义不变。
- [x] 更新 AI routing 和使用示例，明确 concave / compound authoring 属于 M22。

### M22 Compound And Concave Authoring Boundary

- [x] 在 recipe / scene authoring 层支持 compound convex collider groups 或
  validated pre-decomposed concave fixtures。
- [x] 对 unsupported direct concave solver usage、invalid pieces、empty
  decomposition 提供稳定 nested error path。
- [x] 保持 material、sensor、collision filter 和 generated-piece ordering 可解释。
- [x] 增加至少一个 lab/example fixture 行为锁，展示 compound /
  concave-looking authoring 边界。
- [x] 明确文档边界：core solver 支持 convex pieces，不直接求解 arbitrary concave
  contacts。

### Later Post-M22

- [ ] 改进 dynamic AABB tree 插入、平衡和 rebuild 策略。
- [ ] 增加 query allocation/perf counters，并用 Criterion baseline variance 解释。
- [x] 增加 ramp-specific friction 回归测试。
- [ ] 在 M19 selected slice 之后，再扩展更多 dynamic-vs-dynamic / all-shape CCD。
- [ ] 扩展 rotational CCD 和更广的 all-shape CCD 覆盖。
- [ ] 在 M20 public scene schema 之后，再明确 live lab editing / scene patch
  语义。
- [ ] 为 compound pieces 增加 artifact/schema/UI provenance，而不让 lab 重新
  计算物理事实。
- [x] 为 dynamic concave-looking / compound authored pieces 补 additive
  mass/inertia 行为锁；是否支持更广 dynamic concave authoring 仍需后续设计。
- [ ] M22 之后再评估是否引入更广的自动 polygon decomposition；默认不把
  arbitrary concave contact 放进 core solver。
- [ ] Absolute performance thresholds 需要多轮 baseline 后再设置。
