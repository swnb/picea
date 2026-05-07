# picea-lab-web 实时模拟性能优化里程碑计划

状态：进行中
计划文档：docs/plans/2026-05-07-picea-lab-web-live-performance-milestones.md
最后更新：2026-05-07
工作目录：/Users/asyncrustacean/projects/picea
工作区状态：dirty；当前工作区已有 profile 埋点、matrix stack 稳定性、lab/server/core 等多处未提交改动，且本计划会触碰部分已 dirty 的 server/web 文件。执行前必须确认这些改动是否视为本计划 baseline，不得覆盖或吸收无关 WIP。
提交策略：不提交
执行策略：完整计划批准后连续执行

## 目标

把 `picea-lab-web` 的 `Rust 实时会话` 从“大矩阵场景下试图用完整 debug frame 跑 30Hz”调整为可解释、可降级、可按需取完整细节的实时调试路径。最终用户应该能在 `matrix_stack 8x6` 下看到最新 authoritative live frame，页面不被完整 payload 和全量 timeline 派生拖死，并且 UI 能诚实显示 degraded realtime，而不是暗示已经稳定 30Hz。

## 约束

- 保持 Rust `World + SimulationPipeline` 为 authoritative source；Web 不重新计算 physics、broadphase、island、contacts、solver 或 diagnostics truth。
- `artifact_replay` 与 `frames.jsonl` 的完整 `FrameRecord` 路径保持兼容，不以 live 性能优化破坏 artifact schema。
- `FrameDiagnostics` 的 source label 与 missing evidence 语义必须保留；旧 artifact 不能被渲染成零风险或零成本。
- 本计划只处理 live realtime 产品性能，不修 contact solver / sleep / matrix_stack 稳定性 correctness。
- 当前 dirty 工作区里已有 solver/stability WIP，执行里程碑不得触碰 `crates/picea/src/*`，除非先修订计划。
- 所有 cargo/npm/just 验证命令必须通过 `rtk proxy` 运行。
- 浏览器验收优先使用 `browser-use:browser` 打开 `http://127.0.0.1:5173/?picea-profile=1`。

## 待确认问题

### 必须确认

- 当前 dirty 的 server/web/profile 相关改动是否视为本计划 baseline，允许在其上继续；还是需要先隔离、冻结或摘出无关 WIP 后再执行？
- 如果进入执行，是否批准按 `LP-E1 -> LP-E2 -> LP-E3 -> D2 -> LP-E4 -> V1 -> C1` 连续推进，且继续保持“不提交”的默认策略？

### 可带假设推进

- 假设 `profile` 埋点作为本轮前置证据保留，并纳入后续验证面：影响 `test:profile` 与浏览器验收；早期验证是 `rtk proxy npm --prefix crates/picea-lab/web run test:profile`。
- 假设 live 性能优化先以 `matrix_stack 8x6` 为 stress case，小场景作为 regression sanity：影响验收样本；早期验证是同时记录 `falling_box_contact` 或 `stack_4` 的 live payload / latency。
- 假设 live response 可以新增 summary/full 分层，但不改变 artifact `FrameRecord`：影响 server/web API 类型；早期验证是 `server_routes` 覆盖旧 full 行为与新 summary 行为。

## 规划依据

- explorer：已运行。结论是当前 profile 与既有设计文档一致，主瓶颈在 `api.json` / response payload / `timeline.derive`，不是 `canvas.drawWorld` 或 `live.applyFrame`。
- 关键证据：
  - 浏览器 profile：`matrix_stack 8x6` / `Rust 实时会话` 下 `api.json` avg 约 `89ms`、max 约 `189ms`，单次 live step payload 约 `821KB`。
  - 浏览器 profile：`timeline.derive` 随 live buffer 增长，后段约 `60ms`；`canvas.drawWorld` avg 约 `0.6ms`、max 约 `4.8ms`；`live.applyFrame` 约 `0.02ms`。
  - `crates/picea-lab/src/server.rs` 当前 `SessionRecord.latest_frame` 是完整 `FrameRecord`，`step_live_session` 每步生成完整 frame 后克隆进 `runtime.frames` 和 `latest_frame`。
  - `crates/picea-lab/src/artifact.rs` 当前 `FrameRecord` 同时包含 `report` 与顶层 `events`，其中 `events: report.events.clone()` 对 dense contact 场景会放大 payload。
  - `docs/design/matrix-stack-stability-optimization-design.md` 已明确 live performance 应拆成 lightweight live frame / 按需 full frame，不应混入 solver 修复。
  - `docs/design/matrix-stack-stability-acceptance.md` 已明确 `matrix_stack 8x6` 第一阶段不是强行 30Hz，而是 degraded realtime 与响应耗时/响应体/long task 记录。
