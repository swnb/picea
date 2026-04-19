# Picea Physics Engine Milestones

> 日期：2026-04-18
>
> 本文是 supervisor 决策归档，不是实现任务。当前轮次只记录方向、范围、验收门和 subagent 编排；不得修改代码。

## 1. 背景与目标

Picea 当前是一个 2D 刚体物理引擎雏形，已经具备 scene、element、shape、collision、constraint、wasm 绑定等基础模块，但本地基线仍处在不稳定状态：测试和 examples 编译没有形成可持续的红绿闭环，几何、碰撞、求解器、storage 和 wasm API 也有明显的热路径与正确性风险。

目标是把 Picea 演进为高性能、真实感更好的现代物理引擎。执行顺序必须先保基线，再重构：先让当前行为可验证、失败可复现、API 可编译，然后再逐步推进数学/几何契约、确定性 step pipeline、cache-friendly storage、shape pipeline、collision pipeline、solver realism 和 wasm API hardening。

## 2. Supervisor 工作原则

- 主线程只管方向、范围、验收和冲突裁决，不下场成为实现者。
- 实现、测试补全、spec review、code review 交给 subagent；每个 subagent 必须有清晰输入、文件范围、验收命令和停止条件。
- 不得跨 milestone 偷跑。当前 milestone 未通过 supervisor acceptance 前，不进入下一 milestone。
- 不得覆盖、格式化、删除、恢复用户 dirty。当前已知 dirty 为 `crates/picea/src/shape/utils.rs` 和未跟踪 `.DS_Store`，后续 worker 必须先确认并保护。
- 每个 milestone 都要 test-first，或至少先补会失败的行为锁，再写最小实现让测试通过。
- 任何“为了推进而降级”的路线都不接受；红灯必须归因、收敛、修复或明确标注为当前本地状态下失败。
- 文档、测试和代码结论都必须绑定当前仓库证据，避免泛泛而谈的物理引擎建议。

## 3. 当前基线结论

当前本地状态是红的，不能把后续重构建立在“默认可用”的假设上。

- `rtk proxy cargo test`：失败。原因之一是 examples 访问私有 `Context.constraint_parameters`，例如 examples 直接写 `scene.context_mut().constraint_parameters...`，而 `Context` 中该字段不是 public。
- `rtk proxy cargo test -p picea --lib`：当前本地状态下失败。`shape::utils::tests::test_split_concave_polygon1` 会触发 `crates/picea/src/shape/utils.rs:462` 的 `unreachable!` panic；同时 `crates/picea/src/shape/utils.rs` 当前 dirty，必须把该结果标注为“当前本地状态下失败”，不能直接归因到干净主干。
- `rtk proxy cargo test -p picea-macro-tools`：测试通过，但仍有 warning，需要在后续基线整理中记录而不是忽略。
- `rtk proxy cargo test -p picea-web --lib`：编译通过，0 tests。它只说明 lib 编译能过，不说明 wasm public API 行为被覆盖。

2026-04-18 M0 implementer 更新：上述红色状态已在当前工作区通过 M0 最小修复收敛；最终验证门见 M0 执行记录。既有 warning 与 `picea-web --lib` 0 tests 状态仍保留为后续 milestone 风险，不在 M0 扩范围处理。

## 4. Milestones

### M0 Verifiable Baseline

**目标**

把当前红色本地状态收敛为可验证基线：测试命令可重复、失败有行为锁、examples 编译路径合法，后续重构不再踩在漂浮地板上。

**范围**

- 修复或隔离 examples 访问私有 `Context.constraint_parameters` 的编译失败，提供合法配置 API 或改用现有合法 setter/getter。
- 锁住凹多边形拆分 panic，先补能复现 `test_split_concave_polygon1` 的行为锁，再最小修复。
- 明确 `picea-macro-tools` warning 和 `picea-web --lib` 0 tests 的状态，不在 M0 强行扩展 wasm API。
- 补充最小 smoke/contract 测试，确保 M0 验证门能在当前工作区稳定复现。

**测试门**

- `rtk proxy cargo test -p picea --lib`
- `rtk proxy cargo test -p picea --examples --no-run`
- `rtk proxy cargo test -p picea-macro-tools`
- `rtk proxy cargo test -p picea-web --lib`

**subagent 分工**

- Implementer：只负责 examples 配置 API 与凹多边形 panic 的最小行为锁/修复；列出改动文件和测试命令。
- Spec Reviewer：只检查 M0 是否真的恢复 verifiable baseline，是否偷跑到性能、storage、solver 或 wasm API 大改。
- Code Reviewer：findings-first 检查 panic、private API、测试是否锁住失败、是否误碰用户 dirty。

**硬边界**

- 不做性能大改。
- 不做架构大改。
- 不重写 shape/collision/solver。
- 不格式化或覆盖当前用户 dirty 文件，除非 supervisor 明确授权并处理冲突。

**执行记录（2026-04-18，M0 implementer）**

状态：M0 verifiable baseline 在当前工作区已恢复，未进入 M1/M2/M3。

- RED 证据：`rtk proxy cargo test -p picea --lib` 先失败于 `shape::utils::tests::test_split_concave_polygon1` 的 `unreachable!`；`rtk proxy cargo test -p picea --examples --no-run` 先失败于 examples 对私有 `Context.constraint_parameters` 的直接访问。
- Examples 修复：`Context` 已有公开 `constraint_parameters_mut()`，本轮仅给 examples 实际改写的 `ConstraintParameters` 字段开放 `*_mut()` API，并把 examples 改为通过该 API 配置，不把字段 public。
- 凹多边形修复：保留当前 dirty 中“更新最小投影距离”的意图；在切点落在已有端点时，同时清理线性相邻重复点和首尾环形重复点，避免递归处理首尾重复的伪多边形。
- 行为锁：`test_split_concave_polygon1` 现在断言拆分结果非空、每个子多边形至少 3 点、首尾不重复且不再是凹多边形。
- GREEN 证据：`rtk proxy cargo test -p picea --lib` 通过，6 passed；`rtk proxy cargo test -p picea --examples --no-run` 通过；`rtk proxy cargo test -p picea-macro-tools` 通过，6 passed；`rtk proxy cargo test -p picea-web --lib` 通过，0 tests。
- 未处理项：warning 未在 M0 清理；`.DS_Store` 未删除；未做 performance/storage/collision/solver/wasm API 语义重构。

### M1 Math And Geometry Contracts

**目标**

建立数学与几何输入的可验证契约，先把基础几何行为从“隐式假设”变成测试约束。

**范围**

- 为零向量 `normalize`、投影、线段相交、凹凸多边形方向/退化输入补性质测试。
- 定义几何输入策略：非法输入返回错误、保守 fallback、还是明确 panic；策略必须先文档化再实现。
- 为 shape utils 的边界条件建立最小 property-style 或 table-driven tests。

**测试门**

- M0 全局验证门全部通过。
- 新增 math/geometry contract tests，至少覆盖零向量、共线点、重复点、极小边、顺/逆时针输入。

**subagent 分工**

- Implementer：新增/修复 math 与 shape utils 测试和最小实现。
- Spec Reviewer：检查几何输入策略是否明确、是否与现有 public API 兼容。
- Code Reviewer：重点查 NaN、`unwrap`、`recip`、零长度向量、退化 segment、测试只覆盖 happy path 的漏洞。

**硬边界**

- 不改 solver。
- 不改 collision pipeline。
- 不改 storage 模型。

**执行记录（2026-04-18，M1 implementer）**

