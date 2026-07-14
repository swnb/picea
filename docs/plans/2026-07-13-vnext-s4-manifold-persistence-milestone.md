# vNext Handoff §4 SAT Manifold Persistence Milestone

状态：已完成并冻结；S4-IMPL `91698b3`、clippy remediation `57cdb19`，S4-V full verification PASS
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
- S4-ORACLE-RED提交前，既有`edge_swap_candidate_count == 0`与全部matrix数值断言保持原样。只有更强attribution locks先提交、双基线证据与review闭环后，才允许用absorbed/unabsorbed assertions替换含糊counter；这不授权删除任何penetration/速度/sleep/support/ejection门。
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
| S4-ORACLE-D | A14 diagnostic attribution delta与冻结边界 | artifact test或production改动 | docs diff/YAML/scope | architecture + spec reviewer无High/Medium；docs verifier green后单独commit |
| S4-ORACLE-RED | O01-O05 attribution、真实SAT、artifact传播、A14 preservation与symmetric-edge locks | 修改旧A14断言；修改core implementation | 新测试逐exact；旧A14三门原样；pre-commit同一test patch双跑 | test reviewer + verifier + spec reviewer通过后单独commit |
| S4-ORACLE-EVIDENCE | 固定test commit `T`后复跑baseline/working并提交receipt | 修改tests/oracle/core | clean `T`与`T`+固定implementation patch双跑；hash/scope核对 | verifier + spec reviewer通过后docs-only commit |
| S4-ORACLE-REPLACE | 用已批准absorbed/unabsorbed assertions替换唯一含糊counter | 修改matrix数值门、core implementation | O01-O05 + 三条A14；source freeze只允许该语义替换 | test/code reviewer + verifier green/expected RED后单独commit |
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

当前冻结：working implementation已使A03-A13/A15 green，但matrix180的旧`edge_swap_candidate_count == 0`失败为19。S4-ORACLE-RED验收和spec裁决前，不修改或提交这5份working implementation文件。

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
- S4-IMPL必须把A15的`#[cfg(test)]`increment迁到新matcher真实candidate-edge predicate；implementation reviewer必须拒绝旧函数遗留导致的`0 == 0`真空通过，并确认4x4 baseline count为`1..=16`且不随unrelated pairs增加。
- public comments必须保留confirmed solver interaction的source-feature例外，并说明`ExactFeature`还要求geometry-compatible revisions。

Targeted green commands：

```text
rtk proxy cargo test -p picea --lib pipeline::narrowphase::tests::stacked_rectangles_expose_raw_feature_role_swap -- --exact --nocapture
rtk proxy cargo test -p picea --lib pipeline::narrowphase::tests -- --nocapture
rtk proxy cargo test -p picea --lib pipeline::contacts::tests -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_ -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_sat_role_swap_reports_persistent_edge_swap -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_symmetric_edge_index_is_not_role_swap -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance sat_edge_swap_candidate_persists_lifecycle_without_auto_warm_start_impulse -- --exact --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache -- --nocapture
rtk proxy cargo test -p picea --test physics_realism_acceptance stack_4 -- --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_edge_swap_attribution_partitions_every_candidate -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_edge_swap_attribution_has_no_unabsorbed_or_exact_theft -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run artifact_records_persistent_edge_swap_with_stable_ids -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --exact --nocapture
rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --exact --nocapture
```

## S4-ORACLE Gate

### S4-ORACLE-D docs gate

```text
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
rtk proxy git diff --name-only c4298ae -- docs/design/2026-07-13-sat-manifold-persistence-design.md docs/plans/2026-06-17-physics-realism-vnext-milestones.md docs/plans/2026-07-13-vnext-s4-manifold-persistence-milestone.md
rtk proxy ruby -rdigest -e 'paths=%w[crates/picea/src/collider.rs crates/picea/src/debug.rs crates/picea/src/events.rs crates/picea/src/pipeline/contacts.rs crates/picea/src/world/contact_state.rs]; patch=IO.popen(["rtk","proxy","git","diff","--binary","c4298ae","--",*paths], &:read); puts Digest::SHA256.hexdigest(patch)'
rtk proxy git diff --cached --exit-code
rtk proxy git status --short --branch
rtk proxy git add -- docs/design/2026-07-13-sat-manifold-persistence-design.md docs/plans/2026-06-17-physics-realism-vnext-milestones.md docs/plans/2026-07-13-vnext-s4-manifold-persistence-milestone.md
rtk proxy ruby -e 'expected=%w[docs/design/2026-07-13-sat-manifold-persistence-design.md docs/plans/2026-06-17-physics-realism-vnext-milestones.md docs/plans/2026-07-13-vnext-s4-manifold-persistence-milestone.md].sort; actual=IO.popen(["rtk","proxy","git","diff","--cached","--name-only"], &:read).lines.map(&:strip).reject(&:empty?).sort; abort "staged scope mismatch: #{actual.inspect}" unless actual == expected; puts "oracle docs staged scope ok"'
rtk proxy git diff --cached --check
rtk proxy git diff --cached --exit-code -- crates
rtk proxy ruby -rdigest -e 'expected="bf408ea3dd8c4cc93f57f20c3b0949fbb9a18f9e60308f5ea2f154251869ba95"; paths=%w[crates/picea/src/collider.rs crates/picea/src/debug.rs crates/picea/src/events.rs crates/picea/src/pipeline/contacts.rs crates/picea/src/world/contact_state.rs]; patch=IO.popen(["rtk","proxy","git","diff","--binary","c4298ae","--",*paths], &:read); actual=Digest::SHA256.hexdigest(patch); abort "frozen implementation changed: #{actual}" unless actual == expected; puts actual'
```

本节点只stage三份docs。上述cached-name与hash gate只在commit前运行；commit后使用下列命令验证新commit本身的三文件allowlist，并再次复算5份frozen implementation hash。Verifier不执行stage步骤；由supervisor在review/verifier通过后按固定allowlist stage、commit并复核。

```text
rtk proxy ruby -e 'expected=%w[docs/design/2026-07-13-sat-manifold-persistence-design.md docs/plans/2026-06-17-physics-realism-vnext-milestones.md docs/plans/2026-07-13-vnext-s4-manifold-persistence-milestone.md].sort; actual=IO.popen(["rtk","proxy","git","diff-tree","--no-commit-id","--name-only","-r","HEAD"], &:read).lines.map(&:strip).reject(&:empty?).sort; abort "commit scope mismatch: #{actual.inspect}" unless actual == expected; puts "oracle docs commit scope ok"'
rtk proxy ruby -rdigest -e 'expected="bf408ea3dd8c4cc93f57f20c3b0949fbb9a18f9e60308f5ea2f154251869ba95"; paths=%w[crates/picea/src/collider.rs crates/picea/src/debug.rs crates/picea/src/events.rs crates/picea/src/pipeline/contacts.rs crates/picea/src/world/contact_state.rs]; patch=IO.popen(["rtk","proxy","git","diff","--binary","c4298ae","--",*paths], &:read); actual=Digest::SHA256.hexdigest(patch); abort "frozen implementation changed: #{actual}" unless actual == expected; puts actual'
```

### O01 Matrix attribution

新test必须保留旧matrix test不变。Final candidate edge完全独立于warm/lifecycle输出：同ordered pair、clip-family compatible、非symmetric raw edge role swap、normal dot `>=0.98`、双侧final midpoint local-anchor drift `<=0.25`、全部输入finite。Oracle先预留full raw feature exact，再在residual graph做最大基数一对一。`final_candidate_total`只表示最终选中的residual matching数量，不是candidate graph edge数量；该gate是独立的artifact-observable midpoint projection，不对core private surface witnesses主张subset或等价关系。

Matrix fixture在整个run内不得发生shape/local-pose patch；test必须从artifact collider shape/local-pose signature逐帧验证该前提，不满足则fail-closed。只有此前提成立时，artifact缺少private geometry revision才不影响O01归因；该oracle不得泛化到可编辑geometry场景。

分类合同：

