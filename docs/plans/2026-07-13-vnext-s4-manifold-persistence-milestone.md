# vNext Handoff §4 SAT Manifold Persistence Milestone

状态：Draft；用户决策已确认；等待 S4-D spec/architecture review
日期：2026-07-13
基线：`main=origin/main=9427a17`
设计：`docs/design/2026-07-13-sat-manifold-persistence-design.md`
父计划：`docs/plans/2026-06-17-physics-realism-vnext-milestones.md`

## 批准与边界

用户已授权“全部后续任务按序推进”，并要求每个节点经过 subagent review、测试无问题后才 commit。对 S4 又明确批准：

- history-aware persistent `ContactId` 双射取代 raw feature 跨 role swap 不变的字面目标；
- collider shape/local-pose geometry revision invalidation 纳入 S4。

S4 不包含 public API break，因此在 S4-D review 通过后可按 standing authorization 连续执行。§5 revolute joint 仍必须在 public API design gate 停下确认。

## 目标

关闭 handoff §4 的唯一 ignored red lock，并在正确的 history owner 中实现：

- final raw feature authoritative；
- persistent contact identity exact-first、最大基数、一对一；
- warm-start/lifecycle/source-row revision-aware；
- matrix-stack 行为不退化。

## 非目标

- 不改 SAT/clip 几何、solver math、position-row heuristic、sleep、CCD、broadphase或阈值。
- 不增加 public variant/schema/prelude/API。
- 不运行或实现父计划 vNext E4 complex-shape；S4 前缀与父计划 E4 完全不同。
- 不进入 §5、§6 或 damping/手感节点。

## 现场基线

- Branch 起点：`9427a17`，初始 `main...origin/main` clean。
- Ignored RED：

```text
rtk proxy cargo test -p picea --lib pipeline::narrowphase::tests::stacked_rectangles_keep_feature_id_when_sat_reference_face_swaps -- --ignored --exact --nocapture
exit 101; 0 passed / 1 failed; feature ids 16785408 != 16777218
```

- Matrix 180：1 passed；48 sleeping / 0 awake；无 floor ejection。
- Aligned matrix 1200：1 passed；12 sleeping / 0 awake；无 floor ejection。
- 旧 600-frame observation 是历史失败 provenance，不是本 milestone live green gate。Live long-window gate只有 forced E4 acceptance。

## 约束

- 所有验证命令使用 `rtk proxy`。
- 一次最多一个写入 worker；reviewer/verifier 只读且都是叶子 agent。
- 每个实现行为先存在 committed acceptance artifact，并观察当前实现 RED，再写 production code。
- 每个 commit 前必须先完成对应 spec/code review；High/Medium 必须闭环，并由 verifier 给出该节点要求的真实命令结果。
- 不删除、放宽或改阈值来让测试通过。旧 raw equality 的强度迁移为 history-aware 双点对应 equality，并新增 raw role-swap characterization。
- 不 push；用户本轮只明确授权 commit。

## 已确认决策与安全假设

Blocking questions：无。用户已确认 identity contract 与 revision scope。

Safe assumptions：

- current + predicted 与 retained previous active records 每 pair 均可能达到4个。实现使用一般 pair-scoped matcher，S4-RED包含4x4规模锁。
- private revision 使用 `u64`；实际world lifetime不发生wrap。理论wrap记录为残余风险。
- same-index只用于warm-start，不用于public lifecycle reason。

## Milestones