状态：M1 Math And Geometry Contracts 已完成，未进入 M2/M3/M4。

- 几何输入策略：当前 public API 不改成 `Result`，M1 采用保守 fallback。退化向量归一化为零向量；退化投影轴返回零投影/折叠到输入首个有限点；零面积 convex center 回退到顶点平均值；非凹或退化多边形拆分保持原输入作为有限 fallback。
- RED 证据：新增行为锁后，`rtk proxy cargo test -p picea --lib normalize_returns_zero` 失败于 `normalized.x().is_finite()`；`projection_on_zero_vector_returns_finite_collapsed_projection` 失败于返回默认 `(0,0)`；`projection_onto_degenerate_vector_returns_zero` 失败于 `NaN`；`convex_center_point_uses_average_for_zero_area_inputs` 失败于中心点非有限。
- 实现：`Vector::normalize` 对零/极小/非有限向量返回零向量；`Vector >> Vector` 对退化投影轴返回 `0`；`compute_convex_center_point` 对零面积输入返回平均点；关键路径补了简短注释说明退化策略。
- 行为锁：新增 math tests 覆盖 f32/f64 零向量与极小向量、正常 `(3,4)` 归一化、退化轴投影；shape utils table-driven tests 覆盖零方向投影、共线/零面积、重复点、极小边、退化 segment、顺/逆时针凹多边形输入；M0 凹多边形测试复用统一断言，检查非空、每个子多边形至少 3 点、首尾不重复、有限、非凹，并增加面积近似守恒。
- Spec review 返工：review 发现 `split_clockwise_concave_polygon_to_two_convex_polygon` 仍有 no-cut `unreachable!` panic path。新增 `concave_split_falls_back_when_degenerate_input_has_no_cut_edge` 行为锁，RED 时命中该 `unreachable!`；修复后 no-cut 候选继续尝试其它候选点，最终无法切分时走保守 fallback，并清理相邻重复点、首尾重复点和共线中点，保证 formerly-unreachable case 输出有限、首尾不重复、至少 3 点且非凹。
- Code review 返工：review 发现 fallback cleanup 可能输出少于 3 个点，且 `compute_convex_center_point` / `projection_polygon_on_vector` 没有 containment NaN/inf 顶点。新增 `concave_split_fallback_never_returns_too_few_vertices`、`convex_center_point_ignores_non_finite_vertices`、`polygon_projection_ignores_non_finite_vertices`、`finite_fallbacks_use_default_when_no_finite_vertices_exist` 行为锁；RED 时分别暴露 len < 3、中心点非有限、投影点非有限、无有限点未回默认值。修复后 center/projection 只使用有限点，无有限点返回 `Point::default()`；unsplittable fallback 最终校验 len、finite、首尾不重复、非凹，不满足时返回保守有限三角形。
- GREEN 证据：`rtk proxy cargo fmt --all --check` 通过；`rtk proxy cargo test -p picea --lib` 通过，20 passed；`rtk proxy cargo test -p picea --examples --no-run` 通过；`rtk proxy cargo test -p picea-macro-tools` 通过，6 passed；`rtk proxy cargo test -p picea-web --lib` 通过，0 tests。
- residual risk：M1 未做完整 property testing/fuzzing；凹多边形拆分仍是现有算法的行为锁与有限性加固，不是 M4 shape pipeline 重构；warnings 未清理；未改 Scene tick、storage、collision pipeline、solver 或 wasm API。

### M2 Deterministic Step Pipeline

**目标**

把 `Scene::tick` 从直接 clamp 单步推进演进到确定性固定步长 accumulator/substep pipeline，让同一输入序列能产生可复现 tick。

**范围**

- 设计固定步长 accumulator、最大 substep、剩余时间处理与 tick 序号。
- 阶段化 pipeline：integrate velocity、warm start、collision detect、pre-solve、velocity solve、position integrate、position fix、post-solve。
- 增加 determinism tests：同一输入拆成不同 frame delta 后，固定 tick 结果一致或误差受控。

**测试门**

- M0 全局验证门全部通过。
- 新增 deterministic step tests，通过固定 seed/固定输入验证可复现。

**subagent 分工**

- Implementer：只改 step/tick 相关路径和测试。
- Spec Reviewer：检查 pipeline 阶段边界和公开行为是否符合 M2，不引入 storage/collision 算法重构。
- Code Reviewer：重点查 accumulator 丢帧、spiral of death、tick 顺序、浮点误差断言和 callback 时序。

**硬边界**

- 不改 storage。
- 不改 collision 算法。
- 不改物理公式。

**执行记录（2026-04-19，M2 implementer）**

状态：M2 Deterministic Step Pipeline 已完成最小实现，未进入 M3/M4/M5。

- 实现策略：`Scene::tick(delta_time)` 公开 API 保持不变；内部改为私有固定步长 accumulator，固定 step 为 `1 / 60`，单次外部 tick 最多推进 8 个 substeps。超过上限的 backlog 只丢弃 excess whole steps 并计入内部 skipped duration，fractional remainder 保留到后续 tick，避免 spiral of death；不足一个 fixed step 的剩余时间留在 accumulator，下次 tick 继续累计；`clear()` 会清掉 pending accumulator 和 skipped duration。
- pipeline 阶段：单个 fixed step 明确顺序为 integrate velocity -> warm start -> collision detect -> pre-solve -> velocity solve -> integrate position -> position fix -> sleep/apply transform -> post-solve。原有 collision、constraint solve、position fix 公式与迭代次数保持不变。
- TDD RED：新增 `scene::tests::tick_uses_fixed_steps_for_the_same_total_duration_across_frame_splits` 和 `scene::tests::tick_caps_substeps_and_drops_excess_backlog` 后，`rtk proxy cargo test -p picea scene::tests --lib` 按预期失败；旧实现分别表现为 frame_count 仍按外部 frame 计数，以及超大 delta 只 clamp 成单步。
- GREEN：实现 fixed-step pipeline 后，`rtk proxy cargo test -p picea scene::tests --lib` 通过，2 passed。determinism 行为锁使用简单无碰撞重力场景，验证 6 个 fixed steps 在 `[dt; 6]`、`[2dt, 4dt]`、`[0.5dt; 12]` 下得到相同 `frame_count = 6`、`total_duration = 6dt`、位置 `(0, 0.35)`、速度 `(0, 6)`。
- Code review 返工：REQUEST_CHANGES 指出 accumulator boundary semantics 问题后，补充 RED 测试覆盖 `8.5dt + 0.5dt` 保留 fractional remainder、`fixed_dt - epsilon / 2` 不提前 step、`clear()` 重置 skipped duration 与 pending remainder；修复后 `rtk proxy cargo test -p picea scene::tests --lib` 通过，5 passed。
- Spec review 返工：FAIL 指出 fractional remainder 行为锁没有真正触发 `ready_steps > MAX_SUBSTEPS_PER_TICK` 分支；测试改为 `20.5dt + 0.5dt`，并断言首个 tick 只推进 8 步且 `total_skip_durations == 12dt`，确保覆盖 excess whole steps drop 与 fractional remainder 保留。
- 验证结果：`rtk proxy cargo fmt --all --check` 通过；`rtk proxy cargo test -p picea --lib` 通过，25 passed；`rtk proxy cargo test -p picea --examples --no-run` 通过；`rtk proxy cargo test -p picea-macro-tools` 通过，6 passed；`rtk proxy cargo test -p picea-web --lib` 通过，0 tests。
- 边界核对：未改 `ElementStore`；未改 collision 算法；未改 constraints solver 物理公式；未做 wasm API hardening；未删除或纳入 `.DS_Store`。
- residual risk：本轮只锁住无碰撞重力场景与 max-substep/drop backlog 语义，尚未覆盖有碰撞/约束场景的 deterministic 误差；fixed step、max substeps、skipped duration 仍是私有常量/状态，没有公开配置或观测 API；旧有 warning 仍存在，未在 M2 扩范围清理。

