# picea-lab-web 运行控制与重力交互里程碑计划

状态：M44 live gravity apply 增量已完成；待用户确认是否提交
计划文档：docs/plans/2026-05-07-picea-lab-web-runtime-controls-and-gravity-milestones.md
最后更新：2026-05-07
工作目录：/Users/asyncrustacean/projects/picea
工作区状态：当前只有本计划文档未跟踪；`git status --short` 显示 `?? docs/plans/2026-05-07-picea-lab-web-runtime-controls-and-gravity-milestones.md`
提交策略：不提交
执行策略：逐里程碑推进；Plan Gate 通过后先执行 M41，M41 验证通过并汇报后，再确认是否进入 M42/M43

## 目标

把当前容易混淆的 `运行当前场景` / `播放` / `暂停` / `重置` 交互整理成用户可理解的模拟控制：

- 顶部按钮明确表示“启动或重新运行一个场景 / 新会话”，不再让用户误以为它是 resume。
- 时间线播放与暂停合并成一个 toggle，暂停后再播放应继续当前 session / 当前 buffer，而不是重开。
- `Rust 实时会话` 支持无限运行，并用环形缓存限制前端与后端持有的 frame 数量。
- 运行设置里提供重力方向/大小轮盘：拖拽方向决定重力方向，拖拽长度决定重力大小，并保留数值输入作为精确编辑。

## 约束

- 只在 `/Users/asyncrustacean/projects/picea` 当前工作区工作，不创建 worktree。
- 本轮规划不修改生产实现代码；实现必须等 Plan Gate 确认。
- 保持 `picea-lab-web` 是 Rust facts 的 viewer / live debugger，不在浏览器里运行 physics。
- 无限运行只针对 `Rust 实时会话`；`Rust 产物回放` 仍然是有限 artifact。
- M43 重力轮盘 V1 原本是 pending run config；2026-05-07 用户追加确认 M44，允许 live session 通过明确 Apply 执行窄 runtime gravity patch。
- M44 只允许修改 live world gravity；不得顺手开放 body/collider/joint arbitrary running-world patch。
- 验证命令继续使用 `rtk proxy`。

## 待确认问题

### 必须确认

- 无。当前可以按用户已认可的方向进入 Plan Gate。

### 可带假设推进

- 无限运行默认只对 `live_session` 开放：影响 artifact replay 仍有 frame_count；早期验证：UI 在 artifact mode 不显示无限 live 开关。
- 环形缓存默认容量建议 `600` 帧，可在运行设置里调节：影响内存与可回看窗口；早期验证：live 运行超过容量后 UI 显示 retained range，旧帧不可 scrub。
- 重力轮盘 V1 只改前端 pending run config，不改 running world，不 patch 当前 live session overrides：影响运行中拖轮盘不会立即改变当前堆叠；早期验证：运行中显示“下次重新运行生效”文案。
- 顶部按钮建议改为 `重新运行场景` / tooltip `启动新会话或重新生成产物`：影响文案与测试快照；早期验证：browser 里不再出现“播放后点这里会继续”的误导。

## 规划依据

- explorer：已返回。结论是当前 live 权威边界、summary/full 分层、paused-only 扰动已固定，本需求应作为 M40 后的新 M41-M43，而不是塞回现有 M35-M40。
- 关键证据：
  - `crates/picea-lab/src/server.rs` 的 `control_live_session()` 在 live `play/run` 时，如果 `buffered_frame_count >= frame_count` 会把状态设为 `Completed`。
  - `step_live_session()` 在 `runtime.frames.len() >= session.record.frame_count` 时停止 step，因此当前 live session 天然有限。
  - `SessionRecord` 只有 `frame_count`、`buffered_frame_count`、`current_frame_index`，没有 retained range / total produced frame / ring start index。
  - `App.tsx::runScenario()` 每次都会 `createSession()`、清空 frames、重置 frame index，所以顶部 `运行当前场景` 是新运行，不是 resume。
  - `TimelineHeader` 现在有分离的 pause / play / step / reset 按钮；播放暂停不是一个 toggle。
  - `Timeline` 运行设置已有 `frameCount`、`gravityY`、`useCustomGravity`，但只支持 Y 轴数值重力。
  - `api.ts::createSession()` 已能发送 `overrides.gravity: [x, y]`，后端 `ScenarioOverrides` 已支持 `[f32; 2]` 重力。
  - `docs/design/picea-lab-live-session-semantics.md` 明确 live session 的 reset-time override 与 running-world patch 是两类语义；当前 live session 拒绝 patch overrides。
