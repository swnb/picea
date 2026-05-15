# picea-lab 场景运行配置与参数 UI 目标驱动开发计划

状态：已执行并完成 V1 浏览器验收
计划文档：docs/plans/2026-05-15-picea-lab-scene-runtime-config-milestones.md
最后更新：2026-05-15
工作目录：/Users/asyncrustacean/projects/picea
工作区状态：dirty；本轮已按该计划推进实现、测试和浏览器验收，仍不提交。
提交策略：不提交
执行策略：已按 D1/D2 -> E1/E2/E3 -> V1 推进；后续独立 Apply / inline invalid 输入可作为 UI hardening 继续拆里程碑。

## 目标

把 `picea-lab-web` 的运行参数设计成“场景拥有配置 + UI 暴露可调参数 + 运行结果记录 effective config + 场景化验收”的目标驱动流程。

最终用户可感知结果：

- 选择一个场景后，参数面板展示该场景自己的可调参数。
- 参数控件简约统一，包含 slider / range / number input / toggle / segmented control 等必要操作细节。
- 参数 UI 由封装组件完成，不在页面中散写表单。
- 用户能清楚知道参数是默认值、用户覆盖、运行中配置还是只读事实。
- 每次 artifact / live run 都能看到并复现 effective config。
- 牛顿摆这类场景有明确验收指标：绳长误差、径向速度、动能包络、冲量传递、position correction、warm-start 状态。

## 约束

- 以 `/Users/asyncrustacean/projects/picea` 当前工作区为准，不创建 worktree。
- 本计划阶段只写文档，不改生产实现代码。
- `picea-lab-web` 继续作为 Rust artifact/live facts viewer，不在浏览器里运行 physics。
- 参数入口挂在场景，不以独立全局 `SolverProfile` 作为产品中心。
- UI 必须复用/扩展现有 `components/ui/*` 风格。
- 验证命令继续使用 `rtk proxy`。

## 待确认问题

### 必须确认

- 无。当前按“场景拥有配置，Profile 最多是场景内部快捷预设”的方向推进。

### 可带假设推进

- 第一版参数修改默认 `NextRun` 生效：影响 live 运行中不会随 slider 拖动改变世界；本轮实现采用“重新运行场景/启动新运行”作为 apply 边界。
- 第一版先开放牛顿摆参数和少量 solver policy 参数：影响范围控制；早期验证：Stack/Matrix stack 不受影响。
- `RangeField` 第一版只用于验收窗口/阈值，不用于复杂曲线编辑：影响 UI 复杂度；早期验证：验收 tab 有可读区间配置。

## 规划依据

- explorer：已跳过；当前工具边界只允许在用户明确要求 subagent 时启动。本轮由主 Codex 读取仓库文档和前端组件结构。
- 关键证据：
  - `docs/ai/repo-map.md`：`scenario` 负责内置场景和 reset-time overrides，`artifact` / `server` 分别负责 artifact/live。
  - `crates/picea-lab/web/src/components/ui/input.tsx` / `button.tsx` 已有统一基础组件风格。
  - `crates/picea-lab/web/src/components/workbench/GravityDial.tsx` 已有参数控件封装经验。
  - `crates/picea-lab/web/src/components/workbench/Inspector.tsx` 已有事实卡片和右侧面板组织方式。
  - 既有诊断路径要求显式区分 source，不能把缺失证据当作 0。
- 主要未知：
  - `SceneRuntimeConfig` 是否一开始进入 public API，还是先作为 lab 内部 schema。
  - live session 是否需要在本轮支持除重力之外的 running-world apply。
  - 参数 schema 是否由 Rust 全量生成，还是 Rust 提供 schema + Web 做 presentation mapping。

## 架构设计输入

- software-architecture-design：已产出 `docs/design/scene-runtime-config-and-parameter-ui-design.md`
- 关键接口：
  - `ScenarioDescriptor.default_runtime_config`
  - `ScenarioDescriptor.parameter_schema`
  - `SceneRuntimeConfig`
  - `effective_runtime_config`
  - 参数 UI 组件：`ParameterPanel`、`ParameterRow`、`NumberField`、`SliderField`、`RangeField`、`SegmentedField`、`ToggleField`
- 关键取舍：
  - 选择场景拥有配置，放弃独立全局 profile 作为产品中心。
  - 选择组件化参数 UI，放弃页面内散写表单。
  - 选择 explicit effective config 记录，放弃“调参靠 UI 状态回忆”。

## 计划验收