### M3 Storage And Handle Model

**目标**

从 `Rc<UnsafeCell>`/裸指针借用技巧走向 handle/arena/cache-friendly storage，降低 unsafe 面积，为并行和热路径优化打基础。

**范围**

- 设计 element handle、generation/arena 或等价的稳定索引模型。
- 把 query pair、constraint 引用、manifold 对象关系从裸指针逐步迁移为可验证 handle 访问。
- 增加 handle invalidation、remove/reinsert、pair borrow 的契约测试。

**测试门**

- M0 全局验证门全部通过。
- 新增 storage/handle tests，覆盖删除元素、重复 id、同 id pair、constraint 悬挂引用。

**subagent 分工**

- Implementer：只负责 storage 与 handle 层迁移，不改 solver 数学。
- Spec Reviewer：检查 handle 语义、生命周期、错误返回和兼容路径。
- Code Reviewer：重点查 unsafe 残留、别名可变借用、悬挂 handle、clone 后 id 语义和缓存一致性。

**硬边界**

- 不改物理公式。
- 不调 solver 参数。
- 不做 collision 算法替换。

**执行记录（2026-04-19，M3 implementer）**

状态：M3 Storage And Handle Model 已完成最小 storage/handle/query 迁移，未进入 M4/M5/M6。

- 实现策略：`ElementStore` 从 `Vec<Rc<StoredElement<T>>> + BTreeMap<ID, Rc<StoredElement<T>>> + UnsafeCell` 迁移为 `Vec<Box<Element<T>>> + BTreeMap<ID, usize>`；origin element order 继续由 `Vec` 保持，element allocation 地址在 `Vec` realloc 时保持稳定，broadphase sort cache 继续保存 `ID`，public `Scene` API 和 examples 调用方式保持兼容。
- handle/query：新增 `ElementStore::get_pair_mut_by_id`，先通过 `ID -> usize` 查两个 index，再用 `Vec::split_at_mut` 返回两个不同元素的 `&mut Element<T>`；`Scene::query_element_pair_mut` 改为委托该方法，不再通过两次 mutable lookup 后转裸指针构造两个 `&mut`。同 id pair 和 stale id 仍返回 `None`。
- remove/cache：`remove_element` 删除 Vec entry、map entry 和 `region_sort_result` 中的 stale id，并对删除点之后的 map index 重新编号；重复 id `push` 先移除旧 entry，再把 replacement 追加到 origin order 尾部，避免 map/cache 出现同 id 重复项。
- clone 语义：保留 `ElementStore: Clone`，改为自定义 Clone，clone 后保留 element id、meta、shape、bind points 和 data，并避免旧 `Rc` 共享导致 clone/original mutable alias；没有修改 `Element::clone()` 本身“id reset to 0”的语义。
- TDD RED：新增 storage tests 后，`rtk proxy cargo test -p picea element::store::tests --lib` 按预期失败，3 个失败分别锁住 remove 后 sort cache stale、duplicate id cache/map 不一致、clone 共享内部元素污染；新增 Scene pair tests 后，`rtk proxy cargo test -p picea scene::tests::query_element_pair_mut --lib` 当前基线通过，用作同 id/stale id/distinct mut 行为锁。
- GREEN：最小实现后，`rtk proxy cargo test -p picea element::store::tests --lib` 通过，3 passed；`rtk proxy cargo test -p picea scene::tests::query_element_pair_mut --lib` 通过，2 passed。
- Code review 返工：review 指出 inline `Vec<Element<T>>` 会让 active `ContactConstraint` 内长期保存的 `*mut Element` 在 `push_element` realloc 或 `remove_element` compact/drop 后变成 stale pointer，`get_position_fix_map` 等 public path 可能在下一次 `pre_solve` 刷新前解引用旧指针。补充 `tick -> push/remove -> get_position_fix_map` 回归测试后，`rtk proxy cargo test -p picea stale_position_fix --lib` 在修复前触发 SIGSEGV；修复为 `Vec<Box<Element<T>>>` 并在 `Scene::push_element` / `Scene::remove_element` 后清理 contact manifold，修复后该命令 2 passed。
- 验证结果：`rtk proxy cargo fmt --all --check` 通过；`rtk proxy cargo test -p picea --lib` 通过，32 passed；`rtk proxy cargo test -p picea --examples --no-run` 通过；`rtk proxy cargo test -p picea-macro-tools` 通过，6 passed；`rtk proxy cargo test -p picea-web --lib` 通过，0 tests；`rtk git diff --check` 通过。验证输出仍有既有 warning，本轮未扩范围清理。
- 边界核对：未改 constraints/contact/join/point 物理公式；未调 solver 参数；未替换 broad/narrow collision 算法；未改 wasm API；未删除或纳入未跟踪 `.DS_Store`。
- residual risk：M3 只是最小 `ID -> index` 存储迁移，还不是 generation arena，外部长期持有的 stale id 只能返回 `None`，不能区分 removed/reinsert generation；`Vec::remove` 和 map reindex 仍是 O(n)；`region_sort_result` 仍是 ID cache 而不是更强类型 handle；`push_element` / `remove_element` 现在粗粒度清理 contact manifold，会丢弃 warm-start/cache 信息直到下一次 collision detect 重建；`Scene` 里为遍历 constraints/manifolds 留下的 `self_ptr` unsafe 仍存在，未在本轮扩大重构；`collision::sweep_and_prune_collision_detection` 内部仍有既有 raw pointer 读借用技巧，留到 M5 collision pipeline。

### M4 Geometry And Shape Pipeline

**目标**

建立 local-space shape pipeline，把变换、凸/凹分解、AABB/support/mass property 缓存从每帧临时计算中拆出来。

**范围**

- shape 保存 local-space 原始几何，transform 只更新 world cache。
- 凸/凹分解缓存，避免 concave 每次 `sync_transform` 重新拆凸。
- AABB、support point、mass property cache 的失效策略和测试。

**测试门**

- M0 全局验证门全部通过。
- 新增 shape cache tests：transform 后 AABB/support/mass property 正确，重复 transform 不重复拆分。

**subagent 分工**

- Implementer：负责 shape pipeline/cache，不碰 UI/wasm 渲染层。
- Spec Reviewer：检查 local/world space 语义是否明确，cache invalidation 是否可验收。
- Code Reviewer：重点查 stale cache、旋转/平移顺序、mass/inertia 不更新、凹多边形分解退化输入。

**硬边界**

- 不做 UI。
- 不做 3D。
- 不改 solver 真实感参数。

**执行记录（2026-04-19）**

状态：M4 Geometry And Shape Pipeline 已完成最小 concave geometry/shape pipeline，未进入 M5/M6/M7。

