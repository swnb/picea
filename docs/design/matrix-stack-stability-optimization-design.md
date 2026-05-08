# Matrix Stack 稳定性优化设计

> 日期：2026-05-06
>
> 状态：D1 设计产出，服务于
> `docs/plans/2026-05-06-matrix-stack-stability-optimization-milestones.md`。

本文把 Matter.js / Box2D 中对堆叠稳定有价值的设计思想，映射为
`picea` 当前 `World + SimulationPipeline` 架构下可执行的优化边界。

这不是实现文档，也不引入外部物理引擎依赖。它的作用是让后续 E2-E5
知道先改哪里、为什么改、改到什么程度算验收。

## 目标

`matrix_stack 8x6` 暴露的问题不是单点 bug，而是高密度接触下几个系统同时受压：

- contact identity 不够稳定会造成 contact churn；
- warm-start 无法复用会让 sequential impulse 每帧重新爬坡；
- position correction 过强或过弱都会制造穿透、抖动或 sleep reset；
- solver row ordering / iteration 调度会影响压力自下而上的传播；
- sleep 只能作为稳定结果，不能作为掩盖不稳定的手段。

优化顺序因此固定为：

1. 先用验收门和报告基线固定问题。
2. 再稳定接触身份和 warm-start。
3. 然后控制穿透与位置修正噪声。
4. 之后再看 row ordering / iteration 调度。
5. 最后处理 sleep 收敛。

## 参考机制

| 参考来源 | 有价值的机制 | 映射到 picea | 不照搬的部分 |
| --- | --- | --- | --- |
| Box2D contact constraints | 接触约束自动生成，不由用户手写。 | `pipeline/contacts.rs` 保持 contact gather / refresh / event 入口，不把 contact constraints 暴露为 public API。 | 不新增用户可编辑 contact constraint API。 |
| Box2D persistent contact pair / manifold | 持久 contact pair 同时服务 island graph 和 contact manifold。 | `ContactPairKey` / `ContactKey` / `ContactRecord` 应成为 warm-start 和 churn 分析的稳定输入。 | 不暴露内部 storage id；不为了命中率错误复用 impulse。 |
| Box2D sequential solver | O(N) sequential solver，堆叠靠 warm-start 和稳定 row 输入收敛。 | 保持 `solver/contact.rs` 的 sequential impulse 路线，优先改输入稳定性和修正策略。 | 不在本计划重写成 XPBD/PBD 或引入多线程 solver。 |
| Box2D sleep / island | body 或 group 静止后 sleep，接触或 joint 变化会唤醒。 | `pipeline/sleep.rs` 继续以 island 为单位判断 sleep，但需要暴露或测试 sleep blocker / reset 原因。 | 不把 sleep 当作强制冻结开关。 |
| Matter.js position / velocity iterations | 先 position solve，再 velocity solve，并有独立 iteration 配置。 | picea 已有 velocity solve + residual position correction；后续应分开评价 penetration gate 与 velocity/sleep gate。 | 不把“提高 iteration”当成首要修复。 |
| Matter.js sleeping | sleeping 可提升稳定性和性能，但可能牺牲精度。 | sleep 作为最后一层收敛目标，必须依赖 contact / correction 已经稳定。 | 不先调 sleep threshold 掩盖穿透或 churn。 |
| Matter.js constraint stability advice | 不稳定时降低 stiffness 或增加 constraint iterations。 | 对 picea 的对应取舍是先限制 correction 幅度和噪声，再考虑 iteration 调度。 | 不盲目提高 `position_iterations`。 |

## 当前 picea 基线

| 模块 | 当前能力 | 矩阵堆叠下的疑点 |
| --- | --- | --- |
| `pipeline.rs` | `StepConfig` 默认固定步长，`velocity_iterations=10`，`position_iterations=20`，sleep 开启。 | 迭代已经不低，继续加迭代可能掩盖真正根因。 |
| `pipeline/contacts.rs` | 有 `ContactKey`、`ContactPairKey`、`ContactRecord`、warm-start transfer 和 drop reason。 | churn 可能来自 feature id、contact reduction、normal flip 或 correction 造成的 anchor drift。 |
| `solver/contact.rs` | 有 warm-start impulse application、normal/tangent impulse、Coulomb friction、position bias、residual correction。 | 最大穿透接近箱体高度，说明 position correction 或接触输入可能没有收敛。 |
| `pipeline/island.rs` | 有 deterministic island-local body slots 和 row counters。 | 高密度 stack 可能需要更明确的 row ordering 验证，但不应先动 contract。 |
| `pipeline/sleep.rs` | 有 island-level sleep window 和 wake events。 | 180 帧后全部 awake 表明 sleep 被速度、churn 或 correction 持续打断。 |
| `picea-lab` diagnostics | 已能观测 penetration、churn、warm-start、sleep、island、solver rows。 | 证据层已足够启动 solver 优化，不应再重做一套 Web 归因。 |

## 执行映射

| 机制 | 执行里程碑 | 主要文件 | 验收指标 |
| --- | --- | --- | --- |
| 验收门与报告基线 | E1 | `crates/picea-lab/tests/artifact_run.rs`, `crates/picea/tests/physics_realism_acceptance.rs` | `stack_4` clean gate、`matrix_stack 8x6` stress report、first-bad-frame 可重复。 |
| Contact persistence / warm-start | E2 | `pipeline/narrowphase.rs`, `pipeline/contacts.rs`, `world/contact_state.rs` | settling window 后 churn 下降，warm-start hit ratio 上升，drop reason 可解释。 |
| Position correction / penetration | E3 | `solver/contact.rs`, 必要的 additive stats/tests | penetration max/sum 进入 D2 目标范围，不引入 numeric warning，不持续重置 sleep。 |
| Solver row ordering / iteration | E4 | `pipeline/island.rs`, `solver/contact.rs`, `pipeline.rs` | state hash deterministic，row spike 可解释，小 gate 不依赖极端 iteration。 |
| Sleep convergence | E5 | `pipeline/sleep.rs`, `body.rs`, `events.rs` | quiet stack 能 sleep 或进入低速 quiet window；remaining awake 有 blocker reason。 |

## 设计决策

1. 先修 contact persistence，再修 position correction。
   如果 contact identity 每帧抖动，position correction 的 anchor 也会抖；先稳定输入更划算。

2. 保留 conservative warm-start policy。
   旧 impulse 复用错了比 miss 更危险。后续可以降低不必要 drop，但必须保留 normal mismatch、invalid impulse、point drift 保护。

3. Position correction 要有界。
   correction 应有 slop、per-step 或 per-iteration clamp，并且它对 sleep idle reset 的影响要可观察。

