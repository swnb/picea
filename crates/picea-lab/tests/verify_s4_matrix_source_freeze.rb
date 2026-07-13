require "digest"

TARGET = "crates/picea-lab/tests/artifact_run.rs"
FUNCTIONS = %w[
  matrix_stack_artifacts_capture_nxm_grid_stack_facts
  aligned_matrix_stack_artifacts_capture_stable_nxm_behavior_lock
  matrix_stack_long_settle_acceptance_requires_no_ejection_or_runaway_speed
].freeze
REPLACE_FUNCTION = "matrix_stack_artifacts_capture_nxm_grid_stack_facts"
OLD_ORACLE_BLOCK = <<'RUST'.b
    assert_eq!(
        report.feature_churn_trace.edge_swap_candidate_count, 0,
        "contact identity fix: edge-swap churn with compatible reduction, shape, normal, point, and local anchors must be absorbed by the persistent edge-swap path instead of remaining a miss; report={report:?}"
    );
RUST

def skip_quoted(source, index, quote)
  index += 1
  while index < source.bytesize
    byte = source.getbyte(index)
    if byte == 92
      index += 2
    elsif byte == quote
      return index + 1
    else
      index += 1
    end
  end
  abort "unterminated quoted literal"
end

def skip_block_comment(source, index)
  depth = 1
  index += 2
  while index < source.bytesize
    if source.byteslice(index, 2) == "/*"
      depth += 1
      index += 2
    elsif source.byteslice(index, 2) == "*/"
      depth -= 1
      index += 2
      return index if depth.zero?
    else
      index += 1
    end
  end
  abort "unterminated block comment"
end

def raw_string_finish(source, index)
  return nil unless source.getbyte(index) == 114

  cursor = index + 1
  cursor += 1 while source.getbyte(cursor) == 35
  return nil unless source.getbyte(cursor) == 34

  hashes = cursor - index - 1
  terminator = ('"' + ('#' * hashes)).b
  finish = source.index(terminator, cursor + 1)
  abort "unterminated raw string" unless finish
  finish + terminator.bytesize
end

def utf8_width(byte)
  return 1 if byte < 0x80
  return 2 if byte < 0xe0
  return 3 if byte < 0xf0

  4
end

def char_literal_finish(source, index)
  cursor = index + 1
  return nil if cursor >= source.bytesize

  if source.getbyte(cursor) == 92
    cursor += 2
  else
    cursor += utf8_width(source.getbyte(cursor))
  end
  source.getbyte(cursor) == 39 ? cursor + 1 : nil
end

def blank_non_newlines(mask, start_index, finish_index)
  (start_index...finish_index).each do |index|
    byte = mask.getbyte(index)
    mask.setbyte(index, 32) unless byte == 10 || byte == 13
  end
end

def code_mask(source)
  mask = source.dup
  index = 0
  while index < source.bytesize
    pair = source.byteslice(index, 2)
    if pair == "//"
      finish = source.index("\n".b, index + 2) || source.bytesize
      blank_non_newlines(mask, index, finish)
      index = finish
      next
    end
    if pair == "/*"
      finish = skip_block_comment(source, index)
      blank_non_newlines(mask, index, finish)
      index = finish
      next
    end
    raw_finish = raw_string_finish(source, index)
    if raw_finish
      blank_non_newlines(mask, index, raw_finish)
      index = raw_finish
      next
    end

    case source.getbyte(index)
    when 34
      finish = skip_quoted(source, index, 34)
      blank_non_newlines(mask, index, finish)
      index = finish
    when 39
      finish = char_literal_finish(source, index)
      if finish
        blank_non_newlines(mask, index, finish)
        index = finish
      else
        index += 1
      end
    else
      index += 1
    end
  end
  mask
end

def matching_brace(mask, opening)
  depth = 0
  index = opening
  while index < mask.bytesize
    case mask.getbyte(index)
    when 123
      depth += 1
    when 125
      depth -= 1
      return index if depth.zero?
      abort "unbalanced closing brace" if depth.negative?
    end
    index += 1
  end
  abort "unterminated function body"
end