- 主要未知：
  - summary frame 的最小字段集合需要在 D1 冻结，避免前端 inspector/diagnostics 打开后缺 authoritative handle/revision。
  - timeline 增量缓存必须证明与当前全量 marker 派生等价，否则会出现“快但诊断错”。
  - adaptive cadence 的 UI 文案需要避免误导用户，以“实际后端步进速率 / 目标速率”表达。

## 计划验收

- 状态：已批准
- Design Gate：已确认
- Execution Gate：已确认
- reviewer：已运行
- 审查结论：发现 1 个 Plan Gate blocker、3 个执行风险、1 个验收复现性问题；已采纳到当前计划，等待用户确认 dirty baseline 与执行授权。
- 用户确认：已确认，2026-05-07

## 设计 / 执行分离规则

- 设计里程碑只允许修改计划文档、设计文档、调研记录或测试计划，不修改生产实现代码。
- 执行里程碑必须引用已批准的设计结论，或者显式说明为什么不需要设计前置。
- 如果执行中发现 summary/full 分层无法保持 artifact/live 兼容，暂停执行并回到 D1 更新设计。
- 设计里程碑完成不等于允许实现；实现仍需经过 Execution Gate 或用户确认。
- 本计划中的执行里程碑必须通过 `subagent-current-workspace` 执行，主 Codex 只负责监督、集成、验证与计划更新。

## 里程碑映射

| 设计里程碑 | 产出 | 解锁的执行里程碑 | 状态 |
| --- | --- | --- | --- |
| D1 | live summary/full response contract、字段集合、按需 full frame 策略、degraded realtime 文案边界 | LP-E1, LP-E2, LP-E3 | 已完成 |
| D2 | timeline marker 增量缓存等价性测试策略、reset/scrub/locale/selection/summary hydrate 失效规则 | LP-E4 | 待确认 |

## 里程碑

### D1：冻结 live frame 分层协议

类型：设计
状态：已完成

目标：
明确 `Rust 实时会话` 播放路径的 summary/full 数据分层，后续执行可以在不破坏 artifact `FrameRecord` 的前提下降低 live step payload。

要回答的问题：
- `LiveFrameSummary` 至少需要哪些字段才能支持 canvas、状态条、selection handle、session epoch、world revision、degraded realtime 文案？
- `control step` 的默认 detail 应该是什么？手动单步、暂停、inspector/diagnostics 打开时如何获取 full frame？
- 顶层 `events` 与 `report.events` 的重复如何处理：live summary 直接不带，还是 full frame 保持现状只在后续 artifact schema 计划处理？
- `GET /api/sessions/:id/frames/:index` 是否作为按需 full frame 的最小新增端点？

为什么现在做：
当前 profile 已证明 payload 是第一瓶颈；如果不先冻结协议，worker 很容易在 `SessionRecord` / `FrameRecord` / Web 状态之间做隐式兼容改动，造成难以审查的双端 diff。

设计范围：
- `docs/design/matrix-stack-stability-optimization-design.md`
- 本计划文档
- 必要时 `docs/design/picea-lab-live-session-semantics.md`

不做：
- 不改生产实现代码
- 不改 `FrameRecord` artifact schema
- 不改 solver 或 matrix_stack 稳定性逻辑

输入证据：
- `crates/picea-lab/src/server.rs` 的 `SessionRecord.latest_frame` 与 `step_live_session`
- `crates/picea-lab/src/artifact.rs` 的 `FrameRecord`
- `crates/picea-lab/web/src/api.ts` 与 `App.tsx` 的 live step 消费路径
- 浏览器 profile 中 `api.json` payload/latency 数据

输出交付物：
- summary/full 字段表：已写入 `docs/design/matrix-stack-stability-optimization-design.md`
- 新/旧 endpoint 与 query/detail 策略：已写入 `docs/design/picea-lab-live-session-semantics.md`
- full frame 按需触发点：已写入 live frame detail contract
- D1 决策更新到本计划进度记录

