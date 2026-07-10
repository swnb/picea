# Picea Lab Workbench 交互化与运行模型设计(选中即所得 + 刚体拖拽)

状态:待评审
设计文档:docs/design/2026-07-10-workbench-interactive-run-model-design.md
最后更新:2026-07-10
工作目录:/Users/asyncrustacean/projects/picea
适用范围:crates/picea-lab(server/session)与 crates/picea-lab/web;`crates/picea` core 零改动

## 背景与目标

当前 workbench 的两个已确认痛点:

1. **选中 ≠ 运行**:`changeScenario` 选中场景时画布播放 `demo.ts` 的前端手写动画(easing/正弦公式,非物理),点"运行"才走 Rust 真实模拟,两者效果必然不一致。Rust 端 16 个场景中 5 个(`sat_polygon`、`broadphase_sparse`、`concave_decomposition`、`ccd_dynamic_convex_pair`、`ccd_dynamic_compound_wall`)没有对应假动画生成器,选中后画面 fallback 成 falling box,与场景名完全无关。server 失败时还会静默降级回假动画。
2. **场景不明所以**:16 个场景本质是引擎验收 fixtures,描述是工程视角,UI 不回答"这个场景验证什么、该盯着看什么、什么算正常"。

同时引入新的产品方向:**可交互 workbench**——用指针(鼠标/触屏)抓取并拖动画布中的刚体。

总路线分四步,本设计只覆盖前两步:

- **① 地基:选中即所得**(本设计)
- **② 拖拽交互**(本设计)
- ③ 场景包 v1(钉墙网 / 软弹簧 / 凹多边形堆叠 / 迷你游戏场景)——独立设计,依赖 ①②
- ④ revolute + motor joint core 立项(解锁 joint 拼接小车、机械类游戏场景)——独立 core 设计流程,不在 lab 侧偷跑

能力边界结论(已核实):core 现有 `DistanceJointDesc` / `WorldAnchorJointDesc` 均带 `stiffness`/`damping`;`WorldAnchorJointPatch.world_anchor` 支持运行时更新目标点;`BodyPatch` 支持 `pose` / `linear_velocity` / `wake`。①② 所需原语全部已在,**core 零改动**。

## 非目标

- 不改 `crates/picea` 任何 public surface;不新增 joint 类型(revolute/motor 属 ④)。
- 不做多点触控 multi-grab(单 grab,后续需求再扩)。
- 不做 teleport 式硬拖拽(`BodyPatch.pose` 直接设位)——会穿透、破坏动量;direct 模式用速度直驱实现"硬跟手"。若实测后仍需要穿透语义,再加一档。
- 不在 artifact replay / demo 模式下提供拖拽(artifact 是只读回放证据,demo 是离线示意)。
- 不把拖拽历史写入 artifact schema(grab 是 live-only 事实;spring 模式的 joint 本身已进入 `DebugSnapshot.joints`,无需另造 schema)。
- web 不重算物理:拖拽指示线是显示层标注,物理事实(joint、速度)一律来自 Rust 帧。
- 不删除 `demo.ts`(离线模式仍用),仅将其从"选中默认"降级为"显式离线降级"。

## ① 地基:选中即所得

### 行为变更

1. **选中场景 = 自动运行**:`changeScenario` 不再 `setFrames(makeDemoFrames(...))`,改为直接触发现有 `runScenario` 路径(沿用当前 runMode)。连续快速切换场景时复用现有 generation guard 取消过期请求,后选者胜。
2. **默认 runMode 改为 `live_session`**(决策点,评审确认):live 是"所见即所得 + 可交互"的默认体验;`artifact_replay` 保留为"复盘/取证"显式选择。拖拽仅 live 可用,默认 live 使 ② 开箱即用。
3. **demo 帧降级为显式离线模式**:仅当 server 不可达(`fetchScenarios` / `createSession` 失败)才落入 demo 帧,画布叠加显著水印(半透明大字,i18n:"离线演示数据,非真实模拟 / Offline demo data — not a real simulation"),配合现有 source badge。消除静默降级。
4. **场景看点文案**:为 16 个场景在 `i18n.ts` `scenarioMessages` 增加 `watchFor` 字段(中英双语),回答"验证什么 / 盯什么 / 什么算正常"(例:牛顿摆——"释放端单球摆入,另一端单球等高弹出;长窗口动能不衰减")。展示位:工具栏场景描述 tooltip 与画布空态/加载占位。场景分组沿用现有 `scenarioGroupForId`,本期不做分组折叠。

### 影响面

- `web/src/App.tsx`(changeScenario/runScenario/默认 state)、`WorldCanvas.tsx`(水印)、`i18n.ts`(watchFor + 水印文案)、ui-contract / i18n 契约测试。
- lab server 无改动;`demo.ts` 无改动。

## ② 拖拽交互(live session)

### 交互流(web)

- `pointerdown` 命中 dynamic body(复用画布已有拾取逻辑)→ 发起 grab;`pointermove` → 更新拖拽目标(世界坐标,前端负责屏幕→世界换算);`pointerup` / `pointercancel` / `Escape` → 释放。Pointer Events 天然覆盖鼠标与触屏。
- 高频 `pointermove` 合并:前端只保留最新目标点,同一时刻至多一个在途更新请求(参照 `liveStepInFlight` 模式),按 live step 节奏发送,不堆积请求。
- 门控:仅 `source === "live"` 且 session 活跃时启用;artifact/demo 下 hover 提示"运行 live 会话以启用拖拽"。
- 抓取处于 paused 的会话时自动恢复 playing(拖拽需要模拟推进才有反馈);释放后保持 playing。
- 拖 sleeping body:grab 时唤醒;拖 static body:server 拒绝(400),前端不发起。