4. Row ordering 只在 E2/E3 后处理。
   当前 `docs/design/solver-island-ordering-contract.md` 已锁住 separate-phase contract；若 E4 需要改 contract，必须暂停并回到设计。

5. Sleep 是结果，不是修复入口。
   只有当 penetration、churn、warm-start 和 visible velocities 已经进入 quiet window 后，sleep 阈值才值得调整。

6. 不引入第三方物理引擎。
   Matter.js / Box2D 作为设计参照，不作为替代实现。picea 保持自己的 public surface 和 Rust 内部求解路径。

## 明确暂缓

- 全面 XPBD/PBD 重写。
- 多线程 island solver。
- 合并 contact/joint rows 为 unified stream。
- 浏览器端重新计算 physics。
- 仅通过提高 `position_iterations` / `velocity_iterations` 声称修复稳定性。
- 以本机 wall-clock 作为 correctness hard fail。

## Live Session 性能方向

`matrix_stack 8x6` 的实时卡顿需要和 solver 稳定性分开处理。artifact / diagnostics 仍是稳定性验收的权威来源；live session 是产品体验路径，当前每个 `step` 都返回完整 `SessionRecord.latest_frame`，其中包含完整 `DebugSnapshot`、diagnostics、tree、contacts 和 inspector 所需事实。这个协议对小场景足够直接，但在大矩阵下响应体会放大到数百 KB，并把 server export、JSON serialization、network transfer、React append/render 都串在同一帧预算内。

优化方向应参考成熟引擎调试器的常见分层，而不是把浏览器变成第二个 physics runtime：

| 层 | 当前状态 | 优化方向 |
| --- | --- | --- |
| Simulation | Rust `World + SimulationPipeline` 每次请求 step 一帧。 | 保持 authoritative，不在 Web 重算物理；必要时支持 server-side batch step。 |
| Render frame | 每步返回完整 `FrameRecord`。 | 增加 lightweight live frame，只包含 bodies/colliders transform、state hash、少量 counters。 |
| Diagnostics | 每步随 full frame 返回。 | 改为按需获取 full frame / diagnostics，或每 N 帧采样。 |
| Inspector / Tree | 前端每次都可消费完整 snapshot。 | 用户选择实体、打开 Diagnostics/Evidence 时再请求 full frame。 |
| Playback cadence | 前端 30Hz interval 触发 step，in-flight 时跳过。 | 改为 response-aware adaptive cadence，并显示 degraded realtime 状态。 |

第一版技术方案可以拆成：

1. `LiveFrameSummary`：新增 additive server response，不破坏现有 `SessionRecord`。
2. `GET /api/sessions/:id/frames/:index`：按需返回完整 `FrameRecord`，供 inspector / diagnostics / copy context 使用。
3. `control step` 支持 `detail=summary|full`，默认 UI live playback 用 summary，手动单步或暂停后 inspection 用 full。
4. Web 在 live playing 时只 append summary render buffer；暂停、scrub 或打开诊断面板时补 full frame。
5. 保留 artifact replay 的 full-frame 路径，避免影响现有验收报告。

### Live Frame 分层协议

`LiveFrameSummary` 是 live session 专用的轻量帧，不替代 artifact
`FrameRecord`。它只承载 canvas 和状态条需要的 authoritative 事实：

| 字段 | 来源 | 用途 |
| --- | --- | --- |
| `kind = "summary"` | server response | Web 区分 summary 与 full frame，避免把缺失 diagnostics 误当作无风险。 |
| `frame_index` / `simulated_time` / `state_hash` | `FrameRecord` | timeline 游标、状态条、事件去重和 profile 归因。 |
| `session_id` / `session_epoch` | `SessionRecord` | 防止 reset 或 paused edit 后的旧响应回写。 |
| `world_revision` | `StepReportRecord.revision` | velocity perturbation preview/commit 的 freshness gate。 |
| `status` / `buffered_frame_count` / `frame_count` | `SessionRecord` | live loop 是否继续、是否 completed、degraded realtime 文案。 |
| `snapshot.bodies` / `snapshot.colliders` 的 transform 与类型字段 | `DebugSnapshot` 子集 | canvas 绘制、selection handle 保持、比例尺和 overlay。 |
| `stats` 的轻量 summary | `DebugSnapshot.stats` 或 `StepReportRecord.stats` | 状态条 counters，不展开 contacts/tree/islands。 |
| `provenance` 的 live authority 字段 | 当前 session + frame | copy/debug context 可提示“此帧仍需 hydrate full”。 |

summary 明确不携带 `report.events`、顶层 `events`、完整 `snapshot.contacts`、
`snapshot.broadphase`、`snapshot.islands`、`diagnostics`、`compound_provenance`
或完整 inspector tree。UI 如果没有 full frame，必须显示“未取完整证据”而不是
“无 diagnostics / 无 missing evidence”。

完整帧获取规则：

- live playback 默认 `POST /api/sessions/:id/control?action=step&detail=summary`。
- 手动单步、暂停后 inspect、打开 diagnostics/evidence、copy debug context、velocity
  perturbation preview/commit 前，需要同一 `frame_index` 的 full frame。
- full frame 通过 `GET /api/sessions/:id/frames/:index` 取得；允许
  `POST /api/sessions/:id/control?action=step&detail=full` 作为兼容/测试路径。
- full frame 必须校验 `session_id`、`session_epoch`、`frame_index` 和
  `world_revision`；不匹配时 Web 丢弃响应并保留当前 authoritative state。
- 同一帧从 summary hydrate 为 full 后，timeline diagnostics、missing evidence 和
  marker cache 必须失效并重算。

这个分层故意把“大而完整”的证据路径留在 Rust/exported `FrameRecord`，把“快而可画”
的 live 路径限制在 authoritative transform/counter 子集。这样牺牲的是 live 播放时
即时 inspector 的完整性，换来的是不破坏 artifact schema、也不在 Web 重算 physics。

这条路线属于后续 live performance milestone，不应混入 E3 的 contact solver 修复。

## 后续执行输入

E2 必须先证明 churn / warm-start 是当前 first-bad-frame 的主要解释，或至少是可重复贡献因素。

E3 必须带着 E2 后的 penetration 数据进入，不应在 contact identity 仍明显抖动时大调 correction。

E4 只在 E2/E3 后仍有 solver row spike 或收敛慢证据时执行。