设计验收：
- artifact replay 与 live playback 数据路径边界清晰
- summary frame 足够支持 canvas 与状态条，但不携带 diagnostics/full inspector payload
- full frame 获取路径清楚，并且能保持 velocity perturbation 的 authoritative handle/revision 校验

验证方式：
- 文档审查
- reviewer 审查

下一步映射：
- 生成/更新执行里程碑：LP-E1, LP-E2, LP-E3

风险 / 后续：
- 如果 full frame endpoint 需要新的授权/缓存语义，本计划暂停并补 D1。

### LP-E1：锁定 live payload 基线与协议 contract

类型：执行
状态：已完成
来源设计：D1

目标：
用 tests/scripts 锁住当前 profile 与 payload 观测方式，并为 summary/full server API 建立可执行的 contract 准备。这个里程碑不留下失败测试，summary/full 的红绿实现测试放到 LP-E2 内先写后过。

执行输入：
- D1 的 summary/full contract
- 现有 `profile.ts`、`profile-contract.mjs`
- 当前 browser profile 数字作为 baseline，不作为 hard correctness threshold

为什么现在做：
先建立协议与 profile contract，避免 LP-E2 改 payload 时只凭肉眼或浏览器手感判断。

范围：
- 补/调整 `crates/picea-lab/tests/server_routes.rs`，锁住当前 live route 行为和后续 summary/full contract 的非破坏性准备；不得提交预期失败的测试。
- 补/调整 `crates/picea-lab/web/scripts/profile-contract.mjs`，保证 `api.json`、`timeline.derive`、`canvas.drawWorld`、`live.applyFrame` hook 继续存在。
- 必要时增加一个轻量 Node profile smoke 脚本，但不做复杂 benchmark gate。

不做：
- 不实现 summary payload 优化
- 不提交需要 LP-E2 才能通过的失败测试
- 不改 Timeline 增量逻辑
- 不改 solver / artifact runner correctness

所有权：
- 可能触及：
  - `crates/picea-lab/tests/server_routes.rs`
  - `crates/picea-lab/web/scripts/profile-contract.mjs`
  - `crates/picea-lab/web/package.json`
  - 本计划文档
- 不应触及：
  - `crates/picea/src/*`
  - `crates/picea-lab/src/artifact.rs`，除非 D1 要求类型导出辅助
  - `crates/picea-lab/web/src/components/workbench/*`，除非只是 profile contract 入口名同步

验收标准：
- server route/profile contract 能为 summary/full 改造提供基线，且本里程碑结束时所有新增/调整测试必须通过。
- profile contract 能锁住性能观察入口。
- 不引入 visible profile chrome。

本次执行记录（2026-05-07）：
- `server_routes` 现已显式锁住 live `step` 在 LP-E2 前的默认响应仍返回 current full `FrameRecord` 负载；测试会检查 `snapshot` / `report` / `events` / `diagnostics` 字段仍在。`detail=summary|full` 的解析与兼容测试放在 LP-E2 内先写后实现。
- `server_routes` 现已显式记录 `GET /api/sessions/:id/frames/:index` 在 LP-E1 阶段尚未暴露，并要求该探测不改变 live session 状态。
- `profile-contract.mjs` 现已锁住 `controlSession()` 必须继续经由 `requestJson` / `profileJsonRequest`，并要求 `api.json` 观测方法保留 `response.text()`、`JSON.parse(text)`、`bytes`、`readTextMs`、`parseJsonMs`，避免 LP-E2 以后 live step 只剩粗粒度 request timing。

验证方式：
- `rtk proxy cargo test -p picea-lab --test server_routes`
- `rtk proxy npm --prefix crates/picea-lab/web run test:profile`
- `rtk proxy npm --prefix crates/picea-lab/web run build`

Subagent 执行计划：
- explorer：需要；确认 server route 测试现有模式与 profile contract 当前状态。
- worker：`gpt-5.4`；只写测试/contract，不写优化实现。
- reviewer：审查测试是否真的覆盖协议差异、是否误把 wall-clock 变成 hard fail。
- verifier：运行上述命令并报告是否修改生成文件。

提交策略：
- auto-commit：否
- message hint：`Plan live performance payload gates`

风险 / 后续：
- 当前工作区已有 profile 改动，worker 必须只在已授权文件内增量修改。

### LP-E2：实现 live summary payload 与按需 full frame

