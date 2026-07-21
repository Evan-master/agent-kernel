#!/usr/bin/env ruby
# frozen_string_literal: true

# Audits every immutable native Agent image embedded in the x86_64 boot path.
# The script uses only Ruby's standard library so it can run beside Cargo gates.

require "digest"
require "open3"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
IMAGE_ROOT = File.join(ROOT, "crates/agent-kernel-x86_64/src/boot_agent_images")
MAGIC = "AGNTIMG\0".b
CAPSULE_V1_HEADER_BYTES = 32
PACKAGE_V2_HEADER_BYTES = 48
SEGMENT_DESCRIPTOR_BYTES = 24
RELOCATION_BYTES = 24
PAGE_BYTES = 4096
RODATA_BASE = 0x0000_4000_0001_0000
OLD_FIXED_BASE = 0x0000_4000_0001_0000
OLD_FIXED_END = 0x0000_4000_0002_0000

def assert(condition, message)
  raise "audit failed: #{message}" unless condition
end

def rust_source(relative_path)
  File.read(File.join(ROOT, relative_path))
end

def extract_bytes(source, name)
  pattern = /(?:const|static)\s+#{Regexp.escape(name)}:\s*\[u8;\s*([\d_]+)\]\s*=\s*\[(.*?)\n\];/m
  match = source.match(pattern)
  assert(match, "missing byte array #{name}")

  declared_length = match[1].delete("_").to_i
  bytes = match[2].scan(/0x([0-9a-fA-F]{2})/).flatten.map { |value| value.to_i(16) }.pack("C*")
  assert(bytes.bytesize == declared_length, "#{name} declares #{declared_length} bytes, found #{bytes.bytesize}")
  bytes
end

def extract_digest(source, name)
  pattern = /const\s+#{Regexp.escape(name)}:\s*AgentImageDigest\s*=\s*AgentImageDigest::new\(\[(.*?)\]\);/m
  match = source.match(pattern)
  assert(match, "missing digest #{name}")

  bytes = match[1].scan(/0x([0-9a-fA-F]{2})/).flatten.map { |value| value.to_i(16) }.pack("C*")
  assert(bytes.bytesize == 32, "#{name} must contain 32 digest bytes")
  bytes
end

def u16(bytes, offset)
  bytes.byteslice(offset, 2).unpack1("v")
end

def u32(bytes, offset)
  bytes.byteslice(offset, 4).unpack1("V")
end

def i64(bytes, offset)
  bytes.byteslice(offset, 8).unpack1("q<")
end

def verify_digest(name, bytes, expected)
  actual = Digest::SHA256.digest(bytes)
  assert(actual == expected, "#{name} SHA-256 mismatch")
  actual.unpack1("H*")
end

def verify_common_header(name, bytes, version)
  assert(bytes.bytesize >= CAPSULE_V1_HEADER_BYTES, "#{name} header truncated")
  assert(bytes.start_with?(MAGIC), "#{name} magic mismatch")
  assert(u16(bytes, 8) == version, "#{name} format must be #{version}")
  assert(u16(bytes, 10) == 1, "#{name} architecture must be x86_64")
  assert((1..4).cover?(u16(bytes, 12)), "#{name} image kind is unsupported")
  assert(u16(bytes, 14).zero?, "#{name} flags must be zero")
  assert(!u16(bytes, 16).zero? && !u16(bytes, 18).zero?, "#{name} versions must be nonzero")
end

def verify_no_old_fixed_addresses(name, code)
  (OLD_FIXED_BASE...OLD_FIXED_END).step(PAGE_BYTES) do |address|
    encoded = [address].pack("Q<")
    assert(!code.include?(encoded), format("%s retains old fixed address 0x%016x", name, address))
  end
end

