#!/usr/bin/env ruby
# frozen_string_literal: true

require "open3"

ROOT = File.expand_path("../../..", __dir__)
SPEC = "docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md"
MILESTONE_BASE = "28867b5c8eeae3ec290ad7dfd5f52cbf3ca69e80"
PROGRESS_HEADING = "## 16. 进度日志"
PROGRESS_HEADER = "| 日期 | Node | Start HEAD (full, immutable) | 状态 | Commit | Receipt / 备注 |"
PROGRESS_NODES = %w[
  S5-D
  S5-API-RED
  S5-API
  S5-BEHAVIOR-RED
  S5-SOLVER
  S5-REPLAN
  S5-BEHAVIOR-RED-2
  S5-SOLVER-2
  S5-BEHAVIOR-RED-3
  S5-SOLVER-3
  S5-REPLAN-2
  S5-BEHAVIOR-RED-4
  S5-SOLVER-4
  S5-REPLAN-3
  S5-BEHAVIOR-RED-5
  S5-SOLVER-5
  S5-LAB-RED
  S5-LAB
  S5-V
  S5-C
].freeze
PROGRESS_ROW = /\A\|\s*(?<date>[^|]+?)\s*\|\s*(?<node>S5-[A-Z0-9-]+)\s*\|\s*(?<start>[^|]+?)\s*\|\s*(?<status>[^|]+?)\s*\|\s*(?<commit>[^|]+?)\s*\|\s*(?<receipt>[^|]+?)\s*\|\s*\z/

S5_D_REQUIRED = %w[
  docs/ai/doc-catalog.yaml
  docs/ai/index.md
  docs/ai/repo-map.md
  docs/design/README.md
  docs/design/2026-07-14-revolute-joint-v1-design.md
  docs/handoff-2026-07-14-vnext-s5-revolute-joint.md
  docs/plans/2026-06-17-physics-realism-vnext-milestones.md
  docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
].freeze