当前 `matrix_stack 8x6` 的 E4 证据显示，`BodyHandle(48)` 的失败窗口早于最终
floor ejection：frame `38` 仍与 `BodyHandle(40)` 有浅动态接触但 normal /
tangent impulse 已为 `0`，且 solver initial normal speed 已经为正、position /
restitution bias 都为 `0`；frame `39` 开始丢失动态支撑，frame `46` 重新接触
`BodyHandle(32)` 时出现 row spike 和较大 normal/tangent impulse。因此 E4 的下一步
不应继续追 frame `80` 的 floor 边缘接触、sleep threshold 或 normal-bias 调参，而应
优先审查 frame `12..46` 的支撑保持、tangent/rotation 能量、pressure propagation 和
position-row 设计边界。

同一证据还说明“只改 row 顺序”风险很高：当前
`docs/design/solver-island-ordering-contract.md` 明确 contact rows 保持 gather order，
交替正反 traversal 的实验已经让 `stack_4` hard gate 回归。下一步如果要改变 row
order，必须先开独立 contract 设计；本路线更适合先补 row-level provenance，或实现
Box2D-style position row / pseudo pose pass，在不改变 velocity row identity 的前提下
评估 pressure propagation。

E4 中保留的 dense shallow support friction budget 只能视为桥接措施：它在 dense
contact graph 中给浅穿透且已分离的 contact 一个极小 tangent friction budget，帮助
边缘支撑减少侧向滑脱；它不增加 normal support impulse，也不改变 public API。最新
stress 结果显示它能消除 `8x6` 的 floor ejection，但不能解决后半段 velocity spike、
contact churn 或 sleep convergence。因此后续不能继续沿“更大摩擦预算”调参，而应把它
作为 position-row / pseudo-pose 设计前的保守支撑补丁。

late support-gap 证据进一步把 blocker 收窄到 `BodyHandle(32)` 的 frame `164 -> 165`：
缺口前最后 contact 的 depth 只有 `0.001160`，normal impulse 为 `0`，separating
normal speed 为正，且 `correction_depth` / `correction_translation` 都为 `0`。这说明
late spike 不是 residual position correction 把支撑推出去，而是浅动态支撑接触在分离态
没有形成有效几何支撑。后续优化更应朝 contact persistence、浅支撑保留和局部 tangent /
rotation 能量控制走，而不是扩大 slop、强制 sleep 或改全局 row phase。

补充的速度证据显示，frame `164` 的 blocker `BodyHandle(32)` 已有
`previous_speed 1.826026 / 4.953229`，而支撑 counterpart `BodyHandle(24)` 只有
`0.007595 / 0.017564`。因此 late support gap 不是“支撑体高速运动导致接触丢失”，
而是 blocker 自身已经携带显著线速度和角速度滑出浅支撑。后续局部控制应更偏向
blocker-side tangent/rotation energy gate，而不是给静止 counterpart 增加支撑预算。
进一步的 tangent component 分解显示，最后支撑 contact 的 blocker 贡献为
`linear 1.799020 / angular 1.031557`，counterpart 贡献只有 `0.006272 / 0.004983`。
因此下一步若做局部 damping / persistence gate，应优先限制 blocker 的切向线速度和
角速度组合，并要求 counterpart 近似静止、anchor drift 小、不会增加 residual pressure；
不能把它设计成全局摩擦或全局角速度阻尼。

新增的 energy onset 行为锁显示，这个 blocker-side tangent energy 不是 gap 后突发：
`BodyHandle(32)` 在 frame `159` 已经同时超过切向线速度和切向角速度阈值，组件为
`linear 1.035253 / angular 0.912949`，counterpart 仍只有
`0.022849 / 0.000052`，且该 contact 仍有 normal impulse `0.003774`、normal speed
`-0.000522`、depth `0.009087`。frame `159 -> 164` 因此是下一步优化的可控窗口：
接触仍存在、支撑体仍近似静止、法向约束还在工作，但 blocker-side tangent/rotation
能量已经进入高风险区。后续应优先设计 contact lifecycle / position-level
re-evaluation，让浅动态支撑在这个窗口内保持几何一致性；直接在 velocity iteration
末尾阻尼已形成的速度太晚，且已有负向实验显示会提前 support gap。

support lifecycle 行为锁进一步把这个窗口拆开：frame `159` 仍有 normal impulse
`0.003774` 和 position correction `0.088063`；frame `160..162` normal impulse 已经
归零，但 position correction 仍从 `0.072390` 降到 `0.033751`；frame `163..164`
normal impulse 和 position correction 都归零，contact 仍存在但已经是 separating，
normal speed 从 `0.195462` 升到 `0.337866`，frame `165` 才断支撑。也就是说，下一步
不应只问“为什么 contact 消失”，而应问“为什么 normal row 在 contact 仍较深时先失去
support，而 position correction 又在 gap 前两帧停止”。这更指向 position-level
re-evaluation / shallow support lifecycle，而不是单纯调摩擦或 gap 后 damping。

第一版只读 eligibility oracle 进一步证明，这个窗口不是应该被粗暴拒绝的错误 contact：
frame `160..162` 被判为 `retain_candidate_correction_active`，frame `163..164` 被判为
`retain_candidate_needs_position_re_evaluation`；normal anchor drift 仍只有约
`0.004..0.005`，tangent anchor drift 约 `0.028..0.030`，counterpart tangent
contribution 仍低于阈值，没有触发 `reject_*`。因此下一次实现的关键不是“是否应该保留
这个支撑”，而是“如何在 correction 停止后的 frame `163..164` 用 position-level
re-evaluation 安全保留支撑，同时不把 slop 实验那样的错误浅支撑放大成 ejection”。

E4b 的 SAT reference/incident edge 证据说明，真实 `8x6` 的 dominant feature churn
确实集中在 clipped polygon manifold 的 reference face / incident face 角色互换上。
但一次无条件 ordered-pair edge canonicalization 虽然把 180-frame
`MissFeatureId` 从 `418` 降到 `35`，却让 `matrix_stack` 在 frame `83` 出现 floor
ejection，quiet linear 升到 `14.874845`。这说明 contact identity 不能只追求更高
warm-start 命中率；下一版 manifold persistence 必须同时验证 reference/incident role、
ordered local edges、local anchor continuity、clip point ordering 和长窗口 no-ejection。
补充的 top-transition sample 进一步确认 dominant transition 本身是局部连续的：
180-frame retained baseline 中 `k1:r2:i0<-k1:r0:i2 x205` 的 best sample 位于
frame `6`、pair `[ColliderHandle(1), ColliderHandle(9)]`，`point_drift 0.000743`、
`normal_dot 0.999996`、`local_anchor_drift 0.003631`，且 edge-swap transition
count 为 `384`，其中 `366` 个满足 same reduction / same shape / point+normal+
local-anchor continuity 的保守 candidate 条件。因此下一步应该设计“候选持久
manifold”并限制 impulse 迁移，而不是把 feature id 一刀切地改成 canonical。