### Server API(lab,session 持有 grab 状态)

- `POST /api/sessions/:id/grabs`:`{body_handle, grab_point: [x,y](世界坐标), mode: "spring" | "direct", stiffness?, damping?, max_speed?, session_epoch}` → `{grab, session}`。同一 session 最多一个 active grab,新 grab 自动替换旧的(旧 grab 先走释放清理)。
- `PATCH /api/sessions/:id/grabs/:grab_id`:`{target: [x,y], session_epoch}` → `{grab, session}`。
- `DELETE /api/sessions/:id/grabs/:grab_id`:释放;body 保留当前速度(可甩飞,两种模式语义一致)。
- 竞态防护与 velocity perturbation 刻意不同:grab 全程(POST/PATCH/DELETE)只校验 `session_epoch` + 会话/实体有效性,**不校验 `world_revision`**——拖拽发生在 playing 中,revision 每 step 递增,任何 revision 校验都会持续失配(perturbation 只在 paused 下用,所以那里校验 revision 是对的)。server 用自身当前 world 事实(`world.body(handle).pose()`)换算抓取锚点,不依赖客户端所见帧,因此无需 revision 一致性。同理,grab 操作**不 bump `session_epoch`**:它只影响未来帧、不改写历史(perturbation bump epoch 是因为它重写已产生的帧)。`reset`、切场景、session 过期时 server 自动清理 grab。实施时不要"补上" revision 校验或 epoch bump。
- `SessionRecord` 增加 `active_grab` 字段(id、body_handle、mode、grab 局部锚点、当前 target、spring 模式的 joint_handle),前端据此渲染与对齐。

### 两种模式(可切换,默认 spring)

- **spring(软弹簧,默认)**:grab = 在 world 中 `create_world_anchor_joint`(`local_anchor` 为抓取点的 body 局部坐标,stiffness/damping 来自请求);拖动 = `JointPatch.world_anchor` 更新目标;释放 = destroy joint。joint 是真实物理事实,自然出现在 `DebugSnapshot.joints`,画布上的拖拽弹簧线直接画该 joint;grab joint 以 `user_data` 保留值 + `active_grab.joint_handle` 双重标识,与场景自带 joint 区分。UI 暴露"拖拽强度"滑条(映射 stiffness,damping 按比例联动)。
- **direct(硬跟手)**:server 在每次 step 前对 grab body `apply_body_patch(linear_velocity = (target − 抓取点世界位置)/dt)`,幅值按 `max_speed` clamp。物体尽力贴住指针但仍走完整碰撞管线,**不穿透**。画布画显示层指示线(抓取点→指针),样式与 spring 的真实 joint 线区分。

### 影响面

- lab:`server`(grab 路由 + session 状态 + step 前速度注入);`SessionRecord` 新增 `active_grab` 可选字段属向后兼容演进(该类型无版本号字段;artifact schema 不受影响——grab 不进 artifact)。
- web:`WorldCanvas.tsx`(pointer 状态机 + 指示线)、`App.tsx`(grab 生命周期与门控)、拖拽设置 UI(模式切换 + 强度滑条)、`api.ts`(三个新请求)、i18n。
- core:零改动。

## 测试与验收

- lab 集成测试(`rtk proxy cargo test -p picea-lab --test server_routes`)新增:grab 创建/更新/释放生命周期;epoch/revision 失配拒绝;static body 拒绝;reset/切场景清理;direct 模式速度注入在下一 step 生效且受 max_speed clamp;spring 模式 joint 出现于 debug snapshot 且释放后移除。
- web 契约:`test:ui-contract` 新增——server 可用时选中场景后 source ≠ demo;离线降级出现水印;拖拽状态机(idle→grabbing→dragging→released);非 live 门控。`test:i18n` 覆盖 watchFor 与水印文案双语完整性。
- 构建门:`rtk proxy npm --prefix crates/picea-lab/web run build`;`rtk proxy cargo test -p picea-lab`。
- 浏览器验收(真实 dev URL,三面):离线水印(停 server)、artifact 复盘面、live 面选中即运行 + 拖球砸塔(spring/direct 各一次),记录截图与 console/profile 证据。与 vNext E6 的 browser showcase 验收口径对齐(E6 要求的 source label / not-hydrated / showcase coverage 不因本设计回退)。

## 风险与边界

- **拖拽跟手延迟**受 live cadence(30fps)限制:目标点按 step 节奏生效;体感不足时后续单独评估提升 cadence,不在本期。
- **spring 手感调参**:默认 stiffness/damping 经浏览器验收调定,滑条兜底。
- **grab joint 计入 `active_joint_count` 等统计**:属真实事实,不改 core 统计;Facts/Inspector 以 grab 标识注明来源。
- **选中即运行的请求风暴**:切场景防抖 + generation guard,ui-contract 锁行为。
- **`sat_polygon` 等 5 场景首次获得真实画面**:验收时逐场景过一遍看点文案与实际表现是否相符。

## 路由与后续

- ③ 场景包 v1(钉墙网 / 软弹簧链 / 凹多边形堆叠 / 基于拖拽的迷你游戏)在 ①② 落地后单独出设计:纯 lab scenario 编排 + 看点文案,零 core 改动;届时不再需要为新场景写 `demo.ts` 生成器。
- ④ revolute + motor 为 core 扩展,走独立设计/里程碑流程(参照 `docs/plans/2026-06-17-physics-realism-vnext-milestones.md` E5 的 joint 边界约定,不合并 row stream)。
- 与 deformable 边界:钉墙网/软弹簧仍是 rigid-body + joint 组合,遵守 `docs/design/deformable-body-roadmap.md` 的 "rigid-body lattice proxy ≠ 软体" 文案纪律。
