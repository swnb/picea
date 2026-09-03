## 1. Planning and scope gate

- [x] 1.1 记录 clean `main` Start HEAD、OpenSpec 1.6.0、旧 S5 状态已收口和用户自主 Plan Gate 授权
- [x] 1.2 完成 proposal、design 与 `repository-delivery-gates` spec，固定不改 physics/API/schema、不猜 MSRV/时间阈值、不发布
- [x] 1.3 运行 OpenSpec status、`validate --all --strict`、apply instructions 与 scope review；闭合全部 High/Medium

## 2. Acceptance evidence

- [x] 2.1 增加 Web bundle contract，在既有单 chunk build 上确认 `588,233 > 500,000 bytes` 与单chunk有效 RED
- [x] 2.2 保存 `.github`/runner缺失、`picea-lab`默认可打包且携带本地产物、profile runner缺失的有效 RED
- [x] 2.3 逐项验证240/600/1200帧matrix正向门GREEN，并证明两个旧负向observation因修复生效而exit101
- [x] 2.4 固定 Criterion 九场景 exact expected counter manifest，聚合全窗口`total_*`并明确 wall-clock只作evidence
- [x] 2.5 记录 fresh `npm ci` 的9个high RED、`--omit=dev`为0，以及所有finding汇聚到dev-only `postcss@8.5.10`

## 3. Canonical profiles and CI

- [x] 3.1 实现 `scripts/ci/run.sh` 的 fast/full/nightly/release profiles、RTK 本地适配、Cargo lockfile fail-closed、unknown profile拒绝和exact-test命令状态/唯一完整名保护
- [x] 3.2 在 `Justfile` 增加四个薄入口，且不复制 gate 命令
- [x] 3.3 增加最小权限、并发取消的 PR/main workflow，独立执行 fast/full
- [x] 3.4 增加 scheduled/manual nightly workflow，执行 nightly/release 并上传 Criterion evidence

## 4. Stability, performance, Web, and release hardening

- [x] 4.1 将 240 帧 support-gap 升级为普通 hard test，将 600/1200 帧改为无双重跳过的 nightly exact gates
- [x] 4.2 删除两个断言旧故障必须存在的 observation tests，并在稳定性文档保留历史失败与当前 superseded 状态
- [x] 4.3 实现 Criterion counter manifest verifier，并让 nightly 在 benchmark 后 hard-verify deterministic IDs
- [x] 4.4 实现 Vite vendor manual chunks 与 500,000-byte bundle contract，保持 UI/runtime facts不变
- [x] 4.5 完善 `picea`/`picea-macro-tools` package metadata并设置 `picea-lab publish=false`，不填未验证 MSRV
- [x] 4.6 只同步 `README.md`、`docs/ai/index.md`、`docs/ai/repo-map.md`、`docs/ai/doc-catalog.yaml`、`docs/design/{performance-threshold-policy,matrix-stack-stability-acceptance,matrix-stack-stability-optimization-design}.md`、`docs/plans/{2026-05-08-matrix-stack-physics-engine-stability-refactor-milestones,2026-06-17-physics-realism-vnext-milestones}.md`
- [x] 4.7 将dev-only PostCSS升级到兼容的安全补丁并让full profile执行完整high-level npm audit，不使用force/major升级

## 5. Verification and review

- [x] 5.1 运行 bundle/profile/metadata/manifest targeted contracts并闭合失败
- [x] 5.2 运行 fast 与 full profiles，记录 exact command、exit code、test count、warning和生成文件
- [x] 5.3 运行 nightly 与 release profiles，记录 matrix、Criterion counters、package content/build和耗时边界
- [x] 5.4 运行 Web production preview/browser smoke，核对真实 Rust source、关键交互、network与console
- [x] 5.5 独立 spec reviewer、code reviewer、workflow reviewer 与 fresh verifier findings-first审查，闭合全部 High/Medium/Low；remote GitHub CI明确为`NOT RUN / UNKNOWN`
- [x] 5.6 fresh `npm ci` 后验证单一安全PostCSS版本、完整audit 0、production-only audit 0与既有Web contracts/build

## 6. Closeout

- [x] 6.1 更新tasks receipt、OpenSpec status/`validate --all --strict`、docs YAML/链接/过期陈述、fmt/diff/git hygiene
- [x] 6.2 给出 supervisor Go/No-go，明确已运行项、未运行项、残余风险和未授权的 commit/push/archive/publish 边界

## Acceptance receipt (2026-07-29)