- 实现：`ConcavePolygon::new` 仍在构造期基于 local/origin vertices 调用 `split_concave_polygon_to_convex_polygons` 分解一次；`sync_transform` 不再用 world vertices 重拆，而是把已缓存的 `ConvexPolygon` sub-colliders 按 parent `origin_center_point` 同步到 world-space。
- 实现：`ConvexPolygon` 增加 crate-private `sync_transform_around_point`，用于 parent-space transform；`to_convex_polygons()` 返回前通过 `into_current_pose` 把 returned convex pieces rebase 到当前 world vertices/current center，避免作为独立 shape 后再次 transform snap back 到 local cached pose；`sub_colliders()` 仍返回 world-space convex sub-polygons，parent `projection_on_vector` / `edge_iter` / `center_point` 仍基于 parent world vertices。
- 测试：新增 `shape::concave` 行为锁，覆盖重复 transform 不重复拆分、sub-collider count/local pieces 稳定、parent/sub-collider center/projection 随 translation/rotation 更新、repeated transform 后 area/moment of inertia 稳定，以及 consuming `to_convex_polygons()` 暴露后继续从 current world pose transform。
- 测试 helper：只在 `#[cfg(test)]` 下给 concave split 增加线程本地调用计数，并给 `ConvexPolygon` 暴露测试只读访问器；未扩 public API。
- TDD RED：`rtk proxy cargo test -p picea shape::concave --lib` 在旧实现下失败于 `repeated_transform_uses_cached_local_decomposition`，split count 为 `2`，预期为 `1`。
- Code review RED：review 发现 consuming `to_convex_polygons()` 暴露的 current vertices 与 local origin fields 不一致；新增 `exposed_convex_polygons_continue_transforming_from_current_world_pose` 后，旧实现失败于 returned piece 第二次 transform 从 local cached origin 跳回。
- GREEN：实现后 `rtk proxy cargo test -p picea shape::concave --lib` 通过，4 passed。
- 验证结果：`rtk proxy cargo fmt --all --check` 通过；`rtk proxy cargo test -p picea --lib` 通过，37 passed；`rtk proxy cargo test -p picea --examples --no-run` 通过；`rtk proxy cargo test -p picea-macro-tools` 通过，6 passed；`rtk proxy cargo test -p picea-web --lib` 通过，0 tests。输出仍有既有 warning，本轮未扩范围清理。
- 边界核对：未改 `collision/mod.rs` broadphase、GJK/EPA 或 collision pipeline 语义；未改 `constraints/*` solver；未改 `ElementStore` 存储模型；未改 wasm API；未做 UI/3D；未删除或纳入未跟踪 `.DS_Store`。
- residual risk：M4 只完成 concave decomposition hot path 的最小 local-space cache，未实现 AABB cache、support point cache 或更通用 shape cache invalidation；`ConvexPolygon::compute_moment_of_inertia` 仍沿用现有基于当前 vertices 的算法，只通过 repeated transform 稳定性锁住未漂移；凹多边形拆分算法本身仍沿用 M1 的有限行为锁与 fallback，不在本轮重写；sub-collider cache 假设 polygon 构造后 local geometry 不变。
- 最终 diff 检查：`rtk git diff --check` 通过。

### M5 Collision Pipeline

**目标**

把碰撞从单轴 SAP 与临时 `Vec<ContactPointPair>` 推向更明确的 broadphase/narrowphase/manifold pipeline。

**范围**

- AABB cache 接入 broadphase。
- 定义 broadphase trait，允许 SAP/grid/tree 等策略替换。
- persistent pairs 与 contact manifold 生命周期。
- specialized narrowphase：circle-circle、circle-polygon、polygon-polygon 等逐步拆分。
- contact manifold 减少临时分配，支持 warm start 所需的稳定 contact id。

**测试门**

- M0 全局验证门全部通过。
- 新增 collision pipeline tests：AABB 过滤、pair persistence、contact manifold 更新、退化/NaN 输入不 panic。

**subagent 分工**

- Implementer：负责 collision pipeline 和测试。
- Spec Reviewer：检查 broadphase/narrowphase 接口是否足够小，是否保留现有行为。
- Code Reviewer：重点查 pair 泄漏、接触法线方向、Vec 分配热点、`partial_cmp().unwrap()`、contact id 不稳定。

**硬边界**

- 不调 solver 真实感参数。
- 不改 wasm API。
- 不把 M6 solver 修正混入 M5。

状态：M5 Collision Pipeline 已完成最小 broadphase/narrowphase/manifold lifecycle 重构，未进入 M6/M7。