类型：执行
状态：已完成
来源设计：D1

目标：
让 live playback 默认消费轻量 summary response，大矩阵场景下 live step response bytes 明显下降；full `FrameRecord` 仍可按需获取，用于 inspector、diagnostics、copy debug context、paused review。

执行输入：
- D1 字段表
- LP-E1 server/API contract

为什么现在做：
这是最大瓶颈的根因修复，收益覆盖 Rust serialize、HTTP transport、browser readText/parse、React state append。

范围：
- 在 `crates/picea-lab/src/server.rs` 增加 live summary response 类型或 response builder。
- 增加按需 full frame route 或 `detail=full` 策略。
- 先补 summary/full server route 与 web API contract 的失败测试，再实现到通过。
- 在 `crates/picea-lab/web/src/types.ts` / `api.ts` 增加 summary/full 类型。
- 在 `App.tsx` 的 live playback 路径使用 summary buffer；暂停、scrub、diagnostics/inspector 需要 full 时再取 full。
- 保持 artifact replay 原路径不变。

不做：
- 不删除或重构 artifact `FrameRecord`
- 不修改 physics solver
- 不把 diagnostics 低频采样做复杂策略；第一版只做按需 full

所有权：
- 可能触及：
  - `crates/picea-lab/src/server.rs`
  - `crates/picea-lab/tests/server_routes.rs`
  - `crates/picea-lab/web/src/types.ts`
  - `crates/picea-lab/web/src/api.ts`
  - `crates/picea-lab/web/src/App.tsx`
  - `crates/picea-lab/web/src/components/workbench/Timeline.tsx`
  - 本计划文档
- 不应触及：
  - `crates/picea/src/*`
  - `crates/picea-lab/src/scenario.rs`
  - `crates/picea-lab/src/artifact.rs`，除非只复用已有 frame clone/storage helper

验收标准：
- `matrix_stack 8x6` live playback 的 `api.json bytes` 相对当前约 `821KB` 明显下降。
- `canvas.drawWorld` 仍能显示 latest authoritative live frame。
- 暂停后 inspector/diagnostics/copy context 可获得 full frame 事实。
- 同一 frame 从 summary hydrate 为 full 后，diagnostics、missing evidence、timeline marker 不能保留 stale summary 结果。
- artifact replay、demo replay 不退化。

验证方式：
- `rtk proxy cargo test -p picea-lab --test server_routes`
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run test:profile`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- browser-use：打开 `http://127.0.0.1:5173/?picea-profile=1`，选择 `Rust 实时会话` + `矩阵堆叠 8x6`，记录 `api.json bytes/avg/max` 与 full frame 按需行为。

Subagent 执行计划：
- explorer：需要；确认最小 summary 字段与 full frame consumers。
- worker：`gpt-5.4`；实现 server/web 协议分层，文件所有权按上表。
- reviewer：重点审查 schema 兼容、missing evidence、velocity perturbation authoritative handle/revision。
- verifier：运行命令和 browser acceptance。

提交策略：
- auto-commit：否
- message hint：`Optimize live session frame payload`

本次执行记录（2026-05-07）：
- `server.rs` 已增加 `detail=summary|full` 解析；live `step` 默认返回 sibling `live_frame_summary` 并在热路径隐藏 `session.latest_frame`，`detail=full` 继续保留旧 `latest_frame` full `FrameRecord` 契约，同时新增 `GET /api/sessions/:id/frames/:index` 从 live buffer 只读返回 full frame。
- live runtime 仍把每步完整 `FrameRecord` 存在 Rust `runtime.frames` / `record.latest_frame` 中；本次没有修改 artifact `FrameRecord` schema，也没有触碰 solver/core 行为。
- `server_routes` 已补红绿测试：默认 summary omission、`detail=full` 兼容 full payload、live frame lookup 只读且不改 session state。
- `web/src/types.ts` / `api.ts` / `App.tsx` 已新增 summary/full 类型和 hydration 路径：自动 live playback 走 summary，显式手动 `step` 走 `detail=full`，paused 状态下若当前帧仍是 summary 会按需 `fetchLiveFrame()` hydrate full。
- `Timeline.tsx` 对 summary frame 的 diagnostics 现明确标记为 `not hydrated`，避免把缺失证据渲染成“无问题”。
- `profile-contract.mjs` / `ui-contract.mjs` 已同步锁住 summary step + full hydration 路径，确保 `api.json` profiling 同时覆盖热路径和按需 full fetch。

