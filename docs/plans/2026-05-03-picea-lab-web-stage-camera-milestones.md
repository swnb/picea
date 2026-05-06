# Picea Lab-Web 舞台区镜头与网格效果里程碑计划

状态：已完成
计划文档：docs/plans/2026-05-03-picea-lab-web-stage-camera-milestones.md
最后更新：2026-05-03
提交策略：不提交
执行策略：完整计划批准后连续执行

## 目标

增强 `picea-lab-web` 中间舞台区的调试观感和镜头手感，让用户在 live session / artifact replay 中都能更自然地看清核心区域：

- 网格横线和竖线都清楚可见，线条层级随 zoom 保持稳定，不再给人“只有竖线”的错觉。
- 舞台区支持明确的 camera mode：自由观察、适配全局、锁定核心区域。
- 锁定后能自动 zoom in 到当前核心区域，并在 live step/play 时稳定跟随，而不是每帧跳动或丢失目标。
- 滚轮缩放有阻尼感和缓动，不再是离散倍率突跳。
- camera mode / target / zoom / grid / rulers 能进入证据面板和 debug context，方便复现一次观察。

## 约束

- 工作区：`/Users/asyncrustacean/projects/picea` 当前主工作区。
- 当前工作区已有上一轮 `picea-lab-web` live-first/canvas 改动，且未提交；本计划基于这些 dirty facts，不把它当成干净 baseline。
- 本轮计划阶段只写本计划文档和必要 doc catalog 路由，不改实现代码。
- 保持 `picea-lab-web` 只消费 `FrameRecord` / `DebugSnapshot` / exported facts；不在 Web 重新计算 physics、island grouping、broadphase、collision、solver 或 decomposition。
- 不触碰 `crates/picea/src/*`、`crates/picea-lab/src/server.rs`、server route 语义、artifact schema 或 live session authority，除非计划执行时证明现有 frame facts 不足且先修订计划。
- 所有验证命令使用 `rtk proxy`。
- 最终浏览器验收优先使用 `browser-use:browser`，目标页面沿用本地 dev server。
- Camera 状态机固定为：
  - `free`：用户手动 pan/zoom 或普通页面初始状态。
  - `locked_core`：自动跟随核心区域。
  - `fit` 是一次性动作，不是持久 mode；执行后进入 `free`，并把 camera 调整到全局 scene fit。
  - `reset` 清空 selection，退出锁定，进入 `free`，并恢复 scene fit。
  - 在 `locked_core` 下用户手动 pan/zoom 时立即退出到 `free`；再次点击锁定按钮才重新锁定核心区域。
- 核心区域优先级固定为：selected entity bounds > active dynamic bodies / contacts bounds > all collider bounds > fallback scene bounds。
- 浏览器验收 harness：
  - 后端：`rtk proxy cargo run -p picea-lab -- serve --bind 127.0.0.1:18081`
  - Web：`cd crates/picea-lab/web && VITE_PICEA_LAB_API_BASE=http://127.0.0.1:18081 rtk proxy npm run dev -- --host 127.0.0.1 --port 5176`
  - 如果端口被占用，使用下一个空闲端口并在进度记录里写明。

## 待确认问题

### 必须确认

- 无。`fit/lock/reset` 状态机和核心区域优先级已在约束中固定；如果用户希望不同语义，可在 Plan Gate 修改后再执行。

### 可带假设推进

- `阻尼感` 默认实现：只做前端 camera target/current 的 easing，不引入物理 tick、不改后端、不使用重型 animation library。影响：实现范围集中在 `WorldCanvas`；早期验证：wheel 连续输入时视觉顺滑，hit-test 仍使用同一个当前 camera。

## 规划依据