## E4 Position-Level Re-Evaluation Gate

下一次进入 solver 实现前，需要把“position-level re-evaluation”提升为真正的
position-row pass，而不是继续把逻辑塞进旧 residual correction pass。已回滚的
`True pseudo-pose re-evaluation tiny correction` 证明：即使临时用 pseudo translation
重跑 narrowphase，也只是把 support-gap failure 改写成有接触状态下的晚期速度/修正尖峰，
不能让 `8x6` 进入稳定收敛。

### 目标行为

在 dense contact graph 中，当一个 contact 仍存在但已经进入
“normal impulse lost + separating + blocker-side tangent/rotation risk” 状态时，位置层
不应只复用 frame-start depth 和 slop 判定。它需要在受控 pseudo-position state 中重新
评估该 contact 是否仍是有效支撑：

- 如果 pseudo-state 下 separation、anchor drift 和 counterpart motion 仍满足支撑条件，
  contact 可以继续获得有限 position correction / support eligibility。
- 如果 contact 已经滑出有效支撑几何，必须拒绝保留，避免把错误浅支撑放大成 penetration、
  angular spike 或 ejection。

### 必须保留的边界

- 不改变 public API。
- 不改变 velocity row identity、normal/tangent interleaved solve 顺序或 global row phase。
- 不改全局 SAT/narrowphase skin。
- 不用 sleep threshold 或 body freeze 掩盖未稳定的速度。
- 不在 velocity iteration 末尾追加无几何约束的 damping impulse。
- 仅在 dense contact graph 下启用；`stack_4` 和 aligned `4x3` 是强回归 gate。

### Re-Evaluation 输入

第一版只允许消费已有 solver / debug 已暴露的事实，不引入 Web 或 lab 侧物理：

- contact pair / feature id / anchor drift normal+tangent；
- current contact point、normal、depth、body handles；
- solver final normal speed / tangent speed；
- normal impulse、support friction impulse、position correction depth；
- pseudo-translation state 或 position-row pseudo pose；
- body / counterpart linear-angular tangent components。

### Eligibility 条件

一个浅支撑 contact 只有同时满足以下条件，才允许进入 re-evaluation：

- dense contact graph；
- contact 仍存在，且不是 sensor；
- normal impulse 已归零或接近归零；
- final normal speed 已 separating；
- blocker-side tangent linear + angular components 已进入风险区；
- counterpart 的 linear / angular / tangent contribution 足够小；
- normal anchor drift 小，tangent anchor drift 可解释；
- 当前 contact 不会让 frame-level penetration、late angular spike 或 final outside floor
  body count 退化。

### 拒绝条件

任一条件命中时必须跳过或回滚该 contact 的 retained support：

- counterpart 也在高速运动，不能判定为静止支撑；
- anchor drift 超过 warm-start drift gate；
- pseudo-state re-evaluation 后 separation 已超过 support skin；
- retained support 让 correction total、angular spike、churn 或 ejection 回归；
- `stack_4` hard gate、aligned `4x3` behavior lock 或 staggered `8x6` no-ejection 任一失败。

### 验收指标

下一次 E4 实现不是以“某个局部数值变小”作为成功，而是必须同时满足：

- `stack_4` hard gate 通过；
- aligned `4x3` final hash / penetration / quiet-window 不退化；
- staggered `8x6` 不出现 floor ejection，final outside floor bodies 保持 `0`；
- support gap 不早于 frame `165`，duration 不长于 `10`；
- penetration max 不高于当前 baseline `0.041646`，或若有微小 tradeoff，late linear /
  angular spike、churn、sleep 必须同步改善并单独记录；
- late angular spike 不高于 baseline `5.230875`；
- lifecycle 报告中 normal impulse / correction 失效窗口必须可比较，不能消失或变成
  missing evidence。

两个 contact-persistence 小实验给出了额外边界：

- 全局 SAT speculative skin 会让小规模 `stack_4` quiet linear gate 回归到
  `0.11522542`，因此不能把 Box2D/Matter 风格的 contact skin 直接变成 narrowphase
  默认语义。
- dense-only previous-pair speculative row 能把 `8x6` late support gap 从 `10` 帧缩短到
  `3` 帧，说明方向命中支撑断裂；但它同时让 penetration max 退到 `0.093308` 并重新
  floor ejection，说明 naive persistence 会保留错误支撑。后续若继续该方向，必须加入
  更强的约束：只允许低切向/低角速度、normal/tangent anchor drift 足够小、不会扩大
  residual penetration 的 contact，且先用 `stack_4` / aligned `4x3` / staggered `8x6`
  三重 gate 验证。
- blocker-side dominant tangent damping 不会破坏 `stack_4`，但会让 `8x6` 的 support gap
  从 frame `165` 提前到 `148`，duration 从 `10` 变成 `11`，penetration 退到
  `0.052787`，angular spike 升到 `5.335954`。这说明不能靠 velocity iteration 中追加
  局部 damping impulse 来“吃掉”已形成的切向/角速度；后续应优先让浅支撑几何和 contact
  identity 更稳定，或在 position-level re-evaluation 中处理支撑，而不是事后阻尼。
- `True pseudo-pose re-evaluation tiny correction` 没有破坏 `stack_4`，但让 `8x6` final
  hash 变为 `ad0c3d611e5a053b`，penetration 退到 `0.048148 / 1.728789`，warm-start
  drop 从 `27` 升到 `31`，并让结构化验收因缺少原 `late_support_gap start frame`
  blocker 文本而失败。它说明“在旧 residual correction 内部临时重算一个浅 contact”
  不是足够稳定的架构边界；下一阶段应拆出独立 pseudo pose / position rows，每轮更新
  separation 和 correction，再统一 writeback。
- `Deferred position-row writeback` 是第一步可保留切片：把 residual correction 先映射成
  `ContactPositionRow`，在 pseudo translation state 中累计 correction，最后统一写回 pose。
  它没有让 `8x6` 稳定，但把 support gap 从 frame `165 duration 10` 推迟到
  `171 duration 9`，penetration 降到 `0.041296 / 1.676269`，late angular spike 降到
  `4.568931`，且没有 floor ejection。这个结果支持继续沿 position-row pass 做每轮
  separation / support eligibility re-evaluation，而不是回到 slop、friction 或 damping
  常量实验。