- 实现：`collision/mod.rs` 新增 `Aabb`、`AabbCache`、`BroadphaseStrategy` seam 和 `SweepAndPruneBroadphase`。`rough_collision_detection` 现在先为 `CollisionalCollection` 构建一次 AABB cache，再由 SAP strategy 生成 candidate index pair；旧循环内重复 `projection_on_axis` 和 `partial_cmp().unwrap()` 路径已移除，排序使用 `f32::total_cmp`，degenerate min/max range 会归一化。
- 实现：AABB cache 对任一轴含 NaN/inf 的 collider 采取保守跳过策略，避免 broadphase panic；有限 collider 仍照常进入 candidate filtering。`ElementStore` / `CollisionalCollection` 入口保持兼容，`rough_collision_detection` public 入口未变。
- 实现：narrowphase 未改 GJK/EPA 语义，只把 `accurate_collision_detection_for_sub_collider` 收到 `NarrowphaseStrategy` / `GjkEpaNarrowphase` seam 后再调用原逻辑，给后续 circle-circle / circle-polygon / polygon-polygon specialized path 留出小接口。
- 实现：`ContactConstraintManifold` 增加 collision-pass lifecycle；`Scene::collision_detective` 每个 pass 开始先让 persistent manifold 记录 `was_active_last_pass`，再进入 inactive/current-pending 清空状态。新 pair 直接创建 current contact infos；旧 pair 只有上一 pass active 且当前 pass 再命中时，才把 current contact pairs 暂存在 pending，保留上一帧 constraint infos 给 warm-start 使用；上一 pass inactive 后 re-contact 的 pair 会在 warm-start 前直接刷新成 current contact pairs，丢弃分离前 cached lambda。`warm_start()` 之后、`pre_solve_constraints()` 之前再把 eligible active manifolds 刷新成 current contact pairs。cache 仍保留，不碰撞 pair 不再作为 active pair 被同 pass 后续 query 误报。
- 测试：新增 `collision` pipeline tests，覆盖 AABB candidate filtering、degenerate projection range 归一化、非有限 AABB 不 panic 且跳过 invalid collider、既有 circle GJK/EPA narrowphase 行为不回归。
- 测试：新增 `scene` lifecycle test，覆盖 persistent manifold pair 在下一次 collision pass 未被 broadphase/narrowphase 发现时必须变 inactive，同时保留 cache entry。
- TDD RED：`rtk proxy cargo test -p picea rough_collision_detection --lib` 在旧实现下 1 passed / 2 failed；degenerate range 返回 `[]` 而预期 `(1, 2)`，NaN projection 触发 `partial_cmp().unwrap()` panic。`rtk proxy cargo test -p picea collision_detective_deactivates_pairs_not_seen_in_current_pass --lib` 在旧实现下 0 passed / 1 failed，persistent manifold 仍 active。
- GREEN：实现后 `rtk proxy cargo test -p picea rough_collision_detection --lib` 通过，3 passed；`rtk proxy cargo test -p picea collision_detective_deactivates_pairs_not_seen_in_current_pass --lib` 通过，1 passed；`rtk proxy cargo test -p picea narrowphase_gjk_epa_keeps_existing_circle_collision_behavior --lib` 通过，1 passed。
- Code review 返工：review 指出 inactive stale manifold 虽然不再被 `is_element_collide` 视为 active，但仍会进入 `pre_solve_constraints` 并增加 `contact_count`，影响 active pair 的 position correction；同时 empty vertex projection sentinel `(f32::MAX, f32::MIN)` 会被 range 归一化成 world-sized AABB。新增 `tick_pre_solve_ignores_inactive_manifolds_when_counting_contacts` 和 `rough_collision_detection_skips_empty_convex_polygon_projection_without_panicking` 两个 RED 测试后，旧实现分别失败为 A 的 `contact_count` 为 `2` 而预期 `1`，以及 empty `ConvexPolygon::new(Vec::new())` 产生 broadphase candidate。修复为 `pre_solve_constraints` 只处理 active contact manifold，并在 AABB range 归一化前拒绝 empty projection sentinel；GREEN 后对应 targeted tests 通过，`rough_collision_detection --lib` 为 4 passed，`scene::tests --lib` 为 11 passed。
- Code review 返工：review 继续指出 `step_frame` 旧顺序是 `warm_start()` 早于 `collision_detective()`，上一帧 active 但当前帧已分离的 pair 会在 current collision pass 之前吃到旧 warm-start impulse。新增 `tick_does_not_warm_start_stale_manifold_after_pair_separates` RED 测试：先用真实 tick 生成 cached lambda，再分离 pair 并清零 velocity/angle_velocity；旧实现下一 tick 后 velocity/angle 被 stale impulse 改成非零。修复为 `collision_detective()` 提前到 `warm_start()` 前，current pass 先 mark inactive + detect current pairs；`warm_start()` 只处理 refreshed active manifolds，并不再负责把 active 标 false，从而保留 active 状态给 `pre_solve_constraints`。GREEN 后该 targeted test 通过，`scene::tests --lib` 为 12 passed。
- Code review 返工：final review 指出上一条修复把 separated stale pair 修掉了，但 continuing pair 也因为 pass start `mark_all_inactive()` 后走 `replace_contact_point_pairs()` 而丢掉旧 `total_lambda` / friction lambda，导致 valid continuing contact 没有 warm-start。新增 `continuing_contact_warm_start_uses_cached_lambda_before_pre_solve_refresh` RED 测试：第一帧真实 tick 生成非零 cached lambda，第二次 current collision pass 保持 pair 重叠并清零 velocity/angle_velocity，要求 `warm_start()` 在 `pre_solve` 前消费旧 cached impulse；旧实现失败在 “current collision pass must not discard cached impulse before warm_start consumes it”。修复为 existing manifold 命中时 pending current contact pairs，warm-start 后再 refresh；GREEN 后 targeted test 通过，`scene::tests --lib` 为 13 passed。
- Code review 返工：M5 final review 继续指出 re-contact 漏洞：pair 分离过一个 pass 后又重新接触时，旧 cached manifold 仍留在 map 中，当前实现会把它重新标 active 并在 refresh 前 warm-start，错误复用分离前 cached lambda。新增 `recontact_after_inactive_pass_does_not_warm_start_pre_separation_lambda` RED 测试：A/B 第一帧生成非零 lambda，B 移走并跑一个 collision pass 让 A/B inactive，再移回重叠并清零 velocity/angle_velocity；旧实现失败为 A 的 velocity/angle_velocity 被分离前 impulse 改成非零。修复为 `ContactConstraint` 记录 `was_active_last_pass`，`warm_start()` 只处理 `was_active_last_pass && is_active` 的 manifold；上一 pass inactive 后 re-contact 时直接 `replace_contact_point_pairs` 丢掉旧 lambda。GREEN 后 `warm_start --lib` 为 3 passed，`scene::tests --lib` 为 14 passed。
- 验证结果：`rtk proxy cargo fmt --all --check` 通过；`rtk proxy cargo test -p picea --lib` 通过，47 passed；`rtk proxy cargo test -p picea --examples --no-run` 通过；`rtk proxy cargo test -p picea-macro-tools` 通过，6 passed；`rtk proxy cargo test -p picea-web --lib` 通过，0 tests；`rtk git diff --check` 通过。输出仍有既有 warning，本轮未扩范围清理。
- 边界核对：未改 `constraints/*` solver 公式；未调 restitution/friction/sleep 参数；未改 wasm API；未进入 M6 fixed mass/effective mass 修正；未进入 M7 wasm hardening；未删除或纳入未跟踪 `.DS_Store`。
- residual risk：AABB cache 仍是每次 broadphase pass 临时构建，不是 persistent shape cache/invalidation 系统；非有限 AABB 当前是跳过而非诊断上报；broadphase 只落地 SAP strategy seam，未实现 grid/tree；narrowphase specialized fast paths 尚未实现；contact manifold 仍使用临时 `Vec<ContactPointPair>` 和旧 contact identity。当前 warm-start 只支持上一 pass active 且当前 pass active 的整 pair cache；如果 contact id 改变、接触点重排或多点部分延续，仍没有 stable contact id / per-contact impulse transfer / allocation 优化。

### M6 Solver Realism

**目标**

提升求解器真实感与稳定性，修正 effective mass、固定体质量语义、位置修正、摩擦、反弹阈值和 sleep 行为。

**范围**

- 修正 contact effective mass 可疑公式，确保 A/B 质量、惯量都参与。
- 明确固定体质量语义，避免固定体仍在不该参与的位置/速度修正中贡献错误。
- split impulse/position correction 策略化，避免速度求解掺入过多位置 bias。
- 静/动摩擦、restitution threshold、sleep 进入/唤醒行为测试。

**测试门**

- M0 全局验证门全部通过。
- 新增 solver tests：堆叠稳定、斜坡静摩擦、弹性碰撞阈值、固定体碰撞、sleep/wakeup。

**subagent 分工**

- Implementer：只负责 solver 数学和测试场景。
- Spec Reviewer：检查真实感目标是否可量化，避免只凭视觉调参。
- Code Reviewer：重点查 impulse 符号、法线方向、质量倒数、bias 使用、能量爆炸、测试容差过宽。

**硬边界**

- 不改 wasm API。
- 不改 JS 类型。
- 不做 UI/渲染器。

**执行记录（2026-04-19，M6 implementer）**

状态：M6 Solver Realism 最小修复已完成，严格停在 M6，未进入 M7。