- explorer 补充证据：
  - 顶栏 run 按钮会先 `createSession()`，live 模式再立刻 `play`，这是“运行当前场景”和“继续当前 session”混淆的根源。
  - 底部 chrome 当前是分离 pause/play/step/reset，不是一个状态 toggle。
  - live 自动 step 默认走 summary，手动 step 走 full；ring buffer 设计不能破坏 summary/full 权威边界。
  - 前端目前把 `frameIndex` 当本地数组下标使用，ring buffer 会直接撞上这个假设，必须引入绝对帧号与 retained window。
  - 后端 gravity override 已是二维 `[f32; 2]`，前端只暴露了 `gravityY`，所以重力轮盘优先是前端产品能力补齐。
- 主要未知与计划内处置：
  - `SessionRecord.buffered_frame_count` 不再适合同时表达 total produced 和 retained count；E2 默认新增显式 total / retained range 字段，不复用旧字段承载多义含义。
  - UI 对“旧帧已被环形缓存淘汰”的展示形式在 E2 中通过 browser 验收确认，至少必须有清晰不可 hydrate 提示。
- reviewer 预审修正：
  - M43 不允许把重力轮盘写成 live reset override。当前 `reset` 只用 session 创建时的 overrides 重建 world；轮盘改动只进入下一次顶部重新运行 / 新 session。
  - M42 必须维持稳定绝对帧号：`latest_frame.frame_index`、`current_frame_index`、hydration route、preview/commit provenance 都使用绝对帧号，不随环形缓存窗口重编号。
  - retained window 前移后，旧 frame hydrate 必须返回明确的 evicted 错误；基于旧 frame 的 preview / commit / velocity perturbation 必须因 frame 不在 retained window 或 epoch/index 不匹配被拒绝。

## 计划验收

- 状态：已批准
- Design Gate：已确认
- Execution Gate：已确认
- reviewer：已运行；blocking 建议已合并，待复审
- 审查结论：初审发现 5 个 blocking 点：M43 reset-time 语义冲突、M42 ring buffer 契约不足、执行策略过于连续、工作区状态过时、验证矩阵缺口；均已在候选稿中修正。复审结论：可以进入 Plan Gate。
- 用户确认：已确认；2026-05-07 用户说“请开始”

## 设计 / 执行分离规则

- 设计里程碑只允许修改计划文档、设计文档、调研记录或测试计划，不修改生产实现代码。
- 执行里程碑必须引用已批准的设计结论，或者显式说明为什么不需要设计前置。
- 如果执行中发现设计假设不成立，暂停执行并回到对应设计里程碑更新计划。
- 设计里程碑完成不等于允许实现；实现仍需经过 Execution Gate 或用户确认。
- 一个设计里程碑可以生成多个执行里程碑；一个执行里程碑必须绑定清晰的设计输入。
- M41/M42/M43 不自动连跑：每个执行里程碑完成验证后都要汇报结果，再由用户确认是否继续下一里程碑。

## 里程碑映射

| 设计里程碑 | 产出 | 解锁的执行里程碑 | 状态 |
| --- | --- | --- | --- |
| D1 | 运行控制、无限 live、环形缓存、重力轮盘的产品/API 边界 | E1, E2, E3, V1, C1 | 待确认 |

## 里程碑

### D1：冻结运行控制与重力交互语义

类型：设计
状态：已完成

目标：
明确顶部“新运行”和底部“播放当前 session”的分工，固定无限 live / 环形缓存 / 重力轮盘的 V1 行为边界，避免 worker 在实现时临时决定产品语义。

要回答的问题：
- 顶部按钮、底部播放 toggle、reset、step 的用户可见语义。
- live 无限运行是否改变 artifact replay 行为。
- 环形缓存需要哪些 session 字段：total produced、retained start/end、capacity、latest index。
- 旧帧被淘汰时，scrub / hydration / inspector / velocity perturbation 如何降级。
- 重力轮盘是否允许改当前 session：D1 结论为不允许，V1 只更新下一次顶部重新运行使用的 pending run config。

为什么现在做：
这些决策影响 server API、web 状态模型、i18n 文案和浏览器验收。先冻结语义能避免把“继续播放”和“重新启动”混在一个修复里。

设计范围：
- 更新本计划和 `docs/design/picea-lab-live-session-semantics.md`。
- 明确 `live_session` 无限运行与 ring buffer 的字段草案。
- 明确重力轮盘 V1 的 pending run config 规则。