- `final_candidate_total == final_attributed + final_unabsorbed`；candidate不能从分类漏出。
- `reported_persistent_edge_swap_total == final_attributed + source_boundary_evidence + unexpected`；三个output集合以`(frame, ContactId)`为key互斥。
- Final-attributed必须同时满足`ContactPersisted` event、`PersistentEdgeSwap` reason、oracle对应的previous/current `ContactId`相同且一对一、`ManifoldId`相同。任何candidate错误都归final-unabsorbed。
- Source-boundary evidence必须有同ordered pair唯一same-ID/same-manifold predecessor、full feature不同且无full-exact predecessor、非symmetric raw role swap，并且至少一端为fail-closed source projection：non-sensor、depth0、polygon SAT明确`Some(false)`、finite positive solver normal impulse。缺数据、unsupported/degenerate SAT或无法判定一律归unexpected。
- 所有reported `PersistentEdgeSwap`不属于完整final-attributed或source-boundary evidence时都归unexpected；不能因source endpoint不是final geometry而从reverse completeness漏掉。
- Exact theft按实际输出ID检测：存在final geometry-compatible full exact predecessor时，current必须继承该predecessor `ContactId`；不能由oracle的reservation构造性地产生0。
- 未sanitize的`FrameRecord.events`负责finite/impulse gate；snapshot和落盘artifact只做投影交叉核对。Non-hit transition warm impulses必须为0。

Exact tests：

```text
rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_edge_swap_attribution_partitions_every_candidate -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_edge_swap_attribution_has_no_unabsorbed_or_exact_theft -- --exact --nocapture
```

第一条结构性双守恒/finite test在baseline与working都必须green。第二条是evidence test：要求final-attributed>0、source-boundary evidence>0、final-unabsorbed=0、unexpected=0、exact predecessor>0、exact theft=0；working当前目标为`59 reported = 46 final-attributed + 13 source-boundary + 0 unexpected`及`46 final candidates = 46 attributed + 0 unabsorbed`。Pre-commit verifier记录baseline/working实际值，其结果由S4-ORACLE-EVIDENCE spec gate裁决，不能在RED test commit前改旧counter或core。

### O02/O03 Positive propagation

- Core integration exact：`manifold_persistence_sat_role_swap_reports_persistent_edge_swap`。
- Lab artifact exact：`artifact_records_persistent_edge_swap_with_stable_ids`。
- 两者都必须证明raw role swap、无full exact predecessor、stable `ContactId`/`ManifoldId`、明确`PersistentEdgeSwap`，不是只数全局enum。

```text
rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_sat_role_swap_reports_persistent_edge_swap -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run artifact_records_persistent_edge_swap_with_stable_ids -- --exact --nocapture
```

两条在baseline与working均预期green；O03必须反序列化实际落盘`frames.jsonl`或`debug_render.json`，不得只检查内存`RunResult`。

### O05 Symmetric-edge negative lock

`reference_edge == incident_edge`不是role swap。S4-ORACLE-RED在冻结5文件之外的`physics_realism_acceptance.rs`提交真实rectangle behavior lock：通过固定相对旋转令reference/incident edge index相同，再制造point-slot变化。Test必须先断言两帧full feature不同、raw feature index相同、decode后`reference_edge == incident_edge`、point slot改变且第二帧`warm_start_reason == MissFeatureId`；这些非真空前置全部满足后，才断言lifecycle非`PersistentEdgeSwap`且source-row reason非`EdgeSwap`。S4-IMPL必须修复后才能green：

```text
rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_symmetric_edge_index_is_not_role_swap -- --exact --nocapture
```

### O04 Preservation

在S4-ORACLE-RED commit中，以下旧命令和断言保持原样并运行：

```text
rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --exact --nocapture
rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --exact --nocapture
rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --exact --nocapture
```

Reviewer必须逐行证明O01/O03只增加测试/helper，旧matrix数值断言零删除、零阈值变化。

S4-ORACLE-RED新增`crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb`，结构化抽取上述三条既有test函数并与`c4298ae` blob逐字比较：

```text
rtk proxy ruby crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb c4298ae exact
```

该命令在S4-ORACLE-RED必须green；S4-ORACLE-REPLACE使用`replace-one-oracle`模式，只允许替换旧`edge_swap_candidate_count == 0`语义断言，其他函数内容和阈值仍逐字相同。

### Dual-baseline evidence

S4-ORACLE-RED commit前，test reviewer先批准test-only diff。Supervisor按allowlist stage tests/tool、计算patch与frozen implementation hashes、创建temporary clean `c4298ae` worktree并应用同一cached patch。Verifier只读检查两侧hash/status并运行O01-O05和三条A14，不执行`git add`、`git apply`、worktree create/remove。Verifier完成后由supervisor reverse同一patch、确认temporary worktree clean并remove；之后才允许commit tests。

Pre-commit cached patch allowlist与应用配方：

```text
rtk proxy git diff --cached --exit-code
rtk proxy git add -- crates/picea-lab/tests/artifact_run.rs crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb crates/picea/tests/physics_realism_acceptance.rs
rtk proxy ruby -e 'expected=%w[crates/picea-lab/tests/artifact_run.rs crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb crates/picea/tests/physics_realism_acceptance.rs].sort; actual=IO.popen(["rtk","proxy","git","diff","--cached","--name-only"], &:read).lines.map(&:strip).reject(&:empty?).sort; abort "test patch scope mismatch: #{actual.inspect}" unless actual == expected; puts "oracle test patch scope ok"'
rtk proxy ruby -rdigest -e 'paths=%w[crates/picea-lab/tests/artifact_run.rs crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb crates/picea/tests/physics_realism_acceptance.rs]; patch=IO.popen(["rtk","proxy","git","diff","--cached","--binary","--",*paths], &:read); abort "empty oracle test patch" if patch.empty?; puts Digest::SHA256.hexdigest(patch)'
rtk proxy ruby -rdigest -e 'expected="bf408ea3dd8c4cc93f57f20c3b0949fbb9a18f9e60308f5ea2f154251869ba95"; paths=%w[crates/picea/src/collider.rs crates/picea/src/debug.rs crates/picea/src/events.rs crates/picea/src/pipeline/contacts.rs crates/picea/src/world/contact_state.rs]; patch=IO.popen(["rtk","proxy","git","diff","--binary","c4298ae","--",*paths], &:read); actual=Digest::SHA256.hexdigest(patch); abort "frozen implementation changed: #{actual}" unless actual == expected; puts actual'
rtk proxy ruby -e 'path="/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode"; abort "missing approved temp parent" unless Dir.directory?(path); puts path'
rtk proxy git worktree add --detach "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-baseline" c4298ae
rtk proxy git diff --cached --binary -- crates/picea-lab/tests/artifact_run.rs crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb crates/picea/tests/physics_realism_acceptance.rs | rtk proxy git -C "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-baseline" apply --check -
rtk proxy git diff --cached --binary -- crates/picea-lab/tests/artifact_run.rs crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb crates/picea/tests/physics_realism_acceptance.rs | rtk proxy git -C "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-baseline" apply -
rtk proxy git -C "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-baseline" diff --exit-code c4298ae -- crates/picea/src/collider.rs crates/picea/src/debug.rs crates/picea/src/events.rs crates/picea/src/pipeline/contacts.rs crates/picea/src/world/contact_state.rs
```

Verifier在两侧分别运行exact commands时，baseline使用`rtk proxy env CARGO_TARGET_DIR="/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-target" cargo test --manifest-path "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-baseline/Cargo.toml" ...`；target不写入worktree。Verifier比较三份allowlist文件的`rtk proxy git hash-object`输出完全相同。测试后supervisor执行：

```text
rtk proxy git diff --cached --binary -- crates/picea-lab/tests/artifact_run.rs crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb crates/picea/tests/physics_realism_acceptance.rs | rtk proxy git -C "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-baseline" apply -R --check -
rtk proxy git diff --cached --binary -- crates/picea-lab/tests/artifact_run.rs crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb crates/picea/tests/physics_realism_acceptance.rs | rtk proxy git -C "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-baseline" apply -R -
rtk proxy git -C "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-baseline" status --short
rtk proxy git worktree remove "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-baseline"
```

S4-ORACLE-RED commit得到固定hash `T` 后，S4-ORACLE-EVIDENCE再次验证：