- 实现策略：只处理 contact solver effective mass 与 fixed body effective inverse mass/inertia 语义；不改 wasm API/JS 类型/UI；不改 collision broadphase/narrowphase pipeline；不重写 solver 架构；velocity solver 公式、restitution threshold、velocity bias、摩擦参数和 sleep 参数保持不变。
- RED 证据：新增 `constraints::contact::tests::position_solver_uses_b_inverse_mass_in_effective_mass` 后，旧实现失败为 A 位置修正 `-0.05`，期望 `-0.08`，暴露 `solve_position_constraint` 把 `obj_a_meta.inv_mass()` 加了两次且缺少 B 倒质量；新增 `fixed_body_does_not_contribute_to_position_effective_mass` 后，旧实现中 fixed A / dynamic B 场景 B 只修正 `0.0125`，期望 `0.1`。
- 实现：`Meta` 增加 crate-private `effective_inv_mass()` / `effective_inv_moment_of_inertia()`，fixed body 返回 `0`，动态体返回原普通倒质量/倒惯量；`compute_inv_mass_effective()` 改用该 helper，让 contact pre-solve 的 normal/tangent effective mass 不再把 fixed body 当动态体；contact 位置修正改为复用 `compute_inv_mass_effective()`，移除手写且错误的 A+A 倒质量公式，并对非有限/零/负 effective inverse mass 做 no-op。
- 行为锁：contact 单测覆盖非等质量中心重叠时位置修正按 A/B inverse mass 正确分摊，以及 fixed body 不参与 effective mass 且不被位置修正移动，动态体吸收完整位置修正。
- GREEN 证据：`rtk proxy cargo test -p picea contact::tests --lib` 通过，2 passed。
- Code review 返工：新增 contact invalid mass 行为锁，覆盖 normal/friction solver 在 `mass = 0` / `mass = NaN` 时必须 no-op 且 velocity/angle_velocity 保持有限；新增 join 行为锁，覆盖 both-fixed hard join no-op、`mass = 0` / `mass = NaN` dynamic body no-op、fixed-dynamic hard join 仍能修 dynamic side。RED 时 contact 4 个新增测试失败为 velocity NaN，join 2 个 invalid mass 测试失败为 velocity NaN。
- 返工实现：contact normal/friction solve 在 effective mass 非有限或小于等于 `FloatNum::EPSILON` 时直接跳过，不再 apply zero impulse；join solve 在 `inv_mass_effective + soft_part` 非有限或小于等于 epsilon 时直接 no-op，并防御非有限 lambda。invalid mass 策略保持在 solver 层保守 no-op，不改 builder/API 输入契约。
- Code review 二次返工：新增 scene warm-start invalid mass 行为锁，覆盖 cached contact impulse 在下一帧 warm-start 遇到 `mass = 0` / `mass = NaN` 时不产生非有限 velocity/angle；新增 contact 极小有限 effective mass 与 join 极小有限 denominator 行为锁，防止绝对 EPS cutoff 误 no-op。RED 时三个 targeted tests 分别失败于 warm-start velocity 非有限、contact 极小有限值未求解、join 极小有限 denominator 未求解。
- 二次返工实现：contact/join 可解性 guard 改为只拒绝非有限或小于等于 `0` 的 effective mass / denominator；warm-start 通过 contact info 基于当前 A/B body 重新检查 cached normal/friction impulse 是否可解，并跳过 zero / non-finite cached impulse。有限但很小的正值不再被 EPS 拦截。
- 最终验证结果：`rtk proxy cargo fmt --all --check` 通过；`rtk proxy cargo test -p picea --lib` 通过，60 passed；`rtk proxy cargo test -p picea --examples --no-run` 通过；`rtk proxy cargo test -p picea-macro-tools` 通过，6 passed；`rtk proxy cargo test -p picea-web --lib` 通过，0 tests；`rtk git diff --check` 通过。输出仍有既有 warning，本轮未扩范围清理。
- residual risk：静/动摩擦、restitution threshold、`velocity_bias` 未在本轮改动或调参；sleep/wakeup 未新增行为测试；invalid mass 目前不在 `MetaBuilder::mass()` / public API 边界拒绝，而是在 contact/join solver 层 no-op 防 NaN，后续若要生产化需要统一质量输入校验和错误策略；both-fixed / invalid-mass join 现在 no-op，未引入诊断事件或错误返回。

### M7 WASM API Hardening

**目标**

把 wasm public API 从 panic/unwrap 风格收敛到可报告错误、可隔离 callback 异常、TypeScript 类型可信、public API 有 smoke 覆盖。

**范围**

- Result/error channel：输入解析、序列化、非法 element id、非法 shape 输入都走明确错误路径。
- callback error isolation：JS callback 抛错不能破坏 Rust scene 内部状态。
- TS 类型与 wasm-bindgen public API 对齐。
- public API smoke tests 或 wasm-bindgen test，覆盖 create scene、create shape、tick、query、callback。

**测试门**

- M0 全局验证门全部通过。
- 新增 wasm API smoke tests 或等价 public API 验证；至少覆盖错误输入不 panic。

**subagent 分工**

- Implementer：只负责 wasm API hardening 与 TS 类型/测试。
- Spec Reviewer：检查错误通道是否一致、是否保持兼容策略。
- Code Reviewer：重点查 `unwrap`、callback failure、Rc/UnsafeCell alias、JS 值解析、错误吞掉后状态不一致。

**硬边界**

- 不做 JS 渲染器。
- 不引入 UI 框架。
- 不把 M5/M6 的 collision/solver 改动塞进 wasm hardening。

**执行记录（2026-04-19，M7 implementer）**

状态：M7 WASM API Hardening 已完成最小 public API 加固；未改 core solver/collision/storage/shape；未做 JS UI/renderer；未 commit。

- RED 证据：先新增 `picea-web` 行为锁后，`rtk proxy cargo test -p picea-web try_methods_return_errors_for_invalid_js_values --lib` 在旧实现上失败：缺少 `try_set_gravity`、`try_create_polygon`、`try_create_line`、`try_update_element_meta_data`、`try_create_point_constraint`、`try_create_join_constraint` 和 callback error isolation helper。
- native 测试边界：`wasm-bindgen::JsValue` 在 non-wasm target 上对 object/string 反序列化与 `JsValue::from_str` 会触发 non-unwinding abort，因此 public invalid-JS smoke tests 保留为 `wasm32` cfg；native `picea-web --lib` 使用纯 helper tests 覆盖 invalid vector/point/meta/config/polygon 校验不 panic，以及 callback error path 不 unwrap。
- 实现：新增 `trySetGravity`、`tryCreatePolygon`、`tryCreateLine`、`tryUpdateElementMeta`、`tryCreatePointConstraint`、`tryCreateJoinConstraint`、constraint `tryUpdateMovePoint` / `tryUpdateConfig`、`tryGetKinetic`、`tryIsPointValidAddIntoPolygon`。旧 API 保持兼容包装：void update 失败 no-op，返回 `u32` 的旧 create 失败返回 `0`，constraint create 失败返回 `undefined`，`getKinetic` 缺失 element 返回 `null`，polygon validity 旧函数失败返回 `false`。
- callback isolation：`registerElementPositionUpdateCallback` 与 `forEachElement` 的 `call*().unwrap()` 改为捕获 `Err(JsValue)` 并忽略；wasm32 下可选 console log，不让 JS callback throw 破坏 Rust scene。
- TS 类型同步：`crates/picea-web/src/type.d.ts` 标注新增 `try*` 方法和旧 API error behavior；Result/`JsValue` 在 TypeScript 侧表现为 throw。
- GREEN 证据：`rtk proxy cargo test -p picea-web --lib` 通过，3 passed。
- residual risk：本轮未引入 wasm-bindgen browser/Node smoke runner，`wasm32` public invalid-JS tests 未纳入本地验证门；`getElementVertices` / `forEachElement` 对 Arc edge 仍保留既有 `todo!()`；部分 serde serialization `to_value(...).unwrap()` 仍基于内部可序列化数据假设，未扩成全 API Result 化。

**Spec review 返工（2026-04-19，M7 implementer）**

- 补齐剩余 public shape creation error channel：新增 `tryCreateRect`、`tryCreateCircle`、`tryCreateRegularPolygon`，并同步 TypeScript。策略明确为 rect `width/height > 0`、circle/regular polygon `radius > 0`、regular polygon `edgeCount >= 3`，所有数值参数必须 finite；旧 `createRect` / `createCircle` / `createRegularPolygon` 失败返回 `0`。
- 明确 `cloneElement(metaData)` 在 public error-input 范围内：新增 `tryCloneElement`，source id 缺失或 meta invalid 返回 `JsValue` error；旧 `cloneElement` 保持原兼容，source id 缺失返回 `None`/`undefined`，invalid meta 继续 fallback default meta。
- 行为锁：新增 native helper test 覆盖 numeric shape validation；新增 native legacy wrapper test 覆盖 numeric fallback 返回 `0` 与 clone missing source 返回 `None`；新增 wasm32 cfg public smoke test 覆盖 `tryCreateRect` / `tryCreateCircle` / `tryCreateRegularPolygon` / `tryCloneElement` 的 invalid input 和 invalid meta error path。
- 返工验证结果：`rtk proxy cargo fmt --all --check` 通过；`rtk proxy cargo test -p picea-web --lib` 通过，5 passed；`rtk proxy cargo test -p picea --lib` 通过，60 passed；`rtk proxy cargo test -p picea --examples --no-run` 通过；`rtk proxy cargo test -p picea-macro-tools` 通过，6 passed；`rtk git diff --check` 通过。输出仍有既有 warning，本轮未扩范围清理。