| Node | Scope | Out of scope | Runnable acceptance | Commit gate |
| --- | --- | --- | --- | --- |
| S4-D | architecture、living spec、routing、父计划addendum | `crates/**` | YAML parse；`git diff --check`；`git diff --exit-code -- crates` | spec + architecture reviewer无High/Medium；docs verifier green |
| S4-RED | 一次性提交全部acceptance-as-code、ignored test迁移和RED receipt | production implementation、SAT/clip production code | 下方“RED commands”逐条运行；至少A02-A09中的当前缺口稳定RED，既有边界锁可保持green | test reviewer无High/Medium；verifier确认预期RED/green分类；commit tests + plan receipt |
| S4-IMPL | private revision、pair grouping、residual最大基数reservation、三个consumer policy | solver/public schema/阈值、narrowphase production code | 下方“Targeted green commands”全部exit0 | spec reviewer + code reviewer无High/Medium；targeted verifier green |
| S4-V | full Rust/lab/matrix/scope acceptance | §5/§6/web功能扩展 | 下方“Full verification”全部exit0 | verifier receipt；reviewer复核最终diff无High/Medium |
| S4-C | living spec/父计划/handoff closeout与残余风险 | 新行为 | YAML、diff、base-relative scope、status | docs reviewer + verifier green后commit closeout |

## S4-D 文档 Gate

```text
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy ruby -e 'required={"docs/design/2026-07-13-sat-manifold-persistence-design.md"=>"# SAT Manifold Persistence Design","docs/plans/2026-07-13-vnext-s4-manifold-persistence-milestone.md"=>"# vNext Handoff §4 SAT Manifold Persistence Milestone"}; required.each { |path, heading| abort "missing s4 doc: #{path}" unless File.file?(path) && File.size(path) > 0; text=File.read(path); abort "missing heading: #{path}" unless text.start_with?(heading); abort "trailing whitespace: #{path}" if text.lines.any? { |line| line.chomp.end_with?(" ", "\t") } }; puts "s4 docs ok"'
rtk proxy git diff --check
rtk proxy git diff --exit-code -- crates
rtk proxy git status --short --branch
```

## S4-RED Acceptance-as-Code

新增/迁移测试必须在 production code 前一次性落地：

1. `stacked_rectangles_expose_raw_feature_role_swap`：S4-RED即改名、移除ignore并保留原 fixture、normal `>0.999`、local drift `<0.25`，断言 raw feature indices不同。该characterization在RED commit中应green；production narrowphase仍未修改。
2. `manifold_persistence_sat_role_swap_preserves_both_contact_ids_by_local_anchor`：两个 points 全部按 local anchors 双射持久，final raw feature仍来自第二帧，slots distinct，不自动warm-start。
3. `lifecycle_reservation_keeps_later_exact_match`。
4. `lifecycle_reservation_maximizes_edge_swap_cardinality`，覆盖缺边图反例。
5. `warm_start_reservation_keeps_distinct_impulses_with_local_witnesses`：同时锁定不重复消费和不互换双点sentinel impulses。
6. `warm_start_reservation_maximizes_residual_cardinality_after_exact_matches`：exact不可牺牲，maximum只作用于residual graph。
7. `source_row_revision_is_rejected_before_solver_rows_are_built` 与 `source_row_reservation_does_not_reuse_previous_point`。
8. `manifold_persistence_geometry_patch_invalidates_all_history_consumers`。
9. `manifold_persistence_world_commands_geometry_patch_is_atomic`，同时覆盖成功与rejected scratch transaction。
10. `manifold_persistence_sensor_transitions_do_not_expand_edge_swap_identity`。
11. `manifold_persistence_two_to_one_to_two_preserves_only_surviving_point`。
12. `manifold_persistence_normalizes_geometric_a_b_order_with_revisions`。
13. `manifold_persistence_history_only_separation_does_not_fabricate_contact`，保留confirmed solver interaction例外。
14. matrix180、aligned1200和forced E4 600 baseline gates。
15. `reservation_stays_pair_scoped_for_four_by_four_inputs_and_unrelated_pairs`。

RED commands：

```text
rtk proxy cargo test -p picea --lib pipeline::narrowphase::tests::stacked_rectangles_expose_raw_feature_role_swap -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_ -- --nocapture
rtk proxy cargo test -p picea --lib pipeline::contacts::tests::lifecycle_reservation_ -- --nocapture
rtk proxy cargo test -p picea --lib pipeline::contacts::tests::warm_start_reservation_ -- --nocapture
rtk proxy cargo test -p picea --lib pipeline::contacts::tests::source_row_reservation_ -- --nocapture
rtk proxy cargo test -p picea --lib pipeline::contacts::tests::source_row_revision_is_rejected_before_solver_rows_are_built -- --exact --nocapture
rtk proxy cargo test -p picea --lib pipeline::contacts::tests::reservation_stays_pair_scoped_for_four_by_four_inputs_and_unrelated_pairs -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache_uses_normalized_pair_identity_when_geometric_a_b_order_is_swapped -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --exact --nocapture
rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --exact --nocapture
```