- 基线：clean `main` Start HEAD `671a216bf92d6a5723540b1e36f78446156a9dab`；最终只保留本 change 预期的 tracked/untracked 修改，`Cargo.lock` 无漂移。
- `bash scripts/ci/run.sh fast`：exit 0；fmt、139 个 core lib tests、public example compile、fresh npm install、UI/i18n/profile、production build和bundle contract通过。
- `bash scripts/ci/run.sh full`：最终 `--locked` 版本 exit 0；417 个顶层 tests通过、2 个nightly tests按设计ignored、38 个nested trybuild cases与9个benchmark smoke通过；strict Clippy、完整npm audit、Web contracts/build/dev-server通过。
- `bash scripts/ci/run.sh nightly`：最终默认 sample-size 20 版本 exit 0；600帧门23.85s、1200帧门48.07s；两者保持max penetration `0.027232`、no ejection、final outside `0`、`0 awake / 48 sleeping`和quiet speed `0/0`；Criterion verifier确认9个exact场景。
- `bash scripts/ci/run.sh release`：exit 0；metadata contract通过；dirty local candidate中`picea`为63 files、1.4MiB/235.1KiB compressed，`picea-macro-tools`为76 files、65.3KiB/17.3KiB compressed，二者均从package重新构建；registry cache写入警告不影响exit，receipt不得称clean/reproducible/publish-ready。
- Web production preview/browser smoke：真实Rust source返回成功；pause/single-step使frame 521变为522且state hash变化；matrix 8x6显示49 bodies；文档、CSS、三组JS chunks和Rust API均为2xx，network failure 0、console warn/error 0；验收服务已停止。
- 安全：fresh npm依赖树使用`postcss@8.5.25`与其兼容的`nanoid@3.3.18`，完整与production-only audit均为0；bundle为271,256/143,383/172,191 bytes，均不超过500,000。
- 审查：独立spec、code、workflow与fresh verifier最终均为0 High / 0 Medium / 0 Low；审查发现的unused import、exact discovery、文档命令/归因、Cargo lockfile和别名歧义均已修复并由相关门重跑。
- 治理：OpenSpec 3/3 strict pass；脚本语法、Node语法、JSON/YAML、8份修改文档本地链接、`just --list`、fmt和`git diff --check`通过。
- Supervisor结论：本地implementation acceptance为GO；GitHub-hosted clean CI、clean package receipt与任何publish/deploy仍为`NOT RUN / UNKNOWN`和NO-GO。本轮未执行commit、push、PR、archive、sync、deploy或publish。

## Custody refresh (2026-08-24)

- 用户授权把本 change 独立落到本地 `feat/harden-delivery-and-validation-gates`，供 S6 rebase；仍不授权 push、merge、PR、archive/sync、deploy 或 publish。
- fresh `fast` exit 0，但其中的 `npm ci` 发现新发布的 `nanoid<3.3.18` high advisory；当时 lock 为 `nanoid@3.3.16`，经 `postcss@8.5.25` 间接引入。该结果推翻 2026-07-29 的 audit 0 快照，不能沿用旧安全结论。
- 只执行 `npm update nanoid --package-lock-only --ignore-scripts`，将 transitive lock 收敛到兼容的 `nanoid@3.3.18`；未增加 direct dependency、未 force、未 major 升级。随后 live audit 为 0。
- fresh canonical `full` exit 0：workspace all-targets、strict Clippy、fresh `npm ci`、完整 high-level audit 0、UI/i18n/profile、Web build、bundle contract 和 dev-server contract 全通过；bundle仍为 271,256/143,383/172,191 bytes。
- OpenSpec `validate --all --strict` 为 3/3 PASS；Shell/Node/YAML syntax、fmt 与 `git diff --check` PASS。remote GitHub CI、clean package receipt、nightly/release 本次未重跑，继续沿用 `NOT RUN / UNKNOWN` 或历史 receipt 边界。

## 7. Delivery continuation (2026-09-03)

