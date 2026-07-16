## ADDED Requirements

### Requirement: Revolute pendulum builtin scenario
系统 SHALL 暴露稳定标识 `revolute_pendulum` 的 builtin scenario，并通过 schema-v1 fixture 创建两个 bodies 与一个 pin-only revolute joint。初始两个 world anchors MUST 对齐，场景 MUST 产生可观察的自由相对旋转，且不得引入 motor、limits 或 damping。

#### Scenario: Catalog and fixture instantiate the capability
- **WHEN** 客户端列出场景并以 `revolute_pendulum` 创建 run 或 live session
- **THEN** 场景目录包含明确的 pendulum/hinge/free-rotation描述，实例化结果包含两个 bodies和一个 revolute joint

#### Scenario: Pendulum remains finite and pivoted
- **WHEN** authoritative pipeline运行该场景至少240个display frames
- **THEN** 两个 world anchors保持finite且无可见跳变或显著分离，dynamic body角度发生变化

### Requirement: Authoritative revolute debug facts
Artifact 与 server SHALL 沿既有 Rust `DebugSnapshot` / `StepStats` 路径原样输出 revolute kind、descriptor-order bodies、两个 world anchors 与一个 logical active joint row，不得降级成 existing kind或由Web重算。

#### Scenario: Artifact exports revolute facts
- **WHEN** artifact runner生成 `revolute_pendulum` frames
- **THEN** active frame包含恰好一个 `kind == "revolute"` joint、两个finite anchors和 `joint_row_count == 1`

#### Scenario: Live server preserves revolute facts
- **WHEN** 客户端创建该场景的live session并取得full frame
- **THEN** response保留相同kind、anchors、body endpoints和row count

#### Scenario: Old closed consumer rejects the new kind
- **WHEN** 只认识distance/world_anchor的closed debug consumer读取`"revolute"`
- **THEN** consumer明确返回unknown-variant错误，producer不得伪造fallback

### Requirement: Explicit Web consumption and localization
Web workbench SHALL 在场景选择器、Hierarchy、Inspector及Timeline/source evidence中显式消费 revolute facts，并在 `zh-CN` 与 `en-US` 提供明确标签。

#### Scenario: User inspects a revolute joint
- **WHEN** 用户在真实Rust live session选择`revolute_pendulum`并选择joint row
- **THEN** UI显示显式revolute标签、两个body endpoints、两个world anchors、authoritative source和joint row count

#### Scenario: Unsupported runtime kind is not silently accepted
- **WHEN** runtime返回TypeScript union与label map均未声明的joint kind
- **THEN** contract/build或可观察UI明确暴露unknown值，不能将其显示成distance或world_anchor

### Requirement: Chrome critical journey
真实 Chrome 验收 MUST 绑定同一40位HEAD、实际URL、时间、版本与1440x900 viewport，并运行目标场景到frame >= 240。

#### Scenario: Browser acceptance passes
- **WHEN** targeted/broader gates全绿后执行真实Chrome旅程
- **THEN** 场景、source、kind、anchors、row count与free-rotation/pivot检查全部PASS，console新增warning/error和关键network失败均为0，并保存至少三张repo外截图
