#!/usr/bin/env ruby
# frozen_string_literal: true

# Builds a signed native State Signer package around a selected provider.
# The image key only authenticates the resulting Agent Package and is never
# copied into the output.

require "digest"
require "open3"
require "optparse"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
ENTRY_SOURCE = File.join(
  ROOT,
  "crates/agent-state-signer/native/state_signer_entry.S"
)
LINKER_SCRIPT = File.join(
  ROOT,
  "crates/agent-state-signer/native/state_signer.ld"
)
TPM_PROVIDER_SOURCE = File.join(
  ROOT,
  "crates/agent-state-signer/native/tpm_call_provider.S"
)
MAGIC = "AGNTIMG\0".b
HEADER_BYTES = 88
SEGMENT_BYTES = 24
SIGNATURE_BYTES = 64
MAX_SEGMENT_BYTES = 65_536
ENTRY_ADDRESS = 0x0000_4000_0000_0000
RODATA_ADDRESS = ENTRY_ADDRESS + MAX_SEGMENT_BYTES
SIGNER_DOMAIN = "AGENT_KERNEL_ED25519_SIGNER_V1\0".b
ED25519_SPKI_PREFIX = ["302a300506032b6570032100"].pack("H*")
U64_MAX = (1 << 64) - 1
SIGNATURE_ALGORITHMS = {
  "ed25519" => 1,
  "ecdsa-p256-sha256" => 2
}.freeze

def fail_with(message)
  warn "build State Signer failed: #{message}"
  exit 1
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

def capture_command(*command)
  output, error, status = Open3.capture3(*command)
  return output if status.success?

  fail_with("command #{command.first} exited #{status.exitstatus}\n#{output}#{error}")
end

def run_command(*command)
  capture_command(*command)
  nil
end

def parse_u64(name, raw)
  value = Integer(raw, 0)
  fail_with("#{name} must be in 1..2^64-1") unless (1..U64_MAX).cover?(value)
  value
rescue ArgumentError
  fail_with("#{name} must be an integer")
end

def private_input_path(name, raw)
  path = File.expand_path(raw)
  fail_with("#{name} does not exist: #{path}") unless File.file?(path)
  fail_with("#{name} must not grant group or other access") unless (File.stat(path).mode & 0o077).zero?
  File.realpath(path)
end

def canonical_output_path(raw)
  path = File.expand_path(raw)
  directory = File.dirname(path)
  fail_with("output directory does not exist: #{directory}") unless Dir.exist?(directory)
  File.join(File.realpath(directory), File.basename(path))
end

def inside_root?(path)
  root = File.realpath(ROOT)
  path == root || path.start_with?("#{root}#{File::SEPARATOR}")
end

def segment(kind, flags, file_offset, length)
  [kind, flags].pack("v2") + [4096, file_offset, length, length, 0].pack("V5")
end

def config_source(values, signer_id)
  entries = %i[
    nonce
    archive_authority
    storage_authority
    root
    storage
    through_sequence
    call_data_generation
    policy_generation
    signature_algorithm
  ]
  assembly = <<~ASM.dup
    /* Generated non-secret State Signer policy. */
    .section .rodata.state_signer_config,"a",@progbits
    .balign 8
  ASM
  entries.each do |name|
    symbol = "state_signer_config_#{name}"
    assembly << <<~ASM
      .global #{symbol}
      .type #{symbol}, @object
      #{symbol}:
          .quad 0x#{values.fetch(name).to_s(16)}
      .size #{symbol}, 8
    ASM
  end
  assembly << <<~ASM
    .global state_signer_config_signer_id
    .type state_signer_config_signer_id, @object
    state_signer_config_signer_id:
        .byte #{signer_id.bytes.map { |byte| format("0x%02x", byte) }.join(", ")}
    .size state_signer_config_signer_id, 32
    .section .note.GNU-stack,"",@progbits
  ASM
  assembly
end

def symbol_address(nm, elf, symbol)
  output = capture_command(nm, "--defined-only", "--format=posix", elf)
  line = output.lines.find { |candidate| candidate.start_with?("#{symbol} ") }
  fail_with("linked image does not define #{symbol}") unless line
  fields = line.split
  fail_with("unable to read #{symbol} address") if fields.length < 3
  Integer(fields[2], 16)
rescue ArgumentError
  fail_with("unable to read #{symbol} address")
end

def elf_value(bytes, offset, format, length, field)
  value = bytes.byteslice(offset, length)
  fail_with("linked ELF truncates #{field}") unless value && value.bytesize == length
  value.unpack1(format)
end

