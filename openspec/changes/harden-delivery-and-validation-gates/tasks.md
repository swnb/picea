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
