#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "open3"
require "openssl"
require "rbconfig"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
INSPECTOR = File.join(ROOT, "scripts/inspect-tpm-state-signer.rb")
SIGNER_DOMAIN = "AGENT-KERNEL-DURABLE-STATE-SIGNER-V2\0".b

def assert(condition, message)
  raise message unless condition
end

def run(*command)
  Open3.capture3(*command)
end

assert(File.file?(INSPECTOR), "TPM State Signer inspector is missing")

Dir.mktmpdir("agent-kernel-tpm-profile-test") do |directory|
  private_key = File.join(directory, "private.pem")
  public_key = File.join(directory, "public.pem")
  name_path = File.join(directory, "name.bin")
  output, error, status = run(
    "openssl", "genpkey", "-algorithm", "EC",
    "-pkeyopt", "ec_paramgen_curve:P-256", "-out", private_key
  )
  assert(status.success?, "OpenSSL key generation failed\n#{output}#{error}")
  output, error, status = run(
    "openssl", "pkey", "-in", private_key, "-pubout", "-out", public_key
  )
  assert(status.success?, "OpenSSL public-key export failed\n#{output}#{error}")

  name = [0x000b].pack("n") + ("\xa5".b * 32)
  File.binwrite(name_path, [name.bytesize].pack("n") + name)
  output, error, status = run(
    RbConfig.ruby, INSPECTOR,
    "--handle", "0x81010001",
    "--command", "sign-digest-v185",
    "--policy-generation", "7",
    "--name", name_path,
    "--public-key", public_key
  )
  assert(status.success?, "inspector rejected a canonical profile\n#{output}#{error}")

  key = OpenSSL::PKey.read(File.binread(public_key))
  compressed = key.public_key.to_octet_string(:compressed)
  signer_id = Digest::SHA256.hexdigest(SIGNER_DOMAIN + [2].pack("v") + compressed)
  assert(output.include?("persistent_handle=0x81010001"), "handle evidence missing")
  assert(output.include?("digest_sign_command=SignDigestV185"), "command evidence missing")
  assert(output.include?("policy_generation=7"), "generation evidence missing")
  assert(output.include?("expected_name=#{name.unpack1("H*")}"), "Name evidence missing")
  assert(
    output.include?("expected_public_key=#{compressed.unpack1("H*")}"),
    "public-key evidence missing"
  )
  assert(output.include?("state_signer_id=#{signer_id}"), "signer ID evidence missing")

  File.binwrite(name_path, "\x00\x0c".b + ("\xa5".b * 32))
  _output, error, status = run(
    RbConfig.ruby, INSPECTOR,
    "--handle", "0x81010001",
    "--command", "sign-v184",
    "--policy-generation", "7",
    "--name", name_path,
    "--public-key", public_key
  )
  assert(!status.success?, "inspector accepted a non-SHA256 Name")
  assert(error.include?("TPM_ALG_SHA256"), "wrong Name failure")
end

puts "[ OK ] TPM State Signer public provisioning profile"