def validate_linked_elf(path)
  bytes = File.binread(path)
  fail_with("linked image is not ELF64 little-endian") unless
    bytes.byteslice(0, 7) == "\x7fELF\x02\x01\x01".b
  fail_with("linked image is not an x86_64 executable") unless
    elf_value(bytes, 16, "v", 2, "type") == 2 &&
    elf_value(bytes, 18, "v", 2, "machine") == 62
  fail_with("linked ELF entry address mismatch") unless
    elf_value(bytes, 24, "Q<", 8, "entry") == ENTRY_ADDRESS

  table_offset = elf_value(bytes, 40, "Q<", 8, "section table offset")
  entry_bytes = elf_value(bytes, 58, "v", 2, "section entry size")
  section_count = elf_value(bytes, 60, "v", 2, "section count")
  names_index = elf_value(bytes, 62, "v", 2, "section name index")
  fail_with("linked ELF uses an unsupported section table") unless
    entry_bytes == 64 && section_count.positive? && names_index < section_count
  table_end = table_offset + entry_bytes * section_count
  fail_with("linked ELF section table is out of bounds") if table_end > bytes.bytesize

  sections = section_count.times.map do |index|
    offset = table_offset + index * entry_bytes
    {
      name_offset: elf_value(bytes, offset, "V", 4, "section name"),
      type: elf_value(bytes, offset + 4, "V", 4, "section type"),
      flags: elf_value(bytes, offset + 8, "Q<", 8, "section flags"),
      address: elf_value(bytes, offset + 16, "Q<", 8, "section address"),
      file_offset: elf_value(bytes, offset + 24, "Q<", 8, "section file offset"),
      length: elf_value(bytes, offset + 32, "Q<", 8, "section length")
    }
  end
  names = sections.fetch(names_index)
  names_end = names[:file_offset] + names[:length]
  fail_with("linked ELF section names are out of bounds") if names_end > bytes.bytesize
  names_bytes = bytes.byteslice(names[:file_offset], names[:length])
  sections.each do |section|
    name = names_bytes.byteslice(section[:name_offset], names_bytes.bytesize)
    fail_with("linked ELF has an invalid section name") unless name
    section[:name] = name.split("\0", 2).first
  end

  loadable = sections.select do |section|
    (section[:flags] & 0x2) != 0 && section[:length].positive?
  end
  unexpected = loadable.reject { |section| [".text", ".rodata"].include?(section[:name]) }
  unless unexpected.empty?
    fail_with("provider introduces loadable section #{unexpected.first[:name]}")
  end
  text = loadable.find { |section| section[:name] == ".text" }
  rodata = loadable.find { |section| section[:name] == ".rodata" }
  fail_with("linked ELF lacks code or read-only policy") unless text && rodata
  fail_with("linked ELF code layout is noncanonical") unless
    text[:type] == 1 && text[:flags] == 0x6 && text[:address] == ENTRY_ADDRESS
  fail_with("linked ELF policy layout is noncanonical") unless
    rodata[:type] == 1 && rodata[:flags] == 0x2 && rodata[:address] == RODATA_ADDRESS
end

options = {}
OptionParser.new do |parser|
  parser.banner = <<~USAGE
    usage: scripts/build-state-signer-package.rb [options]
  USAGE
  parser.on("--image-key PATH", "external Ed25519 Agent-image private key") do |path|
    options[:image_key] = path
  end
  parser.on("--provider-object PATH", "external x86_64 provider object") do |path|
    options[:provider_object] = path
  end
  parser.on(
    "--kernel-tpm-provider",
    "compile the built-in Agent Call 56 TPM provider"
  ) do
    options[:kernel_tpm_provider] = true
  end
  parser.on("--output PATH", "Package output outside the source tree") do |path|
    options[:output] = path
  end
  parser.on("--nonce VALUE", "Describe nonce") { |value| options[:nonce] = value }
  parser.on("--archive-authority VALUE", "archive authority ID") do |value|
    options[:archive_authority] = value
  end
  parser.on("--storage-authority VALUE", "storage authority ID") do |value|
    options[:storage_authority] = value
  end
  parser.on("--root VALUE", "event archive root resource ID") { |value| options[:root] = value }
  parser.on("--storage VALUE", "durable storage resource ID") { |value| options[:storage] = value }
  parser.on("--through-sequence VALUE", "archive sequence upper bound") do |value|
    options[:through_sequence] = value
  end
  parser.on("--call-data-generation VALUE", "private call-data generation") do |value|
    options[:call_data_generation] = value
  end
  parser.on("--policy-generation VALUE", "provider policy generation") do |value|
    options[:policy_generation] = value
  end
  parser.on(
    "--signature-algorithm NAME",
    "durable signature algorithm: ed25519 or ecdsa-p256-sha256"
  ) do |value|
    options[:signature_algorithm] = value
  end
  parser.on("--state-signer-id HEX", "32-byte durable State Signer ID") do |value|
    options[:state_signer_id] = value
  end
  parser.on_tail("-h", "--help", "show this help") do
    puts parser
    exit
  end