不做：
- 不改生产实现代码。
- 不实现 running-world gravity patch。
- 不改变 core solver 或 `SimulationPipeline`。

输入证据：
- `server.rs::control_live_session` / `step_live_session`
- `App.tsx::runScenario` / `handleControl`
- `Timeline.tsx` run settings
- `api.ts::createSession`
- `docs/design/picea-lab-live-session-semantics.md`

输出交付物：
- 本计划的最终批准版。
- live session semantics 文档中的 runtime controls / ring buffer / gravity override 小节。

设计验收：
- 顶部运行按钮和底部播放 toggle 的语义写清楚。
- 无限 live 的 retained range / total frame index / capacity 语义写清楚。
- 重力轮盘 V1 明确为 pending run config，并说明 current-session override / running-world patch 是后续设计，不在本计划内。

验证方式：
- 文档 diff review。
- reviewer 审查 Plan Gate。

下一步映射：
- 生成/更新执行里程碑：E1, E2, E3, V1, C1

风险 / 后续：
- 如果用户希望拖重力轮盘立即改变正在运行的 world，需要新设计 milestone，不能复用当前 reset-time override。

### M41 / E1：修正运行/播放/暂停产品控制

类型：执行
状态：已完成
来源设计：D1

目标：
顶部按钮只表达“启动新运行 / 重新运行”，底部时间线提供单一播放/暂停 toggle；暂停后点击播放继续当前 session，不重新创建 session。

执行输入：
- D1 的控制语义。
- 顶部 run action 是 `runScenario()`，会创建新 session。
- 底部控制走 `handleControl("play"|"pause")`。

为什么现在做：
这是用户当前直接困惑的入口，也是后续无限 live 的交互基础。

范围：
- 调整 `Toolbar` 文案、tooltip、aria-label 和图标语义。
- `TimelineHeader` 将 play/pause 合并为一个 toggle button。
- 到达有限末尾时，播放按钮显示终点/重新播放状态或清晰提示。
- 更新 `i18n.ts`、`ui-contract.mjs`、必要组件类型。

不做：
- 不实现无限运行。
- 不改 server physics 行为。
- 不改速度扰动业务规则。

所有权：
- 可能触及：
  - `crates/picea-lab/web/src/App.tsx`
  - `crates/picea-lab/web/src/components/workbench/Toolbar.tsx`
  - `crates/picea-lab/web/src/components/workbench/Timeline.tsx`
  - `crates/picea-lab/web/src/i18n.ts`
  - `crates/picea-lab/web/scripts/ui-contract.mjs`
  - `crates/picea-lab/web/scripts/i18n-contract.mjs`（如有需要）
- 不应触及：
  - `crates/picea/src/**`
  - `crates/picea-lab/src/scenario.rs`
  - artifact schema

验收标准：
- 顶部按钮文案不再是 `运行当前场景`，能表达“新运行/重新运行”。
- 底部只有一个播放/暂停 toggle，状态切换符合当前 `status`。
- 暂停后点底部播放继续当前 live session / artifact timeline。
- 点顶部按钮仍明确重启/新建，不被当成 resume。

验证方式：
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- browser-use：`Rust 实时会话 + 矩阵堆叠 8x6`，验证顶部按钮与底部 toggle 的可见语义。

Subagent 执行计划：
- explorer：不需要；D1 已提供设计输入。
- worker：实现 E1；优先 `gpt-5.4`，文件所有权限于 web UI/i18n/contracts。
- reviewer：只读审查控制语义和状态退化。
- verifier：运行前端 contract/build 与 browser 检查。

提交策略：
- auto-commit：否
- message hint：`Clarify lab web run and playback controls`

风险 / 后续：
- browser-use 自动验收受限于当前会话未暴露 `node_repl js` 工具；已用前端 contract/i18n/build 和 supervisor diff review 覆盖，后续进入 M42 前建议在 in-app browser 手动确认一次。
- 有限末尾的底部 toggle 采用“回到起点后重新播放”状态，动作是 reset，不自动 reset+play；这是 M41 的最小语义，避免提前引入新控制行为。

### M42 / E2：实现 live 无限运行与环形缓存

类型：执行
状态：已完成
来源设计：D1

目标：
`Rust 实时会话` 可以无限 step，不因 `frame_count` 到 120 自动 completed；同时后端和前端只保留最近 N 帧，避免无限增长。