- [x] 7.1 核验原交付分支 `a7b3cb42`、远端 main `671a216b`、本地 S6 main `80f08927`，建立独立 worktree；记录用户继续 commit/push/PR/green merge 的授权与真实审批/线性历史约束
- [x] 7.2 完成本轮 proposal/design/spec 增量并通过 OpenSpec status、strict validate 和 apply instructions
- [x] 7.3 在实际 Bash 3.2 建立 clean release RED，最小修复并验证 clean/dirty local、clean/dirty CI、metadata/package 失败传播 contract；接入 fast/full
- [x] 7.3a 保留 stable full 的七条 macro span RED，补 Deref 尾逗号锁；只修复 attribute/meta 诊断跨度，并为 full 显式安装 rust-src，不改旧 stderr/生成 API，在 stable/nightly 上验证
- [x] 7.3b 闭合 full dev-server contract 的 just 环境依赖：保留无 just 的 ENOENT RED，workflow 显式安装本地已验证版本，并以远端实际 job 验证
- [x] 7.3c 闭合首轮 hosted Full 的 RTK 依赖泄漏：保留远端 exit 127，新增无 RTK 的隔离 PATH 真实 Vite 回归锁；Justfile 仅可选使用代理，重新通过本地 full 和 exact-head hosted Fast/Full
- [x] 7.4 对 fresh audit 命中的 Browserslist/postcss-selector-parser 做兼容 lock-only 补丁；使用明确 stable Rust/Node 24 重跑 fast/full/nightly，记录测试、长窗口、Criterion、fresh install/audit、Web build/contract 与真实 Rust source 的生产预览浏览器证据
- [x] 7.5 完成 scoped commit，并对相同 source commit 执行真实 clean release；复查差异、锁文件、OpenSpec、文档与工作区 custody
- [x] 7.6 只推送交付分支、创建 PR，核验最终 exact head 的远端 fast/full，不接受 missing 或旧 head 检查
- [ ] 7.7 满足真实批准、讨论解决、线性历史和远端检查后合并，核验远端 main 与合并后检查；否则保留明确阻塞，不绕过保护、不修改本地 S6 main

### Current boundary

2026-09-03 的结果在对应任务完成后逐项追加；2026-07-29/2026-08-24 是历史快照。
本轮不执行 OpenSpec archive/sync、deploy 或 publish。远端夜间/打包门在实际运行前
仍为 `NOT RUN / UNKNOWN`，不由本地 profile 代替。

### Fresh acceptance receipt (2026-09-03, code/CI accepted; merge awaiting review)