**Spec review 二次返工（2026-04-19，M7 implementer）**

- `createRegularPolygon` / `tryCreateRegularPolygon` 的 wasm public `edgeCount` 从 `usize` 改为 `f64`，先在 JS number 对应的 f64 层验证 finite、integer、`3..=1024`，再 cast 到 `usize`；TypeScript 仍为 `number`，文档标注必须是 3 到 1024 的整数。
- `tryIsPointValidAddIntoPolygon` 调整为先解析/校验 `point`，再处理 `vertices.len() <= 2` 的 early return，避免 invalid point 被短路吞掉。
- `getElementVertices` 新增 `tryGetElementVertices` error channel；旧 `getElementVertices` 对 missing element、circle edge、arc edge 或序列化失败统一 fallback `[]`，不再在 public query path `todo!()` panic。`tryGetElementVertices` 对 missing element / circle / arc 返回 `JsValue` error。
- 行为锁：native helper 覆盖 `edgeCount` NaN、Infinity、valid exact 1024、negative、near-integer fractional、too-small、too-large；native public test 覆盖 circle `getElementVertices` fallback 不 panic；wasm32 cfg smoke 覆盖 invalid point 必须先 error、`tryGetElementVertices` circle error/fallback，以及 `tryCreateRegularPolygon` 的 valid/invalid edgeCount。
- 二次返工验证结果：`rtk proxy cargo fmt --all --check` 通过；`rtk proxy cargo test -p picea-web --lib` 通过，6 passed；`rtk proxy cargo test -p picea-web --lib --target wasm32-unknown-unknown --no-run` 通过；`rtk proxy cargo test -p picea --lib` 通过，60 passed；`rtk proxy cargo test -p picea --examples --no-run` 通过；`rtk proxy cargo test -p picea-macro-tools` 通过，6 passed；`rtk git diff --check` 通过。输出仍有既有 warning，本轮未扩范围清理。
- edgeCount 窄返工：review 指出 `FloatNum=f32` 会在 wasm binding 入参处吞掉 `3.00000001` / `1024.00001` 这类 near-integer JS number；最终实现改为 public `edgeCount: f64`，validator 也接收 `f64`，确保近整数小数不会先舍入成整数后通过。

**Post-M7 known issues hardening（2026-04-19，public wasm panic hardening implementer）**

状态：完成 M7 后残留 public wasm panic 风险收口；范围只触碰 `crates/picea-web/src/*` 与本计划文档；未改 core picea solver/collision/shape/storage；未做 UI/renderer；未 commit。

- RED 证据：先新增 `for_each_element_shape_builder_skips_unsupported_arc_edges_without_panic` 行为锁，旧实现下 `rtk proxy cargo test -p picea-web for_each_element_shape_builder_skips_unsupported_arc_edges_without_panic --lib` 编译失败：缺少可覆盖 Arc fallback 的 builder/helper，且旧 `forEachElement` 仍停在 `Edge::Arc` 的 `todo!()` 分支。当前 public constructors 无法直接构造 Arc edge，因此用纯 helper 锁定 Arc fallback 语义。
- 实现：`forEachElement` 改为通过 `build_for_each_element_shape` 生成 callback payload；`Edge::Arc` 返回 `UnsupportedEdge`，策略为跳过该 element、记录 wasm32 console log、继续迭代后续 element，不让一个 unsupported edge 中断整个 callback API。circle/polygon 仍按既有 shape payload 回调，callback throw 仍被捕获忽略。
- public serialization unwrap 收口：新增 `to_js_value_result` / `to_js_value_or_null` / `to_web_point_result` helper；`getElementMetaData`、`getElementVertices`、`tryGetElementVertices`、`getElementCenterPoint`、`getPositionFixMap` 与 `From<&Point> for WebPoint` 不再直接 `serde_wasm_bindgen::to_value(...).unwrap()`。try path 返回 `JsValue` error；legacy query fallback 分别保持 `undefined` / `[]` / `null`。
- `isPointValidAddIntoPolygon`：`vertices.last().unwrap()` 原本由 `vertices.len() <= 2` guard 保护，parse 后长度不变；本轮仍改为显式 `last().copied()` fallback，避免 public helper path 保留 panic unwrap。
- TypeScript 文档：`forEachElement` 注释补充 unsupported arc edge 会被跳过且不会停止 iteration。
- GREEN 证据：`rtk proxy cargo test -p picea-web for_each_element_shape_builder_skips_unsupported_arc_edges_without_panic --lib` 通过；`rtk proxy cargo test -p picea-web --lib` 通过，7 passed；`rtk proxy cargo test -p picea-web --lib --target wasm32-unknown-unknown --no-run` 通过。
- residual risk：仍未新增 wasm-bindgen browser/Node smoke runner；Arc edge 目前仍无 public constructor 覆盖真实 JS callback 流，只通过 helper 锁定 fallback。core `shape::utils` 内部仍存在 Arc `unimplemented!()`，不属于本轮 public wasm 边界范围。

**Post-M7 warning hygiene 批次 1（2026-04-19，warning hygiene implementer）**

状态：完成低风险 warning hygiene 批次 1；范围限定在 `crates/macro-tools/src/*`、`crates/macro-tools/tests/*` 与本计划文档；复核 `crates/picea-web/src/*` 但当前 native/wasm 编译未发现 picea-web crate-local unused warning；未改 core `crates/picea/src`、examples、public API 语义或功能；未 commit。

- `picea-macro-tools`：删除 `fields.rs` 与 `lib.rs` 未使用的 `syn` imports，保持 proc-macro 生成逻辑不变。
- `macro-tools` tests：把只用于类型证明但未读取的 bindings 改为断言或实际写入；`static mut META` 测试改用 raw pointer 访问，不再创建 `&mut static mut`；同时读取 `field_b`，避免 dead-code warning。
- `picea-web` 复核：`rtk proxy cargo rustc -p picea-web --lib -- -D warnings` 对 picea-web lib target 通过；`picea-web --lib` native/wasm 验证输出中没有 picea-web crate-local unused warning。legacy wrapper tests 中涉及 `create_circle` / `create_regular_polygon` / `create_line` 的 `mut scene` 仍是当前 API 所需，保留不改。
- 验证结果：`rtk proxy cargo fmt --all --check` 通过；`rtk proxy cargo test -p picea-web --lib` 通过，7 passed；`rtk proxy cargo test -p picea-web --lib --target wasm32-unknown-unknown --no-run` 通过；`rtk proxy cargo test -p picea-macro-tools` 通过，6 passed；`rtk proxy cargo test -p picea --lib` 通过，60 passed。
- residual risk：core `crates/picea/src` 仍有既有 warnings（contact/scene/shape/utils/snapshot/dead_code 等），本批次按硬边界未处理；未新增 wasm-bindgen browser/Node smoke runner；未做大规模 `cargo fix`。

