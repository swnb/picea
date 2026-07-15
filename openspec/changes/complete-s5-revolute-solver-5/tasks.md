## 1. S5-REPLAN-3 规划与文档门

- [x] 1.1 在clean detached worktree固定immutable Start HEAD `29bafce59a0913be7b4d37055ab57ed5e5233116`；按exact allowlist导入living blob `6bc4dc8...`和6个OpenSpec paths，复核design blob `f945b0f...`及主worktree 5个production blobs未变
- [x] 1.2 更新 Revolute design 的状态/acceptance owner与living spec的STOP冲突、新执行链、scope、receipt和OpenSpec单向指针；保持§7、§9.1-§9.8、§10、ADR-S5-1..5零语义差异
- [x] 1.3 固定REPLAN-3 exact 8、RED-5 exact 5、SOLVER-5 required exact 7/optional unit path；明确tasks.md是唯一task checkbox进度源、living只追加identity/evidence receipt
- [x] 1.4 运行OpenSpec `status`/strict `validate`/`instructions apply`、docs YAML/链接/forbidden grep、`git diff --check`与exact-scope gates
- [x] 1.5 完成独立spec reviewer和docs verifier，闭合全部High/Medium findings
- [x] 1.6 仅stage批准的REPLAN-3 paths，复核cached exact allowlist并提交 `docs: replan revolute pose-state oracle`

## 2. S5-BEHAVIOR-RED-5 acceptance-as-code

- [ ] 2.1 从REPLAN-3 commit记录RED-5 immutable Start HEAD，并在scope verifier/living receipt登记新三节点、PENDING/SHA双分支与PRE_V排除四个STOPPED solver
- [ ] 2.2 将shadow comparator收窄为CCD/role/residual-position/StepStats pose-effect facts；保留`1e-6`、全部velocity-response finite/diagnostic输出及subject normal impulse positive门，并用`position_correction_input_max_depth + 1e-3` negative control证明comparator可证伪
- [ ] 2.3 扩展现有exact `post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows`：新增contact唤醒sleeping Distance后的projection子case，锁定enabled`<=1e-4`、disabled`>1e-3`及固定signature；总数仍为16
- [ ] 2.4 在clean baseline逐条运行8个unit与16条integration exact，确认仅批准symbol RED、严格`12 RED / 4 GREEN`及两条唯一pose-effect signature
- [ ] 2.5 临时应用保留production patch作compatibility witness，确认两条目标exact转绿且Distance新边界锁能检出旧plan复用风险；撤回临时production文件后证明tests/docs/scope blobs不变
- [ ] 2.6 运行scope self-tests、fmt、diff、cached/untracked/hygiene与OpenSpec strict validate gates
- [ ] 2.7 完成独立test/spec reviewer和RED verifier，闭合全部High/Medium findings
- [ ] 2.8 仅stage批准的RED-5 acceptance paths与tasks receipt，复核cached exact allowlist并提交 `test: lock revolute pose-state oracle`

## 3. S5-SOLVER-5 production实现

- [ ] 3.1 先移开同内容untracked OpenSpec并恢复已被commits覆盖的local living copy，再把detached REPLAN-3/RED-5 commits cherry-pick到主worktree当前feature branch；复核5个production blobs不变并记录SOLVER-5 immutable Start HEAD
- [ ] 3.2 复用冻结2x2/COM/atomic/wake/post-contact实现，恢复contact后按最新wake state重建optional velocity plan，并让mandatory plan继续独占stats/post rows
- [ ] 3.3 撤销WorldAnchor无关helper重构；按reviewer触发的optional unit path先锁住sub-EPSILON actual pose/velocity change与would-wake，再修复exact-change判定；完成owned Rust文件rustfmt，committed integration/scope/design保持zero-diff
- [ ] 3.4 逐条运行既有8个unit、新增sub-EPSILON unit与16条integration exact，确认全部GREEN、两条pose-effect等价、Distance contact-wake projection、row/warning/full-pose/drift/negative signatures全部通过
- [ ] 3.5 运行StepConfig、island、stack、world/API/lifecycle定向门，以及`picea --lib`、`picea --tests`、examples、workspace all-targets/clippy等与影响面相称的broader gates
- [ ] 3.6 运行OpenSpec strict validate、scope self-test/exact/cached、fmt、diff、forbidden zero-diff、untracked/filesystem hygiene gates
- [ ] 3.7 完成独立solver spec reviewer、code reviewer与verifier，闭合全部High/Medium findings
- [ ] 3.8 仅stage批准的SOLVER-5 production/living/tasks paths，复核cached exact allowlist并准备单一final commit candidate；此步不写自引用commit SHA，也不在4.2前提交

## 4. Supervisor closeout

- [ ] 4.1 在最终commit前完成所有repo evidence receipt，并准备核对外部报告所需的已验证事实、实际命令输出、未运行项与残余风险；living只保留本node Start HEAD与`COMMIT PENDING`，不记录自引用SHA
- [ ] 4.2 复核最终pre-commit candidate已是tasks 24/24、cached exact allowlist与全部gate PASS，并授权supervisor执行外部closeout；最终`git commit`及commit后只读审计不属于repo task checkbox，不能再写回或amend receipt