- `Retained shallow-support position row on deferred writeback` 进一步证明，哪怕在 deferred
  writeback seam 上、且把 correction cap 降到 `0.00025`，也不能把 retained-support oracle
  直接变成 correction depth。该实验让 support gap 提前到 frame `157`，penetration 退到
  `0.046520 / 1.689001`，late angular spike 升到 `5.393579`。下一步必须由 pseudo-state
  下的几何有效性决定 contact 是否进入 position row，而不是只看 tangent energy gate。
- `Broaden velocity-level position bias` 也被小规模 gate 否决：把当前只在近静止 normal row
  上启用的 position bias 扩大为“当前分离速度低于目标分离速度就启用”，会让 `stack_4`
  settled penetration 退到 `0.079559326`。因此 E4c 不应全局扩大 velocity bias；如果需要更强
  的 position/velocity coupling，必须先拆到 position-row 或 manifold-level 局部门。

## E4 Next Gate: Per-Iteration Geometry Re-Evaluation

下一步实现必须在 `ContactPositionRow` / `QueuedPositionTranslation` seam 上增加几何重评估，
而不是继续添加 retained-support depth budget。第一版目标不是扩大 contact，而是让已有
position rows 每轮从 pseudo state 重新计算有效 separation。

### 必须满足

- 只在 dense contact graph 中启用，`stack_4` 和 aligned `4x3` 仍是 hard regression gates。
- 每轮 position iteration 使用 pseudo pose / pseudo translation 重新评估 row separation；
  不能复用 frame-start raw depth 再额外塞 shallow correction。
- 对于 shallow retained support，只有 pseudo-state 下仍满足 support normal、separation skin、
  anchor drift 和 counterpart motion gate，才允许进入 correction。
- 所有 re-evaluation 都必须保持 contact lifecycle 可诊断：报告里仍要能比较 energy onset、
  normal impulse lost、correction active、correction stopped、support gap start。
- 成功标准仍按 E4 acceptance：support gap 不早于当前 retained baseline frame `171`，duration
  不长于 `9`，penetration 不高于 `0.041296`，late angular 不高于 `4.568931`，无 ejection。

### 明确不做

- 不再测试“更小 retained-support correction depth”。
- 不在 solver correction loop 中直接重跑 full narrowphase 作为每轮 separation 口径；该实验会重新引入 ejection。
- 不把 residual penetration 直接换算成更大的 dense support friction budget；该实验会把 8x6 support gap 提前到 frame `82 duration 98`，并重新引入 frame `79` floor ejection。
- 不把所有 position rows 直接升级为 angular position correction；该实验会让 `stack_4` settled penetration 退到 `0.060225487`，突破小规模 hard gate。
- 不通过 sleep threshold、body freeze、late damping 或 friction budget 隐藏未收敛速度。
- 不改变 velocity row normal/tangent solve 顺序。
- 不改全局 narrowphase speculative skin。

### 负向边界更新

`Per-iteration narrowphase re-evaluation for existing position rows` 已被验证为负向：
它把 penetration 降到 `0.037364 / 1.643677`，但让 late linear/angular spike 升到
`16.546148 / 11.421408`，support gap 退到 frame `85 duration 95`，并让
BodyHandle(48) 在 frame `82` 出界。结论是：solver 内部不能用 full narrowphase 临时
替代 position constraint geometry。下一步若继续 per-iteration re-evaluation，应使用固定
contact feature、local anchors、support normal 和轻量 separation 函数，或把 manifold
persistence 前移到 contact lifecycle，而不是在 correction loop 里重算完整 manifold。

600-frame long-settle observation 也说明，deferred writeback 的 180 帧改善不能视为稳定：
长窗口下 first exit 出现在 frame `214`，final outside floor bodies 为 `7`，late linear speed
升到 `62.754536`。因此 E4 不能提前转 E5；sleep convergence 必须等长窗口 no-ejection
之后再处理。

E5 必须验证不是通过强制 sleep 掩盖未修复的穿透和速度问题。

## Long-Settle Acceptance Gate

`matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed` 是后续 E4b/E5 的
显式硬门。默认运行 ignored test 时它只打印 skip 并通过，避免当前未完成状态污染普通
套件；设置 `PICEA_MATRIX_STACK_E4_ACCEPTANCE=1` 后才强制执行 600-frame 验收。

目标条件：

- 600 帧 `matrix_stack` 不出现 floor ejection；
- final outside floor bodies 为 `0`；
- long-window penetration max 不超过 `0.05`；
- quiet-window linear speed 不超过 `4.0`；
- quiet-window angular speed 不超过 `6.0`。

当前 retained baseline 在强制模式下仍会红：first floor exit 是 frame `214`，BodyHandle(41)，
quiet-window linear speed 是 `62.754536`。这条红灯是 goal 未完成的直接证据，也是后续
E4b 的验收入口。

## E4b Contact Lifecycle Route

下一阶段不要在 `prepare_contact_warm_start` 中直接把所有 `MissFeatureId` 升级为 pair-level
warm-start fallback。当前已有 `warm_start_cache_reports_feature_id_miss_when_pair_persists_on_different_features`
行为锁，说明 shape / feature family 改变时必须保持 miss，避免旧 impulse 跨几何语义迁移。

一次简单的 pair-level fallback 实验已经验证过这个边界：在 same collider pair、same
reduction reason、normal / anchor drift 检查通过时，从上一帧同 pair contact 继承 cache。
它能把 180-frame `matrix_stack` 的 `MissFeatureId` 从 `418` 降到 `156`，并把短窗口
penetration / linear speed / support-gap duration 略微改善；但它让短窗口 angular gate 仍红，
600-frame 仍在 frame `224` 出现 floor ejection，quiet-window linear speed 升到 `64.363495`。
因此该实现不可保留；E4b 需要更强的 lifecycle 语义，而不是只靠 pair + drift 兜底。

E4b 应拆成三步：

1. **Lifecycle Evidence**：在 artifact / report 层区分“same pair but feature changed”
   和“same pair + shape signature compatible + local anchors still close enough”。这一步只增加诊断事实，不改变 impulse。
   当前 artifact report 已暴露 `same_shape_signature` 和 `close_local_anchor`：它们用于证明
   `MissFeatureId` 中有多少属于同类形状、局部锚点连续的候选，而不是跨几何语义迁移。