- explorer：已运行。结论：当前 `WorldCanvas` 已有 `makeCamera`、`canvasCameraState`、drag pan、wheel zoom、fit/reset、HiDPI、hit-test；`drawGrid` 已经同时绘制 x/y 两向网格，问题更像视觉权重和 camera 体验，而不是缺少横线绘制。
- 关键证据：
  - `docs/ai/repo-map.md` / `docs/ai/index.md`：`picea-lab-web` 属于 `crates/picea-lab/web/src/*`，Web 不运行 physics，不发明 server-side autoplay。
  - `crates/picea-lab/web/src/components/workbench/WorldCanvas.tsx`：当前拥有 camera state、wheel zoom、pan、fit/reset、drawGrid、drawRulers、hitTest，是主实现边界。
  - `crates/picea-lab/web/src/components/workbench/types.ts`：`CanvasDebugView` 只有 `zoom/center/grid/rulers`，还没有 camera mode、lock target 或 damping 状态。
  - `crates/picea-lab/web/src/components/workbench/Toolbar.tsx`：已有 layer menu，适合接入 lock/focus 图标控制。
  - `crates/picea-lab/web/src/components/workbench/Timeline.tsx`：已有 evidence/debug context 面板，适合展示 camera mode / target / lock 状态。
  - `crates/picea-lab/web/scripts/ui-contract.mjs` / `i18n-contract.mjs`：已有 canvas camera/grid/ruler/debug context 入口锁点，应扩展到 stage lock / damping 文案和控件。
- 主要未知：
  - 核心区域是否应优先追踪 selected entity、active island、contact manifold，还是 dynamic body。计划用可配置优先级把默认行为做小，后续可再加 mode。
  - 滚轮阻尼的主观“手感”只能靠浏览器验收，contract 只能锁住状态和入口。
  - 如果当前横线在实际视觉里仍不明显，需要用截图和像素/视觉验收调线条透明度，而不是改数据。

## 计划验收

- 状态：已批准
- reviewer：已运行
- 审查结论：有发现，已采纳：固定 `fit/lock/reset` 状态机；拆开 M2 与 M4 evidence 范围；补 browser-use 可复跑 harness；调整 doc-catalog 路由状态，避免旧计划和新计划重叠。
- 用户确认：已确认执行

## 里程碑

## M1：Stage Grid Readability

状态：已完成（2026-05-03）

目标：
把舞台网格从“能画出来”推进到“在真实页面上横线/竖线都清楚、层级稳定、不会抢主体”的调试背景。

为什么现在做：
用户首先反馈“中间舞台区不光有竖线，还有横线”。代码现状已经画了两向网格，所以第一步应先把 stage rendering quality 固定住，避免后续 lock/zoom 放大后网格观感继续漂。

范围：
- 调整 `drawGrid` 的 major/minor step、横线/竖线视觉权重和 alpha，让两向网格在常见 zoom 下都可见。
- 让 grid step 随 zoom 使用稳定的 nice step，避免高 zoom 过密、低 zoom 消失。
- 保持 axes / rulers / scale label 的可读层级：grid 是背景，axes/rulers 是坐标证据，shape/contact 是主体。
- 更新 UI/i18n contract，锁住 grid/ruler/stage readability 入口。

不做：
- 不新增物理事实。
- 不改 rulers 的功能语义。
- 不做 stage theme 大换肤、landing-page 风格装饰或非调试型视觉效果。

所有权：
- 可能触及：`crates/picea-lab/web/src/components/workbench/WorldCanvas.tsx`、`crates/picea-lab/web/scripts/ui-contract.mjs`。
- 可能少量触及：`crates/picea-lab/web/src/i18n.ts`、`i18n-contract.mjs`，仅限文案。
- 不应触及：`crates/picea/src/*`、`crates/picea-lab/src/*`。

验收标准：
- 浏览器里横向和纵向网格线在默认 demo、live session、artifact replay 中都可见。
- zoom in/out 后网格密度平滑变化，不遮挡 shapes、contacts、velocity arrows。
- 关闭 `grid` layer 后网格完全隐藏；关闭 `rulers` 不影响 grid。

验证方式：
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- 按本文浏览器验收 harness 启动后端和 Web。
- `browser-use:browser`：默认视角、zoom in、zoom out、grid/ruler toggle 截图验收。

