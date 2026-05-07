# Matrix Stack 稳定性验收设计

> 日期：2026-05-06
>
> 状态：D2 设计产出，服务于
> `docs/plans/2026-05-06-matrix-stack-stability-optimization-milestones.md`。

本文冻结 matrix stack 稳定性优化的第一版验收门。它补充
`docs/design/stack-stability-repro-diagnostics.md`，不覆盖既有 performance /
stability diagnostics 复现文档。

## 场景分层

| 场景 | 角色 | 用途 |
| --- | --- | --- |
| `stack_4` | Clean behavior lock | 小规模、低噪声，用于写 hard gate 和 TDD 回归。 |
| `matrix_stack` 默认 `8x6` | Dense stress gate | 高密度矩阵堆叠，用于观察 solver 压力、churn、penetration、sleep blocker。 |
| 浏览器 `矩阵堆叠 8x6` | Product acceptance | 验证用户能看到同一条 diagnostics story，不作为独立 physics oracle。 |

不在 D2 新增场景。若 `stack_4` 仍然太噪，后续单独开里程碑新增小矩阵 fixture。

## Frame Windows

| Gate | 场景 | 帧数 | 作用 |
| --- | --- | --- | --- |
| Smoke | `stack_4` | 120 | 确认 artifact、diagnostics、无 numeric warning。 |
| Behavior lock | `stack_4` | 300 | 后续 E1-E5 的 hard gate 主窗口。 |
| Stress report | `matrix_stack` | 180 | 固定当前 dense failure story 和 first-bad-frame。 |
| Long-settle observation | `matrix_stack` | 600 | 非默认；当 E5 需要评估 sleep 收敛时启用。 |

## Hard Gates

这些 gate 应优先落在 `stack_4`。E1 可以先以 diagnostic report 或 ignored
target test 锁住当前失败，但最终进入 V1 时 hard gates 必须是普通通过测试。

| Gate | 阈值 | 说明 |
| --- | --- | --- |
| Determinism | 相同场景、相同帧数的最终 `state_hash` 稳定。 | 防止优化引入非确定性。 |
| Numeric safety | `numeric_warnings == 0`。 | 非有限数值优先级最高。 |
| Bounded penetration | frame 120 后 `penetration_max <= 0.04` 且 `penetration_sum <= 0.16`。 | `0.04` 是当前 correction slop 的 8 倍，先作为现实目标。 |
| Churn settles | frame 180 后 24 帧 rolling window 中 `enter + exit <= 2`。 | 允许早期 settling，不允许后期接触身份持续洗牌。 |
| Warm-start continuity | frame 180 后持续接触窗口内 warm-start drop 为 0；miss 必须能解释为新接触。 | 防止错误 drop 持续破坏收敛。 |
| Quiet or sleeping | frame 240 后动态体全体 sleeping，或最大线速度 / 角速度进入 D2 quiet window。 | 第一阶段允许 quiet window 替代全部 sleeping，避免过早强制 sleep。 |

Quiet window 的第一版定义：

- 24 帧 rolling window；
- 最大线速度不超过 `0.04` world units / second；
- 最大角速度不超过 `0.08` rad / second；
- 窗口内无 severe penetration / churn / warm-start marker。

## Stress Gates

`matrix_stack 8x6` 第一阶段不要求全绿。它必须产出报告，并且报告要可比较。

| 指标 | 第一阶段要求 |
| --- | --- |
| Dynamic body count | 48。 |
| Static body count | 1。 |
| Diagnostics | penetration、churn、warm-start、sleep、island、solver row facts 不应 missing。 |
| First-bad-frame | 必须给出 earliest warning/severe marker 和 state hash。 |
| Penetration | 记录 max/sum/top contact，不在 E1 作为 hard fail。 |
| Churn | 记录 entered/persisted/exited 和 involved bodies。 |
| Sleep | 记录 awake/sleeping counts；若全部 awake，必须记录 blocker 或 missing blocker。 |
| Solver rows | 记录 max row count、spike frame、row initial/final relative speeds、bias 和 impulse delta。 |
| Dense support friction | 若 E4 使用 shallow support friction budget，记录最大预算、出现帧和使用 contact 数。 |
| Late velocity spike | floor ejection 被压住后，记录 quiet-window 线速度/角速度 blocker frame 的接触、impulse、bias、support friction、correction、churn 和 late support-gap 摘要；support-gap 摘要必须包含缺口前最后 contact 的 normal impulse / normal speed / blocker speed / counterpart speed / tangent linear-angular components / correction depth / correction translation，并记录 blocker-side tangent linear+angular energy onset frame 以及 onset 到最后支撑 contact 的 lifecycle，用于区分“能量逐步积累”“normal impulse 先丢失”“position correction 随后停止”和“gap 后突发”。 |
| Early pressure trace | E4 继续追踪 frame `12..46` 的切向速度、角速度、position correction、row-level residual correction provenance、peak row contact-graph coupling、churn、warm-start drop/reason 分布、anchor drift normal/tangent 分解、row count 和 support friction 峰值，用于定位晚期 spike 之前的能量注入。 |