S4-RED commit可包含测试与本计划的实际RED receipt；不得包含 production implementation。若某个新锁在基线已green，receipt必须如实记录，它仍作为边界锁保留；代表实现缺口的锁必须至少有一个稳定RED，否则暂停重审spec。

S4-RED receipt按下表逐项填写，不能用一个filter结果代替分类：

| Acceptance | Artifact / exact filter | Baseline expectation |
| --- | --- | --- |
| A01 | `stacked_rectangles_expose_raw_feature_role_swap` | green characterization |
| A02 | `manifold_persistence_sat_role_swap_preserves_both_contact_ids_by_local_anchor` | 运行后记录；若已green仍保留 |
| A03 | `lifecycle_reservation_keeps_later_exact_match` | RED |
| A04 | `lifecycle_reservation_maximizes_edge_swap_cardinality` | RED |
| A05 | `warm_start_reservation_keeps_distinct_impulses_with_local_witnesses` | RED |
| A06 | `warm_start_reservation_maximizes_residual_cardinality_after_exact_matches` | RED |
| A07 | 两个 `source_row_*` tests | RED |
| A08 | `manifold_persistence_geometry_patch_invalidates_all_history_consumers` | RED |
| A09 | `manifold_persistence_world_commands_geometry_patch_is_atomic` | 运行后记录，至少successful commit子场景RED |
| A10 | `manifold_persistence_sensor_transitions_do_not_expand_edge_swap_identity` | 运行后记录 |
| A11 | `manifold_persistence_two_to_one_to_two_preserves_only_surviving_point` | 运行后记录 |
| A12 | `manifold_persistence_normalizes_geometric_a_b_order_with_revisions` + existing normalized-pair exact | 新revision锁RED；existing锁green |
| A13 | `manifold_persistence_history_only_separation_does_not_fabricate_contact` | green boundary或如实记录 |
| A14 | matrix180/aligned1200/forced E4 600 exact commands | 全部必须green才能进入implementation |
| A15 | 4x4 pair-scoped exact test | RED |

## S4-IMPL 最小实现

允许修改：

- `crates/picea/src/collider.rs`：private revision `u64` + crate-private getter。
- `crates/picea/src/world/contact_state.rs`：previous records保存ordered revisions与private sensor fact。
- `crates/picea/src/pipeline/contacts.rs`：pair-scoped grouping、maximum-cardinality reservation、warm/lifecycle/source-row policies。
- `crates/picea/src/pipeline/narrowphase.rs`：S4-RED已完成测试迁移；S4-IMPL不改production SAT/clip。
- `crates/picea/src/events.rs` / `debug.rs`：只澄清raw feature与persistent ids、`MissFeatureId` doc comments，不改variant/field/serde。
- S4-RED测试和living spec进度。

算法门：

- geometry-compatible exact先全局预留且不可为更多fallback牺牲；
- 剩余residual candidate graph枚举所有合法一对一 matching；
- residual graph先最大cardinality，再policy kind，再最大/总drift，最后stable keys；
- warm-start/lifecycle/source-row独立执行，不共享一次match裁决；
- lifecycle same-index禁止；edge-swap仅solid->solid；
- source-row revision mismatch必须non-candidate。
- revision mismatch的Started events必须分配新ContactId与新ManifoldId。
- public comments必须保留confirmed solver interaction的source-feature例外，并说明`ExactFeature`还要求geometry-compatible revisions。

Targeted green commands：

