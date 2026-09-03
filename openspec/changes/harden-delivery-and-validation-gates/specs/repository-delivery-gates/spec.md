## ADDED Requirements

### Requirement: Canonical validation profiles

仓库 SHALL 提供 fast、full、nightly、release 四个命名稳定的验证 profile；本地入口与 CI MUST 调用同一仓库脚本，任一 hard gate 失败 MUST 使所属 profile 最终返回非零状态。除不解析依赖的格式检查外，Cargo 验证 MUST 使用受版本控制的 lockfile 并在 manifest/lockfile 漂移时 fail closed。fast/full SHALL fail-fast；nightly SHALL 尝试执行全部独立 evidence 子门后聚合结果。

#### Scenario: Developer reproduces a CI profile locally
- **WHEN** 开发者在仓库根目录运行任一受支持 profile
- **THEN** 脚本执行与 CI 相同的有序命令集合；fast/full在第一条失败命令处退出，nightly尝试全部独立evidence子门后聚合非零状态

#### Scenario: Unknown profile is rejected
- **WHEN** 调用者传入未声明的 profile
- **THEN** 脚本输出受支持 profile并以用法错误退出

#### Scenario: Cargo lockfile drift is rejected
- **WHEN** manifest修改后未同步受版本控制的`Cargo.lock`
- **THEN** 任一会解析或构建Cargo依赖的profile非零失败，不得由runner自动更新lockfile后继续误绿

#### Scenario: Stable macro diagnostics preserve the expected syntax range
- **WHEN** stable 或 nightly Rust 编译被宏合同拒绝的 attribute/meta 输入
- **THEN** 两者输出同一错误文本并覆盖同一完整语法节点，既有 compile-fail 快照无需按工具链降级或跳过；接受规则和生成 API 保持不变

#### Scenario: Full CI provisions the recipe runner used by its contracts
- **WHEN** full job 在 hosted runner 上准备 Web dev-server contract
- **THEN** workflow 显式安装固定且已验证的 just 版本，不能依赖本机 PATH 或未声明的镜像预装工具

#### Scenario: Full CI has the source required by compile-fail diagnostics
- **WHEN** full job 从 minimal Rust 工具链准备环境
- **THEN** 它显式安装 `rust-src`，使包含标准库源码提示的既有 compile-fail 合同与本地验证一致；本地前置条件在验证文档中可发现

### Requirement: Layered continuous integration

仓库 SHALL 在 pull request 与 main 更新时执行 fast/full correctness profiles，并在 scheduled 或 manual workflow 中执行 nightly/release profiles。Workflow MUST 使用 `pull_request` 而不是 `pull_request_target` 执行 PR code，MUST NOT 向 PR code 注入 secrets，MUST 使用只读仓库权限、不可变 action SHA、取消同 ref 的过期运行，且不得用容错标志吞掉 hard-gate failure。

#### Scenario: Pull request receives bounded feedback
- **WHEN** pull request 修改 Rust、Web、验证脚本或相关配置
- **THEN** fast 与 full profiles 在独立 job 中运行并分别报告结果

#### Scenario: Long evidence runs outside the PR critical path
- **WHEN** nightly schedule 或手动触发发生
- **THEN** matrix 长窗口、Criterion evidence 与 package verification运行，相关 artifacts 在可用时被保存

### Requirement: Matrix-stack stability acceptance

240 帧 support-gap 验收 SHALL 是普通 hard test；600 帧 no-ejection/runaway 与 1200 帧 sleep-convergence SHALL 是 nightly exact-name hard gates。验收 MUST 保持现有 penetration、speed、floor-support 与 sleeping 阈值，不得通过放宽阈值获得绿色。

#### Scenario: Current stack remains supported
- **WHEN** 执行 240 帧 matrix-stack support-gap test
- **THEN** 测试证明没有当前已修复的 ejection window，且静默窗口速度保持在批准阈值内

#### Scenario: Nightly proves long-window convergence
- **WHEN** nightly 分别执行 600 与 1200 帧 exact tests
- **THEN** 所有动态体保持在 floor support 内、无 runaway，最终 48 个动态体 sleeping

#### Scenario: Historical failure evidence is not a live oracle
- **WHEN** 当前实现已经推翻旧 ejection/non-convergence 故障叙事
- **THEN** 旧负向 observation仅保留在历史文档，不得继续作为断言故障必须存在的可执行测试