2. **Constrained Fallback**：只对同 shape family、同 support normal、local anchor normal/tangent
   drift 都低于 warm-start 阈值、且上一帧 contact 不是 sensor / invalid impulse 的 point 启用
   fallback。fallback 不合成新 contact，只允许当前 narrowphase 已存在的 row 继承 cache。
   第一条保留实现只覆盖更窄的 point-slot 情况：同 collider pair、同 reduction reason、同
   `ContactFeatureId` index、不同 point slot，并且仍通过 normal / local-anchor drift / sensor /
   finite-impulse 检查。这能保护 clipped manifold point 换位，但在当前 8x6 retained baseline 中
   还没有产生可见稳定性改善。后续 trace 进一步确认它在真实 180-frame `matrix_stack` 中的适用面为
   `point_slot_fallback_eligible 0`；主要问题是同 shape/local-anchor 下的 feature index 变化，而不是
   point-slot 换位。
   180-frame 与 600-frame observation 的 dominant transition 都指向
   `k1:r2:i0<-k1:r0:i2`，即 clipped SAT feature index 在 reference / incident 相对边之间跳转；
   后续应优先设计 SAT feature-selection / manifold-persistence 方案，而不是继续扩大 cache fallback。
   最新 edge-swap limited warm-start fallback 实验也验证了这个边界：即使只迁移 `25%` normal impulse
   并清零 tangent impulse，180-frame `matrix_stack` 仍提前到 frame `87` 出现 floor ejection。
   因此 E4b 不能继续把问题压到 warm-start 事后兜底层；必须前移到 manifold 生命周期和支撑有效性。
3. **Long-Settle Gate**：每个实现切片必须同时通过 `stack_4`、aligned `4x3`、180-frame
   `matrix_stack`，再用 `PICEA_MATRIX_STACK_E4_ACCEPTANCE=1` 跑 600-frame gate。只有长窗口
   no-ejection / no-runaway 后，才能进入 E5 sleep convergence。

明确边界：

- 不跨 shape family transfer impulse；
- 不为 broadphase pair 合成 solver row；
- 不把 fallback 结果写成 `Hit`，除非 drift / normal / sensor / finite impulse 检查全部通过；
- 不把 `MissFeatureId` 统计消掉，报告必须仍能看出 fallback 前后的 feature churn。

## E4b Next Gate: Persistent Manifold Candidate

下一次 E4b 实现必须从 contact lifecycle / manifold candidate 进入，而不是继续在
`prepare_contact_warm_start` 中事后补救。已有负向实验给出的边界已经足够明确：

- 无条件 SAT ordered-pair edge canonicalization 会把 feature churn 降低，但会让
  `matrix_stack` 在 frame `83` ejection。
- simple pair-level warm-start fallback 会改善短窗口 penetration / linear speed，但
  600-frame 仍 ejection，且 quiet-window linear speed 升到 `64.363495`。
- edge-swap limited warm-start fallback 即使只迁移 `25%` normal impulse、清零 tangent
  impulse，180-frame 仍提前到 frame `87` ejection。

因此第一版 persistent manifold candidate 只能做 **identity / lifecycle 选择**，不能直接迁移
solver impulse。它的目标是：当 SAT clipped manifold 在 reference / incident role 之间切换，
但局部几何仍连续时，选择一个稳定的 contact identity 或 point ordering；如果该选择不能在
不迁移 impulse 的情况下通过 180-frame gate，就没有资格进入 warm-start 迁移阶段。

### 第一版允许做

- 在 `narrowphase` / contact lifecycle 层识别 clipped SAT 的 reference / incident edge swap
  candidate。
- 只对同 shape family、同 collider pair、normal continuity、local-anchor continuity 和 clip
  point ordering 都满足的 candidate 选择稳定 identity。
- 先保持 warm-start impulse 为 miss 或原有保守策略，不因为 candidate 存在就自动变成 `Hit`。
- 保留 `MissFeatureId`、edge-swap transition / candidate、top transition best sample 等报告字段，
  让验收能看到 identity 变化前后的 churn 和 stability tradeoff。

### 第一版禁止做

- 不再新增 pair-level 或 edge-swap warm-start fallback。
- 不在 solver correction loop 中重跑 full narrowphase。
- 不把 feature id 全局 canonicalize 成 ordered edge pair。
- 不改变 velocity row solve order、position correction percent、sleep threshold 或 friction budget。
- 不让 `stack_4`、aligned `4x3` 或 180-frame `matrix_stack` 任一 gate 退化。

### 第一版验收

- behavior lock `sat_edge_swap_candidate_persists_lifecycle_without_auto_warm_start_impulse`
  必须通过：SAT edge-role swap 后 contact id / manifold id 能持久化，而 warm-start 仍是
  miss/zero impulse。这把问题限定在 lifecycle identity，不把红灯误读成“需要更多 warm-start
  fallback”。
- `stack_4` hard gate 通过，warm-start / contact churn 不回归。
- aligned `4x3` 无 ejection，penetration / quiet-window 不高于 retained baseline。
- 180-frame `matrix_stack` 无 ejection，final outside floor bodies 保持 `0`。
- feature churn 可以下降，但不能用“下降”替代稳定性：若 penetration、late speed、support gap 或
  floor ejection 回归，candidate 选择必须回滚。
- 只有上述全部满足后，才允许设计第二阶段的有限 impulse migration；第二阶段仍必须以
  `PICEA_MATRIX_STACK_E4_ACCEPTANCE=1` 的 600-frame hard gate 为最终入口。

## E4c Next Route: Support-Gap / Position Re-Evaluation

Identity-only persistence 收口后，600-frame hard gate 仍在 frame `214` ejection，且证据更集中：

- ejection support gap 从 frame `180` 开始，frame `229` 起 late support gap 持续到结尾。
- last dynamic support 在 frame `213` 仍有接触与少量 impulse，但 frame `214` 后支撑断开。
- support-gap lifecycle 多次显示 `retain_candidate_needs_position_re_evaluation` 或
  `reject_counterpart_motion`，说明问题不再是“找不到同一几何 identity”，而是动态支撑有效性和
  position row 何时刷新/保留。

下一步 E4c 不应继续扩大 warm-start fallback，而应在 contact lifecycle 和 position correction 之间增加
更明确的支撑有效性判断：

1. **Support Candidate Evidence**：把 support gap 前后的 candidate normal speed、local anchor drift、
   counterpart motion、correction depth 和是否仍几何重叠固化成行为锁。
2. **Position Re-Evaluation Slice**：只对已存在的 support candidate 做局部 position contact
   re-evaluation 或 retain/drop 决策，不合成任意 speculative contact，不改变 velocity solver 顺序。
3. **Dynamic Support Validity**：当 counterpart motion 过大时，必须证明保留支撑不会注入能量；当
   correction 仍可解释为有效支撑时，必须避免过早断行导致 body 进入 long support gap。