Subagent 执行计划：
- explorer：不需要再跑；沿用本计划调研证据。
- worker：叶子 agent；优先 `gpt-5.4`。只改 Web canvas/grid 和 contract，不改 Rust。
- reviewer：叶子 agent；审查视觉层级、是否引入主题膨胀、grid/ruler toggle 是否互相污染。
- verifier：叶子 agent；运行 Web gates 和浏览器截图验收，报告是否产生或修改本地产物。

提交策略：
- auto-commit：否。
- message hint：`Improve lab-web stage grid readability`

风险 / 后续：
- 横线“可见”是视觉判断，contract 只能锁入口；最终必须用 browser screenshot 验收。

## M2：Stage Lock And Core Focus

状态：已完成（2026-05-03）

目标：
增加明确的舞台锁定模式，让 camera 能自动聚焦到核心区域，并在 live step/play 时保持稳定跟随。

为什么现在做：
网格可读后，用户需要的是“自动锁定 zoom in 到核心区域”。这必须先定义 camera mode 和 target 优先级，否则阻尼和手动输入都会变成互相打架的状态。

范围：
- 为 canvas 增加持久 camera mode：`free`、`locked_core`。`fit` / `reset` 只作为动作，不作为 mode。
- 增加锁定按钮和 tooltip，使用图标按钮，不增加说明性 UI 文案。
- 定义核心区域 bounds：selected entity bounds > active dynamic body/contact bounds > all collider bounds > fallback scene bounds。
- 锁定时计算目标 camera，带 padding 自动 zoom in；目标丢失时回退到 scene fit。
- 手动 pan/zoom 时从 `locked_core` 立即退出到 `free`，避免用户输入被神秘覆盖。
- `fit` 动作回到全局 scene fit 并进入 `free`；`reset` 清 selection、退出锁定并恢复 scene fit。
- `CanvasDebugView` 增加 mode / target 描述，为 M4 evidence 收口提供字段；M2 不展开 evidence 面板。

不做：
- 不新增 server lock mode。
- 不追踪 live patch、editor selection mutation 或 paused transaction。
- 不重新计算 island/contact 过程，只读取 frame 中已导出的 body/collider/contact/island facts。

所有权：
- 可能触及：`WorldCanvas.tsx`、`types.ts`、`Toolbar.tsx` 或 canvas controls 区、`WorkbenchLayout.tsx`、`i18n.ts`、contract scripts。
- 不应触及：Rust server、core physics、artifact writer。

验收标准：
- 用户可以切换锁定核心区域，并看到 camera 自动 zoom in 到核心对象。
- live step/play 时锁定视角稳定跟随，不因每帧 bounds 小变化抖动。
- 选中 collider/body 后锁定优先聚焦选中对象；清除选择后回到动态主体或全局。
- 手动 pan/zoom 与锁定语义不冲突。
- canvas 顶部或控制区能明确表达当前 lock/free 状态；evidence/debug context 展示留到 M4。

验证方式：
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- 按本文浏览器验收 harness 启动后端和 Web。
- `browser-use:browser`：live session 创建、step、play/pause、选择 collider、lock/unlock、fit/reset 验收。

Subagent 执行计划：
- explorer：不需要再跑。
- worker：叶子 agent；优先 `gpt-5.4`。实现 camera mode/target 和 UI 入口，保持 draw/hit-test 共用同一 camera。
- reviewer：叶子 agent；审查 lock 语义、selection/auto-focus 优先级、fit/reset 行为是否和状态机一致。
- verifier：叶子 agent；运行 Web gates，并用 browser-use 检查 live/artifact 两条路径。

提交策略：
- auto-commit：否。
- message hint：`Add lab-web stage camera lock`

风险 / 后续：
- “核心区域”默认规则可能需要用户试用后调优；M2 只实现保守优先级，不做多种 focus preset。

## M3：Damped Wheel And Smooth Camera Motion

状态：已完成（2026-05-03）

目标：
让滚轮/触控板缩放有阻尼感，camera 在 free/locked 模式下都平滑过渡，同时保持 hit-test、rulers 和 scale label 准确。

为什么现在做：
M2 定义了 camera mode 和 target，M3 才能安全地把离散 wheel zoom 改成 target/current camera 过渡；如果先做动画，容易让 draw 和 hit-test 使用不同 camera。