SCOPES = {
  "S5-API-RED" => {
    required: %w[
      crates/picea/tests/verify_revolute_public_api.rb
      crates/picea/tests/verify_revolute_scope.rb
      crates/picea/tests/fixtures/revolute_public_api/Cargo.toml
      crates/picea/tests/fixtures/revolute_public_api/src/lib.rs
      crates/picea/tests/fixtures/revolute_public_api/src/bin/exhaustive.rs
      crates/picea/tests/world_step_review_regressions.rs
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: []
  },
  "S5-API" => {
    required: %w[
      crates/picea/src/joint.rs
      crates/picea/src/lib.rs
      crates/picea/src/world/api.rs
      crates/picea/src/recipe.rs
      crates/picea/src/debug.rs
      crates/picea/src/pipeline/island.rs
      crates/picea/tests/core_model_world.rs
      crates/picea/tests/world_step_review_regressions.rs
      crates/picea-lab/src/scenario/fixture.rs
      crates/picea-lab/src/scenario/fixture/tests.rs
      crates/picea-lab/src/scenario/scene_lattice.rs
      crates/picea-lab/web/src/types.ts
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
      crates/picea-lab/src/scenario/mod.rs
      crates/picea-lab/src/lib.rs
      crates/picea/tests/verify_revolute_scope.rb
    ],
    # Compiler-discovered consumers must be approved in the living spec first.
    optional: []
  },
  "S5-BEHAVIOR-RED" => {
    required: %w[
      crates/picea/src/pipeline/joints.rs
      crates/picea/src/pipeline/joints/tests.rs
      crates/picea/tests/physics_realism_acceptance.rs
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: []
  },
  "S5-SOLVER" => {
    required: %w[
      crates/picea/src/pipeline.rs
      crates/picea/src/pipeline/island.rs
      crates/picea/src/pipeline/joints.rs
      crates/picea/src/solver/body_state.rs
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: %w[crates/picea/src/pipeline/joints/tests.rs]
  },
  "S5-REPLAN" => {
    required: %w[
      docs/design/2026-07-14-revolute-joint-v1-design.md
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: []
  },
  "S5-BEHAVIOR-RED-2" => {
    required: %w[
      crates/picea/tests/physics_realism_acceptance.rs
      crates/picea/tests/world_step_review_regressions.rs
      crates/picea/tests/verify_revolute_scope.rb
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: []
  },
  "S5-SOLVER-2" => {
    required: %w[
      crates/picea/src/pipeline.rs
      crates/picea/src/pipeline/step.rs
      crates/picea/src/pipeline/island.rs
      crates/picea/src/pipeline/joints.rs
      crates/picea/src/solver/body_state.rs
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: %w[crates/picea/src/pipeline/joints/tests.rs]
  },
  "S5-BEHAVIOR-RED-3" => {
    required: %w[
      crates/picea/tests/physics_realism_acceptance.rs
      crates/picea/tests/verify_revolute_scope.rb
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: []
  },
  "S5-SOLVER-3" => {
    required: %w[
      crates/picea/src/pipeline.rs
      crates/picea/src/pipeline/step.rs
      crates/picea/src/pipeline/island.rs
      crates/picea/src/pipeline/joints.rs
      crates/picea/src/solver/body_state.rs
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: %w[crates/picea/src/pipeline/joints/tests.rs]
  },
  "S5-REPLAN-2" => {
    required: %w[
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: []
  },
  "S5-BEHAVIOR-RED-4" => {
    required: %w[
      crates/picea/tests/physics_realism_acceptance.rs
      crates/picea/tests/verify_revolute_scope.rb
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: []
  },
  "S5-SOLVER-4" => {
    required: %w[
      crates/picea/src/pipeline.rs
      crates/picea/src/pipeline/step.rs
      crates/picea/src/pipeline/island.rs
      crates/picea/src/pipeline/joints.rs
      crates/picea/src/solver/body_state.rs
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: %w[crates/picea/src/pipeline/joints/tests.rs]
  },
  "S5-REPLAN-3" => {
    required: %w[
      docs/design/2026-07-14-revolute-joint-v1-design.md
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
      openspec/config.yaml
      openspec/changes/complete-s5-revolute-solver-5/.openspec.yaml
      openspec/changes/complete-s5-revolute-solver-5/proposal.md
      openspec/changes/complete-s5-revolute-solver-5/design.md
      openspec/changes/complete-s5-revolute-solver-5/specs/revolute-joint-solver/spec.md
      openspec/changes/complete-s5-revolute-solver-5/tasks.md
    ],
    optional: []
  },
  "S5-BEHAVIOR-RED-5" => {
    required: %w[
      crates/picea/tests/physics_realism_acceptance.rs
      crates/picea/tests/world_step_review_regressions.rs
      crates/picea/tests/verify_revolute_scope.rb
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
      openspec/changes/complete-s5-revolute-solver-5/tasks.md
    ],
    optional: []
  },
  "S5-SOLVER-5" => {
    required: %w[
      crates/picea/src/pipeline.rs
      crates/picea/src/pipeline/step.rs
      crates/picea/src/pipeline/island.rs
      crates/picea/src/pipeline/joints.rs
      crates/picea/src/solver/body_state.rs
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
      openspec/changes/complete-s5-revolute-solver-5/tasks.md
    ],
    optional: %w[crates/picea/src/pipeline/joints/tests.rs]
  },
  "S5-LAB-RED" => {
    required: %w[
      crates/picea-lab/tests/artifact_run.rs
      crates/picea-lab/tests/server_routes.rs
      crates/picea-lab/web/scripts/ui-contract.mjs
      crates/picea-lab/web/scripts/i18n-contract.mjs
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: %w[crates/picea-lab/src/scenario/fixture/tests.rs]
  },
  "S5-LAB" => {
    required: %w[
      crates/picea-lab/src/scenario/mod.rs
      crates/picea-lab/src/scenario/scene_dispatch.rs
      crates/picea-lab/src/scenario/scene_revolute.rs
      crates/picea-lab/web/src/i18n.ts
      crates/picea-lab/web/src/types.ts
      crates/picea-lab/web/src/components/workbench/SceneHierarchy.tsx
      crates/picea-lab/web/src/components/workbench/Inspector.tsx
      crates/picea-lab/web/src/components/workbench/Timeline.tsx
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
    ],
    optional: %w[
      crates/picea-lab/src/artifact.rs
      crates/picea-lab/src/server.rs
      crates/picea-lab/web/src/components/workbench/types.ts
    ]
  },
  "S5-C" => {
    required: %w[
      docs/design/2026-07-14-revolute-joint-v1-design.md
      docs/plans/2026-07-14-vnext-s5-revolute-joint-milestone.md
      docs/plans/2026-06-17-physics-realism-vnext-milestones.md
      docs/handoff-2026-07-14-vnext-s5-revolute-joint.md
      docs/ai/index.md
      docs/ai/repo-map.md
      docs/ai/doc-catalog.yaml
      docs/design/README.md
      docs/design/solver-island-ordering-contract.md
    ],
    optional: []
  }
}.freeze

PRE_V_NODES = %w[
  S5-API-RED
  S5-API
  S5-BEHAVIOR-RED
  S5-REPLAN
  S5-BEHAVIOR-RED-2
  S5-BEHAVIOR-RED-3
  S5-REPLAN-2
  S5-BEHAVIOR-RED-4
  S5-REPLAN-3
  S5-BEHAVIOR-RED-5
  S5-SOLVER-5
  S5-LAB-RED
  S5-LAB
].freeze

class ContractError < StandardError; end

def git(*args, allow_failure: false)
  stdout, stderr, status = Open3.capture3("rtk", "proxy", "git", *args, chdir: ROOT)
  unless status.success? || allow_failure
    raise ContractError, "git #{args.join(' ')} failed: #{stderr.strip}"
  end
  [stdout, stderr, status]
end

def validate_sha!(sha, label)
  raw = if sha.nil?
          "<nil>"
        elsif sha.empty?
          "<empty>"
        else
          sha
        end
  if sha.nil? || sha.empty? || sha == "PENDING"
    raise ContractError, "#{label}: empty or PENDING SHA raw=#{raw}"
  end
  unless sha.match?(/\A[0-9a-f]{40}\z/)
    raise ContractError, "#{label}: expected 40 lowercase hexadecimal characters raw=#{raw}"
  end

  _stdout, _stderr, status = git("cat-file", "-e", "#{sha}^{commit}", allow_failure: true)
  raise ContractError, "#{label}: SHA does not resolve to a commit" unless status.success?

  sha
end

def progress_rows(text)
  lines = text.lines
  headings = lines.each_index.select { |index| lines[index].strip == PROGRESS_HEADING }
  unless headings.length == 1
    raise ContractError, "expected exactly one #{PROGRESS_HEADING.inspect} section"
  end

  section_start = headings.first + 1
  section_end = (section_start...lines.length).find { |index| lines[index].match?(/\A##\s+/) } || lines.length
  section = lines[section_start...section_end]
  headers = section.each_index.select { |index| section[index].strip == PROGRESS_HEADER }
  raise ContractError, "expected exactly one strict progress table header" unless headers.length == 1

  header = headers.first
  separator = section[header + 1]&.strip
  unless separator&.match?(/\A\|(?:\s*---\s*\|){6}\z/)
    raise ContractError, "invalid progress table separator"
  end

  data_lines = section[(header + 2)..-1].to_a.take_while { |line| line.lstrip.start_with?("|") }
  raise ContractError, "progress table has no data rows" if data_lines.empty?

  data_lines.map do |line|
    match = PROGRESS_ROW.match(line.chomp)
    raise ContractError, "malformed progress table data row" unless match
    raise ContractError, "unknown progress node #{match[:node]}" unless PROGRESS_NODES.include?(match[:node])

    start_cell = match[:start].strip
    raw = if (quoted = /\A`([^`]*)`\z/.match(start_cell))
            quoted[1]
          else
            start_cell
          end
    [match[:node], raw]
  end
end

def receipt_raw_from_text(text, node)
  matches = progress_rows(text).select { |candidate, _raw| candidate == node }
  unless matches.length == 1
    raise ContractError, "expected exactly one progress receipt row for #{node}, found #{matches.length}"
  end

  matches.first[1]
end

def receipt_sha(node)
  path = File.join(ROOT, SPEC)
  raw = receipt_raw_from_text(File.read(path), node)
  validate_sha!(raw, "#{node} receipt")
end

def validate_cli_sha_against_receipt!(node, cli_sha, recorded)
  validate_sha!(recorded, "#{node} receipt")
  validate_sha!(cli_sha, "#{node} CLI")
  raise ContractError, "#{node}: CLI SHA does not match receipt" unless cli_sha == recorded

  recorded
end

def validate_cli_sha!(node, cli_sha)
  validate_cli_sha_against_receipt!(node, cli_sha, receipt_sha(node))
end

def changed_paths(base, cached: false)
  tracked_args = cached ? %w[diff --cached --name-only] : ["diff", "--name-only", base]
  tracked = git(*tracked_args).first.lines.map(&:strip).reject(&:empty?)
  return tracked.uniq.sort if cached

  untracked = git("ls-files", "--others", "--exclude-standard").first.lines.map(&:strip).reject(&:empty?)
  (tracked + untracked).uniq.sort
end

def verify_paths!(label, actual, required, optional)
  actual = actual.sort
  required = required.sort
  optional = optional.sort
  missing = required - actual
  unexpected = actual - required - optional

  puts "S5_SCOPE_NODE=#{label}"
  puts "S5_SCOPE_ACTUAL=#{actual.join(',')}"
  puts "S5_SCOPE_MISSING=#{missing.join(',')}"
  puts "S5_SCOPE_UNEXPECTED=#{unexpected.join(',')}"
  unless missing.empty? && unexpected.empty?
    raise ContractError, "#{label}: binary scope mismatch"
  end

  puts "S5_SCOPE_PASS"
end

def scope_for(node)
  SCOPES.fetch(node) { raise ContractError, "unknown node #{node}" }
end

def expect_contract_error(label)
  failed_as_expected = false
  begin
    yield
  rescue ContractError
    failed_as_expected = true
  end
  raise ContractError, "self-test #{label}: expected failure" unless failed_as_expected

  puts "S5_SCOPE_SELF_TEST=#{label}:PASS"
end

def self_test!
  valid = receipt_sha("S5-API-RED")

  parser_fixture = <<~MARKDOWN
    正文中的伪造内容不得成为receipt：S5-BEHAVIOR-RED
    ```text
    | fake | S5-API | `#{valid}` | fake | fake | fake |
    ```

    #{PROGRESS_HEADING}

    #{PROGRESS_HEADER}
    | --- | --- | --- | --- | --- | --- |
    | 2026-07-14 | S5-API-RED | `#{valid}` | current | PENDING | real |
    | - | S5-API | `PENDING` | future | - | real future row |

    ## 17. next
  MARKDOWN
  parsed_current = receipt_raw_from_text(parser_fixture, "S5-API-RED")
  parsed_future = receipt_raw_from_text(parser_fixture, "S5-API")
  unless parsed_current == valid && parsed_future == "PENDING"
    raise ContractError, "self-test parser body/code shadowed the strict progress table"
  end
  puts "S5_SCOPE_SELF_TEST=parser-section-isolation:PASS"
  expect_contract_error("parser-future-pending") { validate_sha!(parsed_future, "test parser") }

  replan_start = "385c4c350ceee98947c65da5e3f63384804c69e1"
  red_2_start = "d987d1f7a85964f8c26121bc70b2a96b7f406f6e"
  red_3_start = "07b2bd6cd3ff4b80d1b6c124a4752e36e8e8d88f"
  replan_2_start = "6ffa1e3f29e902b68708b62e11d5fae160db97ec"
  red_4_start = "4e8bd4616e1f43e330d75212a74c459e8e693ca7"
  replan_3_start = "29bafce59a0913be7b4d37055ab57ed5e5233116"
  red_5_start = "195215119a3544e71849f21cb7bdaf245188de13"
  replan_fixture = <<~MARKDOWN
    #{PROGRESS_HEADING}

    #{PROGRESS_HEADER}
    | --- | --- | --- | --- | --- | --- |
    | 2026-07-15 | S5-REPLAN | `#{replan_start}` | committed | `#{red_2_start}` | current |
    | 2026-07-15 | S5-BEHAVIOR-RED-2 | `#{red_2_start}` | current | PENDING | current |
    | 2026-07-15 | S5-SOLVER-2 | `#{red_3_start}` | stopped | NOT COMMITTED | conflict |
    | 2026-07-15 | S5-BEHAVIOR-RED-3 | `#{red_3_start}` | current | PENDING | current |
    | 2026-07-15 | S5-SOLVER-3 | `#{replan_2_start}` | stopped | NOT COMMITTED | conflict |
    | 2026-07-15 | S5-REPLAN-2 | `#{replan_2_start}` | committed | `#{red_4_start}` | current |
    | 2026-07-15 | S5-BEHAVIOR-RED-4 | `#{red_4_start}` | current | PENDING | current |
    | 2026-07-15 | S5-SOLVER-4 | `#{replan_3_start}` | stopped | NOT COMMITTED | conflict |
    | 2026-07-15 | S5-REPLAN-3 | `#{replan_3_start}` | committed | `#{red_5_start}` | current |
    | 2026-07-15 | S5-BEHAVIOR-RED-5 | `#{red_5_start}` | current | PENDING | current |
    | - | S5-SOLVER-5 | `PENDING` | future | - | future |

    ## 17. next
  MARKDOWN
  unless receipt_raw_from_text(replan_fixture, "S5-REPLAN") == replan_start &&
         receipt_raw_from_text(replan_fixture, "S5-BEHAVIOR-RED-2") == red_2_start &&
         receipt_raw_from_text(replan_fixture, "S5-SOLVER-2") == red_3_start &&
         receipt_raw_from_text(replan_fixture, "S5-BEHAVIOR-RED-3") == red_3_start &&
         receipt_raw_from_text(replan_fixture, "S5-SOLVER-3") == replan_2_start &&
         receipt_raw_from_text(replan_fixture, "S5-REPLAN-2") == replan_2_start &&
         receipt_raw_from_text(replan_fixture, "S5-BEHAVIOR-RED-4") == red_4_start &&
         receipt_raw_from_text(replan_fixture, "S5-SOLVER-4") == replan_3_start &&
         receipt_raw_from_text(replan_fixture, "S5-REPLAN-3") == replan_3_start &&
         receipt_raw_from_text(replan_fixture, "S5-BEHAVIOR-RED-5") == red_5_start &&
         receipt_raw_from_text(replan_fixture, "S5-SOLVER-5") == "PENDING"
    raise ContractError, "self-test new-node parser did not preserve all receipt values"
  end
  puts "S5_SCOPE_SELF_TEST=parser-new-nodes:PASS"
  solver_2_fixture_raw = receipt_raw_from_text(replan_fixture, "S5-SOLVER-2")
  validate_cli_sha_against_receipt!("S5-SOLVER-2", solver_2_fixture_raw, solver_2_fixture_raw)
  puts "S5_SCOPE_SELF_TEST=s5-solver-2-sha-fixture-positive:PASS"
  expect_contract_error("s5-solver-2-sha-fixture-mismatch") do
    validate_cli_sha_against_receipt!("S5-SOLVER-2", replan_start, solver_2_fixture_raw)
  end
  solver_3_fixture_raw = receipt_raw_from_text(replan_fixture, "S5-SOLVER-3")
  validate_cli_sha_against_receipt!("S5-SOLVER-3", solver_3_fixture_raw, solver_3_fixture_raw)
  puts "S5_SCOPE_SELF_TEST=s5-solver-3-sha-fixture-positive:PASS"
  expect_contract_error("s5-solver-3-sha-fixture-mismatch") do
    validate_cli_sha_against_receipt!("S5-SOLVER-3", replan_start, solver_3_fixture_raw)
  end
  red_4_pending_fixture = replan_fixture.sub(
    "| 2026-07-15 | S5-BEHAVIOR-RED-4 | `#{red_4_start}` | current | PENDING | current |",
    "| - | S5-BEHAVIOR-RED-4 | `PENDING` | future | - | future |"
  )
  expect_contract_error("s5-behavior-red-4-pending-fixture") do
    validate_sha!(receipt_raw_from_text(red_4_pending_fixture, "S5-BEHAVIOR-RED-4"), "test RED-4")
  end
  solver_4_fixture_raw = receipt_raw_from_text(replan_fixture, "S5-SOLVER-4")
  validate_cli_sha_against_receipt!("S5-SOLVER-4", solver_4_fixture_raw, solver_4_fixture_raw)
  puts "S5_SCOPE_SELF_TEST=s5-solver-4-sha-fixture-positive:PASS"
  expect_contract_error("s5-solver-4-sha-fixture-mismatch") do
    validate_cli_sha_against_receipt!("S5-SOLVER-4", replan_start, solver_4_fixture_raw)
  end
  red_5_pending_fixture = replan_fixture.sub(
    "| 2026-07-15 | S5-BEHAVIOR-RED-5 | `#{red_5_start}` | current | PENDING | current |",
    "| - | S5-BEHAVIOR-RED-5 | `PENDING` | future | - | future |"
  )
  expect_contract_error("s5-behavior-red-5-pending-fixture") do
    validate_sha!(
      receipt_raw_from_text(red_5_pending_fixture, "S5-BEHAVIOR-RED-5"),
      "test RED-5"
    )
  end
  solver_5_sha_fixture = replan_fixture.sub(
    "| - | S5-SOLVER-5 | `PENDING` | future | - | future |",
    "| 2026-07-15 | S5-SOLVER-5 | `#{red_5_start}` | current | PENDING | current |"
  )
  solver_5_fixture_raw = receipt_raw_from_text(solver_5_sha_fixture, "S5-SOLVER-5")
  validate_cli_sha_against_receipt!("S5-SOLVER-5", solver_5_fixture_raw, solver_5_fixture_raw)
  puts "S5_SCOPE_SELF_TEST=s5-solver-5-sha-fixture-positive:PASS"
  expect_contract_error("s5-solver-5-sha-fixture-mismatch") do
    validate_cli_sha_against_receipt!("S5-SOLVER-5", replan_start, solver_5_fixture_raw)
  end
  duplicate_fixture = parser_fixture.sub(
    "| - | S5-API | `PENDING` | future | - | real future row |",
    "| - | S5-API | `PENDING` | future | - | real future row |\n" \
    "| - | S5-API | `#{valid}` | duplicate | - | must reject |"
  )
  expect_contract_error("parser-duplicate-row") do
    receipt_raw_from_text(duplicate_fixture, "S5-API")
  end

  expect_contract_error("empty") { validate_sha!("", "test") }
  expect_contract_error("pending") { validate_sha!("PENDING", "test") }
  expect_contract_error("short") { validate_sha!("abc123", "test") }
  expect_contract_error("nonhex") { validate_sha!("g" * 40, "test") }
  expect_contract_error("unresolvable") { validate_sha!("0" * 40, "test") }

  tree = git("rev-parse", "#{valid}^{tree}").first.strip
  expect_contract_error("noncommit") { validate_sha!(tree, "test") }
  expect_contract_error("mismatch") { validate_cli_sha!("S5-API-RED", MILESTONE_BASE) }
  validate_cli_sha!("S5-API", "6312a5ddd655a7c5ff2ba77a1f86cb3931ed9d68")
  puts "S5_SCOPE_SELF_TEST=current-s5-api-receipt:PASS"

  validate_cli_sha!("S5-REPLAN", replan_start)
  puts "S5_SCOPE_SELF_TEST=current-s5-replan-receipt:PASS"
  validate_cli_sha!("S5-BEHAVIOR-RED-2", red_2_start)
  puts "S5_SCOPE_SELF_TEST=current-s5-behavior-red-2-receipt:PASS"
  solver_2_raw = receipt_raw_from_text(File.read(File.join(ROOT, SPEC)), "S5-SOLVER-2")
  # The strict row has two legitimate lifecycle states: pending before supervisor handoff,
  # then one immutable Start HEAD that must obey the same CLI/receipt contract as prior nodes.
  if solver_2_raw == "PENDING"
    expect_contract_error("current-s5-solver-2-pending") { receipt_sha("S5-SOLVER-2") }
    puts "S5_SCOPE_SELF_TEST=current-s5-solver-2-pending-branch:PASS"
  else
    validate_cli_sha!("S5-SOLVER-2", solver_2_raw)
    mismatch_sha = [valid, replan_start, red_2_start].find { |candidate| candidate != solver_2_raw }
    raise ContractError, "self-test S5-SOLVER-2: no distinct mismatch SHA" unless mismatch_sha

    expect_contract_error("current-s5-solver-2-sha-mismatch") do
      validate_cli_sha!("S5-SOLVER-2", mismatch_sha)
    end
    puts "S5_SCOPE_SELF_TEST=current-s5-solver-2-sha-branch:PASS"
  end
  validate_cli_sha!("S5-BEHAVIOR-RED-3", red_3_start)
  puts "S5_SCOPE_SELF_TEST=current-s5-behavior-red-3-receipt:PASS"
  solver_3_raw = receipt_raw_from_text(File.read(File.join(ROOT, SPEC)), "S5-SOLVER-3")
  # S5-SOLVER-3 is state-aware from its first commit: PENDING is valid before handoff, and a
  # future immutable Start HEAD must use the same strict CLI/receipt equality as current nodes.
  if solver_3_raw == "PENDING"
    expect_contract_error("current-s5-solver-3-pending") { receipt_sha("S5-SOLVER-3") }
    puts "S5_SCOPE_SELF_TEST=current-s5-solver-3-pending-branch:PASS"
  else
    validate_cli_sha!("S5-SOLVER-3", solver_3_raw)
    mismatch_sha = [valid, replan_start, red_2_start, red_3_start].find do |candidate|
      candidate != solver_3_raw
    end
    raise ContractError, "self-test S5-SOLVER-3: no distinct mismatch SHA" unless mismatch_sha

    expect_contract_error("current-s5-solver-3-sha-mismatch") do
      validate_cli_sha!("S5-SOLVER-3", mismatch_sha)
    end
    puts "S5_SCOPE_SELF_TEST=current-s5-solver-3-sha-branch:PASS"
  end
  validate_cli_sha!("S5-REPLAN-2", replan_2_start)
  puts "S5_SCOPE_SELF_TEST=current-s5-replan-2-receipt:PASS"
  red_4_raw = receipt_raw_from_text(File.read(File.join(ROOT, SPEC)), "S5-BEHAVIOR-RED-4")
  if red_4_raw == "PENDING"
    expect_contract_error("current-s5-behavior-red-4-pending") do
      receipt_sha("S5-BEHAVIOR-RED-4")
    end
    puts "S5_SCOPE_SELF_TEST=current-s5-behavior-red-4-pending-branch:PASS"
  else
    validate_cli_sha!("S5-BEHAVIOR-RED-4", red_4_raw)
    mismatch_sha = [valid, replan_start, red_2_start, red_3_start, replan_2_start].find do |candidate|
      candidate != red_4_raw
    end
    raise ContractError, "self-test S5-BEHAVIOR-RED-4: no distinct mismatch SHA" unless mismatch_sha

    expect_contract_error("current-s5-behavior-red-4-sha-mismatch") do
      validate_cli_sha!("S5-BEHAVIOR-RED-4", mismatch_sha)
    end
    puts "S5_SCOPE_SELF_TEST=current-s5-behavior-red-4-sha-branch:PASS"
  end
  solver_4_raw = receipt_raw_from_text(File.read(File.join(ROOT, SPEC)), "S5-SOLVER-4")
  if solver_4_raw == "PENDING"
    expect_contract_error("current-s5-solver-4-pending") { receipt_sha("S5-SOLVER-4") }
    puts "S5_SCOPE_SELF_TEST=current-s5-solver-4-pending-branch:PASS"
  else
    validate_cli_sha!("S5-SOLVER-4", solver_4_raw)
    mismatch_sha = [valid, replan_start, red_2_start, red_3_start, replan_2_start, red_4_start].find do |candidate|
      candidate != solver_4_raw
    end
    raise ContractError, "self-test S5-SOLVER-4: no distinct mismatch SHA" unless mismatch_sha

    expect_contract_error("current-s5-solver-4-sha-mismatch") do
      validate_cli_sha!("S5-SOLVER-4", mismatch_sha)
    end
    puts "S5_SCOPE_SELF_TEST=current-s5-solver-4-sha-branch:PASS"
  end
  validate_cli_sha!("S5-REPLAN-3", replan_3_start)
  puts "S5_SCOPE_SELF_TEST=current-s5-replan-3-receipt:PASS"
  red_5_raw = receipt_raw_from_text(File.read(File.join(ROOT, SPEC)), "S5-BEHAVIOR-RED-5")
  if red_5_raw == "PENDING"
    expect_contract_error("current-s5-behavior-red-5-pending") do
      receipt_sha("S5-BEHAVIOR-RED-5")
    end
    puts "S5_SCOPE_SELF_TEST=current-s5-behavior-red-5-pending-branch:PASS"
  else
    validate_cli_sha!("S5-BEHAVIOR-RED-5", red_5_raw)
    mismatch_sha = [valid, replan_start, red_2_start, red_3_start, replan_2_start,
                    red_4_start, replan_3_start].find { |candidate| candidate != red_5_raw }
    raise ContractError, "self-test S5-BEHAVIOR-RED-5: no distinct mismatch SHA" unless mismatch_sha

    expect_contract_error("current-s5-behavior-red-5-sha-mismatch") do
      validate_cli_sha!("S5-BEHAVIOR-RED-5", mismatch_sha)
    end
    puts "S5_SCOPE_SELF_TEST=current-s5-behavior-red-5-sha-branch:PASS"
  end
  solver_5_raw = receipt_raw_from_text(File.read(File.join(ROOT, SPEC)), "S5-SOLVER-5")
  if solver_5_raw == "PENDING"
    expect_contract_error("current-s5-solver-5-pending") { receipt_sha("S5-SOLVER-5") }
    puts "S5_SCOPE_SELF_TEST=current-s5-solver-5-pending-branch:PASS"
  else
    validate_cli_sha!("S5-SOLVER-5", solver_5_raw)
    mismatch_sha = [valid, replan_start, red_2_start, red_3_start, replan_2_start,
                    red_4_start, replan_3_start, red_5_start].find do |candidate|
      candidate != solver_5_raw
    end
    raise ContractError, "self-test S5-SOLVER-5: no distinct mismatch SHA" unless mismatch_sha

    expect_contract_error("current-s5-solver-5-sha-mismatch") do
      validate_cli_sha!("S5-SOLVER-5", mismatch_sha)
    end
    puts "S5_SCOPE_SELF_TEST=current-s5-solver-5-sha-branch:PASS"
  end
  progress_text = File.read(File.join(ROOT, SPEC))
  PROGRESS_NODES.each { |node| receipt_raw_from_text(progress_text, node) }
  puts "S5_SCOPE_SELF_TEST=all-progress-nodes-have-one-row:PASS"
  expect_contract_error("current-s5-behavior-red-2-mismatch") do
    validate_cli_sha!("S5-BEHAVIOR-RED-2", replan_start)
  end
  expect_contract_error("current-s5-behavior-red-3-mismatch") do
    validate_cli_sha!("S5-BEHAVIOR-RED-3", replan_start)
  end
  expect_contract_error("current-s5-replan-2-mismatch") do
    validate_cli_sha!("S5-REPLAN-2", replan_start)
  end
  expect_contract_error("current-s5-replan-3-mismatch") do
    validate_cli_sha!("S5-REPLAN-3", replan_start)
  end

  replan_scope = scope_for("S5-REPLAN")
  verify_paths!(
    "self-test:S5-REPLAN-positive",
    replan_scope[:required],
    replan_scope[:required],
    replan_scope[:optional]
  )
  red_2_scope = scope_for("S5-BEHAVIOR-RED-2")
  verify_paths!(
    "self-test:S5-BEHAVIOR-RED-2-positive",
    red_2_scope[:required],
    red_2_scope[:required],
    red_2_scope[:optional]
  )
  expect_contract_error("s5-behavior-red-2-missing") do
    verify_paths!(
      "self-test:S5-BEHAVIOR-RED-2-missing",
      red_2_scope[:required].drop(1),
      red_2_scope[:required],
      red_2_scope[:optional]
    )
  end
  expect_contract_error("s5-behavior-red-2-unexpected") do
    verify_paths!(
      "self-test:S5-BEHAVIOR-RED-2-unexpected",
      red_2_scope[:required] + ["unexpected/path"],
      red_2_scope[:required],
      red_2_scope[:optional]
    )
  end
  solver_2_scope = scope_for("S5-SOLVER-2")
  verify_paths!(
    "self-test:S5-SOLVER-2-optional",
    solver_2_scope[:required] + solver_2_scope[:optional],
    solver_2_scope[:required],
    solver_2_scope[:optional]
  )
  red_3_scope = scope_for("S5-BEHAVIOR-RED-3")
  verify_paths!(
    "self-test:S5-BEHAVIOR-RED-3-positive",
    red_3_scope[:required],
    red_3_scope[:required],
    red_3_scope[:optional]
  )
  expect_contract_error("s5-behavior-red-3-missing") do
    verify_paths!(
      "self-test:S5-BEHAVIOR-RED-3-missing",
      red_3_scope[:required].drop(1),
      red_3_scope[:required],
      red_3_scope[:optional]
    )
  end
  expect_contract_error("s5-behavior-red-3-unexpected") do
    verify_paths!(
      "self-test:S5-BEHAVIOR-RED-3-unexpected",
      red_3_scope[:required] + ["unexpected/path"],
      red_3_scope[:required],
      red_3_scope[:optional]
    )
  end
  solver_3_scope = scope_for("S5-SOLVER-3")
  verify_paths!(
    "self-test:S5-SOLVER-3-optional",
    solver_3_scope[:required] + solver_3_scope[:optional],
    solver_3_scope[:required],
    solver_3_scope[:optional]
  )
  replan_2_scope = scope_for("S5-REPLAN-2")
  verify_paths!(
    "self-test:S5-REPLAN-2-positive",
    replan_2_scope[:required],
    replan_2_scope[:required],
    replan_2_scope[:optional]
  )
  red_4_scope = scope_for("S5-BEHAVIOR-RED-4")
  verify_paths!(
    "self-test:S5-BEHAVIOR-RED-4-positive",
    red_4_scope[:required],
    red_4_scope[:required],
    red_4_scope[:optional]
  )
  expect_contract_error("s5-behavior-red-4-missing") do
    verify_paths!(
      "self-test:S5-BEHAVIOR-RED-4-missing",
      red_4_scope[:required].drop(1),
      red_4_scope[:required],
      red_4_scope[:optional]
    )
  end
  expect_contract_error("s5-behavior-red-4-unexpected") do
    verify_paths!(
      "self-test:S5-BEHAVIOR-RED-4-unexpected",
      red_4_scope[:required] + ["unexpected/path"],
      red_4_scope[:required],
      red_4_scope[:optional]
    )
  end
  solver_4_scope = scope_for("S5-SOLVER-4")
  verify_paths!(
    "self-test:S5-SOLVER-4-optional",
    solver_4_scope[:required] + solver_4_scope[:optional],
    solver_4_scope[:required],
    solver_4_scope[:optional]
  )
  replan_3_scope = scope_for("S5-REPLAN-3")
  verify_paths!(
    "self-test:S5-REPLAN-3-positive",
    replan_3_scope[:required],
    replan_3_scope[:required],
    replan_3_scope[:optional]
  )
  red_5_scope = scope_for("S5-BEHAVIOR-RED-5")
  verify_paths!(
    "self-test:S5-BEHAVIOR-RED-5-positive",
    red_5_scope[:required],
    red_5_scope[:required],
    red_5_scope[:optional]
  )
  expect_contract_error("s5-behavior-red-5-missing") do
    verify_paths!(
      "self-test:S5-BEHAVIOR-RED-5-missing",
      red_5_scope[:required].drop(1),
      red_5_scope[:required],
      red_5_scope[:optional]
    )
  end
  expect_contract_error("s5-behavior-red-5-unexpected") do
    verify_paths!(
      "self-test:S5-BEHAVIOR-RED-5-unexpected",
      red_5_scope[:required] + ["unexpected/path"],
      red_5_scope[:required],
      red_5_scope[:optional]
    )
  end
  solver_5_scope = scope_for("S5-SOLVER-5")
  verify_paths!(
    "self-test:S5-SOLVER-5-optional",
    solver_5_scope[:required] + solver_5_scope[:optional],
    solver_5_scope[:required],
    solver_5_scope[:optional]
  )

  approved_pre_v = %w[
    S5-API-RED
    S5-API
    S5-BEHAVIOR-RED
    S5-REPLAN
    S5-BEHAVIOR-RED-2
    S5-BEHAVIOR-RED-3
    S5-REPLAN-2
    S5-BEHAVIOR-RED-4
    S5-REPLAN-3
    S5-BEHAVIOR-RED-5
    S5-SOLVER-5
    S5-LAB-RED
    S5-LAB
  ]
  raise ContractError, "self-test PRE_V_NODES drift" unless PRE_V_NODES == approved_pre_v

  stopped_solvers = %w[S5-SOLVER S5-SOLVER-2 S5-SOLVER-3 S5-SOLVER-4]
  unless (PRE_V_NODES & stopped_solvers).empty?
    raise ContractError, "self-test stopped solver entered milestone union"
  end
  puts "S5_SCOPE_SELF_TEST=pre-v-excludes-four-stopped-solvers:PASS"

  validate_cli_sha!("S5-API-RED", valid)
  puts "S5_SCOPE_SELF_TEST=valid:PASS"
  puts "S5_SCOPE_SELF_TEST_PASS"
end

def main(argv)
  mode = argv.shift
  raise ContractError, "usage: receipt-head NODE | NODE SHA | cached NODE SHA | milestone SHA | self-test" unless mode

  case mode
  when "receipt-head"
    node = argv.shift
    raise ContractError, "receipt-head requires NODE" unless node && argv.empty?
    puts receipt_sha(node)
  when "cached"
    node, cli_sha = argv
    raise ContractError, "cached requires NODE SHA" unless node && cli_sha && argv.length == 2
    start = validate_cli_sha!(node, cli_sha)
    scope = scope_for(node)
    verify_paths!("cached:#{node}", changed_paths(start, cached: true), scope[:required], scope[:optional])
  when "milestone"
    base = argv.shift
    raise ContractError, "milestone requires the fixed base SHA" unless base && argv.empty?
    validate_sha!(base, "milestone base")
    raise ContractError, "milestone base mismatch" unless base == MILESTONE_BASE

    required = S5_D_REQUIRED + PRE_V_NODES.flat_map { |node| SCOPES.fetch(node)[:required] }
    optional = PRE_V_NODES.flat_map { |node| SCOPES.fetch(node)[:optional] }
    verify_paths!("milestone", changed_paths(base), required.uniq, optional.uniq)
  when "self-test"
    raise ContractError, "self-test accepts no arguments" unless argv.empty?
    self_test!
  else
    node = mode
    cli_sha = argv.shift
    raise ContractError, "node mode requires NODE SHA" unless cli_sha && argv.empty?
    start = validate_cli_sha!(node, cli_sha)
    scope = scope_for(node)
    verify_paths!(node, changed_paths(start), scope[:required], scope[:optional])
  end
end

begin
  main(ARGV)
rescue ContractError, KeyError => error
  warn "S5_SCOPE_FAILURE: #{error.message}"
  exit 1
rescue StandardError => error
  warn "S5_SCOPE_FAILURE: unexpected #{error.class}: #{error.message}"
  exit 1
end