- `git diff --exit-code c4298ae..T -- crates/picea/src/collider.rs crates/picea/src/debug.rs crates/picea/src/events.rs crates/picea/src/pipeline/contacts.rs crates/picea/src/world/contact_state.rs`必须为0；即`T`只新增tests/tool/receipt，不含working implementation。
- Clean baseline worktree checkout `T`；当前root也位于`T`但叠加同5份dirty implementation。
- 记录5文件implementation patch SHA-256、两边test blob hashes、exact theft/absorbed/unabsorbed和三条A14结果。
- Evidence receipt经verifier/spec reviewer通过后docs-only commit；此时仍不替换旧counter。

Post-commit evidence recipe以实际`T`替换下列占位符。Supervisor负责worktree create/remove；verifier只读检查并运行测试：

```text
rtk proxy git diff --exit-code c4298ae..T -- crates/picea/src/collider.rs crates/picea/src/debug.rs crates/picea/src/events.rs crates/picea/src/pipeline/contacts.rs crates/picea/src/world/contact_state.rs
rtk proxy git worktree add --detach "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-evidence" T
rtk proxy ruby -rdigest -e 'expected="bf408ea3dd8c4cc93f57f20c3b0949fbb9a18f9e60308f5ea2f154251869ba95"; paths=%w[crates/picea/src/collider.rs crates/picea/src/debug.rs crates/picea/src/events.rs crates/picea/src/pipeline/contacts.rs crates/picea/src/world/contact_state.rs]; patch=IO.popen(["rtk","proxy","git","diff","--binary","T","--",*paths], &:read); actual=Digest::SHA256.hexdigest(patch); abort "implementation patch mismatch: #{actual}" unless actual == expected; puts actual'
rtk proxy git diff --exit-code -- crates/picea-lab/tests/artifact_run.rs crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb crates/picea/tests/physics_realism_acceptance.rs
```

Clean evidence worktree使用`rtk proxy env CARGO_TARGET_DIR="/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-evidence-target" cargo test --manifest-path "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-evidence/Cargo.toml" ...`逐条运行；root使用同一`T` tests叠加固定implementation patch运行。两边记录`git rev-parse T:<test-path>`，三个blob hashes必须分别相同。Verifier结束后supervisor执行：

```text
rtk proxy git -C "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-evidence" status --short
rtk proxy git worktree remove "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s4-oracle-evidence"
```

`status --short`必须无输出；否则禁止remove并先调查非target文件来源。Cargo target位于worktree外，不参与Git或清理门。

双跑预期矩阵：

| Command | `c4298ae` + test patch | working implementation + same patch |
| --- | --- | --- |
| O01 partition exact | exit 0 | exit 0 |
| O01 desired zero exact | exit 101：`105=93+9+3`、`96=93+3`、exact predecessor25450/theft0；唯一desired缺口为final-unabsorbed3与unexpected3 | exit 0：`59=46+13+0`、`46=46+0`、source-boundary>0、exact predecessor25431/theft0 |
| O02 core positive exact | exit 0 | exit 0 |
| O03 disk artifact positive exact | exit 0 | exit 0 |
| O05 symmetric-edge negative exact | exit 101且唯一失败为symmetric edge误判 | exit 101且同一失败；S4-IMPL修复 |
| Matrix180 existing exact | exit 0 | exit 101且唯一旧counter actual 19；所有数值稳定门先通过 |
| Aligned1200 existing exact | exit 0 | exit 0 |
| Forced600 existing exact | exit 0 | exit 0 |
| Source freeze tool `exact` | exit 0 | exit 0；工具只比较test source，与dirty core无关 |

任一compile error、unrelated panic、数值门失败或test blob/hash不一致均为S4-ORACLE-RED FAIL。

只有S4-ORACLE-EVIDENCE证明新oracle完整且replacement语义获review批准，S4-ORACLE-REPLACE才可修改`artifact_run.rs`唯一含糊counter assertion。Replacement commit保留全部matrix数值门，允许baseline production对新的desired oracle保持intentional RED；随后S4-IMPL修symmetric-edge并让全部gates green。

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
- S4-ORACLE-D：`docs: design manifold attribution oracle`
- S4-ORACLE-RED：`test: attribute persistent manifold candidates`
- S4-ORACLE-EVIDENCE：`docs: record manifold attribution evidence`
- S4-ORACLE-REPLACE：`test: distinguish absorbed manifold candidates`
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

### 2026-07-13 - S4-D D4 acceptance

- D4 review：PASS；architecture/spec findings已闭环。
- S4-D已提交为`6045bd2 docs: design sat manifold persistence`。
- S4-RED进入test review；production implementation仍未开始。

### 2026-07-13 - S4-RED review remediation worker receipt（未commit）

#### 1. 成功标准

在`HEAD=6045bd2`的production baseline上补强A01-A15 acceptance artifacts；所有新测试compile，RED只来自行为断言，A14三条baseline保持green，且除批准的纯`#[cfg(test)]`candidate counter外不包含production implementation。

#### 2. 检查结果

编译预检：

- `rtk proxy cargo test -p picea --lib --no-run`：exit 0。
- `rtk proxy cargo test -p picea --test physics_realism_acceptance --no-run`：exit 0。
- `rtk proxy cargo check -p picea --lib`：exit 0；普通lib build不编译test-only counter storage/reset/read/increment路径。

逐项结果：