范围：
- 把 wheel input 改为更新 target camera，而不是立即跳到最终 scale。
- 使用 `requestAnimationFrame` 做轻量 easing / damping，不引入重型动画依赖。
- `prefers-reduced-motion` 下禁用或缩短 smoothing。
- 保证 draw、hit-test、ruler、scale label 都使用同一个当前 camera；debug context 的最终展示留到 M4。
- 连续 wheel 输入会合并到同一个 target，不堆积无限动画。
- 增加必要 cleanup，避免组件卸载后动画继续跑。

不做：
- 不做惯性 pan、mini-map、多 viewport 或复杂物理弹簧调参面板。
- 不改变 timeline 播放速度或后端 step 节奏。

所有权：
- 可能触及：`WorldCanvas.tsx`、`types.ts`、contract scripts。
- 可能少量触及：`i18n.ts`，仅限 damping 文案。
- 不应触及：Rust crates、server routes、artifact schema。

验收标准：
- 滚轮连续缩放视觉上顺滑，不出现跳变、反向抖动或越界。
- zoom 后点击命中仍准确，ruler/scale label 跟随当前 camera。
- 切换 lock/free、fit/reset 后没有残留动画覆盖新状态。
- 页面 console error 为空。

验证方式：
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- 按本文浏览器验收 harness 启动后端和 Web。
- `browser-use:browser`：wheel zoom、fit/reset、selection hit-test、console error 检查。

Subagent 执行计划：
- explorer：不需要再跑。
- worker：叶子 agent；优先 `gpt-5.4`。只在 canvas camera 内实现 damping，不引入新库。
- reviewer：叶子 agent；审查动画 cleanup、single-camera invariant、reduced-motion fallback。
- verifier：叶子 agent；运行 Web gates 和 browser-use 手感验收。

提交策略：
- auto-commit：否。
- message hint：`Add damped lab-web canvas zoom`

风险 / 后续：
- 阻尼手感是主观项，最终以浏览器体验为准；如果过重或过轻，调参数仍应留在 M3 范围内。

## M4：Stage Evidence And Browser Closeout

状态：已完成（2026-05-03）

目标：
把新舞台能力收口成可复现的调试证据，并通过真实浏览器验收 live/artifact 两条路径。

为什么现在做：
镜头锁定和阻尼都是交互状态，如果不进入 evidence/debug context，用户很难复现“我当时看的是哪个区域、什么 zoom、什么 mode”。

范围：
- evidence/debug context 增加 camera mode、lock target、target bounds、damping enabled、center、zoom、grid/ruler 状态。
- Timeline evidence 面板显示 compact camera state，不让底部 panel 变成说明文。
- 更新 `ui-contract` / `i18n-contract` 锁住新控件、文案和 evidence 字段。
- 更新本计划进度记录，并把 `docs/ai/doc-catalog.yaml` 中本计划状态保持为当前 active 路由；旧 debuggability 计划保持 completed 路由，避免范围重叠。
- 用 browser-use 执行最终验收矩阵。

不做：
- 不新增 M35 过程 trace carrier。
- 不做 UI 截图归档系统或自动视觉回归平台。

所有权：
- 可能触及：`App.tsx`、`Timeline.tsx`、`types.ts`、`i18n.ts`、contract scripts、本计划文档、`docs/ai/doc-catalog.yaml`。
- 不应触及：core physics、server routes。

验收标准：
- evidence/debug context 可读出当前 camera mode、target、center、zoom、grid/ruler 和 damping 状态。
- live session 下 lock/auto-focus/wheel damping 可以复现；artifact replay 下行为一致。
- 页面无 console error，布局没有按钮/文本重叠。
- 计划文档记录已完成的验证和剩余风险。

