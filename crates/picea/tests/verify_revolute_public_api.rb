#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "json"
require "open3"

ROOT = File.expand_path("../../..", __dir__)
FIXTURE = File.join(__dir__, "fixtures/revolute_public_api")
APPROVED_TEMP = "/var/folders/20/mtxygnnn3w7f0wd4t4dfgwq80000gn/T/opencode/picea-s5-revolute-api"
ALLOWED_RED_CODES = %w[E0412 E0422 E0432 E0433 E0599].freeze
ENUM_FEATURES = {
  "JointKind" => "exhaustive-joint-kind",
  "JointDesc" => "exhaustive-joint-desc",
  "JointPatch" => "exhaustive-joint-patch",
  "JointBundle" => "exhaustive-joint-bundle",
  "DebugJointKind" => "exhaustive-debug-joint-kind",
  "SceneJointFixture" => "exhaustive-scene-joint-fixture"
}.freeze

class HarnessFailure < StandardError; end

CargoRun = Struct.new(:stdout, :stderr, :status, :diagnostics, keyword_init: true)

def cargo(*args)
  stdout, stderr, status = Open3.capture3("rtk", "proxy", "cargo", *args, chdir: APPROVED_TEMP)
  diagnostics = []
  stdout.each_line do |line|
    begin
      payload = JSON.parse(line)
    rescue JSON::ParserError
      next
    end
    next unless payload["reason"] == "compiler-message"

    message = payload["message"]
    diagnostics << message if message && message["level"] == "error"
  end
  CargoRun.new(stdout: stdout, stderr: stderr, status: status, diagnostics: diagnostics)
end

def diagnostic_text(diagnostic)
  [diagnostic["message"], diagnostic["rendered"]].compact.join("\n")
end

def fixture_diagnostic?(diagnostic)
  diagnostic.fetch("spans", []).select { |span| span["is_primary"] }.any? do |span|
    file = span["file_name"]
    next false unless file

    expanded = File.expand_path(file, APPROVED_TEMP)
    expanded == APPROVED_TEMP || expanded.start_with?("#{APPROVED_TEMP}/")
  end
end

def validate_expected_red!(label, run, required_terms: [])
  raise HarnessFailure, "#{label}: compiler unexpectedly succeeded" if run.status.success?
  if run.diagnostics.empty?
    detail = run.stderr.lines.last(12).join.strip
    raise HarnessFailure, "#{label}: cargo failed without JSON compiler diagnostics: #{detail}"
  end

  run.diagnostics.each do |diagnostic|
    code = diagnostic.dig("code", "code")
    text = diagnostic_text(diagnostic)
    unless ALLOWED_RED_CODES.include?(code)
      raise HarnessFailure, "#{label}: unapproved compiler error #{code.inspect}: #{text.lines.first&.strip}"
    end
    unless diagnostic["message"].to_s.downcase.include?("revolute")
      raise HarnessFailure, "#{label}: error does not identify a missing Revolute symbol/variant"
    end
    unless fixture_diagnostic?(diagnostic)
      raise HarnessFailure, "#{label}: compiler error originated outside the isolated fixture"
    end
  end

  joined = run.diagnostics.map { |diagnostic| diagnostic_text(diagnostic) }.join("\n")
  required_terms.each do |term|
    raise HarnessFailure, "#{label}: missing required diagnostic term #{term}" unless joined.include?(term)
  end

  codes = run.diagnostics.map { |diagnostic| diagnostic.dig("code", "code") }.uniq.sort
  puts "S5_PUBLIC_API_RED_DIAGNOSTIC=#{label}:#{codes.join(',')}"
end