风险 / 后续：
- summary buffer 会引入 summary/full 混合状态，必须防止 UI 在缺 full 数据时显示假 diagnostics。

### LP-E3：实现 response-aware adaptive live cadence

类型：执行
状态：计划中
来源设计：D1

目标：
把 live playback 从固定 30Hz `setInterval` 调整为响应感知的 loop。后端 step 慢时不追债、不空转，并在 UI/日志/profile 中诚实显示实际 cadence 或 degraded realtime。

执行输入：
- D1 degraded realtime 文案边界
- LP-E2 summary payload 后的 live step profile

为什么现在做：
即使 payload 变小，复杂场景仍可能无法稳定 30Hz。adaptive cadence 让 UI 变得可控、可解释，而不是把 interval 触发堆到主线程。

范围：
- 替换 live playback 的固定 `setInterval(1000 / 30)` 为 response-aware loop。
- 保留 artifact/demo replay 的本地 30Hz 播放语义。
- 增加 profile/counter：目标 fps、实际 step latency、skip/backoff。
- 增加可脚本化的 live-loop contract，覆盖单 in-flight step、慢响应不追债、pause/reset/change scenario 后旧响应丢弃。
- 在状态条或日志中显示 degraded realtime，不增加大段说明文。

不做：
- 不做 server-side autoplay / background tick
- 不做 worker thread
- 不修改 session semantics 中 `play` 只改变 status、physics only moves through step 的约束

所有权：
- 可能触及：
  - `crates/picea-lab/web/src/App.tsx`
  - `crates/picea-lab/web/src/i18n.ts`
  - `crates/picea-lab/web/src/components/workbench/Toolbar.tsx`
  - `crates/picea-lab/web/src/components/workbench/Timeline.tsx`
  - `crates/picea-lab/web/scripts/ui-contract.mjs`
  - `crates/picea-lab/web/scripts/i18n-contract.mjs`
- 不应触及：
  - `crates/picea-lab/src/server.rs`，除非 LP-E2 后需要类型字段同步
  - `crates/picea/src/*`

验收标准：
- live step latency 高于 33ms 时 UI 不积压 interval 空转。
- 用户能看到 degraded realtime/actual cadence 信号。
- pause/reset/change scenario 能立即停止旧 loop，旧响应不会回写。
- 延迟 step promise 下不会并发发多个 step 请求，也不会在恢复后回放旧响应。
- artifact/demo playback 行为不变。