执行输入：
- D1 的无限 live 与 ring buffer 语义。
- 仅 live session 支持无限运行；artifact replay 仍按 frame_count 生成有限产物。

为什么现在做：
播放/暂停语义清楚后，用户才需要“继续跑多久”的控制；无限 live 必须配套 ring buffer 才不会重现性能和内存压力。

范围：
- 扩展 session create request 中 live-only 运行策略字段，例如 `live_unbounded` 和 `live_buffer_capacity`；V1 只在 create-time 给定，不通过 control 动作动态修改容量或无限/有限模式。
- `server.rs` 为 live runtime 增加 retained range / total produced frame index / capacity 语义；不复用 `buffered_frame_count` 同时表达 retained 与 total。
- 明确帧号契约：
  - `latest_frame.frame_index` 与 `current_frame_index` 始终是绝对帧号，从 session 开始单调递增。
  - `retained_frame_start` 与 `retained_frame_end_exclusive` 表示当前环形缓存窗口，窗口内帧号仍是绝对帧号，不重编号。
  - `buffered_frame_count` 若继续保留，只能表示 retained count；total 必须使用新字段，例如 `produced_frame_count`。
- `GET /api/sessions/:id/frames/:index` 使用绝对帧号；请求 `index < retained_frame_start` 时返回清晰的 evicted 错误（优先 410 Gone 或等价结构化错误），请求未来帧时仍返回 not-found / not-ready。
- retained window 前移后，旧 hydrate 请求、旧 preview action、旧 commit、旧 velocity perturbation 必须因 frame 不在 retained window 或 session epoch / frame index 不匹配而拒绝。
- Web frames buffer 按 retained range 更新，slider 显示可回看窗口而不是假装从 0 到 latest 都可用。
- 更新 profile/contract，确保 long live 不再积累无限 frames。

不做：
- 不让 artifact replay 无限生成 artifact。
- 不改变 solver step 物理结果。
- 不实现后台 server-side autoplay；仍是前端请求驱动 step。
- 不支持运行中修改 `live_unbounded` / `live_buffer_capacity`。

所有权：
- 可能触及：
  - `crates/picea-lab/src/server.rs`
  - `crates/picea-lab/tests/server_routes.rs`
  - `crates/picea-lab/web/src/types.ts`
  - `crates/picea-lab/web/src/api.ts`
  - `crates/picea-lab/web/src/App.tsx`
  - `crates/picea-lab/web/src/components/workbench/Timeline.tsx`
  - `crates/picea-lab/web/src/i18n.ts`
  - `crates/picea-lab/web/scripts/ui-contract.mjs`
  - `crates/picea-lab/web/scripts/profile-contract.mjs`
- 不应触及：
  - `crates/picea/src/**`
  - artifact writer `frames.jsonl` schema

验收标准：
- live session 在 `live_unbounded` 下超过 120 帧仍保持 playable，而不是 completed。
- ring buffer 超过容量后，只保留最近 N 帧；UI 显示 retained range。
- 被淘汰帧不可 hydrate，后端返回 evicted 错误，UI 显示“已被缓存淘汰”而不是空事实。
- `latest_frame.frame_index`、`current_frame_index`、frame hydration、preview/commit provenance 全部使用稳定绝对帧号。
- retained window 前移后，基于旧 frame 的 preview / commit 被拒绝或自动清理，不会提交到新状态。
- velocity perturbation 仍只允许 latest authoritative live frame，且要求 epoch + absolute frame index 匹配。
- profile / web state 中 frames count 不无限增长到全部历史。