def verify_capsule_v1(name, bytes, digest)
  verify_common_header(name, bytes, 1)
  entry_offset = u32(bytes, 20)
  code_length = u32(bytes, 24)
  assert(u32(bytes, 28).zero?, "#{name} reserved header word must be zero")
  assert((1..65_536).cover?(code_length), "#{name} code length is outside the V1 bound")
  assert(bytes.bytesize == CAPSULE_V1_HEADER_BYTES + code_length, "#{name} length mismatch")
  assert(entry_offset < code_length, "#{name} entry is outside code")

  code = bytes.byteslice(CAPSULE_V1_HEADER_BYTES, code_length)
  verify_no_old_fixed_addresses(name, code)
  verify_digest(name, bytes, digest)
end

def segment(bytes, offset)
  {
    kind: u16(bytes, offset),
    flags: u16(bytes, offset + 2),
    alignment: u32(bytes, offset + 4),
    file_offset: u32(bytes, offset + 8),
    file_length: u32(bytes, offset + 12),
    memory_length: u32(bytes, offset + 16),
    reserved: u32(bytes, offset + 20)
  }
end

def verify_package_v2(name, bytes, digest)
  verify_common_header(name, bytes, 2)
  assert(bytes.bytesize >= PACKAGE_V2_HEADER_BYTES, "#{name} V2 header truncated")
  assert(u16(bytes, 20).zero? && u16(bytes, 22).zero?, "#{name} entry segment or reserved word is invalid")
  entry_offset = u32(bytes, 24)
  segment_count = u16(bytes, 28)
  relocation_count = u16(bytes, 30)
  assert(segment_count == 2, "#{name} must contain two segments")
  assert((0..64).cover?(relocation_count), "#{name} relocation count is outside the V2 bound")
  assert(u32(bytes, 32) == PACKAGE_V2_HEADER_BYTES, "#{name} segment table offset is noncanonical")
  assert(u32(bytes, 36) == 96, "#{name} relocation table offset is noncanonical")
  assert(u32(bytes, 40) == bytes.bytesize, "#{name} package length mismatch")
  assert(u32(bytes, 44).zero?, "#{name} reserved header word must be zero")

  code_segment = segment(bytes, PACKAGE_V2_HEADER_BYTES)
  rodata_segment = segment(bytes, PACKAGE_V2_HEADER_BYTES + SEGMENT_DESCRIPTOR_BYTES)
  assert(code_segment.values_at(:kind, :flags, :alignment) == [1, 5, PAGE_BYTES], "#{name} code descriptor is invalid")
  assert(rodata_segment.values_at(:kind, :flags, :alignment) == [2, 1, PAGE_BYTES], "#{name} rodata descriptor is invalid")
  [code_segment, rodata_segment].each do |descriptor|
    assert((1..65_536).cover?(descriptor[:file_length]), "#{name} segment length is outside the V2 bound")
    assert(descriptor[:memory_length] == descriptor[:file_length], "#{name} segment memory length mismatch")
    assert(descriptor[:reserved].zero?, "#{name} segment reserved word must be zero")
  end

  payload_offset = 96 + relocation_count * RELOCATION_BYTES
  code_end = payload_offset + code_segment[:file_length]
  rodata_end = code_end + rodata_segment[:file_length]
  assert(code_segment[:file_offset] == payload_offset, "#{name} code offset is noncanonical")
  assert(rodata_segment[:file_offset] == code_end, "#{name} rodata offset is noncanonical")
  assert(rodata_end == bytes.bytesize, "#{name} payload has gaps or trailing bytes")
  assert(entry_offset < code_segment[:file_length], "#{name} entry is outside code")

  code = bytes.byteslice(payload_offset, code_segment[:file_length])
  rodata = bytes.byteslice(code_end, rodata_segment[:file_length])
  previous_target = nil
  relocation_count.times do |index|
    offset = 96 + index * RELOCATION_BYTES
    target = u32(bytes, offset + 8)
    addend = i64(bytes, offset + 16)
    assert([u16(bytes, offset), u16(bytes, offset + 2), u16(bytes, offset + 4)] == [0, 1, 1], "#{name} relocation #{index} type is invalid")
    assert(u16(bytes, offset + 6).zero? && u32(bytes, offset + 12).zero?, "#{name} relocation #{index} reserved fields are nonzero")
    assert(target + 8 <= code.bytesize, "#{name} relocation #{index} target is outside code")
    assert(target / PAGE_BYTES == (target + 7) / PAGE_BYTES, "#{name} relocation #{index} crosses a code page")
    assert(addend >= 0 && addend < rodata.bytesize, "#{name} relocation #{index} addend is outside rodata")
    assert(previous_target.nil? || target >= previous_target + 8, "#{name} relocations are unordered or overlapping")
    assert(code.byteslice(target, 8) == "\0" * 8, "#{name} relocation #{index} placeholder is nonzero")
    resolved = RODATA_BASE + addend
    patched = code.dup
    patched[target, 8] = [resolved].pack("Q<")
    assert(patched.byteslice(target, 8).unpack1("Q<") == resolved, "#{name} relocation #{index} simulation failed")
    previous_target = target
  end

  assert(rodata == "AGENT_KERNEL_PACKAGE_V2_RODATA\0".b, "#{name} rodata proof payload mismatch")
  verify_no_old_fixed_addresses(name, code)
  verify_digest(name, bytes, digest)