```text
rtk proxy cargo test -p picea --lib pipeline::narrowphase::tests::stacked_rectangles_expose_raw_feature_role_swap -- --exact --nocapture
rtk proxy cargo test -p picea --lib pipeline::narrowphase::tests -- --nocapture
rtk proxy cargo test -p picea --lib pipeline::contacts::tests -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_ -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance sat_edge_swap_candidate_persists_lifecycle_without_auto_warm_start_impulse -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4 -- --nocapture
```

## S4-V Full Verification

旧 observation不运行；其 frame 245 ejection只作为归档负证据。Forced acceptance是唯一600-frame live green gate。

```text
rtk proxy cargo fmt --all --check
rtk proxy cargo test -p picea --lib
rtk proxy cargo test -p picea --test physics_realism_acceptance
rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --exact --nocapture
rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run
rtk proxy cargo test --workspace --all-targets
rtk proxy cargo clippy --workspace --all-targets
rtk proxy cargo test -p picea --examples --no-run
rtk proxy cargo bench --workspace --no-run
rtk proxy git diff --check
rtk proxy git diff --exit-code 9427a17..HEAD -- crates/picea/src/lib.rs
rtk proxy git diff --exit-code 9427a17..HEAD -- crates/picea/src/solver
rtk proxy git diff --name-only 9427a17..HEAD
rtk proxy git status --short --branch
```

`git diff --name-only 9427a17..HEAD` 由 reviewer 人工核对只包含批准文件；status只作hygiene，不单独作为scope proof。

## S4-C Closeout Gate

以下命令在closeout docs仍位于working tree时运行，因此使用单点base diff而不是只看`HEAD`：

```text
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check 9427a17
rtk proxy git diff --exit-code 9427a17 -- crates/picea/src/lib.rs
rtk proxy git diff --exit-code 9427a17 -- crates/picea/src/solver
rtk proxy git diff --name-only 9427a17
rtk proxy git status --short --branch
```

S4-C commit后复跑：

```text
rtk proxy git diff --check 9427a17..HEAD
rtk proxy git diff --exit-code 9427a17..HEAD -- crates/picea/src/lib.rs crates/picea/src/solver
rtk proxy git status --short --branch
```

## Review Chain

每个节点严格执行：

1. Worker：唯一写者；S4-RED先锁行为，S4-IMPL最小实现。
2. Spec reviewer：检查scope、用户裁决、hard boundary、acceptance强度。
3. Code reviewer：findings first，重点检查错误对应、重复消费、NaN、stale revision、sensor/separation和weak tests。
4. Verifier：逐条运行本节点命令，报告exact command、exit、counts、关键matrix指标和Git-visible产物。
5. Supervisor：只有High/Medium闭环且该节点预期gate满足后才commit。

所有subagent均为叶子，不允许再启动subagent。Reviewer/verifier不修文件。

## Commit Nodes

- S4-D：`docs: design sat manifold persistence`
- S4-RED：`test: lock manifold persistence matching`
- S4-IMPL：`fix: persist sat manifold point identity`
- S4-C：`docs: close sat manifold persistence milestone`

不 amend，不 push。若hook/review失败，修复后重新review并创建新commit。

## 进度日志

### 2026-07-13 - Baseline

- `main=origin/main=9427a17`，clean。
- Ignored raw equality fixture稳定RED，actual indices `16785408 != 16777218`。
- Matrix180与aligned1200基线green、无ejection。
- Explorer确认history owner在contacts finalization/lifecycle，不在stateless narrowphase。
- 用户批准persistent IDs supersede raw equality，并批准geometry revision纳入S4。

### 2026-07-13 - S4-D review round 1

- Reject：发现互斥600 gates、greedy非最大双射、遗漏source-row、same-index public reason误标、revision/sensor/2->1->2/规模合同不完整。
- 当前修订已改为maximum-cardinality三consumer设计；等待S4-D round 2。未commit，未进入S4-RED。

## 验收报告模板

每个已完成节点按固定顺序记录：

1. 成功标准
2. 检查结果：exact command、exit、captured output/artifact
3. 复跑方式
4. 范围外
5. 残余风险