| Acceptance | Exact command / filter | Exit / count | Baseline与关键事实 |
| --- | --- | --- | --- |
| A01 | `rtk proxy cargo test -p picea --lib pipeline::narrowphase::tests::stacked_rectangles_expose_raw_feature_role_swap -- --exact --nocapture` | exit 0；1 passed / 0 failed | Green characterization；原fixture保留normal/local-drift门，raw feature index sets明确不同。 |
| A02 | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_sat_role_swap_preserves_both_contact_ids_by_local_anchor -- --exact --nocapture` | exit 0；1 passed / 0 failed | Baseline已green；两点均按唯一双侧local-witness对应保留各自`ContactId`和同一`ManifoldId`，raw index sets变化、第二帧slots distinct，且无warm `Hit`/impulse。 |
| A03 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::lifecycle_reservation_keeps_later_exact_match -- --exact --nocapture` | exit 101；0 passed / 1 failed | RED；test先按完整`ContactKey`排序并确认fallback稳定排在exact前；fallback仍偷走later exact previous。 |
| A04 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::lifecycle_reservation_maximizes_edge_swap_cardinality -- --exact --nocapture` | exit 101；0 passed / 1 failed | RED；缺边图只得到1个persisted，未实现期望cardinality 2。 |
| A05 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::warm_start_reservation_keeps_distinct_impulses_with_local_witnesses -- --exact --nocapture` | exit 101；0 passed / 1 failed | RED；两个current都消费sentinel normal impulse `1.0`，第二点未得到对应previous的`2.0`；edge-swap tangent均为0。 |
| A06 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::warm_start_reservation_maximizes_residual_cardinality_after_exact_matches -- --exact --nocapture` | exit 101；0 passed / 1 failed | RED；exact-hard子图同时输出`(Hit,30)`两次；residual子图输出normal impulses `[10,10,30]`而非`[20,10,30]`。 |
| A07 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::source_row_reservation_does_not_reuse_previous_point -- --exact --nocapture`；`rtk proxy cargo test -p picea --lib pipeline::contacts::tests::source_row_revision_is_rejected_before_solver_rows_are_built -- --exact --nocapture` | 两条均exit 101；各0 passed / 1 failed | RED；duplicate test中两个current同时借同一previous；revision test使用dynamic/static真实row，首帧`contact_row_count > 0`前置断言通过；patch后直接调用`collect_contact_observations`、`take_active_contacts`、`prepare_contact_warm_start`，solver前两点仍为`(Hit,false,Candidate)`而非精确`MissFeatureId`。 |
| A08 | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_geometry_patch_invalidates_all_history_consumers -- --exact --nocapture` | exit 101；0 passed / 1 failed | RED；successful shape和valid local-pose patch均保持两点overlap，但两者都错误为`(ExactFeature,Hit,false,Candidate)`；期望`Started`、新contact/manifold ids、精确`MissFeatureId`、source false。 |
| A09 | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_world_commands_geometry_patch_is_atomic -- --exact --nocapture` | exit 101；0 passed / 1 failed | 初始contacts精确2点；rejected scratch transaction子场景green并保持old ids/history；successful `WorldCommand::PatchCollider`子场景RED，仍为两点`(ExactFeature,Hit,false,Candidate)`而非精确`MissFeatureId`。 |
| A10 | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_sensor_transitions_do_not_expand_edge_swap_identity -- --exact --nocapture`；`rtk proxy cargo test -p picea --lib pipeline::contacts::tests::sensor_transition_same_index_does_not_persist_lifecycle -- --exact --nocapture` | integration exit 101，0 passed / 1 failed；same-index unit exit 0，1 passed / 0 failed | solid->sensor与sensor->sensor edge-swap均`Started/SkippedSensor`并保留pair manifold；sensor->solid错误`PersistentEdgeSwap/MissFeatureId`并复用contact ids。exact solid->sensor为`ExactFeature/SkippedSensor`，exact sensor->solid为`ExactFeature/MissPreviousSensor`，exact sensor->sensor为`ExactFeature/SkippedSensor`，三者均保持contact/manifold ids。same-index solid->sensor、sensor->sensor、sensor->solid均green并锁`Started`/新contact id/非`PersistentEdgeSwap`或`ExactFeature`。所有case impulse为0。 |
| A11 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::manifold_persistence_two_to_one_to_two_preserves_only_surviving_point -- --exact --nocapture`；`rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_two_to_one_to_two_preserves_only_surviving_point -- --exact --nocapture` | 两条均exit 0；各1 passed / 0 failed | Green boundary；unit以normal/tangent双sentinel锁住不复活；真实pipeline固定rectangle poses得到严格2/1/2，frame2为1 Persisted+1 Ended，frame3幸存继续、返回新`ContactId`、共享existing `ManifoldId`且warm normal/tangent为0。 |
| A12 | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_normalizes_geometric_a_b_order_with_revisions -- --exact --nocapture`；`rtk proxy cargo test -p picea --test physics_realism_acceptance warm_start_cache_uses_normalized_pair_identity_when_geometric_a_b_order_is_swapped -- --exact --nocapture` | 新锁exit 101，0 passed / 1 failed；existing锁exit 0，1 passed / 0 failed | 首次contact前分别给min/max ordered side制造不对称revision；两种fixture跨SAT role swap均保持IDs，随后分别patch min/max均未失效，错误输出两点`(PersistentEdgeSwap,MissFeatureId,false,EdgeSwap)`。existing normalized-pair保持green。 |
| A13 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::manifold_persistence_history_only_separation_does_not_fabricate_contact -- --exact --nocapture` | exit 0；1 passed / 0 failed | Green boundary；unconfirmed history-only source变成Ended；既有positive finite base-solve interaction仍保留depth 0 persisted语义。 |
| A14 | `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --exact --nocapture`；`rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --exact --nocapture`；`rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --exact --nocapture` | 三条均exit 0；各1 passed / 0 failed | 全部green；matrix180为48 sleeping / 0 awake，aligned1200为12 / 0，forced600为48 / 0；三者均无floor ejection，quiet linear/angular speed均0。 |
| A15 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::reservation_stays_pair_scoped_for_four_by_four_inputs_and_unrelated_pairs -- --exact --nocapture` | exit 101；0 passed / 1 failed | RED；baseline non-vacuous/bound门`7 > 0 && 7 <= 16`通过，with-unrelated non-vacuous门`71 > 0`通过；observable结果相同但4x4仍有1个`Started`，candidate evaluations从`7`增至`71`，直接锁住global scan。S4-IMPL必须将increment迁到新matcher真实candidate-edge predicate。 |

Living spec的聚合命令`rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_ -- --nocapture`最终为exit 101，2 passed / 4 failed：A02/A11 integration green，A08/A09/A10/A12 RED。没有新test被`ignore`。

格式与scope结果：

- 最终`rtk proxy cargo fmt --all --check`：exit 0。
- `rtk proxy git diff --check`：exit 0。
- `rtk proxy git diff --exit-code 6045bd2 --`：exit 1，原因是五个批准文件存在未提交改动。
- `rtk proxy git diff --name-only 6045bd2 --`：`crates/picea/src/pipeline/contacts.rs`、`crates/picea/src/pipeline/narrowphase.rs`、`crates/picea/tests/physics_realism_acceptance.rs`、S4 design、S4 living spec。
- `rtk proxy git diff --exit-code 6045bd2 -- crates/picea/src/lib.rs crates/picea/src/solver crates/picea/src/events.rs crates/picea/src/debug.rs crates/picea/src/collider.rs crates/picea/src/world/contact_state.rs`：exit 0。
- `contacts.rs`人工scope审计：现有`#[cfg(test)] mod tests`外只新增`#[cfg(test)]`thread-local counter、reset/read helper和candidate入口递增；非test返回值/排序无改动。`narrowphase.rs`改动仍只在现有tests module。
- `rtk proxy git status --short --branch`：`feat/vnext-s4-manifold-persistence`上仅上述五个文件为unstaged modified。

#### 3. 复跑方式

逐条复跑上表每个exact command；聚合filter只作为附加证据，不替代A03-A15分类。编译性先用上方两个`--no-run`与release `cargo check`确认，格式与scope按本节点末尾Git gate复核。

#### 4. 范围外

本节点没有修改production返回值/排序、struct/enum/API/doc comment、solver、Cargo、阈值或既有assertion；唯一非test-module代码是批准的`#[cfg(test)]`thread-local counter及candidate入口递增。没有实现revision/matcher，没有修改matrix tests，也没有stage/commit/push/branch/fetch。

#### 5. 残余风险

- A02在baseline已green，保留为双点identity/raw-geometry边界锁；production缺口由A03-A10/A12/A15稳定RED证明。
- A11 unit与真实pipeline integration均在baseline green，保留为2/1/2边界锁。
- A10 same-index unit在当前`ContactRecord`尚无private sensor field时只能用previous event的`warm_start_reason`表达previous sensor fact；S4-IMPL必须迁移为private `ContactRecord::is_sensor`事实，不得继续反推。
- Reviewer指出的public docs Low属于S4-IMPL；S4-RED没有修改`events.rs`/`debug.rs`，该Low明确留待implementation节点。
- S4-RED worker当时未commit；后续reviewer/verifier/supervisor已在下述closeout闭环。

### 2026-07-13 - S4-RED acceptance closeout

- Test-quality与scope reviewer最终均无High/Medium/Low；独立verifier逐A复跑PASS。
- S4-RED已提交为`c4298ae test: lock manifold persistence matching`。

### 2026-07-13 - S4-IMPL A14 blocker / S4-ORACLE authorization

- Working implementation使A03-A13/A15 exact green；matrix180稳定指标仍为48 sleeping / 0 awake、无ejection、quiet speed 0，但旧diagnostic counter从0变19，因此未提交。
- 三位只读reviewer确认warm solver-start provenance与final lifecycle阶段独立；现有counter不是一对一lifecycle absorption oracle。另发现symmetric-edge predicate Medium，留在oracle归因后修复。
- 用户明确授权新增S4-ORACLE强化链：先保持旧断言，提交O01-O05更强锁和双基线证据；只有spec/code review通过后才允许语义替换，全部数值稳定性阈值原样保留。

## 验收报告模板

每个已完成节点按固定顺序记录：

1. 成功标准
2. 检查结果：exact command、exit、captured output/artifact
3. 复跑方式
4. 范围外
5. 残余风险

### 2026-07-13 - S4-ORACLE-RED worker receipt（under review，未commit）

#### 1. 成功标准

在`HEAD=4a0c865`与固定5文件working implementation patch上，仅新增O01-O05 test-local oracle/behavior locks、matrix source freeze工具和本receipt；三条旧A14函数逐字冻结，不修改core、旧assertion、阈值、API或artifact schema。

#### 2. Working-side检查结果