验证方式：
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `rtk proxy git diff --check`
- 如更新 `docs/ai/doc-catalog.yaml`：`rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- 按本文浏览器验收 harness 启动后端和 Web。
- `browser-use:browser`：live session step/play/pause、artifact replay、grid/ruler toggle、lock/unlock、auto-focus、wheel damping、selection hit-test、evidence/debug context、console error。

Subagent 执行计划：
- explorer：不需要再跑。
- worker：叶子 agent；优先 `gpt-5.4`。只做 evidence/contract/docs closeout。
- reviewer：叶子 agent；审查 evidence 是否足以复现、是否有说明性 UI 过载、是否遗漏 browser acceptance。
- verifier：叶子 agent；运行全套 Web/docs/browser gates，报告本地产物。

提交策略：
- auto-commit：否。
- message hint：`Close out lab-web stage camera evidence`

风险 / 后续：
- 如果 browser-use 发现 `wheel` 在 in-app browser 中无法精确模拟，需要补充人工手感检查说明，但仍要用 browser-use 验证可观察状态和 console。

## 进度记录

### 2026-05-03 - Plan Draft

- 状态：计划中
- Commit：none
- 验证：只读调研；已运行 `git status --short`，读取 `docs/ai/repo-map.md`、`docs/ai/index.md`、`WorldCanvas.tsx`、`types.ts`、`doc-catalog.yaml`。
- 风险 / 后续：进入 reviewer 复审；用户批准前不执行实现。

### 2026-05-03 - Plan Review

- 状态：已完成
- Commit：none
- 验证：只读 reviewer 已运行；发现集中在 camera 状态机、M2/M4 evidence 边界、browser-use harness 和 doc-catalog 路由重叠。
- 风险 / 后续：已采纳 reviewer 发现：固定 `free` / `locked_core` 状态机，定义 `fit` / `reset` 为动作，M4 专属 evidence 收口，补可复跑 browser harness，并将旧 debuggability 计划在 doc-catalog 中标为 `completed`。

### 2026-05-03 - M4 Implementation Closeout

- 状态：已完成
- Commit：none
- 实现：
  - `CanvasDebugView` 补齐 `dampingEnabled`，并把 camera `mode`、`targetDescription`、`targetBounds`、`center`、`zoom`、`grid`、`rulers` 统一经 `onViewChange` 进入 evidence/debug context。
  - `Timeline` EvidencePanel 改为 compact camera evidence，展示 source/session/run/frame/selection/layers/camera/target/bounds/damping/grid/rulers/perf status。
  - `App.tsx` 复制调试上下文时输出结构化 `camera` 证据对象，和面板展示保持同一字段语义。
  - `ui-contract` / `i18n-contract` 增补 M4 字段与文案锁点；`i18n.ts` 补齐必要的简短标签。
  - 采纳 reviewer 发现：live `play` 等 Rust control ack 后再进入 `playing`，free mode 不再把 passive core candidate 当作当前 lock target，evidence 文案和图层/mode/target 显示改为本地化标签。
  - browser-use 发现旧 debug context preview 会跨 source/frame 残留，已改为在 source/frame/session/camera/layers/perf 等上下文变化时清空旧 preview，重新点击复制才生成当前上下文。
- 验证：
  - `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract` -> 通过
  - `cd crates/picea-lab/web && rtk proxy npm run test:i18n` -> 通过
  - `cd crates/picea-lab/web && rtk proxy npm run build` -> 通过（`vite v6.4.2`; 最新产物 `dist/assets/index-Diu_tVmn.css` 21.21 kB gzip 4.91 kB，`dist/assets/index-C0rf9-rM.js` 430.19 kB gzip 134.26 kB）
  - `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'` -> `yaml ok`
  - `rtk proxy git diff --check -- ...` -> 通过，无输出
  - verifier subagent 复跑命令门禁：五项均通过；验证只产生 ignored `dist/` build output，没有新增 tracked diff。
  - `browser-use:browser`：`http://127.0.0.1:5176/` 真实验收通过。已覆盖 default grid 横/纵线、grid/ruler toggle、lock core auto zoom、wheel 后退出 lock 并保持阻尼缩放、artifact replay perf/evidence/debug context、live session create/step/play/pause、live lock target、本页 console error 为空。
- 风险 / 后续：
  - 阻尼参数仍是主观体验项；当前浏览器验收确认交互语义、可观察状态和无 console error，后续如果用户想要更重或更轻的“手感”，可只调 `WorldCanvas` camera damping 参数。
