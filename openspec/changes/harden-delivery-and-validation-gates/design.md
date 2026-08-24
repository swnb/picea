## Context

当前 `main` 已完成 Revolute S5 收尾，工作区起点为 clean
`671a216bf92d6a5723540b1e36f78446156a9dab`。用户在 2026-07-29 明确授权
自主批准规划、完整实施和本地验收，但未授权 commit、push、PR、archive 或 publish。
core 单元门、lab、macro、examples 和 Web build 都能本地通过，但仓库没有
`.github` workflow；常用命令散落在 README、计划与会话证据中。

规划后 fresh 执行的 matrix-stack 证据为：240 帧 support-gap PASS（9.09s）、
600 帧 no-ejection PASS（23.50s）、1200 帧 sleep convergence PASS（58.07s）；
三者均 max penetration `0.027232`、no ejection、final outside `0`、
`0 awake / 48 sleeping`、quiet linear/angular `0/0`。两个旧 ignored observation
分别在 8.13s/24.62s 以 exit 101 失败，因为它们断言旧版 penetration/ejection/
non-convergence 必须仍存在；这证明旧 oracle 已被当前修复推翻。Web build 仍输出
单个 588,233-byte 主 chunk。M29 只批准 baseline/warn/fail policy，没有批准绝对
wall-clock hard threshold。首次 fresh `npm ci` 还报告 9 个 high severity；
`npm audit --omit=dev` 为 0，完整 audit 的 9 条均汇聚到 dev-only
`postcss@8.5.10`，安全修复下限为 `8.5.18`，现有直接依赖与 peer ranges
兼容该补丁版本。

本 change 的受众是本地开发者、CI runner 和后续 milestone supervisor。所有门必须以仓库脚本为事实源；GitHub workflow 只负责环境准备、调用与 evidence upload。

## Goals / Non-Goals

**Goals:**

- 建立 fast、full、nightly、release 四个命名稳定、可本地复跑的验证 profile。
- 让 PR/main 与 scheduled/manual CI 调用相同脚本，并采用最小权限和并发取消。
- 把 matrix-stack 当前正向事实转成默认或 nightly hard gate，删除已经失真的负向 observation test。
- 采集 Criterion 与 deterministic counter evidence，同时维持“wall-clock 不是 correctness oracle”。
- 通过确定性 chunk 分组和 bundle contract 消除 Web 大 chunk warning。
- 清除 Web 构建依赖的已知 high severity audit findings，不用 force/major upgrade 扩大行为风险。
- 让可发布 crate 能完成 package verification，并明确阻止 `picea-lab` 被发布。

**Non-Goals:**

- 不修改 solver、contact、CCD、step cadence、public API、artifact/schema 或 UI 产品行为。
- 不新增 direct-concave、多线程 solver、deformable、Revolute motor/limit 或 1.0 surface。
- 不在未验证候选工具链前声明 MSRV；不在没有五轮可比 baseline 前设置绝对时间 hard threshold。
- 不执行 OpenSpec archive/sync、commit、push、PR、deploy 或 crates.io publish。

## Decisions

### 1. 仓库脚本是 CI 的单一命令事实源

新增 `scripts/ci/run.sh <fast|full|nightly|release>`。脚本在本地检测到 `rtk` 时通过 `rtk proxy` 运行工具，并把 npm cache 隔离到仓库 `target/`，避免用户级历史权限污染验收；在标准 CI runner 上直接运行同一命令并保留 setup-node 的标准 cache 行为。`Justfile` 只提供易发现的薄别名，GitHub workflows 也只调用该脚本。

没有把完整命令复制进 workflow，因为重复清单会再次产生状态漂移；没有要求 CI 安装本机专用 RTK，因为它不是产品构建依赖。
所有会解析、测试、构建或打包 Cargo 依赖的命令均使用 `--locked`，使 manifest
与受版本控制 `Cargo.lock` 的漂移在本地和 hosted runner 上同样 fail closed。

### 2. 验证按反馈时间和证据性质分层

- `fast`：格式、core lib、public example、Web 静态 contracts 与 production build。
- `full`：workspace all-targets tests、strict Clippy、macro compile contracts、Web 完整 contracts/dev-server contract。
- `nightly`：600/1200 帧 matrix hard gates、Criterion baseline/evidence。
- `release`：package metadata、package content 与从 package tarball 构建验证。

PR 执行 fast/full；main 同样执行以防绕过；schedule/manual 执行 nightly/release。
fast/full 保持 fail-fast；nightly 会尝试收集两条 matrix 与 Criterion 三组证据，
最后聚合返回非零。Workflow 不允许用 `continue-on-error` 把 correctness gate改成提示。

### 3. Matrix 历史失败与当前正向门分离