- Planning：OpenSpec 1.6.0 status 4/4、`validate --all --strict` 3/3 PASS，apply state `ready`；没有当前版本可用的独立 `verify` 命令。
- Release regression RED：`rtk proxy node --test scripts/ci/release-profile.test.mjs` 在实际 Bash `3.2.57(1)-release` 上 exit 1；9 项中 7 PASS、2 FAIL，只有 clean local/CI 报 `package_dirty_flag[@]: unbound variable`。
- Release regression GREEN：显式 clean/dirty argv 修复后同命令 exit 0，9/9 PASS；metadata 无效/命令失败、Git status 失败、两个 package 失败均能阻断流程。fast/full 已接入该 contract，README 同步说明 fixture 与真实 package 证据的区别。
- 主机默认 Node 为 `v26.6.0`，即使 `/opt/homebrew/opt/node@24` 也实际指向该版本；为避免误报 Node 24，在本轮临时工具链目录安装并验证 `Node v24.20.0 / npm 11.19.0`，不修改用户全局默认。后续 profile 使用此版本和已安装的 stable `rustc 1.97.1 (8bab26f4f 2026-07-14)`，远端实际工具链另行记录。
- 首轮 Node 24/stable `fast` exit 0：9 个 runner contracts、139 个 core tests、examples compile、Web UI/i18n/profile/build 和三 chunk budget 通过；但 fresh install 报 1 high/1 low。独立 `npm audit --json` exit 1，确认 Browserslist 两条 high 公告 `GHSA-c83g-rgw3-j3cx`、`GHSA-73wf-gq98-2v4g`，以及 selector parser 的 low `GHSA-w9m9-85wc-3x92`。此 RED 推翻 2026-08-24 的 audit 0 快照；未将 fast 成功当作 full 安全门通过。
- 安全修复候选：限定 `npm update browserslist postcss-selector-parser --package-lock-only --ignore-scripts` 将两个命中包分别更新到 `4.28.8`、`6.1.4`，连带更新 Browserslist 所需五个传递数据/工具依赖；直接 package.json 未变，lock-only audit 为 0，fresh-install 验收仍由后续 full 完成。
- 首轮 stable `full` exit 101，在宏 Accessors 的三条旧 `.stderr` span 对比处失败，尚未进入 Clippy/Web 阶段；core/lab 测试已通过但不代表 full GREEN。随后 `cargo test --locked -p picea-macro-tools --no-fail-fast` exit 101，确认 Accessors 3、Builder 1、Deref 3 条同类错误，总计七条。保留原快照，不使用 `TRYBUILD=overwrite`。
- `rustup check` 确认远端 stable 当前为 `1.98.0 (88d9e12ae 2026-08-18)`，本地原 stable `1.97.1` 较旧；本轮另在临时 RUSTUP_HOME 安装 1.98.0/rustfmt/clippy，随后以它重跑门禁，不改变全局 Rust 默认或已有工具链。
- 新 Deref 尾逗号 fixture 和全部旧 macro fixtures 在原 `nightly-2026-07-28` 上 exit 0，36 个 trybuild cases 通过；当前 stable 1.98.0 在新增后合计八条 span RED，另有一条 `chained_setter` 缺标准库源码提示。向临时 toolchain 添加 `rust-src` 后，`chained_setter` 无代码/快照修改即转绿，证明 full CI 必须显式提供该组件。
- Macro GREEN：仅将 Accessors/Builder/Deref 的 attribute/meta 多 token 错误改为 `syn::Error::new_spanned`，保留旧错误文本和全部旧 `.stderr`；当前 stable `1.98.0` 与原 `nightly-2026-07-28` 各自运行 `cargo test --locked -p picea-macro-tools --no-fail-fast` 均 exit 0，12 个顶层测试、36 个 nested trybuild cases 通过，原有 1 个 doctest 仍按设计 ignored。当前 stable `cargo fmt --all -- --check` PASS；full CI 显式安装 `rust-src`，模块 README 同步前置条件。
- 当前 stable `1.98.0` / Node `24.20.0` 的 fresh `fast`、`full` 均 exit 0；full strict Clippy、fresh install、完整 audit 0、Web contracts/build/dev-server 通过。JS chunks 仍为 `271256 / 143383 / 172191` bytes，均低于 `500000`。
- CI 环境 RED：full 的 dev-server contract 会实际调用 Justfile；本机 `just 1.45.0` 下通过，而限定 PATH 为 Node 24 与系统工具后同一 contract exit 1，明确报 `spawn just ENOENT`。当前 workflow 未安装 just；GitHub runner image 的公开工具清单/配置也未声明该依赖，因此显式安装固定版本，不依赖镜像偶然携带。
- Workflow 候选已补 `cargo install just --locked --version 1.45.0` 与版本回显，README 同步；本地 full 的 dev-server contract 已以同版本通过，远端实际安装/job 尚待 7.6 验证，7.3b 不提前勾选。
- Final local full：417 个顶层 Rust tests、39 个 nested trybuild cases（core 3 + macro 36）和 9 个 benchmark smoke PASS；2 个长窗口门在 full 按设计 ignored，由 nightly 单独执行。另有 9/9 release-runner contracts PASS。
- Final local nightly：当前 stable `1.98.0` / Node `24.20.0`，`bash scripts/ci/run.sh nightly` exit 0；600 帧门 21.42s、1200 帧门 43.73s，均为 exact ignored test，未修改既有阈值。默认 sample-size 20 的 Criterion 完整运行并通过 9 个 exact counter IDs；wall-clock 只作本机观测，没有可比 ABBA/远端性能结论。
- Browser production smoke：本轮独立 Rust binary `127.0.0.1:8080` + Vite production preview `127.0.0.1:4173`，默认 1280x720 桌面视口。fresh reload 的 HTML/CSS/三个 JS chunks 与 scenarios/session/control/SSE/frame 请求均为 2xx，捕获窗口未截断；在线 console warning/error 为 0。真实落箱连续三次单步 `3 -> 4 -> 5 -> 6` 且每次 state hash 改变；矩阵场景为 49 个物体；生成 120 帧真实矩阵产物并回放 `0 -> 1`，hash `45b94e1cac24d858 -> 89531527522016b1`；中英文切换往返通过。停掉本轮 API 后刷新，显示“离线演示数据 · 非真实模拟”，demo 单步 `demo-0000 -> demo-0001`，未冒充 Rust source。浏览器/预览进程均在验收后关闭，未修改主工作区服务。
- 当前本地日志、浏览器结构化记录与截图位于 `/private/tmp/picea-delivery-receipts-20260903/`，Criterion 位于独立 worktree `target/criterion/`；这是本机证据目录，不声称已上传或等价于 hosted CI。
- Scoped source commit：`ccaddfddeb8ff14e88a6e2c8a969cc021866a566`，只提交本轮 16 个 allowlisted 文件，原交付提交 `a7b3cb42` 保留。暂存前后 diff hygiene、Shell/Node syntax、四份 YAML parse、full prerequisite contract、OpenSpec strict validate 3/3 PASS；相对远端基线，core/lab/Web 运行时代码与 `Cargo.lock` 未改动。主工作区仍为 clean `main@80f08927`，没有引入后续 S6 commits。
- Clean local release：在 `ccaddfd` 的空 `git status --porcelain` 上执行当前 stable/Node 24 的 release，exit 0；metadata PASS，`picea` 63 files / 235.0KiB compressed、`picea-macro-tools` 78 files / 17.6KiB compressed，均从 package 重新构建。两个 archive 的 `.cargo_vcs_info.json` 精确指向 `ccaddfddeb8ff14e88a6e2c8a969cc021866a566` 且无 dirty 标记。首次执行仅有 Cargo registry index 缓存写权限 warning，未跳过验证；收据提交后再以最终 head 重跑。此为真实 clean local candidate，不是 hosted release 或 publish 证明。
- 收据 head `c1e58a42cc2afc959949447d5a23c6325c632ba5` 的真实 clean release 再次 exit 0；许可范围内重跑消除了 registry cache warning，两个 archive 的 VCS SHA 均精确匹配该 head，文件数保持 63/78。
- PR #3 已从交付分支推送创建；首轮 Actions run `33714447948` 对应 `c1e58a4`。Fast PASS（job `100520548197`）；Full FAIL（job `100520548315`）：stable/rust-src/just 安装、Rust tests/Clippy、audit 0 和 Web build/bundle 均通过，但实际 dev-server recipe 报 `rtk: command not found`，exit 127。原始日志保留为本机 `remote-full-c1e58a4-red.log`；不得将本地 full GREEN 覆盖这条 hosted RED。
- RTK local RED/GREEN：给现有 dev-server contract 增加只暴露 node/npm/just/bash/sh 的隔离 PATH，真实 Vite 入口先复现相同 exit 127（`no-rtk-dev-server-red.log`）。Justfile 统一可选代理前缀后，同一合同 exit 0（`no-rtk-dev-server-green.log`）；原 RTK orchestration fixture、当前 PATH 和无 RTK PATH 均保留，不跳过门禁。
- RTK 修复后 fresh local full：当前 stable `1.98.0` / Node `24.20.0`，`bash scripts/ci/run.sh full` exit 0（`full-after-rtk-fix.log`）；workspace/trybuild/Clippy、9/9 release contracts、fresh npm ci/audit 0、全部 Web contracts 和 build PASS，JS chunks 的大小与 hash 不变。npm 对 esbuild/fsevents 的未显式批准 install-script 提示仍可见，没有将其混作 audit vulnerability，也未改变脚本授权策略。远端新 head 尚待复跑，因此 7.3b/7.3c/7.6 继续保持未完成。
- Hosted GREEN：修复 head `d7c69833e12f35f044f79d218a889ff7f49aa95e` 的 Actions run `33715665089` 最终 `completed/success`；Fast job `100524205519` 为 1m05s，Full job `100524205558` 为 4m25s，2026-09-03 04:40:56 UTC 完成。完整 Full 原始日志确认 rust-src/just 准备、workspace/Clippy/audit/build 和无 RTK dev-server contract 全部通过；不只是检查记录存在。对应本机日志 `remote-full-d7c6983-green.log`。这条实际结果关闭 7.3b/7.3c/7.6；收据文档提交后仍须对交付最终 head 再核验 CI，结果写入 PR，不能沿用旧 head 的绿色。
- Clean source package：`d7c6983` 的 release exit 0（`release-d7c6983.log`），两包均从 archive 重建通过，VCS SHA 精确匹配完整 head，无 dirty 标记或 registry cache warning。
- Merge readiness：PR #3 无代码冲突（`MERGEABLE`），但 `mergeStateStatus=BLOCKED`、`reviewDecision=REVIEW_REQUIRED`，reviews 为空。现场分支保护要求 1 个有效批准、讨论解决、线性历史；当前具写权限的 collaborator 只有作者 `swnb`，作者不能自批。7.7 保留未完成；没有尝试管理员 bypass、修改保护、伪造批准或合并。
- 其余边界：`required_status_checks` 当前为 null，本轮不擅自修改仓库规则；这是尚未自动强制 Fast/Full 的管理配置缺口，不影响本次实际 exact-head CI 已验事实。hosted nightly/release 仍为 `NOT RUN / UNKNOWN`，没有 publish/deploy 或 OpenSpec archive/sync。默认分支 4 条 Dependabot 告警尚待合并后重扫；PR lock 的 PostCSS 8.5.25 / selector-parser 6.1.4 覆盖全部对应修复下限，不提前关闭告警。主工作区 main/S6 `80f08927` 与后续提交保持不动。
