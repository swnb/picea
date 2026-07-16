## Context

Revolute core API、debug kind、schema-v1 fixture carrier 与 solver 已存在于当前 feature branch，但 builtin scenario/catalog/dispatch、artifact/server acceptance 和 Web 显式本地化尚未闭合。真实 Chrome 能运行既有 Rust live session，却无法选择 `revolute_pendulum`，因此 S5-LAB browser contract 在场景入口即失败。

本 change 以 `docs/design/2026-07-14-revolute-joint-v1-design.md` 与 living spec §10-§11 为冻结上游合同。实现不得修改 2x2/COM/phase/wake、core public surface、contact/CCD/integration 或 fixture schema version。

## Goals / Non-Goals

**Goals:**

- 提供一个固定配置、可复现、能清楚展示 pin-only 与自由相对旋转的 static/dynamic pendulum。
- 让 artifact 与 live server 只透传既有 authoritative debug facts，不增设 Web 派生物理口径。
- 让 Inspector、Hierarchy、Timeline/Source 与两种 locale 显式识别 revolute。
- 建立先 RED 后 GREEN 的 Rust/Web contracts，并以真实 Chrome 复测同一 HEAD。

**Non-Goals:**

- 不增加 motor、limits、damping、break force、warm-start cache 或 live joint patch。
- 不改 solver/contact/CCD/integrate、public API、schema version 或 existing joint semantics。
- 不进入完整 S5-V、S5-C、archive 或 OpenSpec sync。

## Decisions

### 1. 场景由独立 `scene_revolute.rs` 拥有

场景使用 schema-v1 `SceneRecipeFixture` 创建一个 static pivot body、一个 dynamic pendulum body和一个 `SceneRevoluteJointFixture`。两个 local anchors 在初始 world space 对齐；dynamic body从非零初始角度释放并由重力产生摆动，使画面无需用户交互即可证明相对旋转自由。static pivot collider使用query-only filter，既可见又不引入connected-body contact。

未采用在 `scene_dispatch.rs` 中直接调用 `World::create_joint`，因为 fixture path 才能同时验证 schema-v1 authoring 与 runtime facts，且避免 builtin 与 JSON fixture 语义分叉。

### 2. 固定 runtime config，不暴露调参面板

场景使用固定 `dt`、迭代数和 substeps，禁止 sleep 以确保第 240 帧仍有可观察运动；catalog description明确 pin-only/free rotation。用户没有要求可调 motor/limit，因此 parameter schema 为空。

### 3. Artifact/server 沿 generic projection 透传

测试要求 `DebugSnapshot.joints` 中恰有一个 `Revolute`、两个 finite anchors，且 active frame `joint_row_count == 1`。Server 只验证场景目录和 live step response包含相同 facts，不增加 special-case response 类型。

### 4. Web 显式消费复用既有 joint surfaces

`DebugJoint.kind` union 已包含 `revolute`，Hierarchy/Inspector 已消费 generic kind/anchors；本 change 增加显式中英文 dynamic label、scenario message/group 和 contract assertions，确保 raw unknown/fallback 不能被误判为通过。Timeline 保留 authoritative `joint_row_count` 与 live source badge，不新增浏览器计算。

### 5. Browser gate以同一提交和真实 Rust live session为准

Chrome 在 1440x900 下选择 `revolute_pendulum`，运行到 frame >= 240；通过 DOM/选中 joint Inspector 与 network/debug facts核对 kind、anchors、row count。画面只作为 pivot/free-rotation的视觉证据，authoritative数值以 Rust response为准。

## Risks / Trade-offs

- [初始角度不足或重力摆动不明显] -> 场景固定非零初始角度、重力和零 angular damping，并用 artifact 角度变化行为锁验证。
- [summary live frame缺完整 stats] -> browser暂停/选择 joint 触发既有 full-frame hydration；测试同时覆盖 artifact与server full facts。
- [Web generic consumer掩盖 missing label] -> i18n contract直接断言两 locale 的 `revolute` 显式映射，UI contract断言 Inspector/Hierarchy 继续消费 kind/anchors。
- [发布时 main 漂移] -> 推送前 fetch并确认本地 main 与远端关系；只做非 force 的 fast-forward/正常 merge，冲突则停止报告。

## Migration Plan

1. 提交 OpenSpec 规划工件并通过 strict validate。
2. 增加 Rust/Web acceptance，确认缺场景/标签导致有效 RED。
3. 实现最小 scenario/catalog/dispatch/i18n，转 GREEN。
4. 运行 targeted、broader、fmt/clippy/build 与真实 Chrome。
5. 独立 reviewer无 High/Medium 后提交 feature；安全集成至 main并推送。

回滚以最终提交为边界；不需要数据迁移或 schema version变更。

## Open Questions

无阻塞项。完整 S5-V/S5-C 仍由后续单独 change 处理。