### Requirement: Performance evidence safety

Nightly SHALL 执行 Criterion 场景并保留可审计 evidence；冻结 manifest MUST 包含九个场景的 exact expected counter values，场景缺失、额外、重复或 exact mismatch MUST 失败。ordering 与 state facts MUST 继续是 hard correctness gates，wall-clock timing MUST NOT 在没有批准 baseline policy 时成为绝对 hard threshold。

#### Scenario: Benchmark evidence is collected
- **WHEN** nightly performance profile成功运行
- **THEN** Criterion 输出目录可供下载或本地审查，且运行失败会使 profile失败

#### Scenario: Timing variance does not redefine correctness
- **WHEN** 单次 runner wall-clock 相对历史样本波动
- **THEN** 系统保留该差异供review，但不在缺少批准多轮baseline时据此修改 physics 或宣称 regression

### Requirement: Web production bundle budget

Web production build SHALL 生成多个可解释的 JavaScript chunks，任一 JS asset MUST 不超过 500,000 bytes；bundle contract MUST 检查最终产物而不是依赖带 hash 的固定文件名。

#### Scenario: Production bundle stays within budget
- **WHEN** 执行 Web production build
- **THEN** build与bundle contract均通过，入口HTML引用的assets存在且没有JS asset超过预算

#### Scenario: Oversized chunk fails deterministically
- **WHEN** 依赖或代码变化使任一JS asset超过500,000 bytes
- **THEN** bundle contract列出超限文件和实际大小并非零退出

### Requirement: Web build dependency audit

Web full profile SHALL 在 fresh install 后审计完整 npm 依赖树；high 或 critical
finding MUST 使 full profile 失败。修复 SHALL 使用与现有 Vite/Tailwind/PostCSS
契约兼容的最小安全补丁，不得用 force major upgrade 隐藏兼容性风险。

#### Scenario: Build supply chain has no known high finding
- **WHEN** full profile完成 `npm ci`
- **THEN** 完整依赖树的 high/critical audit count为0，audit命令以0退出

#### Scenario: Dev-only finding remains blocking
- **WHEN** 漏洞只存在于CSS或打包工具等dev dependency
- **THEN** full profile仍非零失败，因为该依赖参与生产资产构建；不得以production runtime不可达为由跳过

### Requirement: Release package verification

可发布 crates SHALL 具有有效 license、repository、description、documentation、categories 与 keywords metadata，并通过 `cargo package` 内容和构建验证；本地工具 crate `picea-lab` MUST 明确禁止发布。Dirty implementation worktree 的 `--allow-dirty` receipt MUST 只称 buildable package candidate；只有 clean checkout receipt才可称 clean package verification，本 change未激活的remote CI MUST 标记为 `NOT RUN / UNKNOWN`。

#### Scenario: Dirty core package candidate is buildable
- **WHEN** 执行 release profile
- **THEN** `picea` 与 `picea-macro-tools` 在dirty本地候选上完成content/build验证，输出明确的dirty receipt且不宣称reproducible或publish-ready

#### Scenario: Clean package verification remains externally gated
- **WHEN** workflow尚未提交并在GitHub-hosted clean checkout运行
- **THEN** remote clean package verification状态为`NOT RUN / UNKNOWN`，不得由本地dirty receipt替代

#### Scenario: Clean checkout works on the local system shell
- **WHEN** 在受支持的本地 Bash 3.2 或 CI Bash 中执行 clean checkout 的 release profile
- **THEN** 两个可发布 crates 均以 `--locked` 且不带 `--allow-dirty` 完成 package 命令，不能因空参数数组与 nounset 的兼容性失败

#### Scenario: CI rejects dirty package input
- **WHEN** CI checkout 含 tracked 或 untracked 修改
- **THEN** release profile 在任何 package 命令前非零退出，不得以 `--allow-dirty` 降级验证

#### Scenario: Release command failures remain blocking
- **WHEN** metadata 检查或任一 package 命令失败
- **THEN** release profile 立即非零退出，且不得执行后续 package 命令或宣称验证通过

#### Scenario: Lab cannot be accidentally published
- **WHEN** Cargo读取 `picea-lab` manifest 或调用者尝试打包
- **THEN** manifest明确 `publish = false`，release profile验证该事实