240 帧 support-gap 门转为普通测试。600 帧 no-ejection/runaway 与 1200 帧 sleep-convergence 保持成本较高的 ignored test，但去掉“future”与环境变量双重跳过，nightly 必须按 exact test name 用 `--ignored --exact` 执行。两个断言旧故障必须存在的 observation test 删除；其历史失败帧与根因仍保留在既有 milestone 文档。

没有调宽任何 penetration/speed/sleep 阈值，也没有修改 solver；本轮已在 live `main` 上逐项证明三个正向门为 GREEN。

### 4. 性能门硬分 deterministic correctness 与 wall-clock evidence

现有 counter/order/determinism tests 继续作为硬门。Criterion在nightly使用固定baseline
label运行，verifier只接受本轮开始时间之后写入的对应`benchmark.json`，从而隔离
本地`target/criterion`历史目录；clean CI保留完整`target/criterion` evidence。
时间变化只用于review，不设置新的绝对fail值。后续只有满足M29的多轮、同runner、
review-approved条件才能升级时间阈值。

### 5. Web 使用可解释的 vendor chunk 分组与产物预算

Vite 将 React 核心依赖分到 `react-vendor`，将 Radix 与 icons 等 UI 依赖分到
`ui-vendor`；业务代码不做行为级 lazy-loading。构建后 contract 扫描所有 JS
asset，要求每个文件不超过 500,000 bytes、存在多个 JS chunks，且入口 HTML
引用的文件实际存在。

没有先拆 `App.tsx`/`Timeline.tsx` 状态所有权，因为那是更高行为风险的独立重构；manual chunks 能先解决交付问题且不改变 runtime facts。

### 6. Release hardening 不猜 MSRV

`picea` 与 `picea-macro-tools` 修正 crates.io metadata并执行`cargo package`
verification；`picea-lab`设置`publish = false`，避免把本地 artifacts或工具壳误发布。
本地 implementation worktree 使用`--allow-dirty --locked`只生成“dirty candidate
可打包/可构建”receipt；clean checkout 不传`--allow-dirty`。由于本轮不push，
GitHub-hosted clean receipt保持`NOT RUN / UNKNOWN`，不得称 publish-ready或
reproducible。CI使用stable Rust，但本 change不填`rust-version`，因为当前主机默认
是未来nightly，且本地没有已验证的候选MSRV toolchain。

### 7. Dev-only 漏洞仍作为构建供应链 hard gate

生产依赖 audit 已是 0，但 dev-only PostCSS 参与 CSS 构建，不能因浏览器 runtime
不可达就忽略。将根 `postcss` 约束的安全下限提升到 `^8.5.18` 并刷新 lock；
不使用 `npm audit fix --force`，不升级 Vite/Tailwind major。`full` 在 fresh
`npm ci` 后执行完整 `npm audit --audit-level=high`，确保生产和构建依赖树都没有
high/critical finding。`fast` 保持有界反馈时间，不重复联网 audit。

## Risks / Trade-offs

- [PR full gate 仍可能较慢] -> fast/full 分 job 并允许 GitHub 并行；长 matrix 和 Criterion 只进 nightly。
- [GitHub runner 与本机性能不可比] -> wall-clock 只上传 evidence，不作 correctness hard fail。
- [manual chunk 分组随依赖升级变化] -> bundle contract检查最终产物，不绑定 hash 文件名。
- [nightly exact test 被重命名后静默缺失] -> 脚本先用 `cargo test -- --list`/exact invocation，显式检查list命令状态，并只接受唯一完整测试名；0 tests 视为失败的wrapper contract。
- [manifest修改但漏更Cargo.lock] -> 所有依赖解析与构建命令使用`--locked`，禁止runner自动改写lockfile后误绿。
- [package verification依赖 registry/network] -> release profile独立于 PR correctness；失败如实报告，不改为 `--no-verify` 冒充通过。
- [npm audit依赖registry且公告可能变化] -> 只在full执行并保留非零失败；本轮通过精确安全补丁闭环，不使用force或major自动重写。
- [本轮无法激活远端 workflow] -> 本地只验证 YAML、profiles 与候选内容；最终结论明确 remote CI `NOT RUN / UNKNOWN`。

## Migration Plan

1. 先增加 bundle/profile contracts并确认旧入口产生预期 RED。
2. 实现脚本、workflow、matrix 调度、chunk 分组、dev-build安全补丁和 package metadata。
3. 运行 targeted profiles，再运行 full、nightly、release。
4. 独立 spec/code review闭合 High/Medium，更新 tasks receipt 与文档路由。

回滚以本 change 的新增 workflow/scripts及局部配置为边界；没有数据迁移。任何 physics regression 都应停止并撤回本 change 的调度/配置修改，而不是在本 change 内改 solver。

## Open Questions

无阻塞项。正式 MSRV、semver API diff baseline 与绝对性能阈值均保留为后续独立 change。