E4c 的第一版验收仍按小到大推进：targeted support-gap behavior lock、`stack_4`、
180-frame `matrix_stack`、600-frame observation，最后才是 `PICEA_MATRIX_STACK_E4_ACCEPTANCE=1`
hard gate。

最新 240-frame E4c 红锁把 frame `180` pre-ejection support gap 的 retain/drop 条件进一步
钉住：frame `179` 的最后动态支撑仍有浅几何重叠（depth `0.001882`）且 local-anchor drift 为
`0/0`，但 row 已经 separating（post normal speed `0.229571`），没有 position correction，
并且 counterpart 不是 quiet support：counterpart speed 为 `1.509143/3.746651`，counterpart
tangent components 为 `0.536331/1.113765`。因此第一版 E4c 不能把“局部 anchor 连续”直接
解释成“应该保留支撑”。position-row re-evaluation 必须保留 counterpart-motion reject gate；
否则会把一个已经在动态能量交换中的支撑对误判为静止支撑，重走 naive retained-support 的
ejection 风险。

upstream source row 的补充事实进一步说明，这条高运动 counterpart 不是由 warm-start hit 或
velocity bias 直接推出去的。frame `177` 的 source contact 是 `MissFeatureId` 冷启动，source
normal impulse 只有 `0.000862`，tangent impulse `0.000689`，velocity-level position bias 和
support friction 都是 `0`，但 residual position correction depth 累计到 `0.067205`。同一
source row 的上一帧同 pair contact 仍然局部连续：point drift `0.000697`，normal dot
`0.997916`，local-anchor drift `0.005827`，但 edge-swap candidate 为 `0`。一次把
identity-only lifecycle 扩到这种非 edge-swap source row 的实验只改变 final hash，没有改善
frame `180` support gap 或 frame `214` ejection。因此下一步不应继续扩大 lifecycle identity；
更应把 source-contact 的 local-anchor continuity 与 residual position correction 的作用条件
绑定起来，而不是继续扩大 velocity bias、增加 friction budget，或在 gap 前一帧单点 retain
support。随后一次 “local-continuity source-row residual correction scale 0.5” 实验也失败：
它消除了 180-frame late support gap，但把 penetration 退到 `0.051679`、quiet angular spike
退到 `5.543327`，并把 correction pressure 转移到更晚 frame。由此可见，不能只降低法向
correction；必须让 position-row 几何有效性和 tangent/rotation 能量约束一起进入设计。

由此，E4c 的实际实现顺序应调整为：

1. 先把 ejection support-gap candidate facts 保持在报告与行为锁中，作为 retain/drop 的输入。
2. 对 quiet counterpart candidate 做局部 position-level re-evaluation；当前 frame `179` 这种
   high-counterpart-motion case 必须拒绝直接 retain。
3. 对 high-counterpart-motion case，优先调查上游 energy propagation / row coupling，而不是
   在 support gap 前一帧合成 correction 或增大摩擦预算。
4. 对 manifold-level solve 保持 dense/non-dense 分界：两点 sliding face contact 可以使用窄
   block normal solve 来避免逐点 impulse 失衡和假旋转，但 dense `matrix_stack` 中同样的局部
   block solve 会改变压力链传播，并把 retained support gap 从 frame `171` 提前到 frame `138`。
   因此 dense graph 仍必须走单独的 pressure propagation / position-row 设计，不能复用小场景
   block solve 作为全局稳定性方案。
5. 对 frame `180` ejection support gap 继续前移根因窗口：新增 upstream trace 显示，最后支撑
   counterpart `BodyHandle(33)` 在 frame `177` 已有 speed `1.229398/3.924366`，并且仍通过
   source contact 连接 `BodyHandle(25)`；该 source contact 的 depth 为 `0.007948`，normal/tangent
   impulse 为 `0.000862/0.000689`，position-correction depth/translation 为
   `0.067205/0.002688`。这说明最后支撑不是 isolated retain/drop 问题，而是 dense correction
   和上游动态接触链共同推动出的 pressure propagation 问题。
6. retain/drop oracle 必须先看 counterpart motion，再看 body-side tangent risk。否则像
   frame `192..200` 这种 body-side angular tangent 很低、但 counterpart linear/tangent motion
   很高的阶段会被误标为 `observe_tangent_risk_low`；修正后这些帧应归入
   `reject_counterpart_motion`，只有 counterpart 也 quiet 的 candidate 才能进入 position-level
   re-evaluation。

## E4c Next Gate: Source-Row Constrained Position Row Design

E4c 的下一次实现必须从 source row 的 position-row 约束设计进入，而不是从
support-gap 前一帧的 retain/drop、warm-start fallback 或单个 correction scale 进入。当前证据
已经把失败链条拆成两层：

- gap 前最后支撑体不是 quiet support，必须被 counterpart-motion gate 拒绝直接保留；
- 更上游的 source row 虽然同 pair / local-anchor 连续，但不是 edge-swap candidate，且它的主要
  影响来自 residual position correction，而不是 warm-start hit、velocity bias、support friction
  或 normal impulse。

因此下一版 position-row 设计要同时回答两个问题：

1. source row 在 pseudo-position state 下是否仍是几何有效的支撑约束；
2. 该 row 的 correction 是否会把切向 / 旋转能量传到后续 dynamic support 链。

### 必须使用的 seam

- 继续使用已保留的 `ContactPositionRow` / `QueuedPositionTranslation` 路径。
- `ContactObservation::source_row_continuity_candidate` 已作为输入 seam 落地：它只标记
  `MissFeatureId` 且 same-pair / local-anchor 连续、但不属于 E4b edge-swap lifecycle 的 row；
  当前仅用于后续 position-row gating 输入，不改变 solver 数值行为。
- position iteration 内使用固定 contact feature、local anchors、support normal 和轻量 separation
  function 重新评估 separation；不能在 solver loop 中重跑 full narrowphase。
- source-row candidate 必须消费当前报告已固化的 facts：same-pair continuity、point drift、
  normal dot、local-anchor drift、warm-start reason、normal/tangent speed、position bias、
  support friction、normal/tangent impulse、correction depth / translation。
- artifact report 已落地第一层 source-row dry-run facts，用于在不改变 solver 的情况下区分：
  几何连续 candidate、residual correction pressure source、counterpart motion rejection、
  tangent energy rejection 和最终 dry-run decision。
- artifact report 也已在报告层复刻 dense residual position correction 的 deferred
  pseudo-translation 顺序；当前 240-frame 红锁中的 upstream source row 在 pseudo-state 下仍有
  `0.003264..0.003453` 的 residual depth，并且 20 次 position iteration 都落在 support skin
  内。