验证方式：
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run test:profile`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- browser-use：live matrix 播放、pause、reset、change scenario、artifact replay sanity。

Subagent 执行计划：
- explorer：需要；确认现有 guard/token/interval 依赖。
- worker：`gpt-5.4`；只改 web live loop 与文案。
- reviewer：重点审查 stale response、pause/reset race、UI 文案是否误导。
- verifier：运行 web contract/build 和 browser acceptance。

提交策略：
- auto-commit：否
- message hint：`Add adaptive live playback cadence`

风险 / 后续：
- cadence 变化会改变体感播放速度，需要 UI 明确区分 simulated time 与 wall-clock playback。

### D2：冻结 timeline 增量缓存等价策略

类型：设计
状态：计划中

目标：
为 `timeline.derive` 从全量扫描改为增量缓存建立等价性和失效规则，避免优化后 marker 错误。

要回答的问题：
- `buildStackMarkers`、`buildTrajectoryMarkers`、`buildDiagnosticTimelineMarkers` 哪些可 append-only？
- reset、scenario change、source change、locale change、selection change、trajectorySettings change 应如何失效缓存？
- 同一 frame 从 summary hydrate 为 full 后，diagnostics、missing evidence、marker cache 如何失效并重算？
- 是否需要保留 debug-only 全量比对开关，供 profile/contract 运行时验证增量结果？

为什么现在做：
browser profile 显示 `timeline.derive` 后段可达约 `60ms`。直接改增量缓存风险高，必须先定义“快且等价”的验收。

设计范围：
- `crates/picea-lab/web/src/components/workbench/Timeline.tsx`
- `crates/picea-lab/web/src/components/workbench/stackStability.ts`
- `crates/picea-lab/web/src/components/workbench/trajectory.ts`
- contract/test scripts

不做：
- 不改生产实现代码
- 不改 marker 视觉表现

输入证据：
- 当前 `Timeline.tsx` 每次 render 全量派生 stack/trajectory/diagnostic/rail markers
- browser profile 中 `timeline.derive` 随 frames 增长变重

输出交付物：
- 增量缓存设计说明
- LP-E4 文件边界和等价性测试方案

设计验收：
- 每类 marker 的依赖和失效条件清楚
- 设计包含与当前全量派生的等价性验证

验证方式：
- 文档审查
- reviewer 审查

下一步映射：
- 生成/更新执行里程碑：LP-E4

风险 / 后续：
- 部分 marker 需要窗口回看，可能只能做有限增量而非纯 append-only。

### LP-E4：实现 timeline marker 增量派生

类型：执行
状态：计划中
来源设计：D2

目标：
把 live 播放时的 `timeline.derive` 从每帧全量扫描降到增量/缓存更新，降低长时间 live playback 的主线程压力。

执行输入：
- D2 的失效规则和等价性测试方案

为什么现在做：
LP-E2/LP-E3 降低 transport 与 cadence 压力后，`timeline.derive` 会成为下一层瓶颈。

范围：
- 为 stack/trajectory/diagnostic markers 建立增量缓存或分层 memo。
- 增加 contract 或 unit-like script，比较同一 frames 输入下增量结果与全量结果。
- 保留 profile `timeline.derive`。

不做：
- 不改 marker 语义和视觉优先级
- 不改 server/API payload
- 不优化 Canvas

所有权：
- 可能触及：
  - `crates/picea-lab/web/src/components/workbench/Timeline.tsx`
  - `crates/picea-lab/web/src/components/workbench/stackStability.ts`
  - `crates/picea-lab/web/src/components/workbench/trajectory.ts`
  - `crates/picea-lab/web/scripts/ui-contract.mjs`
  - `crates/picea-lab/web/scripts/profile-contract.mjs`
- 不应触及：
  - `crates/picea-lab/src/*`
  - `crates/picea/src/*`

验收标准：
- marker 输出与当前全量派生等价。
- live matrix 播放后段 `timeline.derive` 明显下降。
- reset/scrub/change scenario/locale/selection/trajectory settings 不显示 stale markers。
- summary frame hydrate 为 full frame 后，diagnostic/missing evidence/marker cache 会重算，不保留旧 summary 派生结果。

验证方式：
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:profile`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- browser-use：live matrix profile 复测 `timeline.derive` summary。

Subagent 执行计划：
- explorer：需要；确认 marker helper 依赖和测试可导出性。
- worker：`gpt-5.4`；只做 Timeline/marker 增量缓存与 contract。
- reviewer：重点审查等价性、缓存失效和复杂度是否过高。
- verifier：运行 web gates 与 browser profile。

提交策略：
- auto-commit：否
- message hint：`Incrementalize live timeline markers`

风险 / 后续：
- 如果缓存复杂度过高，允许降级为“只在非 timeline panel 时暂停 marker 派生”的较保守优化，但必须更新 D2。

### V1：端到端性能验收

类型：验证
状态：计划中

目标：
在实现 LP-E2/LP-E3/LP-E4 后，用同一 profile 入口证明瓶颈改善，并确认 artifact replay / live session / diagnostics / inspector 没有语义退化。

范围：
- 跑 Rust server/web contract/build。
- 用 browser-use 实测 `matrix_stack 8x6` live profile。
- 对比 baseline：`api.json bytes/avg/max`、`timeline.derive avg/max`、`canvas.drawWorld avg/max`、long task/截图可用性、UI 是否可 pause/reset。

不做：
- 不改实现代码
- 不调 solver

所有权：
- 可能触及：本计划文档的进度记录
- 不应触及：生产代码

验收标准：
- profile 显示 payload 与 timeline 成本相对 baseline 改善。
- 页面能稳定 pause/reset/change scenario，Codex browser 能获取截图或 DOM/logs。
- artifact replay 的 diagnostics / evidence / copy debug context 仍可用。

验证方式：
- `rtk proxy cargo test -p picea-lab --test server_routes`
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run test:profile`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- `rtk proxy just picea-lab-web-check`
- 浏览器验收流程：`rtk proxy just picea-lab-web-start` -> browser-use 打开 `http://127.0.0.1:5173/?picea-profile=1` 跑 live profile -> `rtk proxy just picea-lab-web-stop`。如果端口已有前台服务，则记录现有服务并只执行 browser-use 验收。

Subagent 执行计划：
- explorer：不需要
- worker：不需要
- reviewer：可选；若 V1 发现 surprise diff 或 UX regression，再审查
- verifier：`verifier` 运行命令与浏览器验收，报告生成/修改文件

提交策略：
- auto-commit：否
- message hint：无

风险 / 后续：
- 本机 wall-clock 不作为 hard fail；只作为相对改善和体验证据。

### C1：文档和路由收尾

类型：收尾
状态：计划中

目标：
把 live performance 的最终设计、验证结果、后续限制写回 AI 路由和相关设计文档，方便后续会话从正确入口继续。

范围：
- 更新本计划进度记录。
- 必要时更新 `docs/ai/index.md`、`docs/ai/repo-map.md`、`docs/ai/doc-catalog.yaml`。
- 必要时更新 `docs/design/matrix-stack-stability-optimization-design.md` 的 live performance 小节。

不做：
- 不改实现代码
- 不创建新的 solver/stability 计划

所有权：
- 可能触及：
  - `docs/ai/index.md`
  - `docs/ai/repo-map.md`
  - `docs/ai/doc-catalog.yaml`
  - `docs/design/matrix-stack-stability-optimization-design.md`
  - 本计划文档
- 不应触及：
  - `crates/*`

验收标准：
- AI 路由能把“实时模拟卡顿 / live performance”导向本计划和 live performance docs。
- 收尾记录包含 profile before/after 数字、验证命令和剩余风险。

验证方式：
- `rtk proxy git diff --check`
- 文档人工检查

Subagent 执行计划：
- explorer：不需要
- worker：`gpt-5.4-mini` 可选；仅机械文档路由更新
- reviewer：审查路由是否清楚区分 solver stability 与 live performance
- verifier：运行 `rtk proxy git diff --check`

提交策略：
- auto-commit：否
- message hint：`Document live performance optimization path`

风险 / 后续：
- 如果计划执行中改变了 API 名称或验收口径，C1 必须同步文档，不留旧路由。

## 进度记录

### 2026-05-07 - 计划创建

- 状态：进行中
- Commit：none
- 验证：未运行；当前仅创建计划文档
- 风险 / 后续：等待 reviewer 审查与用户 Plan Gate 确认；确认前不执行生产实现。

### 2026-05-07 - reviewer 审查采纳

- 状态：计划修订中
- Commit：none
- 验证：`rtk proxy git diff --check -- docs/plans/2026-05-07-picea-lab-web-live-performance-milestones.md`
- 采纳：
  - 将 dirty server/web/profile 重叠改为 Plan Gate 必须确认问题。
  - 将执行里程碑命名改为 `LP-E*`，避免与 matrix stability solver 线的 `E*` 混淆。
  - 将 LP-E1 改为必须绿门通过，不留下等待 LP-E2 修复的失败测试。
  - 补入 summary hydrate full 的缓存失效验收。
  - 补入 live-loop race contract 和完整 browser-use 启停验收流程。

### 2026-05-07 - D1 live frame 分层协议

- 状态：已完成
- Commit：none
- 改动：
  - `docs/design/matrix-stack-stability-optimization-design.md` 新增 `LiveFrameSummary` 字段表、summary/full 获取规则、hydrate full 缓存失效规则。
  - `docs/design/picea-lab-live-session-semantics.md` 新增 live frame detail contract，固定 `detail=summary|full` 与 `GET /api/sessions/:id/frames/:index` 的语义。
- 验证：文档级检查，后续 LP-E1 用 server/web contract 锁住。
- 风险 / 后续：LP-E1 必须保持所有 contract 通过，不留下等待 LP-E2 修复的失败测试。

### 2026-05-07 - LP-E1 review fix

- 状态：已完成
- Commit：none
- 改动：
  - 移除 LP-E1 测试中未被 server 解析的 `detail: "full"`，避免把 baseline test 误写成已实现 detail contract。
  - 补强 `profile-contract.mjs`，要求 `controlSession()` 继续走 `requestJson`，从而让 live `/control` step 留在 `api.json` profiling 覆盖下。
- 验证：待 rerun LP-E1 gates。
- 风险 / 后续：`detail=summary|full` 的红绿测试必须在 LP-E2 内补齐。

### 2026-05-07 - LP-E2 summary/full live transport

- 状态：已完成
- Commit：none
- 改动：
  - `crates/picea-lab/src/server.rs` 支持 live control `detail=summary|full`；默认 live control 不再返回 full `latest_frame`，但 Rust runtime 仍保留 full `FrameRecord` buffer。
  - `GET /api/sessions/:id/frames/:index` 返回 read-only full `FrameRecord` hydration 响应。
  - Web live autoplay 默认消费 summary；手动 step 使用 full；暂停到 summary frame 时按需 hydrate full。
  - `Timeline` 和 copy debug context 在存在未 hydrate live summary frame 时不导出 stack/trajectory/lattice 派生证据，避免把空 contacts/islands 当成事实。
  - `profile-contract.mjs` / `ui-contract.mjs` 锁住 summary/full 请求路径、hydration freshness guard 和 not-hydrated 语义。
- 验证：
  - `rtk proxy cargo test -p picea-lab --test server_routes`：13 passed。
  - `rtk proxy cargo test -p picea-lab`：29 lib tests passed；artifact_run 22 passed / 4 ignored；server_routes 13 passed；doc tests passed。
  - `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`：passed。
  - `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`：passed。
  - `rtk proxy npm --prefix crates/picea-lab/web run test:profile`：passed。
  - `rtk proxy npm --prefix crates/picea-lab/web run build`：passed，保留既有 Vite large-chunk warning。
- 浏览器观测：
  - `matrix_stack 8x6` / `?picea-profile=1` / in-app browser：live control `api.json bytes` 从修复前约 `821582` 降到约 `103725-104404`。
  - `live.applyFrame` profile 标记 `detail: "summary"`；`timeline.derive` 在 summary 未 hydrate 时输出 `skipped: "live-summary-not-hydrated"`。
- 风险 / 后续：full hydration 仍可能触发一次大 payload；这是 paused review/inspector 的按需成本，不应回到 autoplay 路径。

### 2026-05-07 - LP-E3 response-aware live cadence

- 状态：已完成
- Commit：none
- 改动：
  - `App.tsx` 将 live playback 从固定 `setInterval(1000 / 30)` 改为 response-aware `tick -> await advanceLiveFrame() -> setTimeout(tick, nextDelayMs)`。
  - 新增 `LiveCadenceStatus`，记录 target fps、actual fps、last step ms、next delay、degraded 状态。
  - `TimelineHeader` 显示 live cadence badge，step 超过 30Hz budget 时显示 degraded realtime。
  - `ui-contract.mjs` 锁住 live 路径不能回退到固定 interval，并要求 `live.cadence` profile 与 UI badge 存在。
- 验证：
  - `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`：passed。
  - `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`：passed。
  - `rtk proxy npm --prefix crates/picea-lab/web run test:profile`：passed。
  - `rtk proxy npm --prefix crates/picea-lab/web run build`：passed，保留既有 Vite large-chunk warning。
  - in-app browser live profile：`live.cadence` 输出 `targetFps: 30`，约 `47ms` step 时 `actualFps` 约 `21fps` 且 `degraded: true`；页面可见 `fps` cadence badge。
- 风险 / 后续：adaptive cadence 会诚实降低 wall-clock fps，但模拟时间仍由后端 step 推进；UI 已避免把它说成“仍为 30fps 实时”。

### 2026-05-07 - D2 / LP-E4 live timeline derive hot path

- 状态：已完成（以 summary gating 落地，未引入额外缓存层）
- Commit：none
- 决策：
  - live autoplay 当前只持有 summary frame，缺少 contacts/islands/diagnostics 的 full evidence；因此 timeline/stack/trajectory/lattice 派生不应该做“增量猜测”，而是跳过并显式标记 not hydrated。
  - 这比在 summary 上做缓存/增量推导更保守：性能上移除 live hot path，语义上不把空数组误报为物理事实。
- 验证：
  - `ui-contract.mjs` 锁住 `skipped: "live-summary-not-hydrated"`。
  - in-app browser profile 中 live playback 的 `timeline.derive` 为 `durationMs: 0` 且带 `skipped: "live-summary-not-hydrated"`。
- 风险 / 后续：artifact replay 和 full hydrated frame 的深度派生仍是全量路径；若后续需要长时间 paused full review，再单独做 full-frame incremental cache。
