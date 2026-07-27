#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "open3"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
BUILD_SCRIPT = File.join(ROOT, "crates/agent-kernel-x86_64/build.rs")

def fail_with(message)
  warn("qemu durable profile contract failed: #{message}")
  exit(1)
end

def run!(environment, *command)
  output, status = Open3.capture2e(environment, *command)
  fail_with("#{command.join(' ')}\n#{output}") unless status.success?
  output
end

def run_rejected(environment, *command)
  output, status = Open3.capture2e(environment, *command)
  fail_with("unexpected success: #{command.join(' ')}") if status.success?
  output
end

Dir.mktmpdir("agent-kernel-qemu-durable-profile") do |directory|
  build_binary = File.join(directory, "build-profile")
  run!({}, "rustc", "--edition=2021", BUILD_SCRIPT, "-o", build_binary)

  package = File.join(directory, "state-signer.bin")
  File.binwrite(package, "AGNTIMG\0" + ("\0" * 144))
  profile = File.join(directory, "profile")
  File.write(
    profile,
    <<~PROFILE
      version=1
      root_resource=1
      storage_resource=12
      base_lba=0
      policy_generation=1
      tpm_handle=0x81010001
      tpm_command=sign-v184
      tpm_name_hex=000b#{'11' * 32}
      state_public_key_sec1_hex=02#{'22' * 32}
      pcr_selection_hex=000080
      pcr_digest_hex=#{'33' * 32}
      state_signer_package=#{package}
      state_signer_public_key_hex=#{'44' * 32}
      state_signer_agent=11
      archive_authority=36
      storage_authority=37
      state_signer_nonce=0xa17ce017
      through_sequence=64
      call_data_generation=1
      state_signer_return_offsets=48,112,704,438,512,548
    PROFILE
  )

  disabled_out = File.join(directory, "disabled")
  FileUtils.mkdir_p(disabled_out)
  disabled_environment = {
    "OUT_DIR" => disabled_out,
    "CARGO_FEATURE_QEMU_DURABLE_PROOF" => nil,
    "AGENT_KERNEL_QEMU_DURABLE_ROLE" => nil,
    "AGENT_KERNEL_QEMU_DURABLE_PROFILE" => nil
  }
  run!(disabled_environment, build_binary)
  disabled = File.read(File.join(disabled_out, "qemu_durable_profile.rs"))
  fail_with("disabled role missing") unless disabled.include?("QEMU_DURABLE_ROLE: u8 = 0;")

  feature_disabled_out = File.join(directory, "feature-disabled")
  FileUtils.mkdir_p(feature_disabled_out)
  run!(
    disabled_environment.merge(
      "OUT_DIR" => feature_disabled_out,
      "CARGO_FEATURE_QEMU_DURABLE_PROOF" => "1"
    ),
    build_binary
  )
  feature_disabled = File.read(
    File.join(feature_disabled_out, "qemu_durable_profile.rs")
  )
  fail_with("feature-enabled disabled role missing") unless
    feature_disabled.include?("QEMU_DURABLE_ROLE: u8 = 0;")

  writer_out = File.join(directory, "writer")
  FileUtils.mkdir_p(writer_out)
  writer_environment = {
    "OUT_DIR" => writer_out,
    "CARGO_FEATURE_QEMU_DURABLE_PROOF" => "1",
    "AGENT_KERNEL_QEMU_DURABLE_ROLE" => "writer",
    "AGENT_KERNEL_QEMU_DURABLE_PROFILE" => profile
  }
  run!(writer_environment, build_binary)
  writer = File.read(File.join(writer_out, "qemu_durable_profile.rs"))
  {
    "writer role" => "QEMU_DURABLE_ROLE: u8 = 1;",
    "storage resource" => "QEMU_DURABLE_STORAGE_RESOURCE: u64 = 12;",
    "TPM handle" => "QEMU_DURABLE_TPM_HANDLE: u32 = 0x81010001;",
    "PCR selection" => "QEMU_DURABLE_PCR_SELECTION: [u8; 3] = [0x00, 0x00, 0x80];",
    "return offsets" => "QEMU_STATE_SIGNER_RETURN_OFFSETS: [u32; 6] = [48, 112, 704, 438, 512, 548];",
    "nonce" => "QEMU_STATE_SIGNER_NONCE: u64 = 0x00000000a17ce017;",
    "through sequence" => "QEMU_DURABLE_THROUGH_SEQUENCE: u64 = 64;",
    "StateSigner package" => "include_bytes!"
  }.each do |name, fragment|
    fail_with("#{name} missing") unless writer.include?(fragment)
  end

  recovery_out = File.join(directory, "recovery")
  FileUtils.mkdir_p(recovery_out)
  run!(
    writer_environment.merge(
      "OUT_DIR" => recovery_out,
      "AGENT_KERNEL_QEMU_DURABLE_ROLE" => "recovery"
    ),
    build_binary
  )
  recovery = File.read(File.join(recovery_out, "qemu_durable_profile.rs"))
  fail_with("recovery role missing") unless recovery.include?("QEMU_DURABLE_ROLE: u8 = 2;")

  rejected = run_rejected(
    writer_environment.merge("AGENT_KERNEL_QEMU_DURABLE_ROLE" => "invalid"),
    build_binary
  )
  fail_with("invalid role diagnostic missing") unless rejected.include?("writer or recovery")

  rejected = run_rejected(
    disabled_environment.merge(
      "CARGO_FEATURE_QEMU_DURABLE_PROOF" => "1",
      "AGENT_KERNEL_QEMU_DURABLE_ROLE" => "writer"
    ),
    build_binary
  )
  fail_with("partial profile diagnostic missing") unless
    rejected.include?("configured together")

  malformed_profile = File.join(directory, "malformed-profile")
  File.write(malformed_profile, File.read(profile).sub("tpm_name_hex=000b", "tpm_name_hex=000c"))
  rejected = run_rejected(
    writer_environment.merge("AGENT_KERNEL_QEMU_DURABLE_PROFILE" => malformed_profile),
    build_binary
  )
  fail_with("TPM Name diagnostic missing") unless rejected.include?("tpm_name_hex")
end

puts("qemu durable profile contract: ok")