| Gate | Exact command | Exit / count | 关键事实 |
| --- | --- | --- | --- |
| O01 partition | `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_edge_swap_attribution_partitions_every_candidate -- --exact --nocapture` | exit 0；1 passed / 0 failed | amend后三分类实际为reported `59=46 final-attributed + 13 source-boundary + 0 unexpected`；final candidates `46=46 attributed + 0 unabsorbed`；`exact_predecessor_total=25431`、`exact_theft=0`。Raw events finite、non-Hit warm impulses为0，snapshot与落盘`frames.jsonl`投影一致。 |
| O01 desired | `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_edge_swap_attribution_has_no_unabsorbed_or_exact_theft -- --exact --nocapture` | exit 0；1 passed / 0 failed | `final_attributed=46 > 0`、`source_boundary_evidence=13 > 0`、`final_unabsorbed=0`、`unexpected=0`、exact predecessor非真空且theft为0。 |
| O02 | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_sat_role_swap_reports_persistent_edge_swap -- --exact --nocapture` | exit 0；1 passed / 0 failed | 真实2x2 rectangle fixture；两点无full exact predecessor，按双侧local witness一一保持`ContactId`/`ManifoldId`，第二帧均为`ContactPersisted + PersistentEdgeSwap`。 |
| O03 | `rtk proxy cargo test -p picea-lab --test artifact_run artifact_records_persistent_edge_swap_with_stable_ids -- --exact --nocapture` | exit 0；1 passed / 0 failed | 从实际落盘`frames.jsonl`重反序列化；只从A类final-attributed key选择frame 7样本，raw feature `4311744514 -> 16785408`，稳定`ContactId(24)`/`ManifoldId(13)`且previous恰好一次；双侧local midpoint-anchor drift为`0.0021390484`。 |
| O05 | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_symmetric_edge_index_is_not_role_swap -- --exact --nocapture` | exit 101；0 passed / 1 failed，预期RED | 固定rectangle pose从`(-0.2, 2.0, PI-0.22)`到`(-0.2, 2.0, PI-0.18)`；full raw feature `4311752706 -> 16785410`，共同index `16785410`，decode为`kind=1, reference=2, incident=2`，slot `1 -> 0`，第二帧`MissFeatureId`；唯一最终缺口为当前仍输出`PersistentEdgeSwap/EdgeSwap`。 |
| A14 matrix180 | `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --exact --nocapture` | exit 101；0 passed / 1 failed，预期旧counter RED | 唯一失败为冻结assertion `edge_swap_candidate_count` actual `19` vs expected `0`；此前数值门通过：max penetration `0.027232071`、quiet linear/angular `0/0`、无floor ejection、final `0 awake / 48 sleeping`。 |
| A14 aligned1200 | `rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --exact --nocapture` | exit 0；1 passed / 0 failed | max penetration `0.003365`、无floor ejection、final `0 awake / 12 sleeping`、quiet speed `0/0`。 |
| A14 forced600 | `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --exact --nocapture` | exit 0；1 passed / 0 failed | max penetration `0.027232`、无floor ejection、quiet speed `0/0`、final `0 awake / 48 sleeping`。 |
| Source freeze | `rtk proxy ruby crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb c4298ae exact` | exit 0 | `s4 matrix source freeze exact ok`；缺少第三参数的`replace-one-oracle`验证为非0并报告必须提供approved replacement SHA-256。 |
| Compile | 两个相关test target分别运行`--no-run` | 两条均exit 0 | 无warning。 |
| Format | `rtk proxy rustfmt --edition 2021 --check crates/picea-lab/tests/artifact_run.rs crates/picea/tests/physics_realism_acceptance.rs` | exit 0 | 两份allowlisted Rust测试已格式化。 |
| Workspace fmt observation | `rtk proxy cargo fmt --all --check` | exit 1 | 仅报告冻结`crates/picea/src/pipeline/contacts.rs` working patch的既有格式差异；为保持freeze未运行workspace写入式fmt。 |
| Diff/staging | `rtk proxy git diff --check`；`rtk proxy git diff --cached --exit-code` | 两条均exit 0 | 无whitespace error，无staged内容。 |

5份frozen implementation相对`c4298ae`的binary diff SHA-256复算为`bf408ea3dd8c4cc93f57f20c3b0949fbb9a18f9e60308f5ea2f154251869ba95`。

#### 3. Review remediation

- O01/O03不再用`event.depth`制造surface witness；只在排除confirmed-source transition后，把authoritative final `event.point`按相邻两帧各自两侧collider final pose逆变换，使用双侧local midpoint-anchor drift `<=0.25`。这是独立的artifact-observable midpoint projection，不对core private surface witnesses主张subset或等价关系。
- Production source确认`base_normal_solve_executed`是private `ContactObservation`字段，未导出到`ContactEvent`。Test-local SAT projection使用`FinalSatContact / NoFinalSatManifold / Unprovable`三态；缺collider、sensor、unsupported shape、non-finite transform/vertex/projection或退化轴均为`Unprovable`。B类previous/current每个endpoint都必须分别满足`FinalSatContact || strict_source_endpoint`，并且至少一端是strict source；任何非strict的`NoFinalSatManifold`或`Unprovable`均归unexpected。
- Reverse pass按`(current frame, ContactId)`互斥分类全部59个`ContactPersisted + PersistentEdgeSwap`：46个有完整selected final match的A类；13个有唯一same-ID/same-manifold predecessor、无full exact predecessor、非symmetric raw role swap、至少一端strict source endpoint且完整finite/warm gate的B类；其余归C类，当前为0。Source-boundary previous/current transition key分别一对一消费，不与A或C重叠。
- MatrixStack由本test直接走fresh offline `run_scenario`且无compound/perturbation provenance；test-local authoring signature按manifest已知rect descriptor、原始有序local vertices与local pose `to_bits()`稳定JSON序列化，逐`ColliderHandle`逐帧完全相同，并精确重建每帧ordered world vertices。内建反例确认可区分`+0.0005`宽度patch与同边长shear deformation；O01不再使用旧lossy `shape_signature`。
- Freeze tool先对comments/strings/raw strings做等长lexical mask，只抽取行首真实`fn NAME(`，向前包含连续attributes并向后brace-balance完整item。`exact`逐字比较；`replace-one-oracle`只接受base旧counter assertion的单span替换，要求approved replacement SHA-256且prefix/suffix逐字不变，并拒绝marker或任意真实`return`token。

#### 4. 当前状态与范围外

状态保持`S4-ORACLE-RED under review`。本worker没有创建subagent、stage、commit、push、fetch、branch或worktree；没有执行baseline双跑、supervisor acceptance或S4-ORACLE-REPLACE。O01/O02/O03 working green，O05仍是预期behavior RED，matrix180旧counter保持未改；未修改core或旧A14任何字符。

### 2026-07-13 - S4-ORACLE-RED pre-commit dual-baseline receipt

- Test/tool cached binary patch SHA-256：`1299a4c0082c174d4902260e9a1304831277a7c7bdd118b24c9325565435c0d8`；两侧三文件blob hashes逐项相同。
- Baseline为detached `c4298ae` +同一cached patch；5份production相对`c4298ae`零diff。Working为`4a0c865` +同一tests +冻结implementation hash `bf408ea3dd8c4cc93f57f20c3b0949fbb9a18f9e60308f5ea2f154251869ba95`。
- O01 partition两侧均exit0。Baseline desired按首次证据amend为intentional RED：reported `105=93 final-attributed + 9 source-boundary + 3 unexpected`；final candidates `96=93 attributed + 3 unabsorbed`；exact predecessors `25450`、theft0。Working desired exit0：`59=46+13+0`、`46=46+0`、exact predecessors `25431`、theft0。
- O02/O03两侧均exit0；O03 baseline disk sample为frame8 `ContactId(33)`/`ManifoldId(20)`，working为frame7 `ContactId(24)`/`ManifoldId(13)`。
- O05两侧均为预期exit101，只因symmetric edge仍输出`PersistentEdgeSwap/EdgeSwap`。
- Matrix180 baseline exit0；working预期exit101且仅旧counter `19 != 0`，其余稳定门先通过。Aligned1200与forced600两侧均exit0、无ejection、quiet speed0且全部sleeping。
- 两侧compile no-run与source-freeze exact均green。Verifier结论最初因baseline预期写成exact theft而FAIL；本次只amend预期为实际可复现的unabsorbed/unexpected证据，不修改tests、core或旧A14。
- Supervisor已reverse同一cached patch并clean remove临时baseline worktree。S4-ORACLE-RED仍待spec reviewer按实际证据裁决、最终cached scope复核和commit。

