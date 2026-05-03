# Performance Threshold Policy

M29 defines how Picea should turn benchmark evidence into regression guards
without making one noisy workstation run block correctness work.

## Scenario Set

Initial evidence should cover only the high-signal scenarios already present in
`crates/picea/benches/physics_scenarios.rs`:

- `query_heavy`: query traversal, candidate, prune, filter, and hit counters.
- `sparse_broadphase` and `dense_broadphase`: broadphase traversal/prune cost.
- `stack_stability`: stacked contact rows and solver island counters.
- `ccd_bullet` and `ccd_dynamic_pair`: CCD candidate/hit/clamp counters.
- `api_batch_creation`: recipe/world-command authoring throughput.

## Baseline Collection

Use Criterion for timing and deterministic engine counters for explanation.

1. Run `rtk proxy cargo bench -p picea --bench physics_scenarios -- --test`
   as the local smoke. This proves the benchmark harness builds and executes.
2. For a real baseline, run the same bench without `-- --test` at least five
   times on the same machine, with power/thermal conditions recorded.
3. Save Criterion output under `target/criterion/` and summarize:
   commit, machine, OS, Rust version, scenario, median, MAD or relative variance,
   and the deterministic counters embedded in benchmark IDs.
4. Compare counter changes before interpreting wall-clock changes. A timing
   regression with changed counters may be expected behavior; unchanged counters
   with a large timing change is a stronger performance signal.

## Guard Levels

- `informational`: any single local run, any first-time baseline, or any run on
  an unknown machine. Never fails CI.
- `warn`: median regression is above the provisional threshold across three or
  more comparable runs, or deterministic counters change unexpectedly. Warns in
  local/CI reports but does not fail correctness gates.
- `fail`: only allowed after a scenario has at least five comparable baselines
  and two review-approved thresholds: a wall-clock threshold and a counter
  expectation. Failures must include the comparison baseline and the changed
  counters.

## Provisional Thresholds

No M29 threshold is hard-failing by default. The first provisional warning lines
should be:

- query/broadphase scenarios: warn at 20 percent median regression when
  traversal/prune/candidate counters are within 5 percent.
- stack/CCD scenarios: warn at 25 percent median regression when row/candidate
  counters are within 5 percent.
- api batch creation: warn at 20 percent median regression when created
  body/collider counts are unchanged.

These percentages are intentionally warnings. They become failures only after
the baseline collection rule above is satisfied.

## Fallback

When performance data is noisy or unavailable, run the correctness gates first:

- `rtk proxy cargo test -p picea --test query_debug_contract`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo bench -p picea --no-run`

Do not block physics correctness work on wall-clock data from an unstable
environment.