## 5. Subagent 编排

每个任务都按以下顺序执行：

1. **Implementer**
   - 先写失败测试或行为锁，再写最小实现。
   - 必须列出改动文件、风险点、测试命令和结果。
   - 必须声明是否触碰到当前 dirty 文件；未经 supervisor 允许不得覆盖用户 dirty。

2. **Spec Reviewer**
   - 只看验收条件和是否符合当前 milestone。
   - 检查是否偷跑到下一 milestone、是否扩大范围、是否改变 public contract 但没有说明。
   - 输出通过/不通过和必须修正项，不做代码风格泛评。

3. **Code Reviewer**
   - findings-first。
   - 重点查 bug、unsafe、unwrap、panic、物理量方向、质量/惯量倒数、NaN、测试漏洞和回归风险。
   - 每个 finding 都要绑定文件路径/行号和可复现影响。

4. **Supervisor Acceptance**
   - 主线程只做验收与裁决：是否满足 milestone、是否通过验证门、是否需要返工。
   - 未通过 acceptance 前，不进入下一 milestone。

## 6. 全局验证门

M0 之后，最低验证门固定为：

- `rtk proxy cargo test -p picea --lib`
- `rtk proxy cargo test -p picea --examples --no-run`
- `rtk proxy cargo test -p picea-macro-tools`
- `rtk proxy cargo test -p picea-web --lib`

任何 milestone 想声明完成，至少要跑完上述命令；如果新增了对应模块测试，还必须追加更窄的 targeted tests。若验证门失败，必须记录失败命令、失败原因、是否与当前 milestone 相关、是否受用户 dirty 影响。

## 7. 风险证据索引

1. `crates/picea/src/scene/mod.rs:126-129`：`Scene::tick` 把 delta time clamp 到 `1/60..1/25`，当前不是 accumulator/substep 模型，确定性和大帧处理需要 M2 重新建模。
2. `crates/picea/src/scene/mod.rs:146-160`：速度约束迭代 `10`、位置修正迭代 `20` 是硬编码常量，后续需要配置化并纳入 solver tests。
3. `crates/picea/src/element/store.rs:36-39`：`ElementStore` 使用 `Vec<Rc<StoredElement<T>>>`、`BTreeMap<ID, Rc<StoredElement<T>>>` 与排序缓存，cache locality 和并行化都会受限。
4. `crates/picea/src/element/store.rs:11-19`、`crates/picea/src/element/store.rs:92-95`：`StoredElement` 内部是 `UnsafeCell<Element<D>>`，push 时包进 `Rc`，需要 M3 降低 unsafe/alias 面积。
5. `crates/picea/src/scene/mod.rs:728-739`：`query_element_pair_mut` 通过两次 `get_element_mut` 转裸指针再构造两个 `&mut`，这是 handle/arena 重构的核心风险点。
6. `crates/picea/src/element/mod.rs:23-24`、`crates/picea/src/element/mod.rs:87-92`：`ElementBuilder` 和 `Element` 都持有 `Box<dyn ShapeTraitUnion>`，shape 热路径依赖动态派发。
7. `crates/picea/src/collision/mod.rs:123-129`、`crates/picea/src/collision/mod.rs:225-228`：rough collision 固定 `AxisDirection::X`，排序使用 `partial_cmp(...).unwrap()`，遇到 NaN 会 panic，且 broadphase 策略不可替换。
8. `crates/picea/src/collision/mod.rs:179-204`、`crates/picea/src/collision/mod.rs:534-655`：narrowphase 多处返回/构造 `Vec<ContactPointPair>`，contact manifold 热路径存在临时分配。
9. `crates/picea/src/math/vector.rs:79-80`：`Vector::normalize` 直接对 `self.abs()` 取倒数，零向量会产生非有限值，需要 M1 契约。
10. `crates/picea/src/constraints/contact.rs:487-490`：位置修正 effective mass 使用 `obj_a_meta.inv_mass()` 两次，缺少 `obj_b_meta.inv_mass()`，这是 M6 必须锁住的可疑物理公式。
11. `crates/picea/src/constraints/contact.rs:57`、`crates/picea/src/constraints/contact.rs:364-370`、`crates/picea/src/constraints/contact.rs:402`：`velocity_bias` 被计算和存储，但 velocity solve 的 lambda 使用 restitution factor 直接计算，没有使用 `velocity_bias`，反弹/bias 语义需要 M6 梳理。
12. `crates/picea/src/shape/concave.rs:76-79`：`ConcavePolygon::sync_transform` 标注 `TODO cache this method`，每次 transform 重新拆凸并构造 `ConvexPolygon`，需要 M4 local-space/cache pipeline。
13. `crates/picea/src/shape/utils.rs:462`、`crates/picea/src/shape/utils.rs:832-876`：当前凹多边形测试触发 `unreachable!` panic，是 M0 必须先锁住的基线红灯；该文件当前 dirty，归因必须谨慎。
14. `crates/picea-web/src/wasm.rs:41`、`crates/picea-web/src/wasm.rs:616`：wasm scene 使用 `Rc<UnsafeCell<Scene>>`，与 core storage 的 interior mutability 风险叠加。
15. `crates/picea-web/src/wasm.rs:123`、`crates/picea-web/src/wasm.rs:288`、`crates/picea-web/src/wasm.rs:398`、`crates/picea-web/src/wasm.rs:519-521`：wasm public API 和 callback 路径存在多处 `unwrap()`，错误输入或 JS callback 抛错会转成 panic 风险。

## 8. 下一步执行建议

严格先执行 M0。M0 的目标不是“顺便优化”，而是让仓库获得能被所有后续 milestone 信任的最低验证基线。若 M0 遇到 `crates/picea/src/shape/utils.rs` dirty 冲突，Implementer 必须停止并交给 supervisor 裁决，不得自行恢复或覆盖。

**M0 worker prompt 大纲**

```text
你是 Picea M0 Verifiable Baseline implementer。当前仓库 /Users/asyncrustacean/projects/picea。只做 M0：修复/隔离 examples 访问私有 Context.constraint_parameters 的编译失败，锁住并修复当前凹多边形 panic。先写失败测试或行为锁，再做最小实现。不得做性能/架构大改，不得进入 M1。当前已知用户 dirty：crates/picea/src/shape/utils.rs 和 .DS_Store，改动前必须检查 git status；如需要修改 dirty 文件，先停止并向 supervisor 报告冲突。完成后列出改动文件、测试命令和结果，至少跑：rtk proxy cargo test -p picea --lib；rtk proxy cargo test -p picea --examples --no-run；rtk proxy cargo test -p picea-macro-tools；rtk proxy cargo test -p picea-web --lib。
```

**M0 spec-reviewer prompt 大纲**

```text
你是 Picea M0 spec reviewer。只审核 implementer 的改动是否满足 Verifiable Baseline：examples 不再访问私有字段或有合法配置 API；凹多边形 panic 有行为锁并被最小修复；验证门完整；没有偷跑到性能、storage、collision、solver 或 wasm API 大改。输出通过/不通过、必须修正项和证据路径。
```

**M0 code-reviewer prompt 大纲**

```text
你是 Picea M0 code reviewer。findings-first 审查 implementer patch。重点查：是否覆盖用户 dirty、是否新增 unwrap/panic、是否只改 M0 必要范围、examples 配置 API 是否破坏封装、凹多边形测试是否真实锁住失败、是否有 NaN/退化几何遗漏、验证命令是否可信。每个 finding 绑定文件路径和行号；最后给 residual risk。
```