end.parse!

required = %i[
  image_key
  output
  nonce
  archive_authority
  storage_authority
  root
  storage
  through_sequence
  call_data_generation
  policy_generation
  signature_algorithm
  state_signer_id
]
required.each { |name| fail_with("--#{name.to_s.tr("_", "-")} is required") unless options[name] }
fail_with("unexpected positional arguments") unless ARGV.empty?
if options[:provider_object] && options[:kernel_tpm_provider]
  fail_with("--provider-object and --kernel-tpm-provider are mutually exclusive")
end
unless options[:provider_object] || options[:kernel_tpm_provider]
  fail_with("--provider-object or --kernel-tpm-provider is required")
end

image_key_path = private_input_path("image key", options[:image_key])
provider_object_path = if options[:provider_object]
                         private_input_path("provider object", options[:provider_object])
                       end
output_path = canonical_output_path(options[:output])
output_identity = File.exist?(output_path) ? File.realpath(output_path) : output_path
input_paths = [image_key_path, provider_object_path].compact
if provider_object_path && image_key_path == provider_object_path
  fail_with("image key and provider object must be different files")
end
fail_with("output must differ from inputs") if input_paths.include?(output_identity)
if File.exist?(output_path) && input_paths.any? { |input| File.identical?(output_path, input) }
  fail_with("output must not be a hard link to an input")
end
fail_with("output must be outside the source tree") if inside_root?(output_path)

numeric_values = required
  .filter do |name|
    !%i[
      image_key
      output
      signature_algorithm
      state_signer_id
    ].include?(name)
  end
  .to_h { |name| [name, parse_u64(name.to_s.tr("_", "-"), options.fetch(name))] }
signature_algorithm = SIGNATURE_ALGORITHMS[options[:signature_algorithm]]
unless signature_algorithm
  fail_with("signature-algorithm must be ed25519 or ecdsa-p256-sha256")
end
if options[:kernel_tpm_provider] && signature_algorithm != SIGNATURE_ALGORITHMS["ecdsa-p256-sha256"]
  fail_with("kernel TPM provider requires ecdsa-p256-sha256")
end
numeric_values[:signature_algorithm] = signature_algorithm
signer_id_hex = options[:state_signer_id]
fail_with("state-signer-id must contain exactly 64 hexadecimal digits") unless signer_id_hex.match?(/\A[0-9a-fA-F]{64}\z/)
state_signer_id = [signer_id_hex].pack("H*")
fail_with("state-signer-id must be nonzero") if state_signer_id == "\0" * 32

clang = command_path("CLANG", ["clang", "/usr/bin/clang"])
objcopy = command_path(
  "LLVM_OBJCOPY",
  ["llvm-objcopy", "/opt/homebrew/opt/llvm/bin/llvm-objcopy", "/usr/local/opt/llvm/bin/llvm-objcopy"]
)
nm = command_path(
  "LLVM_NM",
  ["llvm-nm", "/opt/homebrew/opt/llvm/bin/llvm-nm", "/usr/local/opt/llvm/bin/llvm-nm"]
)
openssl = command_path(
  "OPENSSL",
  [
    "openssl",
    "/opt/homebrew/opt/openssl@3/bin/openssl",
    "/opt/homebrew/bin/openssl",
    "/usr/local/opt/openssl@3/bin/openssl"
  ]
)
rust_lld_candidates = Dir[
  File.join(Dir.home, ".rustup/toolchains/*/lib/rustlib/*/bin/rust-lld")
].sort_by { |path| path.include?("/nightly-") ? 0 : 1 }
rust_lld = command_path("RUST_LLD", ["rust-lld", *rust_lld_candidates])
fail_with("clang is unavailable; set CLANG") unless clang
fail_with("llvm-objcopy is unavailable; set LLVM_OBJCOPY") unless objcopy
fail_with("llvm-nm is unavailable; set LLVM_NM") unless nm
fail_with("OpenSSL 3 is unavailable; set OPENSSL") unless openssl
fail_with("rust-lld is unavailable; set RUST_LLD") unless rust_lld

public_der = capture_command(
  openssl,
  "pkey",
  "-in",
  image_key_path,
  "-pubout",
  "-outform",
  "DER"
)
unless public_der.start_with?(ED25519_SPKI_PREFIX) && public_der.bytesize == 44
  fail_with("image key is not Ed25519")
end
public_key = public_der.byteslice(ED25519_SPKI_PREFIX.bytesize, 32)
image_signer_id = Digest::SHA256.digest(SIGNER_DOMAIN + public_key)