- 所有新增字段必须 additive，并保持 `matrix_stack_support_gap_re_evaluation_prevents_current_ejection_window`
  的 upstream trace 可比较。

### Source-row eligibility

source row 只有同时满足以下条件，才允许进入新的 position-row gating：

- dense contact graph；
- 当前 row 是现有 narrowphase contact，不合成 speculative contact；
- 上一帧同 collider pair 存在，normal dot 和 local-anchor drift 仍在 E4c evidence gate 内；
- row 不是 E4b edge-swap lifecycle candidate，避免重复回到 identity-only 路线；
- velocity-level position bias 和 support friction 没有主导该 row；
- residual correction depth 明显大于 normal impulse，说明它确实是 position-row pressure source；
- pseudo-state 下的 separation 仍在 support skin 内，且不会扩大 frame-level penetration；
- counterpart motion gate 与 tangent/rotation energy gate 都通过，不能只看法向几何连续。

当前红锁已经证明 upstream source row 满足前半段几何和 pressure 条件，但不满足
counterpart motion / tangent energy 条件。因此下一次真正改 solver 时，不能把 pseudo-depth
positive 简化为 “retain correction”；必须同时引入能量传播约束。

### 必须同时约束的能量指标

不能再只缩放法向 correction。下一版实现若降低、延迟或重分配 source-row correction，必须同时检查：

- blocker / counterpart 的 tangent linear + angular components；
- source row 到 downstream support counterpart 的速度传播；
- late angular spike；
- warm-start drop / churn 是否被推迟到后续 frame；
- correction max / total 是否只是从 frame `177` 转移到更晚 frame。

### 验收门

实现切片只有同时满足以下条件才算可保留：

- `stack_4` hard gate 通过；
- aligned `4x3` final hash / penetration / quiet-window 不退化；
- 180-frame `matrix_stack` final hash 若改变，必须有更好的稳定性指标解释；
- 180-frame `matrix_stack` 无 floor ejection，final outside floor bodies 保持 `0`；
- late support gap 不早于 retained baseline frame `171`，duration 不长于 `9`；
- penetration max 不高于 retained baseline `0.041296`；
- late quiet-window angular speed 不高于 retained baseline `4.568931`；
- E4c 240-frame ignored red lock 的 first floor exit 不能早于 frame `214`，若仍红，必须证明
  support gap / upstream trace 的根因窗口前移或指标改善；
- upstream trace 仍必须包含 source continuity、solver facts 和 correction facts，不能变成
  missing evidence。

### 明确停止的路线

- 不再调大 shallow support friction budget。
- 不再扩大 velocity-level position bias。
- 不再把非 edge-swap local-anchor continuity 扩成 identity-only lifecycle。
- 不再按 weak impulse 或 local-continuity source row 直接缩放法向 correction。
- 不再只按 source-row 两侧切向能量重分配单行 position correction。该方向能把 240-frame
  ejection 从 frame `214` 推迟到 `215`、降低局部 penetration，但会让 180-frame 正式 gate 的
  baseline story 失败，并把能量推迟到后续 velocity spike。
- 不在 support-gap 前一帧合成 retained support。
- 不在 dense graph 中复用 non-dense two-point block normal solve。

如果下一次实现不能同时满足 source-row 几何有效性和 tangent/rotation energy 约束，应暂停并把
E4c 升级为更完整的 dense pressure propagation / position solver 设计，而不是继续堆局部补丁。

### 2026-05-07 Box2D / Matter.js 对照复核

这次复核的结论是：Picea 当前已经具备 Box2D / Matter.js 稳定堆叠路线中的一部分能力，
包括 SAT + clipped manifold、feature-id contact persistence、warm-started sequential
impulse、Coulomb friction、resting restitution threshold、island solve 和 residual position
correction。`matrix_stack 8x6` 仍然不稳，不是因为缺少一个“打开稳定堆叠”的单开关，而是因为
dense contact graph 中 position-level correction 还没有完整的 pseudo-position / position-row
求解边界。

外部参考给出的共同原则如下：

- Box2D 的 PGS / sequential impulse 依赖 warm starting、accumulated impulse clamp 和固定
  iteration count；accumulated impulse 必须 clamp 总量而不是每次 delta，这是避免 contact jitter
  的核心。
- Box2D v3 进一步强调 sub-stepping / smaller step 往往比单纯加 iteration 更有效；但这改变了
  step cadence 和外力语义，不能作为本轮无脑默认值。
- Matter.js 的更新顺序明确拆出 position iterations 和 velocity iterations；position solve 有
  独立 `positionImpulse` / warming，而不是把 penetration 全部塞进 velocity bias。
- Box2D 文档还强调 contact points 在 step 开始时计算，使 solver 能在 body 移动前看到新接触；
  对 Picea 来说，下一步不能在 solver loop 中重跑 full narrowphase，但必须在 position row 内用
  local anchors / pseudo pose 重新评估 separation。

本轮实验证实两个“看起来像成熟引擎”的简单修法都不能保留：

- 在 dense graph 中直接启用 non-dense two-point block normal solve：`stack_4` 通过，但
  180-frame `matrix_stack` 的 late support gap 提前到 frame `138`，late angular spike 升到
  `6.305823`，违反当前 stress gate。
- 全局把 default velocity iterations 从 `10` 提到 `16`：大矩阵最终速度有所下降，但
  `stack_4` settled penetration 退化到 `0.051698446`，直接突破 D2 hard gate。

因此基础方案保持为三层推进，而不是继续调参：

1. **保留当前 velocity solver contract**：继续使用 PGS accumulated impulse、warm-start 和
   conservative friction；不提高默认 iteration，不在 dense graph 复用 non-dense block solve。
2. **拆出真正的 dense position-row pass**：沿 `ContactPositionRow` /
   `QueuedPositionTranslation` seam，增加 pseudo pose / pseudo translation state。每个 position
   iteration 重新评估 row separation，只写回 pose，不写 velocity，不重跑 full narrowphase。
3. **给 source-row pressure propagation 加 gate**：source-row candidate 进入 position row 前必须
   同时通过 geometry continuity、pseudo-state support skin、counterpart motion、tangent/rotation
   energy、frame-level correction budget。失败时保留 diagnostics，不合成 speculative support。

第一版实现门仍按小到大验收：`stack_4` hard gate、aligned `4x3` matrix、180-frame
`matrix_stack` stress gate、240-frame E4c support-gap red lock。只有四者同时不退化，才能把
E4c 从“诊断/方案”推进到“稳定性修复已完成”。
