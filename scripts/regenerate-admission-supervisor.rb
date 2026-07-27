#!/usr/bin/env ruby
# frozen_string_literal: true

# Rebuilds the immutable Supervisor Capsule from its auditable assembly source.

require "digest"
require "open3"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
SOURCE = File.join(
  ROOT,
  "crates/agent-kernel-x86_64/src/boot_agent_images/admission_supervisor.S"
)
TARGET = File.join(
  ROOT,
  "crates/agent-kernel-x86_64/src/boot_agent_images/admission_supervisor.rs"
)
MAGIC = "AGNTIMG\0".b

def fail_with(message)
  warn "regenerate admission Supervisor failed: #{message}"
  exit 1
end

def command_path(environment_name, candidates)
  configured = ENV[environment_name]
  return configured if configured && File.executable?(configured)

  candidates.each do |candidate|
    output, status = Open3.capture2("which", candidate)
    return output.strip if status.success?
  end
  nil
end

def run_command(*command)
  output, error, status = Open3.capture3(*command)
  fail_with("#{command.first} exited #{status.exitstatus}\n#{output}#{error}") unless
    status.success?
  output
end

def formatted_numbers(values, formatter, per_line)
  values.each_slice(per_line).map do |slice|
    "    #{slice.map { |value| formatter.call(value) }.join(", ")},"
  end.join("\n")
end

clang = command_path("CLANG", ["clang", "/usr/bin/clang"])
objcopy = command_path(
  "LLVM_OBJCOPY",
  ["llvm-objcopy", "/opt/homebrew/opt/llvm/bin/llvm-objcopy"]
)
nm = command_path("LLVM_NM", ["llvm-nm", "/opt/homebrew/opt/llvm/bin/llvm-nm"])
rustfmt = command_path("RUSTFMT", ["rustfmt"])
fail_with("clang is unavailable; set CLANG") unless clang
fail_with("llvm-objcopy is unavailable; set LLVM_OBJCOPY") unless objcopy
fail_with("llvm-nm is unavailable; set LLVM_NM") unless nm
fail_with("rustfmt is unavailable; set RUSTFMT") unless rustfmt

assembly = File.read(SOURCE)
return_symbols = assembly.scan(
  /^\.global (admission_supervisor_[a-z0-9_]+_return)$/
).flatten
fail_with("expected 44 return symbols, found #{return_symbols.length}") unless
  return_symbols.length == 44

code = nil
offsets = nil
Dir.mktmpdir("agent-kernel-admission-supervisor") do |directory|
  object = File.join(directory, "admission_supervisor.o")
  binary = File.join(directory, "admission_supervisor.bin")
  run_command(clang, "-c", "-target", "x86_64-unknown-none", SOURCE, "-o", object)
  run_command(objcopy, "--only-section=.text", "-O", "binary", object, binary)
  code = File.binread(binary)

  symbols = run_command(nm, "--defined-only", "--format=posix", object).lines.to_h do |line|
    fields = line.split
    next [nil, nil] if fields.length < 3
    [fields[0], Integer(fields[2], 16)]
  rescue ArgumentError
    [nil, nil]
  end
  offsets = return_symbols.map do |symbol|
    symbols.fetch(symbol) { fail_with("linked object lacks #{symbol}") }
  end
end

header = MAGIC
header << [1, 1, 4, 0, 1, 1].pack("v6")
header << [0, code.bytesize, 0].pack("V3")
fail_with("Capsule header length mismatch") unless header.bytesize == 32
capsule = header + code
digest = Digest::SHA256.digest(capsule).bytes

rust = File.read(TARGET)
offsets_source = <<~RUST.chomp
  const RETURN_OFFSETS: [u32; #{offsets.length}] = [
  #{formatted_numbers(offsets, ->(value) { value.to_s }, 12)}
  ];
RUST
capsule_source = <<~RUST.chomp
  static CAPSULE: [u8; #{capsule.bytesize}] = [
  #{formatted_numbers(capsule.bytes, ->(value) { format("0x%02x", value) }, 16)}
  ];
RUST
digest_source = <<~RUST.chomp
  const DIGEST: AgentImageDigest = AgentImageDigest::new([
  #{formatted_numbers(digest, ->(value) { format("0x%02x", value) }, 16)}
  ]);
RUST

replacements = [
  [/const RETURN_OFFSETS: \[u32; \d+\] = \[\n.*?\n\];/m, offsets_source],
  [/static CAPSULE: \[u8; \d+\] = \[\n.*?\n\];/m, capsule_source],
  [/const DIGEST: AgentImageDigest = AgentImageDigest::new\(\[\n.*?\n\]\);/m, digest_source]
]
replacements.each do |pattern, replacement|
  fail_with("target pattern is missing") unless rust.match?(pattern)
  rust = rust.sub(pattern, replacement)
end

temporary = "#{TARGET}.tmp-#{Process.pid}"
begin
  File.binwrite(temporary, rust)
  File.rename(temporary, TARGET)
ensure
  File.delete(temporary) if File.exist?(temporary)
end
run_command(rustfmt, "--edition", "2021", TARGET)

puts "capsule=#{TARGET}"
puts "code_bytes=#{code.bytesize}"
puts "return_offsets=#{offsets.length}"
puts "sha256=#{Digest::SHA256.hexdigest(capsule)}"
