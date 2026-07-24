#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "fileutils"
require "open3"
require "rbconfig"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
BUILDER = File.join(ROOT, "scripts/build-state-signer-package.rb")
MAGIC = "AGNTIMG\0".b
SIGNER_DOMAIN = "AGENT_KERNEL_ED25519_SIGNER_V1\0".b
SPKI_PREFIX = ["302a300506032b6570032100"].pack("H*")

def assert(condition, message)
  raise message unless condition
end

def run!(*command)
  output, error, status = Open3.capture3(*command)
  raise "#{command.first} failed\n#{output}#{error}" unless status.success?

  output
end

def u16(bytes, offset)
  bytes.byteslice(offset, 2).unpack1("v")
end

def u32(bytes, offset)
  bytes.byteslice(offset, 4).unpack1("V")
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

def builder_command(key_path, provider_object, output_path, signer_id_hex)
  [
    RbConfig.ruby,
    BUILDER,
    "--image-key", key_path,
    "--provider-object", provider_object,
    "--output", output_path,
    "--nonce", "0xa17ce017",
    "--archive-authority", "25",
    "--storage-authority", "2",
    "--root", "1",
    "--storage", "2",
    "--through-sequence", "64",
    "--call-data-generation", "1",
    "--policy-generation", "1",
    "--state-signer-id", signer_id_hex
  ]
end

assert(File.file?(BUILDER), "state signer package builder is missing")

clang = ENV["CLANG"]
clang = "/usr/bin/clang" unless clang && File.executable?(clang)
assert(File.executable?(clang), "clang is unavailable")
openssl = command_path(
  "OPENSSL",
  [
    "openssl",
    "/opt/homebrew/opt/openssl@3/bin/openssl",
    "/opt/homebrew/bin/openssl",
    "/usr/local/opt/openssl@3/bin/openssl"
  ]
)
assert(openssl, "OpenSSL 3 is unavailable")