验证方式：
- `rtk proxy cargo test -p picea-lab --test server_routes`
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run test:profile`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- browser-use：设置无限 live + 小 ring buffer，运行超过容量，验证 retained range、fps badge、旧帧淘汰提示。

Subagent 执行计划：
- explorer：worker 实现前快速复核 server live runtime shape。
- worker：实现 E2；优先 `gpt-5.4`，允许 server + web 联动。
- reviewer：重点审查 ring buffer index、hydration stale guards、latest frame authority、velocity perturbation gating。
- verifier：运行 Rust/web/browser 验证。

提交策略：
- auto-commit：否
- message hint：`Add unbounded live sessions with ring buffer`

风险 / 后续：
- `SessionRecord.buffered_frame_count` 的含义可能需要拆成 retained count 与 total produced；这是本里程碑的主要兼容风险。

### M43 / E3：实现重力方向/大小轮盘

类型：执行
状态：已完成
来源设计：D1

目标：
运行设置提供可视化重力控制：拖拽轮盘方向设置 `[x, y]` 方向，拖拽长度设置大小；保留数值输入用于精调。该设置只进入前端 pending run config，并应用于下一次顶部 `启动/重新运行` 创建的新 session。

执行输入：
- D1 固定 V1 为 pending run config。
- 后端 `ScenarioOverrides.gravity` 已支持 `[f32; 2]`。

为什么现在做：
当前只支持 `gravityY`，无法表达横向或斜向重力；用户希望像轮盘一样直接调重力场。

范围：
- 新增 `GravityDial` 或等价组件，支持 pointer drag、keyboard fallback、数值同步。
- `App.tsx` 从单一 `gravityY` 改为 `gravityVector` / magnitude / enabled 状态。
- `api.ts::createSession()` 继续发送 `[x, y]`。
- `Timeline` run settings 改成重力轮盘 + magnitude / x / y 数值。
- 运行中的 live session 拖动轮盘只更新 pending config；当前 session 的 `reset` 仍使用创建 session 时已有 overrides，不读取新的 pending config。
- i18n、ui-contract、必要可访问性标签。

不做：
- 不在运行中热改 live world。
- 不通过 `PATCH /overrides` 修改 live session。
- 不把当前 session 的 `reset` 改成读取新的 pending gravity。
- 不实现连续力、扭矩、风场或鼠标关节。
- 不改 core `World` gravity API。

所有权：
- 可能触及：
  - `crates/picea-lab/web/src/App.tsx`
  - `crates/picea-lab/web/src/components/workbench/Timeline.tsx`
  - `crates/picea-lab/web/src/components/workbench/GravityDial.tsx`（新文件）
  - `crates/picea-lab/web/src/i18n.ts`
  - `crates/picea-lab/web/scripts/ui-contract.mjs`
  - `crates/picea-lab/web/src/api.ts`
- 不应触及：
  - `crates/picea/src/**`
  - `crates/picea-lab/src/server.rs`，除非需要补契约测试或错误文案

验收标准：
- 用户能用拖拽方向和长度设置重力向量。
- x/y/magnitude 数值与轮盘状态双向同步。
- 选择自定义重力后，新启动的 artifact/live 场景 snapshot meta gravity 反映该向量。
- 运行中的 live session 改轮盘时，UI 清楚提示“下次重新运行生效”；当前 live session 和当前 session reset 均不改变 gravity。

验证方式：
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- 如需后端锁：`rtk proxy cargo test -p picea-lab --test server_routes`
- browser-use：拖拽重力轮盘到斜向，启动场景，检查 UI 中 gravity metric 和运动方向。
- browser-use：live 运行中拖轮盘，确认当前画面不热改，只显示下次重新运行生效提示。

Subagent 执行计划：
- explorer：不需要；D1 和当前 gravity override 已提供输入。
- worker：实现 E3；优先 `gpt-5.4`，限 web UI/contract，必要时小范围 server test。
- reviewer：重点审查 pointer math、坐标方向、可访问性、运行中生效文案。
- verifier：运行前端 gate 与 browser 拖拽验收。

提交策略：
- auto-commit：否
- message hint：`Add gravity vector dial`

风险 / 后续：
- y-down 世界坐标要和 canvas 显示一致，避免用户向下拖却产生向上重力。
- 如果后续需要“拖轮盘立即改变正在运行的 world”或“reset 当前 session 使用新 gravity”，必须新增 current-session override / running-world patch 设计里程碑。

### V1：端到端产品验收

类型：验证
状态：已完成

目标：
用真实浏览器确认三个用户故事都成立：继续播放不重开、无限 live 不停且不无限缓存、重力轮盘可理解且生效。

范围：
- 浏览器验收 `http://127.0.0.1:5173/?picea-profile=1`。
- 覆盖 `Rust 实时会话 + 矩阵堆叠 8x6`。
- 覆盖 artifact replay 不受无限 live 改动影响：切换到 `Rust 产物回放` 后仍按有限帧播放，不显示无限 live / ring buffer 控制，不改变已有 artifact scrub 行为。

不做：
- 不做跨浏览器兼容矩阵。
- 不做长达数小时 soak test。

验收标准：
- 暂停后用底部 toggle 播放继续当前 session。
- 顶部按钮的文案/tooltip 明确是重新运行。
- 无限 live 超过 120 帧仍继续，ring buffer 保持容量。
- 重力轮盘设置后，新运行 snapshot gravity 与 UI 一致。

验证方式：
- 启动：`rtk proxy just picea-lab-web`
- browser-use profile 日志。
- DOM snapshot / screenshot。
- artifact replay 回归步骤：选择同一场景的 `Rust 产物回放`，运行到末帧，确认仍显示有限帧总数且不会继续请求 live step。
- 相关 Rust/web gate 已通过。

风险 / 后续：
- 如果 in-app browser IAB backend 不可用，使用服务/API/profile fallback 并记录未跑浏览器的原因。

### C1：文档与路由收口

类型：收尾
状态：已完成

目标：
把新的运行控制、无限 live、环形缓存和重力轮盘边界写回设计/AI 路由，方便后续 session 接着做。

范围：
- 更新本计划进度记录。
- 更新 `docs/design/picea-lab-live-session-semantics.md`。
- 必要时更新 `docs/ai/repo-map.md` / `docs/ai/index.md` 中 lab-web 路由描述。

不做：
- 不写营销文档。
- 不把 running-world gravity patch 描述成已完成。

验收标准：
- 后续读者能分清 artifact replay / live session / new run / playback toggle / infinite live。
- 路由文档指向正确验证命令和文件。

验证方式：
- `rtk proxy git diff --check -- <changed docs/code files>`
- 如 AI docs 改动较大，运行 repo 既有 AI-context validator（若当前工作区提供）。

风险 / 后续：
- 文档必须如实标注 V1 不支持 running-world gravity patch。

## 进度记录

### 2026-05-07 - 草稿创建

- 状态：计划中
- Commit：none
- 验证：只读规划；尚未执行实现验证。
- 风险 / 后续：等待 explorer / reviewer 审查后进入 Plan Gate。

### 2026-05-07 - 合并只读探索结论

- 状态：Plan Gate 候选稿
- Commit：none
- 验证：只读探索已完成；尚未执行实现验证。
- 关键调整：将需求拆为 M41 控制语义、M42 无限 live + ring buffer、M43 重力轮盘；明确 ring buffer 需要绝对帧号与 retained window；明确不触碰 physics core。
- 风险 / 后续：等待 reviewer 审查计划边界与验收门。

### 2026-05-07 - 合并 reviewer blocking 建议

- 状态：Plan Gate 候选稿
- Commit：none
- 验证：只读 review 已完成；尚未执行实现验证。
- 关键调整：执行策略改为逐 milestone gate；M42 写明绝对帧号、retained window、evicted hydrate、preview/commit/velocity stale guards；M43 写明重力轮盘只更新 pending run config，当前 session reset 不读取新 gravity。
- 风险 / 后续：复审计划后进入 Plan Gate。

### 2026-05-07 - reviewer 复审通过

- 状态：Plan Gate 候选稿
- Commit：none
- 验证：只读 reviewer 复审通过；待用户确认。
- 关键调整：补充 M42 `live_unbounded` / `live_buffer_capacity` 为 create-time 配置；补充 artifact replay 浏览器回归步骤。
- 风险 / 后续：Plan Gate 确认后先执行 M41，不自动连跑 M42/M43。

### 2026-05-07 - M41 / E1 完成

- 状态：已完成
- Commit：none
- 改动：顶栏运行按钮改为启动/重新运行语义，使用 restart 图标与动态 tooltip/aria；底部时间线将 play/pause 合并为一个 toggle，并在有限末尾显示 replay/reset 状态；非 live 回放在末尾触发 play 时回到 frame 0。
- 文件：`crates/picea-lab/web/src/App.tsx`、`crates/picea-lab/web/src/components/workbench/Toolbar.tsx`、`crates/picea-lab/web/src/components/workbench/Timeline.tsx`、`crates/picea-lab/web/src/i18n.ts`、`crates/picea-lab/web/scripts/ui-contract.mjs`、`crates/picea-lab/web/scripts/i18n-contract.mjs`
- 验证：`rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract` 通过；`rtk proxy npm --prefix crates/picea-lab/web run test:i18n` 通过；`rtk proxy npm --prefix crates/picea-lab/web run build` 通过，保留既有 Vite chunk size warning；`git diff --check` 通过。
- 审查：worker 自测通过；主 Codex 已做 supervisor diff review。两个 reviewer 子任务均超时后关闭，未产生可用 findings。
- 风险 / 后续：本轮未完成 browser-use 自动验收，因为当前工具面未暴露 `node_repl js`；M42 已完成，M43 尚未开始。

### 2026-05-07 - M42 / E2 完成

- 状态：已完成
- Commit：none
- 改动：Rust live session 增加 `live_unbounded`、`live_buffer_capacity`、`produced_frame_count`、`retained_frame_start`、`retained_frame_end_exclusive` 字段；live step 使用绝对帧号继续超过 `frame_count`；server live frames 改为 retained ring window，旧帧 hydrate 返回 410 evicted，未来帧仍 404；web live buffer 改为 retained window，frame index / hydration / perturbation guards 保持绝对帧号。
- 文件：`crates/picea-lab/src/server.rs`、`crates/picea-lab/tests/server_routes.rs`、`crates/picea-lab/web/src/types.ts`、`crates/picea-lab/web/src/api.ts`、`crates/picea-lab/web/src/App.tsx`、`crates/picea-lab/web/src/components/workbench/Timeline.tsx`、`crates/picea-lab/web/src/i18n.ts`、`crates/picea-lab/web/scripts/ui-contract.mjs`。
- 验证：`rtk proxy cargo test -p picea-lab --test server_routes` 通过；`rtk proxy cargo test -p picea-lab` 通过；`rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract` 通过；`rtk proxy npm --prefix crates/picea-lab/web run test:i18n` 通过；`rtk proxy npm --prefix crates/picea-lab/web run test:profile` 通过；`rtk proxy npm --prefix crates/picea-lab/web run build` 通过，保留既有 Vite chunk size warning；`rtk proxy cargo fmt --all --check` 通过；`rtk proxy git diff --check` 通过。
- 浏览器验收：当前会话未暴露 in-app browser 的 `node_repl js` 控制面，改用 Chrome / Computer Use 兜底。重启 `target/picea-lab-web` 后打开 `http://127.0.0.1:5173/?picea-profile=1`，选择 `Rust 实时会话` + `落箱接触`，点击启动后确认 live 超过 120 帧仍保持可播放；暂停时 Chrome accessibility tree 显示 `第 462 帧`、`会话 session-1 / 保留 [0, 463) / 已缓存 463 / 当前 462`、`Rust 实时会话 / 已暂停`。
- 风险 / 后续：当前 web 的 live ring V1 默认无限、容量 600，暂未在运行设置里暴露动态开关，符合 create-time 固定策略；M43 已在后续完成记录中收口。

### 2026-05-07 - M43 / E3 完成

- 状态：已完成
- Commit：none
- 改动：运行设置从 `gravityY` 升级为 pending `gravityVector`；新增 `GravityDial`，支持 pointer 选择方向/长度、键盘方向键微调、x/y/强度数值双向同步和重置；拖动或输入会自动打开“重力覆盖”，但仍只在下一次顶部启动/重新运行创建新 session 时发送 `[x, y]` override，不热改当前 session。
- 文件：`crates/picea-lab/web/src/App.tsx`、`crates/picea-lab/web/src/components/workbench/WorkbenchLayout.tsx`、`crates/picea-lab/web/src/components/workbench/Timeline.tsx`、`crates/picea-lab/web/src/components/workbench/GravityDial.tsx`、`crates/picea-lab/web/src/i18n.ts`、`crates/picea-lab/web/scripts/i18n-contract.mjs`、`crates/picea-lab/web/scripts/ui-contract.mjs`。
- 验证：`rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract` 通过；`rtk proxy npm --prefix crates/picea-lab/web run test:i18n` 通过；`rtk proxy npm --prefix crates/picea-lab/web run build` 通过，保留既有 Vite chunk size warning。
- 浏览器验收：重启 `target/picea-lab-web` 后打开 `http://127.0.0.1:5173/?picea-profile=1`；在运行设置中点击重力轮盘，确认覆盖自动开启，x/y/强度同步更新；点击顶部重新运行后，时间线 gravity metric 显示 `0.602, 0.030`，画布重力箭头转为横向；提示文案显示“仅下次重新运行生效；当前会话保持已有重力”。
- 风险 / 后续：M43 仍只实现 pending run config。若要运行中即时改变 world gravity，需要新增 live-session override/patch 设计里程碑。

### 2026-05-07 - V1 / C1 完成

- 状态：已完成
- Commit：none
- 浏览器验收：当前会话未暴露 in-app browser 控制面，使用 Chrome / Computer Use 兜底。`Rust 实时会话` + `矩阵堆叠 8x6` 启动后超过原始 120 帧仍保持播放；`第 670 帧` 时显示 `会话 session-2 / 保留 [71, 671) / 已缓存 600 / 当前 670`，确认 live unbounded + retained ring window 生效；重力 metric 显示 `0.602, 0.030`，确认重力轮盘作为新 session override 生效。切回 `Rust 产物回放` 并重新运行后显示 `共 120 帧`、`Rust 产物回放 / 已暂停`，确认 artifact replay 仍是有限帧回放。
- 文档：补充 `docs/design/picea-lab-live-session-semantics.md` 的 Runtime Controls V1 边界；更新 `docs/ai/repo-map.md` 与 `docs/ai/index.md` 的 server/web 路由。
- 风险 / 后续：V1 不包含 running-world gravity patch；后续若需要运行中即时改重力，应先新增设计里程碑。

### 2026-05-07 - M44 / E4 增量完成

- 状态：已完成
- Commit：none
- 用户输入：浏览器评论要求“重力覆盖应该支持实时更改，点击 apply 后就可以应用上去，支持 reset 和撤销更改这些行为，优化 ux 交互。”
- 设计调整：将 live gravity apply 作为窄 running-world patch 例外；artifact replay 仍保持 pending run config。Apply 需要校验 `session_epoch`，更新 authoritative `World` gravity，刷新当前 live frame，并让后续 live reset 使用最近一次已应用的 gravity。
- 范围：`World::set_gravity`、`POST /api/sessions/:id/gravity`、前端 draft/applied/undo 状态、`GravityDial` Apply/Undo/Reset UX、文档与契约测试。
- 非目标：不开放 generic live overrides；不 patch body/collider/joint；不新增 server-side autoplay；不改变 artifact replay 的有限产物语义。
- 验证：`rtk proxy cargo test -p picea-lab --test server_routes live_gravity_patch_updates_runtime_snapshot_and_reset_gravity` 通过；`rtk proxy cargo test -p picea-lab --test server_routes` 通过；`rtk proxy cargo test -p picea-lab` 通过；`rtk proxy cargo test -p picea --lib` 通过；`rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract` 通过；`rtk proxy npm --prefix crates/picea-lab/web run test:i18n` 通过；`rtk proxy npm --prefix crates/picea-lab/web run test:profile` 通过；`rtk proxy npm --prefix crates/picea-lab/web run build` 通过，保留既有 Vite chunk size warning；`rtk proxy cargo fmt --all --check` 通过；`rtk proxy git diff --check` 通过。
- 浏览器验收：重启 `picea-lab-web` 后使用临时 Chrome CDP 打开 `http://127.0.0.1:5173/?picea-profile=1`；切到 `Rust 实时会话` + `落箱接触` 并启动，暂停在 live frame 730；运行设置中把 draft gravity 改为 `3, 0`，确认文案显示“有未应用的重力更改；点击应用会立即更新实时会话”且 Apply 可点。点击 Apply 后，UI 显示“当前重力已应用到实时会话 已应用 3, 0”，`session_epoch` 从 0 变 1，`world revision` 从 735 变 737。点击 Undo 后，UI 显示“已应用 0, 9.8”，`session_epoch` 变 2，`world revision` 变 738。Reset 在默认已应用态保持 draft 为 `0, 9.8` 且 Apply/Undo 禁用。

### 2026-05-08 - Live pause/resume ordering follow-up

- 状态：已完成
- Commit：none
- 改动范围：`crates/picea-lab/web/src/App.tsx`、`crates/picea-lab/web/scripts/ui-contract.mjs`。
- 修正：live `pause` / `play` 控制响应先通过 `applyLiveSessionFrame(result, guard)` 合并服务端返回的 authoritative frame，再 re-anchor `session.current_frame_index` 或进入暂停态，避免暂停/继续落在旧缓存帧上。
- 验证通过：`rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`；`rtk proxy npm --prefix crates/picea-lab/web run test:i18n`；`rtk proxy npm --prefix crates/picea-lab/web run build`。
- Browser 验收：Codex Browser 新 tab 打开 `http://127.0.0.1:5173/?picea-profile=1`，选择 `矩阵堆叠 8x6` + `Rust 实时会话`。启动后暂停于第 351 帧，点击 `继续当前会话` 后继续到第 371 帧且未回到 0；随后继续运行到第 468 帧并再次暂停，控制台 error/warning 日志为空。