### 2026-07-13 - S4-ORACLE-EVIDENCE post-commit receipt

#### 1. 成功标准

在固定test commit `T=4a32ddb`与`T + bf408ea3...ba95` working implementation上复跑同一O01-O05/A14矩阵；tests/tool blobs完全相同，baseline production零diff，实际counts与pre-commit evidence精确一致，且不修改tests/oracle/core。

#### 2. 检查结果

- Clean evidence与root working HEAD均为`4a32ddbe79ecc7afd44a8d12b4ce1e8f1f44317b`。
- `c4298ae..4a32ddb`的5份production文件零diff；root三份tests/tool相对`T`零diff。Blob hashes：artifact `cd26660035b720e7b3b956cd9bdc6fd93669ff45`、freeze tool `1f6dbbeb4c3d9e00f52e2a3c661c24c2670f122e`、physics acceptance `40a5240ded334c78d8899899c87094a99b62f22a`。
- Working implementation binary patch SHA-256精确为`bf408ea3dd8c4cc93f57f20c3b0949fbb9a18f9e60308f5ea2f154251869ba95`。
- O01 partition两侧exit0。Clean `T` desired为预期exit101：reported `105=93 final-attributed + 9 source-boundary + 3 unexpected`；final candidates `96=93 attributed + 3 unabsorbed`；exact predecessors `25450`、theft0。Working desired exit0：`59=46+13+0`、`46=46+0`、exact predecessors `25431`、theft0。两个baseline非零“3”只分别记录分类计数，不主张相同transition keys。
- O02/O03两侧均exit0。O03 clean disk sample为frame8、`ContactId(33)`/`ManifoldId(20)`；working为frame7、`ContactId(24)`/`ManifoldId(13)`。
- O05两侧均为预期exit101且仅symmetric edge误标；留给S4-IMPL。
- Matrix180 clean exit0；working预期exit101且仅冻结旧counter `19 != 0`，其余稳定门先通过。Aligned1200与forced600两侧均exit0。
- Matrix180/forced600 clean penetration max/sum `0.028555/0.799082`，working `0.027232/0.927301`；aligned两侧`0.003365/0.054014`。全部无ejection、quiet speed0，matrix/forced为0 awake/48 sleeping，aligned为0/12。
- 两侧compile no-run与source-freeze exact均green。Evidence worktree最终clean并已由supervisor移除。

#### 3. 复跑方式

使用本计划Post-commit evidence recipe，以`T=4a32ddb`替换占位符；Cargo target置于批准temp目录，verifier只读运行，worktree create/remove由supervisor负责。

#### 4. 范围外

本节点未修改tests/oracle/core、旧A14 assertion、任何数值阈值或public schema；只追加design状态与本docs receipt。S4-ORACLE-REPLACE尚未开始。

#### 5. 残余风险

- Baseline的3个final-unabsorbed和3个unexpected属于独立统计域，未建立key一一对应关系。
- Artifact midpoint projection不等价于core private surface witnesses；A02/O02继续拥有core双侧identity合同。
- O05 symmetric-edge行为锁仍RED；S4-IMPL必须修复。

### 2026-07-13 - S4-ORACLE-REPLACE working-side receipt

- Replacement只修改`matrix_stack_artifacts_capture_nxm_grid_stack_facts`中的旧`edge_swap_candidate_count == 0` assertion span；改为调用同一次`run.frames`的固定O01 attribution helper，并断言两套守恒、final/source正向非真空、final-unabsorbed/unexpected/theft为0且exact predecessor非真空。
- `rtk proxy ruby crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb c4298ae replace-one-oracle ec463132197639a58bf7b1287dcdb8d9c91059839d263d149a23530ed643a0df`：exit0。Approved replacement SHA-256为`ec463132197639a58bf7b1287dcdb8d9c91059839d263d149a23530ed643a0df`；完整test item的attributes/signature/prefix/suffix逐字冻结。
- Working O01 desired exit0：`59=46 final-attributed + 13 source-boundary + 0 unexpected`、`46=46 attributed + 0 unabsorbed`、exact predecessors25431/theft0。
- Working O03 exit0，disk sample仍为frame7 `ContactId(24)`/`ManifoldId(13)`。
- Working matrix180由旧counter RED转为exit0；max penetration/sum`0.027232/0.927301`、无ejection、quiet speed0、0 awake/48 sleeping，其他既有断言与阈值未改。
- O05仍为预期exit101，且唯一失败为symmetric raw edge输出`PersistentEdgeSwap/EdgeSwap`；留给S4-IMPL。
- `rustfmt --check`与`git diff --check`green。5份frozen implementation hash仍为`bf408ea3dd8c4cc93f57f20c3b0949fbb9a18f9e60308f5ea2f154251869ba95`。
- 当前仅完成working-side验证；clean baseline +同一replacement patch的intentional RED、aligned1200/forced600及最终replacement commit gate仍待独立verifier。

### 2026-07-13 - S4-ORACLE-REPLACE dual verifier receipt

- Replacement cached binary patch SHA-256：`50cb9dd5110e21e2f6613a6a7810d0e5354f073a8037d747674eff879749cbf6`；artifact file两侧hash均为`6cb678a15df4d4bff099173b4648886e8855f609`。
- Baseline为clean`cca2475` +同一replacement patch，5份production零diff；working为`cca2475` +同一replacement +固定`bf408ea3...ba95` implementation patch。
- Source-freeze `replace-one-oracle`两侧均exit0，approved replacement digest为`ec463132197639a58bf7b1287dcdb8d9c91059839d263d149a23530ed643a0df`。
- O01 partition两侧exit0。Baseline desired与replacement matrix均为预期exit101：`105=93+9+3`、`96=93+3`、exact predecessors25450/theft0；working O01与matrix180均exit0：`59=46+13+0`、`46=46+0`、exact predecessors25431/theft0。
- O02补充exact verifier：clean baseline与working均exit0、各1 passed / 0 failed。O03两侧exit0；O05两侧保持同一symmetric intentional RED。Aligned1200、forced600与两个compile no-run两侧均exit0。
- Matrix180/forced600 baseline penetration max/sum`0.028555/0.799082`，working`0.027232/0.927301`；aligned两侧`0.003365/0.054014`。全部无ejection、quiet speed0并全部sleeping。
- Baseline的3个final-unabsorbed与3个unexpected仍只分别表示各自分类计数，不主张相同transition keys。
- Supervisor已reverse同一replacement patch并clean remove临时baseline worktree。Replacement仍待最终test/code与scope reviewer确认后提交。

### 2026-07-14 - S4-IMPL worker receipt（under review，未commit）

#### 1. 成功标准

在`HEAD=d9d96b0`已提交ORACLE-REPLACE上完成private revision与sensor facts、pair-scoped三consumer reservation、exact-hard/residual maximum-cardinality matching和严格non-symmetric edge-swap分类；全部implementation targeted/full core gates green，tests/oracle/solver/lib/narrowphase production零diff。

#### 2. 检查结果

逐项实现锁：