package = nil
Dir.mktmpdir("agent-kernel-state-signer") do |directory|
  entry_object = File.join(directory, "state_signer_entry.o")
  built_in_provider_object = File.join(directory, "tpm_call_provider.o")
  config_assembly = File.join(directory, "state_signer_config.S")
  config_object = File.join(directory, "state_signer_config.o")
  linked_elf = File.join(directory, "state_signer.elf")
  code_path = File.join(directory, "state_signer.text.bin")
  rodata_path = File.join(directory, "state_signer.rodata.bin")
  signed_path = File.join(directory, "state_signer.signed.bin")
  signature_path = File.join(directory, "state_signer.signature.bin")

  File.write(config_assembly, config_source(numeric_values, state_signer_id))
  run_command(clang, "-c", "-target", "x86_64-unknown-none", ENTRY_SOURCE, "-o", entry_object)
  if options[:kernel_tpm_provider]
    run_command(
      clang,
      "-c",
      "-target",
      "x86_64-unknown-none",
      TPM_PROVIDER_SOURCE,
      "-o",
      built_in_provider_object
    )
  end
  selected_provider_object = built_in_provider_object if options[:kernel_tpm_provider]
  selected_provider_object ||= provider_object_path
  run_command(clang, "-c", "-target", "x86_64-unknown-none", config_assembly, "-o", config_object)
  run_command(
    rust_lld,
    "-flavor",
    "gnu",
    "-m",
    "elf_x86_64",
    "-nostdlib",
    "-static",
    "--no-undefined",
    "--build-id=none",
    "-z",
    "noexecstack",
    "-T",
    LINKER_SCRIPT,
    entry_object,
    selected_provider_object,
    config_object,
    "-o",
    linked_elf
  )
  validate_linked_elf(linked_elf)
  unless symbol_address(nm, linked_elf, "state_signer_entry") == ENTRY_ADDRESS
    fail_with("State Signer entry is not at the fixed Agent code address")
  end

  run_command(objcopy, "--only-section=.text", "-O", "binary", linked_elf, code_path)
  run_command(objcopy, "--only-section=.rodata", "-O", "binary", linked_elf, rodata_path)
  code = File.binread(code_path)
  rodata = File.binread(rodata_path)
  unless (1..MAX_SEGMENT_BYTES).cover?(code.bytesize)
    fail_with("code length is outside the Package v3 bound")
  end
  unless (1..MAX_SEGMENT_BYTES).cover?(rodata.bytesize)
    fail_with("rodata length is outside the Package v3 bound")
  end

  segment_table_offset = HEADER_BYTES
  relocation_table_offset = HEADER_BYTES + 2 * SEGMENT_BYTES
  code_offset = relocation_table_offset
  rodata_offset = code_offset + code.bytesize
  signature_offset = rodata_offset + rodata.bytesize
  package_length = signature_offset + SIGNATURE_BYTES
  header = MAGIC + [3, 1, 5, 1, 1, 1, 0, 0].pack("v8")
  header << [0].pack("V")
  header << [2, 0].pack("v2")
  header << [
    segment_table_offset,
    relocation_table_offset,
    signature_offset,
    package_length
  ].pack("V4")
  header << image_signer_id
  header << [1, SIGNATURE_BYTES].pack("v2")
  header << [0].pack("V")
  fail_with("internal header length mismatch") unless header.bytesize == HEADER_BYTES

  signed_bytes = header
  signed_bytes << segment(1, 5, code_offset, code.bytesize)
  signed_bytes << segment(2, 1, rodata_offset, rodata.bytesize)
  signed_bytes << code << rodata
  fail_with("internal signature offset mismatch") unless signed_bytes.bytesize == signature_offset
  File.binwrite(signed_path, signed_bytes)
  File.chmod(0o600, signed_path)
  run_command(
    openssl,
    "pkeyutl",
    "-sign",
    "-rawin",
    "-inkey",
    image_key_path,
    "-in",
    signed_path,
    "-out",
    signature_path
  )
  signature = File.binread(signature_path)
  fail_with("Ed25519 signature length mismatch") unless signature.bytesize == SIGNATURE_BYTES
  package = signed_bytes + signature
end

temporary_output = "#{output_path}.tmp-#{Process.pid}"
begin
  File.open(
    temporary_output,
    File::WRONLY | File::CREAT | File::EXCL | File::BINARY,
    0o600
  ) { |file| file.write(package) }
  File.chmod(0o600, temporary_output)
  File.rename(temporary_output, output_path)
ensure
  File.delete(temporary_output) if File.exist?(temporary_output)
end

puts "kind=state-signer"
puts "provider=#{options[:kernel_tpm_provider] ? "kernel-tpm-agent-call-56" : "external"}"
puts "signature_algorithm=#{options[:signature_algorithm]}"
puts "package=#{output_path}"
puts "bytes=#{package.bytesize}"
puts "public_key=#{public_key.unpack1("H*")}"
puts "image_signer_id=#{image_signer_id.unpack1("H*")}"
puts "sha256=#{Digest::SHA256.hexdigest(package)}"
