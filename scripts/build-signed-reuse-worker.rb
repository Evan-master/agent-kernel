#!/usr/bin/env ruby
# frozen_string_literal: true

# Builds the deferred reuse Worker as a canonical signed Agent Package. The
# Ed25519 private key remains external and is never copied into repository
# output or printed by this tool.

require "digest"
require "open3"
require "openssl"
require "optparse"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
SOURCE = File.join(ROOT, "crates/agent-kernel-x86_64/src/boot_agent_images/reuse_worker.S")
MAGIC = "AGNTIMG\0".b
HEADER_BYTES = 88
SEGMENT_BYTES = 24
SIGNATURE_BYTES = 64
SIGNER_DOMAIN = "AGENT_KERNEL_ED25519_SIGNER_V1\0".b
ED25519_SPKI_PREFIX = ["302a300506032b6570032100"].pack("H*")

def fail_with(message)
  warn "build signed reuse Worker failed: #{message}"
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

def run_command(*command)
  output, error, status = Open3.capture3(*command)
  return if status.success?

  fail_with("command #{command.first} exited #{status.exitstatus}\n#{output}#{error}")
end

def segment(kind, flags, file_offset, length)
  [kind, flags].pack("v2") + [4096, file_offset, length, length, 0].pack("V5")
end

options = {}
OptionParser.new do |parser|
  parser.banner = "usage: scripts/build-signed-reuse-worker.rb --key PATH --output PATH"
  parser.on("--key PATH", "Ed25519 private key in PEM or DER form") { |path| options[:key] = path }
  parser.on("--output PATH", "Package output path") { |path| options[:output] = path }
end.parse!
fail_with("--key is required") unless options[:key]
fail_with("--output is required") unless options[:output]
fail_with("unexpected positional arguments") unless ARGV.empty?

key_path = File.expand_path(options[:key])
output_path = File.expand_path(options[:output])
fail_with("key does not exist: #{key_path}") unless File.file?(key_path)
fail_with("output directory does not exist: #{File.dirname(output_path)}") unless Dir.exist?(File.dirname(output_path))
fail_with("key must not grant group or other access") unless (File.stat(key_path).mode & 0o077).zero?
key_identity = File.realpath(key_path)
output_identity = File.exist?(output_path) ? File.realpath(output_path) : output_path
fail_with("key and output must be different files") if key_identity == output_identity

clang = command_path("CLANG", ["clang", "/usr/bin/clang"])
objcopy = command_path(
  "LLVM_OBJCOPY",
  ["llvm-objcopy", "/opt/homebrew/opt/llvm/bin/llvm-objcopy", "/usr/local/opt/llvm/bin/llvm-objcopy"]
)
fail_with("clang is unavailable; set CLANG") unless clang
fail_with("llvm-objcopy is unavailable; set LLVM_OBJCOPY") unless objcopy

key = OpenSSL::PKey.read(File.binread(key_path))
public_der = key.public_to_der
fail_with("key is not Ed25519") unless public_der.start_with?(ED25519_SPKI_PREFIX) && public_der.bytesize == 44
public_key = public_der.byteslice(ED25519_SPKI_PREFIX.bytesize, 32)
signer_id = Digest::SHA256.digest(SIGNER_DOMAIN + public_key)

package = nil
Dir.mktmpdir("agent-kernel-reuse-worker") do |directory|
  object = File.join(directory, "reuse_worker.o")
  code_path = File.join(directory, "reuse_worker.text.bin")
  rodata_path = File.join(directory, "reuse_worker.rodata.bin")
  run_command(clang, "-c", "-target", "x86_64-unknown-none", SOURCE, "-o", object)
  run_command(objcopy, "--only-section=.text", "-O", "binary", object, code_path)
  run_command(objcopy, "--only-section=.rodata", "-O", "binary", object, rodata_path)
  code = File.binread(code_path)
  rodata = File.binread(rodata_path)
  fail_with("code length is outside the Package v3 bound") unless (1..65_536).cover?(code.bytesize)
  fail_with("rodata length is outside the Package v3 bound") unless (1..65_536).cover?(rodata.bytesize)

  relocation_offset = HEADER_BYTES + 2 * SEGMENT_BYTES
  code_offset = relocation_offset
  rodata_offset = code_offset + code.bytesize
  signature_offset = rodata_offset + rodata.bytesize
  package_length = signature_offset + SIGNATURE_BYTES
  header = MAGIC + [3, 1, 1, 1, 1, 1, 0, 0].pack("v8")
  header << [0].pack("V")
  header << [2, 0].pack("v2")
  header << [HEADER_BYTES, relocation_offset, signature_offset, package_length].pack("V4")
  header << signer_id
  header << [1, SIGNATURE_BYTES].pack("v2")
  header << [0].pack("V")
  fail_with("internal header length mismatch") unless header.bytesize == HEADER_BYTES

  signed_bytes = header
  signed_bytes << segment(1, 5, code_offset, code.bytesize)
  signed_bytes << segment(2, 1, rodata_offset, rodata.bytesize)
  signed_bytes << code << rodata
  fail_with("internal signature offset mismatch") unless signed_bytes.bytesize == signature_offset
  signature = key.sign(nil, signed_bytes)
  fail_with("Ed25519 signature length mismatch") unless signature.bytesize == SIGNATURE_BYTES
  package = signed_bytes + signature
end

File.binwrite(output_path, package)
puts "package=#{output_path}"
puts "bytes=#{package.bytesize}"
puts "public_key=#{public_key.unpack1("H*")}"
puts "signer_id=#{signer_id.unpack1("H*")}"
puts "sha256=#{Digest::SHA256.hexdigest(package)}"