E2-E5 后，V1 可把部分 stress 指标提升为 behavior gate；提升前必须更新本文。

## Browser Acceptance

浏览器只验证 diagnostics story 是否可见且一致：

- 用 `rtk proxy just picea-lab-web-start` 启动本地 API/Web。
- 打开 `http://127.0.0.1:5173/`。
- 选择 `矩阵堆叠 8x6`。
- 运行 Rust artifact replay。
- 检查 scene tree 显示 49 colliders、48 dynamic bodies、1 static floor。
- 检查 Diagnostics 面板显示 penetration、churn、warm-start、sleep、island、solver rows。
- 检查 console error/warn 为空。

如果 `browser-use:browser` 不可用，允许降级 Playwright CLI 或手动浏览器检查，但必须记录降级原因。

## Live Realtime Acceptance

`Rust live session` 是产品体验验收项，不替代 artifact correctness gate。大矩阵下需要单独记录：

- live step API 的响应耗时：avg / p95 / max。
- 单步响应体大小。
- 浏览器 long task 数量和最大时长。
- 当前 UI 是否仍能稳定显示最新 authoritative frame。

2026-05-06 的浏览器验收显示，`matrix_stack 8x6` 的实时卡顿主要不是单个前端组件的小问题：

- 小场景 `falling_box_contact` live step 前 20 步平均约 `30.5ms`，响应约 `7.4KB`。
- `matrix_stack 8x6` live step 前 20 步平均约 `60.9ms`，p95 约 `92.4ms`，响应均值约 `280KB`。
- 矩阵 120 步采样曾出现 long task，说明完整 debug frame 的生成、序列化、传输和 React 全量消费叠加后，已超过 30Hz 实时预算。

因此实时验收的第一阶段目标不是让 `matrix_stack 8x6` 强行维持 30Hz，而是明确标注为 degraded realtime，并进入单独优化方案：

1. 把 authoritative simulation step 与 debug snapshot export 解耦。
2. live playback 默认传输 lightweight frame，用于 canvas 位姿和状态条。
3. diagnostics / broadphase tree / full inspector 按需拉取或低频采样。
4. 对大矩阵启用 adaptive playback cadence，避免前端把 30Hz 控制文案伪装成实际 30Hz。

## First-Bad-Frame 报告

E1/V1 的报告必须包含：

```markdown
## Matrix Stack Stability Report

- Scenario:
- Frame count:
- Run path:
- Final state hash:
- First bad frame:
- Markers:
- Penetration max / sum:
- Contact churn:
- Warm-start hit / miss / drop:
- Sleep awake / sleeping:
- Sleep blocker speeds:
- Quiet window f>=120:
- Late linear spike trace:
- Late angular spike trace:
- Early pressure trace f=12..46:
- Late correction/churn f>=120:
- Dense support friction:
- Floor ejection:
- Ejection trace:
- Ejection dynamic-support trace:
- Ejection dynamic-impulse trace:
- Ejection velocity trace:
- Ejection support-gap trace:
- Solver row max:
- Missing evidence:
- Suspected next milestone:
- Verification commands:
```

## E1 输入

E1 的最小目标是：

- 为 `stack_4` 建立 clean behavior lock 的测试入口。
- 为 `matrix_stack 8x6` 建立 stress report。
- 不修改 solver。
- 不新增 parallel diagnostics schema。
- 如果当前 hard gate 失败，保留为明确的 red/target 测试或 diagnostic report，不让常规 suite 长期红。

## V1 输入

V1 必须证明：

- `stack_4` hard gates 通过；
- `matrix_stack 8x6` stress report 的 first-bad-frame story 可解释；
- 浏览器 UI 与 artifact report 一致；
- 如果 `matrix_stack 8x6` 仍未完全稳定，剩余问题被归到 E2/E3/E4/E5 之外的新计划，而不是藏在“通过”里。