- 状态：已完成本轮 V1。
- Design Gate：已形成 `docs/design/scene-runtime-config-and-parameter-ui-design.md`。
- Execution Gate：已执行 E1/E2/E3/V1。
- reviewer：已运行只读 review；高优先级 `joint_velocity_projection` 开关语义问题已修复并补行为锁。
- 审查结论：通过本轮完成；独立 Apply / inline invalid 输入仍作为 UI hardening 后续项。
- 用户确认：待最终确认。

## 设计 / 执行分离规则

- 设计里程碑只允许修改计划文档、设计文档、调研记录或测试计划，不修改生产实现代码。
- 执行里程碑必须引用已批准的设计结论。
- 如果执行中发现设计假设不成立，暂停执行并更新设计文档与本计划。
- 设计里程碑完成不等于允许实现；实现仍需经过 Execution Gate 或用户确认。

## 目标验收标准

### 产品 / UI 验收

- 参数面板由场景驱动，切换场景后参数组和默认值随场景变化。
- UI 控件必须简约统一，沿用 `lab-panel`、`lab-line`、`lab-muted`、`lab-accent` 风格。
- 数字参数同时提供 slider 和 number input；slider 粗调，number 精调。
- range 参数提供双端 slider / range 与双 number input。
- 布尔参数使用 toggle；枚举参数使用 segmented control 或现有 Select。
- 所有参数都有 label、help、unit、min/max/step/default、source badge。
- 非法值显示 inline error，Apply disabled，不静默修正。
- 参数组件必须封装复用，不允许在 `App.tsx` / `Inspector.tsx` 中散写重复表单。
- 移动/窄宽度下文字不溢出、不遮挡、不造成布局跳动。

### 可复现验收

- `/api/scenarios` 返回参数 schema 和默认 runtime config。
- artifact manifest 记录 `effective_runtime_config` 和 config hash。
- live session metadata 或 frame summary 可以查看当前 effective config 摘要。
- copy debug context 包含 scene id、参数摘要和 effective config hash。
- 同一 scene + effective config 的 deterministic artifact run 可以复现。

### 牛顿摆验收

- UI 暴露牛顿摆关键参数：球数、半径、绳长、释放角/偏移、restitution、friction、solver iterations、contact position correction、joint velocity projection。
- 验收 tab 显示最大绳长误差、最大径向速度、动能包络、右球/中间球峰值比、position correction 总量、warm-start hit/miss/drop。
- 6000 帧 artifact 验收不能只看截图，必须读取 artifact facts。
- 浏览器 live 验收至少运行 60 秒，确认无球体闪离、无绳索断裂、UI 不闪屏。

### 回归验收

- Stack / Matrix stack 场景仍使用适合稳定性的默认配置。
- 牛顿摆参数不会成为全局 solver 默认。
- 现有 live gravity narrow apply 语义不被扩大成 arbitrary running-world patch。
- Web contract、i18n、build 和 Rust lab/core 测试通过。

## 里程碑映射

| 设计里程碑 | 产出 | 解锁的执行里程碑 | 状态 |
| --- | --- | --- | --- |
| D1 | 场景配置 schema / effective config / UI 控件规范冻结 | E1, E2, E3 | 已完成 |
| D2 | 验收指标与牛顿摆目标锁定 | E3, V1 | 已完成 |

## 里程碑

### D1：冻结场景运行配置与参数 UI 规范

类型：设计
状态：已完成

目标：
确认 `SceneRuntimeConfig`、`parameter_schema`、effective config 合并语义和参数控件组件规范。

范围：
- 更新 `docs/design/scene-runtime-config-and-parameter-ui-design.md`。
- 明确 Rust descriptor、server API、web schema renderer 的边界。
- 明确 slider / range / number input / toggle / segmented control 的 UI/UX 标准。

不做：
- 不改 Rust 实现。
- 不改 React 实现。

验收标准：
- 设计文档包含架构图、数据模型、接口草案、UI 组件表和取舍。
- 文档明确“配置挂在场景，不以独立 profile 为中心”。
- 文档明确参数 source、draft、Apply/Reset/Revert 语义。

验证方式：
- 文档 diff review。
- 后续 reviewer gate。

### E1：Rust 场景配置 schema 与 effective config 记录

类型：执行
状态：已完成
来源设计：D1

目标：
让场景在 Rust 端声明默认 runtime config 和参数 schema，并让 artifact/live 记录 effective config。

范围：
- `crates/picea-lab/src/scenario.rs`
- `crates/picea-lab/src/artifact.rs`
- `crates/picea-lab/src/server.rs`
- 对应 Rust tests

不做：
- 不开放任意 running-world patch。
- 不实现前端参数 UI。