| Gate | Exact command | Exit / count | 关键事实 |
| --- | --- | --- | --- |
| A01 | `rtk proxy cargo test -p picea --lib pipeline::narrowphase::tests::stacked_rectangles_expose_raw_feature_role_swap -- --exact --nocapture` | exit 0；1 passed / 0 failed | Raw SAT role swap characterization保持green。 |
| A03 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::lifecycle_reservation_keeps_later_exact_match -- --exact --nocapture` | exit 0；1 passed / 0 failed | Compatible full exact保持硬预留。 |
| A04 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::lifecycle_reservation_maximizes_edge_swap_cardinality -- --exact --nocapture` | exit 0；1 passed / 0 failed | 缺边residual graph选择cardinality 2。 |
| A05 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::warm_start_reservation_keeps_distinct_impulses_with_local_witnesses -- --exact --nocapture` | exit 0；1 passed / 0 failed | 两个sentinel impulses一对一且edge-swap tangent清零。 |
| A06 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::warm_start_reservation_maximizes_residual_cardinality_after_exact_matches -- --exact --nocapture` | exit 0；1 passed / 0 failed | Exact后先最大cardinality，再应用warm same-index偏好。 |
| A07 | 两条`source_row_reservation_does_not_reuse_previous_point` / `source_row_revision_is_rejected_before_solver_rows_are_built` exact命令 | 均exit 0；各1 passed / 0 failed | Source-row独立一对一；revision mismatch在solver前拒绝。 |
| A08/A09 | 两条`manifold_persistence_geometry_patch_invalidates_all_history_consumers` / `manifold_persistence_world_commands_geometry_patch_is_atomic` exact命令 | 均exit 0；各1 passed / 0 failed | Direct/transaction geometry patch分配新ContactId与ManifoldId；rejected transaction保留history。 |
| A10 | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_sensor_transitions_do_not_expand_edge_swap_identity -- --exact --nocapture`；same-index unit exact | 两条均exit 0；各1 passed / 0 failed | Previous sensor只读private `ContactRecord::is_sensor`；sensor fallback不扩大lifecycle。 |
| A11 | 两条`pipeline::contacts::tests::manifold_persistence_two_to_one_to_two_preserves_only_surviving_point` / integration同名exact命令 | 均exit 0；各1 passed / 0 failed | Unit与真实pipeline 2->1->2均保持幸存id并拒绝复活返回point history。 |
| A12 | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_normalizes_geometric_a_b_order_with_revisions -- --exact --nocapture` | exit 0；1 passed / 0 failed | Ordered revisions按normalized collider handles对齐。 |
| A13 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::manifold_persistence_history_only_separation_does_not_fabricate_contact -- --exact --nocapture` | exit 0；1 passed / 0 failed | Separation与confirmed interaction例外保持。 |
| A15 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::reservation_stays_pair_scoped_for_four_by_four_inputs_and_unrelated_pairs -- --exact --nocapture` | exit 0；1 passed / 0 failed | Test-only counter位于真实lifecycle residual candidate-edge gate；实际`base=4`、`with_unrelated=4`。 |
| O01 partition/desired | 两条`matrix_stack_edge_swap_attribution_*` exact命令 | 均exit 0；各1 passed / 0 failed | Reported `59=46 final-attributed + 13 source-boundary + 0 unexpected`；final candidates `46=46+0`；exact predecessors `25431`、theft 0。 |
| O02/O03 | `manifold_persistence_sat_role_swap_reports_persistent_edge_swap` / `artifact_records_persistent_edge_swap_with_stable_ids` exact命令 | 均exit 0；各1 passed / 0 failed | O03落盘样本frame 7，raw `4311744514 -> 16785408`，`ContactId(24)` / `ManifoldId(13)`，drift `0.0021390484`。 |
| O05 | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_symmetric_edge_index_is_not_role_swap -- --exact --nocapture` | exit 0；1 passed / 0 failed | Same raw index/slot drift与`reference_edge == incident_edge`均不再进入edge-swap；lifecycle/source-row共享严格predicate。 |

Matrix与聚合门：

| Gate | Command | Exit / count | 指标 |
| --- | --- | --- | --- |
| Matrix180 replacement | `rtk proxy cargo test -p picea-lab --test artifact_run matrix_stack_artifacts_capture_nxm_grid_stack_facts -- --exact --nocapture` | exit 0；1 passed / 0 failed | penetration max/sum `0.027232/0.927301`；0 awake / 48 sleeping；quiet linear/angular `0/0`；无ejection。 |
| Aligned1200 | `rtk proxy cargo test -p picea-lab --test artifact_run aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock -- --exact --nocapture` | exit 0；1 passed / 0 failed | penetration `0.003365/0.054014`；0 / 12；quiet `0/0`；无ejection。 |
| Forced600 | `rtk proxy env PICEA_MATRIX_STACK_E4_ACCEPTANCE=1 cargo test -p picea-lab --test artifact_run matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed -- --ignored --exact --nocapture` | exit 0；1 passed / 0 failed | penetration `0.027232/0.927301`；0 / 48；quiet `0/0`；无ejection。 |
| Narrowphase targeted | `rtk proxy cargo test -p picea --lib pipeline::narrowphase::tests -- --nocapture` | exit 0；18 passed / 0 failed | Raw SAT/clip behavior保持。 |
| Contacts targeted | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests -- --nocapture` | exit 0；31 passed / 0 failed | 三consumer、matching、ranking、finite和5x5边界全部green；A15同时输出`4/4`。 |
| Manifold targeted | `rtk proxy cargo test -p picea --test physics_realism_acceptance manifold_persistence_ -- --nocapture` | exit 0；8 passed / 0 failed | A02/A08-A12/O02/O05全部green。 |
| Existing warm/stack | `sat_edge_swap_candidate_persists_lifecycle_without_auto_warm_start_impulse -- --exact`；`warm_start_cache`；`stack_4` filters | 均exit 0；分别1、13、3 passed | Warm provenance未由matrix需求改写；stack稳定合同保持。 |
| Source freeze | `rtk proxy ruby crates/picea-lab/tests/verify_s4_matrix_source_freeze.rb c4298ae replace-one-oracle ec463132197639a58bf7b1287dcdb8d9c91059839d263d149a23530ed643a0df` | exit 0 | `s4 matrix source freeze replace-one-oracle ok`。 |
| Core lib | `rtk proxy cargo test -p picea --lib` | exit 0；128 passed / 0 failed | 无ignored或filtered失败。 |
| Full realism | `rtk proxy cargo test -p picea --test physics_realism_acceptance` | exit 0；79 passed / 0 failed | 全target green。 |
| Static | `rtk proxy cargo check -p picea --lib`；`rtk proxy cargo fmt --all --check`；`rtk proxy git diff --check` | 全部exit 0 | `cargo fmt --all`后tests/tool相对`d9d96b0`零diff。 |

Review findings的TDD RED证据：

| Focused lock | Exact command | RED exit / count | 当前失败断言 |
| --- | --- | --- | --- |
| Previous sensor source rejection | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::source_row_rejects_previous_sensor_feature_miss -- --exact --nocapture` | exit 101；0 passed / 1 failed | Warm reason已为`MissFeatureId`，但`!source_row_continuity_candidate`失败，previous sensor仍错误授权source row。 |
| Warm-hit source bypass | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::source_row_keeps_unknown_for_exact_and_same_index_warm_hits -- --exact --nocapture` | exit 101；0 passed / 1 failed | Exact与same-index fixture先锁定warm `Hit`；目标断言actual `PairMismatch`、expected `Unknown`。首次fixture因current witness未对齐而在`DroppedPointDrift != Hit`提前失败，修正仅限test witness后取得本目标RED。 |
| Edge warm-hit distractor | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::edge_swap_warm_hit_ignores_source_compatible_distractor -- --exact --nocapture` | exit 101；0 passed / 1 failed | Actual `Unknown`、expected `EdgeSwap`；source-compatible distractor覆盖了actual warm edge classification。 |
| Exact single-sided NaN | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::exact_warm_start_rejects_single_sided_nonfinite_witness -- --exact --nocapture` | exit 101；0 passed / 1 failed | Actual `Hit`、expected `DroppedPointDrift`；单侧NaN被projection中的`FloatNum::max`吞掉。 |

Review findings的GREEN与Low边界：