def validate_expected_exhaustive_failure!(label, run)
  raise HarnessFailure, "#{label}: external exhaustive match unexpectedly compiled" if run.status.success?
  raise HarnessFailure, "#{label}: missing JSON compiler diagnostic" if run.diagnostics.empty?

  run.diagnostics.each do |diagnostic|
    code = diagnostic.dig("code", "code")
    text = diagnostic_text(diagnostic)
    unless code == "E0004" && (text.include?("non-exhaustive") || text.include?("`_` not covered"))
      raise HarnessFailure, "#{label}: expected only E0004 non-exhaustive wildcard failure, got #{code.inspect}"
    end
    unless fixture_diagnostic?(diagnostic)
      raise HarnessFailure, "#{label}: exhaustive diagnostic originated outside the fixture"
    end
  end
  puts "S5_PUBLIC_API_EXHAUSTIVE_GREEN=#{label}:E0004"
end

def prepare_fixture!
  raise HarnessFailure, "fixture source is missing" unless File.directory?(FIXTURE)

  FileUtils.mkdir_p(File.dirname(APPROVED_TEMP))
  FileUtils.rm_rf(APPROVED_TEMP)
  FileUtils.cp_r(FIXTURE, APPROVED_TEMP)

  manifest = File.join(APPROVED_TEMP, "Cargo.toml")
  text = File.read(manifest)
  text = text.gsub("__PICEA_PATH__", File.join(ROOT, "crates/picea"))
  text = text.gsub("__PICEA_LAB_PATH__", File.join(ROOT, "crates/picea-lab"))
  if text.include?("__PICEA_PATH__") || text.include?("__PICEA_LAB_PATH__")
    raise HarnessFailure, "path placeholders were not fully replaced"
  end
  File.write(manifest, text)
end

def expect_red!
  positive = cargo("check", "--lib", "--message-format=json")
  validate_expected_red!(
    "positive-surface",
    positive,
    required_terms: %w[
      RevoluteJointDesc
      RevoluteJointPatch
      JointBundle
      DebugJointKind
      SceneRevoluteJointFixture
    ]
  )
  puts "S5_PUBLIC_API_INTERNAL_COMPILE_EXIT=#{positive.status.exitstatus}"

  ENUM_FEATURES.each do |label, feature|
    run = cargo(
      "check", "--bin", "exhaustive", "--no-default-features", "--features", feature,
      "--message-format=json"
    )
    validate_expected_red!("enum-#{label}", run, required_terms: ["Revolute"])
  end

  puts "S5_PUBLIC_API_RUNTIME=NOT_RUN"
  puts "S5_PUBLIC_API_EXPECTED_RED"
end

def expect_green!
  positive = cargo("test", "--lib")
  unless positive.status.success?
    detail = (positive.stdout + positive.stderr).lines.last(30).join
    raise HarnessFailure, "positive external tests failed: #{detail.strip}"
  end
  puts "S5_PUBLIC_API_POSITIVE_GREEN"

  ENUM_FEATURES.each do |label, feature|
    run = cargo(
      "check", "--bin", "exhaustive", "--no-default-features", "--features", feature,
      "--message-format=json"
    )
    validate_expected_exhaustive_failure!(label, run)
  end

  puts "S5_PUBLIC_API_EXPECTED_GREEN"
end

def main(argv)
  mode, temp = argv
  raise HarnessFailure, "usage: expect-red|expect-green APPROVED_TEMP" unless argv.length == 2
  unless temp == APPROVED_TEMP
    raise HarnessFailure, "second argument must exactly equal #{APPROVED_TEMP}; refusing cleanup"
  end
  raise HarnessFailure, "unknown mode #{mode.inspect}" unless %w[expect-red expect-green].include?(mode)

  prepare_fixture!
  mode == "expect-red" ? expect_red! : expect_green!
end

begin
  main(ARGV)
rescue HarnessFailure => error
  warn "S5_PUBLIC_API_HARNESS_FAILURE: #{error.message}"
  exit 1
rescue StandardError => error
  warn "S5_PUBLIC_API_HARNESS_FAILURE: unexpected #{error.class}: #{error.message}"
  exit 1
end