验收标准：
- `/api/scenarios` 包含默认配置和 schema。
- artifact manifest 包含 effective runtime config。
- live session 创建后可查询 effective config 摘要。
- Newton cradle 使用场景内配置覆盖 solver policy。

验证方式：
- `rtk proxy cargo test -p picea-lab --test server_routes`
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy cargo test -p picea --lib`

### E2：参数 UI 组件库与 schema renderer

类型：执行
状态：已完成
来源设计：D1

目标：
新增统一参数控件组件，并用 schema 渲染参数面板。

范围：
- `crates/picea-lab/web/src/components/ui/*`
- `crates/picea-lab/web/src/components/workbench/parameters/*`
- `crates/picea-lab/web/src/types.ts`
- `crates/picea-lab/web/src/api.ts`
- `crates/picea-lab/web/src/i18n.ts`
- `crates/picea-lab/web/scripts/*contract.mjs`

不做：
- 不在 UI 中运行 physics。
- 不为每个场景手写重复表单。

验收标准：
- `NumberField`、`SliderField`、`RangeField`、`ToggleField`、`SegmentedField` 可复用。
- slider + number input 联动。
- range + 双 number input 联动。
- source badge、dirty 状态、Apply/Reset/Revert 可见。
- 非法值阻止 Apply 并显示错误。
- UI 风格与现有 workbench 一致。

验证方式：
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- Browser 验收：切换场景、拖 slider、输入 number、range 联动、Reset/Revert/Apply。

### E3：牛顿摆场景参数面板与验收面板

类型：执行
状态：已完成
来源设计：D1, D2

目标：
以牛顿摆作为第一条完整场景参数化样板，展示可调参数并提供场景化验收。

范围：
- 牛顿摆 `SceneRuntimeConfig`
- 牛顿摆参数 schema
- 验收 tab / inspector summary
- artifact diagnostics 派生指标，如能量包络、绳长误差、径向速度

不做：
- 不把牛顿摆配置变成全局默认。
- 不承诺本里程碑直接解决 100 秒完全无损；先把指标暴露并锁定目标。

验收标准：
- UI 可调整球数、半径、绳长、释放角/偏移、restitution、friction、solver iterations、contact correction、joint velocity projection。
- 运行结果展示 effective config。
- 6000 帧 artifact 显示牛顿摆验收指标。
- live 运行可观察参数改变后的效果。

验证方式：
- `rtk proxy cargo test -p picea-lab --test artifact_run newton_cradle -- --nocapture`
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- Browser 验收：Newton cradle 参数调节 + 60 秒 live run。

### V1：全链路验收与回归确认

类型：验证
状态：已完成

目标：
证明场景参数化没有破坏已有场景、已有 live/replay 语义和 UI 稳定性。

范围：
- Rust core/lab targeted tests。
- Web contract/i18n/build。
- Browser 验收。
- 文档更新。

验收标准：
- Newton cradle 参数化样板通过。
- Stack / Matrix stack 仍稳定。
- Artifact replay / live session 均可查看 effective config。
- UI 控件无明显布局抖动、溢出、遮挡。

验证方式：
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy cargo test -p picea --lib`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`
- `rtk proxy npm --prefix crates/picea-lab/web run build`
- Browser：牛顿摆、矩阵堆叠、运行设置、参数面板、验收面板。

本轮实测记录：

- `rtk proxy cargo test -p picea`：通过。
- `rtk proxy cargo test -p picea-lab`：通过。
- `rtk proxy cargo test -p picea-lab --test artifact_run newton_cradle -- --nocapture`：5 项牛顿摆定向测试通过。
- `rtk proxy npm --prefix crates/picea-lab/web run test:ui-contract`：通过。
- `rtk proxy npm --prefix crates/picea-lab/web run test:i18n`：通过。
- `rtk proxy npm --prefix crates/picea-lab/web run build`：通过。
- `rtk proxy cargo fmt --all -- --check`：通过。
- Browser：`http://127.0.0.1:5173/` 牛顿摆 artifact replay 6000 帧，最终第 5999 帧 / 100.000s / 96000 steps，未见球体闪离或绳索断裂，console warn/error 为空。

## Plan Gate

当前计划已完成本轮执行。后续若继续 hardening，需要确认：

- 是否继续补独立 Apply / inline invalid 输入状态。
- 是否把验收 tab 的动能包络、径向速度等指标做成显式图表，而不只依赖 artifact tests 和 inspector facts。
- 是否给堆叠 / 矩阵堆叠也补场景参数 schema。

后续执行策略：

- 先补 UI hardening 的失败输入/独立 Apply 行为锁。
- 再补牛顿摆 acceptance panel 的显式图表。
- 最后扩展更多场景的参数 schema。