Dir.mktmpdir("agent-kernel-state-signer-package-test") do |directory|
  key_path = File.join(directory, "image-key.pem")
  run!(openssl, "genpkey", "-algorithm", "ED25519", "-out", key_path)
  File.chmod(0o600, key_path)

  provider_source = File.join(directory, "provider.S")
  provider_object = File.join(directory, "provider.o")
  File.write(
    provider_source,
    <<~ASM
      .intel_syntax noprefix
      .section .text
      .global state_signer_provider_sign
      .type state_signer_provider_sign, @function
      state_signer_provider_sign:
          mov rdi, rsi
          mov ecx, 64
          mov al, 0xa7
          rep stosb
          xor eax, eax
          ret
      .section .note.GNU-stack,"",@progbits
    ASM
  )
  run!(clang, "-c", "-target", "x86_64-unknown-none", provider_source, "-o", provider_object)
  File.chmod(0o600, provider_object)

  output_path = File.join(directory, "state-signer.pkg")
  signer_id_hex = "53" * 32
  output = run!(*builder_command(key_path, provider_object, output_path, signer_id_hex))
  assert(output.include?("kind=state-signer"), "builder omitted public kind evidence")
  assert((File.stat(output_path).mode & 0o777) == 0o600, "package mode is not 0600")

  package = File.binread(output_path)
  assert(package.byteslice(0, 8) == MAGIC, "package magic mismatch")
  assert(u16(package, 8) == 3, "format version mismatch")
  assert(u16(package, 10) == 1, "architecture mismatch")
  assert(u16(package, 12) == 5, "State Signer image kind mismatch")
  assert(u16(package, 14) == 1, "signed flag mismatch")
  assert(u16(package, 16) == 1 && u16(package, 18) == 1, "ABI version mismatch")
  assert(u16(package, 20).zero? && u16(package, 22).zero?, "entry encoding mismatch")
  assert(u32(package, 24).zero?, "entry must start at code offset zero")
  assert(u16(package, 28) == 2, "segment count mismatch")
  assert(u16(package, 30).zero?, "fixed-address package must have no relocations")
  assert(u32(package, 32) == 88, "segment table offset mismatch")
  assert(u32(package, 36) == 136, "relocation table offset mismatch")

  signature_offset = u32(package, 40)
  assert(u32(package, 44) == package.bytesize, "package length mismatch")
  assert(u16(package, 80) == 1 && u16(package, 82) == 64, "signature envelope mismatch")
  assert(u32(package, 84).zero?, "reserved signature field is nonzero")
  code = segment(package, 88)
  rodata = segment(package, 112)
  assert(code == {
    kind: 1,
    flags: 5,
    alignment: 4096,
    file_offset: 136,
    file_length: code[:file_length],
    memory_length: code[:file_length],
    reserved: 0
  }, "code segment mismatch")
  assert(rodata[:kind] == 2 && rodata[:flags] == 1, "rodata segment mismatch")
  assert(rodata[:alignment] == 4096, "rodata alignment mismatch")
  assert(rodata[:file_offset] == 136 + code[:file_length], "rodata offset mismatch")
  assert(rodata[:file_length] == rodata[:memory_length], "rodata length mismatch")
  assert(signature_offset == rodata[:file_offset] + rodata[:file_length], "signature offset mismatch")

  rodata_bytes = package.byteslice(rodata[:file_offset], rodata[:file_length])
  assert(rodata_bytes.include?([signer_id_hex].pack("H*")), "state signer ID missing from policy")
  [25, 2, 1, 2, 64].each do |value|
    assert(rodata_bytes.include?([value].pack("Q<")), "policy value #{value} missing")
  end

  public_der = run!(openssl, "pkey", "-in", key_path, "-pubout", "-outform", "DER")
  assert(public_der.start_with?(SPKI_PREFIX), "image key is not Ed25519")
  public_key = public_der.byteslice(SPKI_PREFIX.bytesize, 32)
  expected_signer_id = Digest::SHA256.digest(SIGNER_DOMAIN + public_key)
  assert(package.byteslice(48, 32) == expected_signer_id, "image signer ID mismatch")
  signed_bytes = package.byteslice(0, signature_offset)
  signature = package.byteslice(signature_offset, 64)
  public_key_path = File.join(directory, "image-key.pub.pem")
  signed_path = File.join(directory, "package.signed.bin")
  signature_path = File.join(directory, "package.signature.bin")
  run!(openssl, "pkey", "-in", key_path, "-pubout", "-out", public_key_path)
  File.binwrite(signed_path, signed_bytes)
  File.binwrite(signature_path, signature)
  run!(
    openssl,
    "pkeyutl",
    "-verify",
    "-pubin",
    "-inkey",
    public_key_path,
    "-rawin",
    "-in",
    signed_path,
    "-sigfile",
    signature_path
  )

  original_key = File.binread(key_path)
  _output, error, status = Open3.capture3(
    *builder_command(key_path, provider_object, key_path, signer_id_hex)
  )
  assert(!status.success?, "builder accepted an input/output alias")
  assert(error.include?("output must differ"), "builder reported the wrong alias failure")
  assert(File.binread(key_path) == original_key, "builder modified the image key")

  hardlink_path = File.join(directory, "key-hardlink.pkg")
  File.link(key_path, hardlink_path)
  _output, error, status = Open3.capture3(
    *builder_command(key_path, provider_object, hardlink_path, signer_id_hex)
  )
  assert(!status.success?, "builder accepted a hard-linked output alias")
  assert(error.include?("hard link"), "builder reported the wrong hard-link failure")
  assert(File.binread(key_path) == original_key, "builder modified the hard-linked image key")

  source_output = File.join(ROOT, ".state-signer-rejected-#{Process.pid}.pkg")
  assert(!File.exist?(source_output), "source-tree rejection path already exists")
  _output, error, status = Open3.capture3(
    *builder_command(key_path, provider_object, source_output, signer_id_hex)
  )
  assert(!status.success?, "builder accepted an output inside the source tree")
  assert(error.include?("outside the source tree"), "builder reported the wrong output-path failure")
  assert(!File.exist?(source_output), "source-tree rejection left an output")

  writable_source = File.join(directory, "writable-provider.S")
  writable_object = File.join(directory, "writable-provider.o")
  File.write(
    writable_source,
    <<~ASM
      .intel_syntax noprefix
      .section .text
      .global state_signer_provider_sign
      .type state_signer_provider_sign, @function
      state_signer_provider_sign:
          xor eax, eax
          ret
      .section .data
      .quad 1
      .section .note.GNU-stack,"",@progbits
    ASM
  )
  run!(clang, "-c", "-target", "x86_64-unknown-none", writable_source, "-o", writable_object)
  File.chmod(0o600, writable_object)
  writable_output = File.join(directory, "writable-provider.pkg")
  _output, error, status = Open3.capture3(
    *builder_command(key_path, writable_object, writable_output, signer_id_hex)
  )
  assert(!status.success?, "builder accepted provider writable state")
  assert(error.include?("writable data"), "builder reported the wrong writable-state failure")
  assert(!File.exist?(writable_output), "writable provider rejection left an output")

  File.chmod(0o644, provider_object)
  rejected_path = File.join(directory, "rejected.pkg")
  _output, _error, status = Open3.capture3(
    *builder_command(key_path, provider_object, rejected_path, signer_id_hex)
  )
  assert(!status.success?, "builder accepted a broadly readable provider object")
  assert(!File.exist?(rejected_path), "rejected build left an output")

  File.chmod(0o600, provider_object)
  File.chmod(0o644, key_path)
  rejected_key_path = File.join(directory, "rejected-key.pkg")
  _output, _error, status = Open3.capture3(
    *builder_command(key_path, provider_object, rejected_key_path, signer_id_hex)
  )
  assert(!status.success?, "builder accepted a broadly readable image key")
  assert(!File.exist?(rejected_key_path), "image-key rejection left an output")
end

puts "[ OK ] State Signer Package v3 / kind 5 / external provider / mode 0600"