def include_contiguous_attributes(source, function_line_start)
  item_start = function_line_start
  while item_start.positive?
    previous_line_end = item_start
    previous_newline = source.rindex("\n".b, item_start - 2)
    previous_line_start = previous_newline ? previous_newline + 1 : 0
    line = source.byteslice(previous_line_start, previous_line_end - previous_line_start)
    break unless line.match?(/\A[ \t]*#\[[^\r\n]*\][ \t]*(?:\r?\n)?\z/)

    item_start = previous_line_start
  end
  item_start
end

def extract_item(source, name)
  mask = code_mask(source)
  pattern = /^[ \t]*fn[ \t]+#{Regexp.escape(name)}[ \t]*\(/
  matches = mask.enum_for(:scan, pattern).map { Regexp.last_match }
  abort "expected one real function #{name}, found #{matches.length}" unless matches.length == 1

  function_line_start = matches.first.begin(0)
  opening = mask.index("{".b, matches.first.end(0))
  abort "missing body for #{name}" unless opening
  closing = matching_brace(mask, opening)
  item_start = include_contiguous_attributes(source, function_line_start)
  source.byteslice(item_start, closing - item_start + 1)
end

def git_blob(base)
  output = IO.popen(["rtk", "proxy", "git", "show", "#{base}:#{TARGET}"], "rb", &:read)
  abort "git show failed for #{base}:#{TARGET}" unless $?.success?
  output.b
end

base, mode, *rest = ARGV
abort "usage: ruby #{__FILE__} BASE MODE [APPROVED_REPLACEMENT_SHA256]" unless base && mode
abort "unknown mode: #{mode}" unless %w[exact replace-one-oracle].include?(mode)
if mode == "exact"
  abort "exact mode takes no replacement digest" unless rest.empty?
else
  abort "replace-one-oracle requires one approved replacement SHA-256" unless rest.length == 1
  abort "invalid replacement SHA-256: #{rest.first}" unless rest.first.match?(/\A[0-9a-f]{64}\z/)
end

baseline_source = git_blob(base)
current_source = File.binread(TARGET)
baseline = FUNCTIONS.to_h { |name| [name, extract_item(baseline_source, name)] }
current = FUNCTIONS.to_h { |name| [name, extract_item(current_source, name)] }

if mode == "exact"
  FUNCTIONS.each do |name|
    abort "frozen function item changed: #{name}" unless current.fetch(name) == baseline.fetch(name)
  end
  puts "s4 matrix source freeze exact ok"
  exit 0
end

changed = FUNCTIONS.select { |name| current.fetch(name) != baseline.fetch(name) }
abort "replace-one-oracle must change only #{REPLACE_FUNCTION}: #{changed.inspect}" unless changed == [REPLACE_FUNCTION]

base_item = baseline.fetch(REPLACE_FUNCTION)
current_item = current.fetch(REPLACE_FUNCTION)
old_start = base_item.index(OLD_ORACLE_BLOCK)
abort "baseline oracle block missing" unless old_start
abort "baseline oracle block must be unique" if base_item.index(OLD_ORACLE_BLOCK, old_start + 1)
prefix = base_item.byteslice(0, old_start)
suffix_start = old_start + OLD_ORACLE_BLOCK.bytesize
suffix = base_item.byteslice(suffix_start, base_item.bytesize - suffix_start)
unless current_item.start_with?(prefix) && current_item.end_with?(suffix)
  abort "replace-one-oracle changed item prefix/suffix outside the old counter assertion"
end
minimum_size = prefix.bytesize + suffix.bytesize
abort "replacement span is truncated" if current_item.bytesize < minimum_size
replacement_size = current_item.bytesize - minimum_size
replacement = current_item.byteslice(prefix.bytesize, replacement_size)
abort "replacement span must not be empty" if replacement.empty?
abort "replacement span did not replace the old assertion" if replacement == OLD_ORACLE_BLOCK
abort "replacement markers are forbidden" if replacement.include?("S4-ORACLE-REPLACE")
if code_mask(replacement).match?(/\breturn\b/)
  abort "replacement span must not bypass the test with return"
end

actual_digest = Digest::SHA256.hexdigest(replacement)
approved_digest = rest.first
abort "replacement digest mismatch: #{actual_digest}" unless actual_digest == approved_digest

puts "s4 matrix source freeze replace-one-oracle ok #{actual_digest}"