| Focused lock | Exact command | Exit / count | 结果 |
| --- | --- | --- | --- |
| Previous sensor source rejection | 同上`source_row_rejects_previous_sensor_feature_miss` exact | exit 0；1 passed / 0 failed | `MissFeatureId`但previous sensor时`false/Sensor`。 |
| Compatible warm-hit bypass | 同上`source_row_keeps_unknown_for_exact_and_same_index_warm_hits` exact | exit 0；1 passed / 0 failed | 仅Exact/SameFeatureIndex warm `Hit`为`false/Unknown`；edge-swap Hit仍`EdgeSwap`，真实no-pair仍`NoPreviousPair/PairMismatch`。 |
| Source contention | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::source_row_contention_keeps_unreserved_reason_unknown -- --exact --nocapture` | exit 0；1 passed / 0 failed | 同pair合法candidate被另一current消费后，未reserved row为`false/Unknown`。 |
| Kind/max/total ranking | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::residual_matching_ranks_kind_then_max_and_total_drift -- --exact --nocapture` | exit 0；1 passed / 0 failed | Equal cardinality依次验证warm SameFeatureIndex数量、最小max drift、最小total drift。 |
| Lex/permutation | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::residual_matching_lexicographic_tie_break_is_input_order_independent -- --exact --nocapture` | exit 0；1 passed / 0 failed | 完全同分时按完整mapping字典序；反转current输入结果不变。 |
| Finite/5x5 | `rtk proxy cargo test -p picea --lib pipeline::contacts::tests::reservation_rejects_nonfinite_edges_and_handles_five_by_five -- --exact --nocapture` | 首跑exit 101，0 passed / 1 failed；修复后exit 0，1 passed / 0 failed | RED为单侧NaN被`FloatNum::max`吞掉；共享drift显式检查双侧finite后NaN fail-closed，5x5完成5个distinct reservations。 |
| Edge warm-hit distractor | 同上`edge_swap_warm_hit_ignores_source_compatible_distractor` exact | RED exit 101，0 passed / 1 failed；GREEN exit 0，1 passed / 0 failed | Actual warm reservation kind直接决定source reason；PersistentEdgeSwap为`false/EdgeSwap`，不再被其他previous覆盖成`Unknown`。 |
| Exact single-sided NaN | 同上`exact_warm_start_rejects_single_sided_nonfinite_witness` exact | RED exit 101，0 passed / 1 failed；GREEN exit 0，1 passed / 0 failed | `warm_start_transfer`在任何projection/max前检查两侧drift vector与length finite；输出`DroppedPointDrift`且全部warm/solver seeds为0。 |

Changed files：`crates/picea/src/collider.rs`、`crates/picea/src/world/contact_state.rs`、`crates/picea/src/pipeline/contacts.rs`、`crates/picea/src/events.rs`、`crates/picea/src/debug.rs`，以及本次状态/receipt的S4 design与living spec。`contacts.rs`同时包含private-field test constructor适配、8条additive inline tests和A15计数输出；external `crates/picea/tests`、`crates/picea-lab/tests`及oracle assertions相对`d9d96b0`零diff。五core binary diff SHA-256为`3880f3c9a9625dbd67bfb0f22e42ff8e851525883c29e2650b9547d631517416`。

#### 3. 复跑方式

按本计划“Targeted green commands”顺序复跑，再运行上述A03-A15先前RED exact、O01-O05、source-freeze、`cargo test -p picea --lib`、full `physics_realism_acceptance`及三条static命令。Matrix命令保留`--exact --nocapture`，forced600必须同时给出`PICEA_MATRIX_STACK_E4_ACCEPTANCE=1`与`--ignored`。

#### 4. 范围外

未修改external committed tests/helper/oracle assertions、任何阈值、Cargo、solver、`lib.rs`、narrowphase production、public enum/field/serde或artifact schema。`contacts.rs`内已提交inline test constructors因新增private fields而适配，本轮又增加8条focused inline tests并为A15增加nocapture计数输出；所有旧assertion保留。未运行workspace full、clippy、bench或进入S4-V；未创建subagent、stage、commit、push、fetch、branch或worktree。

#### 5. 残余风险

- `u64` geometry revision理论上可wrap；当前合同只假设实际world生命周期内不发生。
- Residual matching按当前每pair至多4x4枚举全部合法matching；未来多点manifold扩大`k`时需重新评估组合复杂度。
- A08/A09尚未额外锁定“先制造nonzero revision再进入WorldCommands scratch clone”与revision mismatch时old `ContactEnded`事件；现有direct/transaction tests已锁三consumer invalidation和新ids，本轮为避免扩大external test diff将这两项保留为Low residual。
- S4-IMPL的spec/code review与独立targeted verifier已通过；S4-V workspace/clippy/bench尚未执行，不宣称milestone complete。

### 2026-07-14 - S4-IMPL independent targeted verifier

- 独立verifier结论PASS；全程只读，未修改、stage、commit、fetch、worktree或format。
- A01-A15逐exact全部exit0；A07、A10、A11、A12的多条unit/integration均逐条通过。A15 candidate evaluations稳定为`base=4 / with_unrelated=4`。
- 8条review新增inline exact锁全部exit0：previous sensor、exact/same warm Hit、edge distractor、single-sided NaN、source contention、kind/max/total、lex/permutation、finite/5x5。
- Narrowphase `18/18`、contacts `31/31`、picea lib `128/128`、full physics realism `79/79`、manifold `8/8`、warm `13/13`、stack `3/3`。
- O01 partition/desired均green：reported `59=46+13+0`、final candidates `46=46+0`、exact predecessors25431/theft0。O02/O03/O05均green；O03为frame7 `ContactId(24)`/`ManifoldId(13)`。
- Matrix180 replacement：penetration max/sum`0.027232/0.927301`、0 awake/48 sleeping、quiet0/0、无ejection。Aligned1200：`0.003365/0.054014`、0/12。Forced600：`0.027232/0.927301`、0/48；均quiet0/0、无ejection。
- `cargo fmt --all --check`、`cargo check -p picea --lib`、source-freeze replacement、`git diff --check`全部exit0。
- External tests/oracle、narrowphase、solver、`lib.rs`相对`d9d96b0`零diff；最终仅5core+2docs unstaged dirty，无untracked/staged。5core binary diff SHA-256为`3880f3c9a9625dbd67bfb0f22e42ff8e851525883c29e2650b9547d631517416`。
- Code reviewer round3无High/Medium；保留A08/A09 nonzero revision scratch clone与old `ContactEnded`专项锁的Low residual，不阻塞S4-IMPL commit。

### 2026-07-14 - S4-V full verification与closeout

- S4-IMPL已提交为`91698b3 fix: persist sat manifold point identity`。首次S4-V运行到workspace clippy时发现唯一warning：`bool::then`闭包应改为`then_some`；虽然进程exit0，仍按warning-zero规则判FAIL并停止。
- Remediation仅把无副作用的`then(|| match ...)`机械改为`then_some(match ...)`。独立code reviewer确认语义等价且无High/Medium/Low；contacts `31/31`、fmt、clippy与diff check先行green，随后提交为`57cdb19 chore: satisfy manifold matching clippy`。
- 独立full verifier从头复跑PASS：fmt exit0；picea lib `128/128`；physics realism `79/79`；artifact `30 passed / 5 ignored`；workspace all-targets harness合计`369 passed / 5 ignored`，9个Criterion场景全部`Success`；clippy exit0且0 warnings；examples与workspace bench no-run均构建成功。
- Matrix180为penetration max/sum`0.027232/0.927301`、0 awake/48 sleeping、quiet0/0、无ejection、hash`0efffe6f80f71d72`。Aligned1200为`0.003365/0.054014`、0/12、hash`34902715d547abc5`。Forced600为`0.027232/0.927301`、0/48、hash`9f5998a3db236c97`；后两者同样quiet0/0、无ejection。
- Source-freeze replacement digest保持`ec463132197639a58bf7b1287dcdb8d9c91059839d263d149a23530ed643a0df`。`lib.rs`与整个solver相对`9427a17`零diff。
- `9427a17..57cdb19`共16个批准路径：4个S4 tests/tool、5个core implementation、7个docs/routing/plans；无§5、solver、`lib.rs`、Cargo或public schema改动。S4-V结束时worktree clean。
- 保留Low residual：A08/A09尚无nonzero revision scratch-clone与old `ContactEnded`专项锁；`u64` revision理论wrap；residual matcher未来超过当前每pair4点上界时需重新评估组合复杂度。以上均不阻塞S4 closeout。
- S4-C只同步living spec、父计划、handoff与AI routing，不新增行为。Handoff §4至此完成；下一节点是§5 revolute joint public API gate，§6与damping/grab仍按既定顺序等待。