end

def command_path(environment_name, candidates)
  configured = ENV[environment_name]
  return configured if configured && File.executable?(configured)

  candidates.each do |candidate|
    if candidate.include?(File::SEPARATOR)
      return candidate if File.executable?(candidate)
      next
    end

    output, status = Open3.capture2("which", candidate)
    return output.strip if status.success?
  end
  nil
end

def run_command(*command)
  output, error, status = Open3.capture3(*command)
  return if status.success?

  raise "audit failed: command #{command.first} exited #{status.exitstatus}\n#{output}#{error}"
end

def assembled_sections(source_name, sections, clang, objcopy, temporary_directory)
  source = File.join(IMAGE_ROOT, source_name)
  object = File.join(temporary_directory, "#{File.basename(source_name, ".S")}.o")
  run_command(clang, "-c", "-target", "x86_64-unknown-none", source, "-o", object)

  sections.to_h do |section|
    binary = File.join(temporary_directory, "#{File.basename(source_name, ".S")}.#{section.delete_prefix(".")}.bin")
    run_command(objcopy, "--only-section=#{section}", "-O", "binary", object, binary)
    [section, File.binread(binary)]
  end
end

def verify_assembly_sources(images)
  clang = command_path("CLANG", ["clang", "/usr/bin/clang"])
  objcopy = command_path(
    "LLVM_OBJCOPY",
    ["llvm-objcopy", "/opt/homebrew/opt/llvm/bin/llvm-objcopy", "/usr/local/opt/llvm/bin/llvm-objcopy"]
  )
  assert(clang, "--assembly requires clang; set CLANG to its executable path")
  assert(objcopy, "--assembly requires llvm-objcopy; set LLVM_OBJCOPY to its executable path")

  image_map = images.to_h { |name, format, bytes, _digest| [name, [format, bytes]] }
  Dir.mktmpdir("agent-image-audit") do |directory|
    {
      "fault-worker" => "fault_worker.S",
      "reuse-worker" => "reuse_worker.S",
      "admission-supervisor" => "admission_supervisor.S"
    }.each do |name, source|
      _format, capsule = image_map.fetch(name)
      assembled = assembled_sections(source, [".text"], clang, objcopy, directory).fetch(".text")
      assert(assembled == capsule.byteslice(CAPSULE_V1_HEADER_BYTES..), "#{source} differs from embedded code")
    end

    _format, package = image_map.fetch("resource-manager")
    code_descriptor = segment(package, PACKAGE_V2_HEADER_BYTES)
    rodata_descriptor = segment(package, PACKAGE_V2_HEADER_BYTES + SEGMENT_DESCRIPTOR_BYTES)
    assembled = assembled_sections("resource_manager.S", [".text", ".rodata"], clang, objcopy, directory)
    code = package.byteslice(code_descriptor[:file_offset], code_descriptor[:file_length])
    rodata = package.byteslice(rodata_descriptor[:file_offset], rodata_descriptor[:file_length])
    assert(assembled.fetch(".text") == code, "resource_manager.S .text differs from embedded code")
    assert(assembled.fetch(".rodata") == rodata, "resource_manager.S .rodata differs from embedded rodata")
  end

  puts "[ OK ] 4 assembly sources / exact embedded .text and .rodata bytes"
end

def occurrence_count(bytes, needle)
  count = 0
  offset = 0
  while (found = bytes.index(needle, offset))
    count += 1
    offset = found + 1
  end
  count
end

def verify_release_elf(images, path)
  assert(File.file?(path), "ELF does not exist: #{path}")
  elf = File.binread(path)
  image_map = images.to_h { |name, format, bytes, _digest| [name, [format, bytes]] }

  _format, package = image_map.fetch("resource-manager")
  code_descriptor = segment(package, PACKAGE_V2_HEADER_BYTES)
  rodata_descriptor = segment(package, PACKAGE_V2_HEADER_BYTES + SEGMENT_DESCRIPTOR_BYTES)
  resource_code = package.byteslice(code_descriptor[:file_offset], code_descriptor[:file_length])
  resource_rodata = package.byteslice(rodata_descriptor[:file_offset], rodata_descriptor[:file_length])
  _format, admission = image_map.fetch("admission-supervisor")
  admission_code = admission.byteslice(CAPSULE_V1_HEADER_BYTES..)

  {
    "Resource Manager Package v2" => package,
    "Resource Manager code" => resource_code,
    "Resource Manager rodata" => resource_rodata,
    "Admission Supervisor Capsule v1" => admission,
    "Admission Supervisor code" => admission_code
  }.each do |name, bytes|
    count = occurrence_count(elf, bytes)
    assert(count == 1, "#{name} occurs #{count} times in #{path}; expected exactly one")
  end

  puts "[ OK ] Release ELF / unique Package, Capsule, code, and rodata payloads"
end

arguments = ARGV.dup
assembly_audit = !arguments.delete("--assembly").nil?
elf_path = nil
if (elf_index = arguments.index("--elf"))
  assert(elf_index + 1 < arguments.length, "--elf requires a path")
  elf_path = File.expand_path(arguments.fetch(elf_index + 1), ROOT)
  arguments.slice!(elf_index, 2)
end
assert(arguments.empty?, "usage: scripts/audit-agent-images.rb [--assembly] [--elf PATH]")

images = []

boot_source = rust_source("crates/agent-kernel-x86_64/src/boot_agent_images.rs")
%w[WORKER_A WORKER_B VERIFIER FAULT_WORKER].each do |prefix|
  images << [prefix.downcase.tr("_", "-"), :v1, extract_bytes(boot_source, "#{prefix}_CAPSULE"), extract_digest(boot_source, "#{prefix}_DIGEST")]
end

[
  ["fault-handler", "fault_handler.rs"],
  ["reuse-worker", "reuse_worker.rs"],
  ["admission-supervisor", "admission_supervisor.rs"]
].each do |name, file|
  source = File.read(File.join(IMAGE_ROOT, file))
  images << [name, :v1, extract_bytes(source, "CAPSULE"), extract_digest(source, "DIGEST")]
end

resource_source = File.read(File.join(IMAGE_ROOT, "resource_manager.rs"))
images << ["resource-manager", :v2, extract_bytes(resource_source, "PACKAGE"), extract_digest(resource_source, "DIGEST")]

images.each do |name, format, bytes, digest|
  sha = if format == :v1
          verify_capsule_v1(name, bytes, digest)
        else
          verify_package_v2(name, bytes, digest)
        end
  puts format("[ OK ] %-20s %-10s %6d bytes  sha256:%s", name, format.to_s.upcase, bytes.bytesize, sha[0, 12])
end

puts "[ OK ] 8 native Agent images / canonical headers / digests / fixed addresses"
puts "[ OK ] Package v2 / code RX / rodata R+NX / ABS64 simulation"
verify_assembly_sources(images) if assembly_audit
verify_release_elf(images, elf_path) if elf_path
