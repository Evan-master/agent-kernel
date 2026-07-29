#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "open3"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
SOURCE = File.join(
  ROOT,
  "crates/agent-kernel-x86_64/src/boot_agent_images/network_driver.S"
)
OUTPUT = File.join(
  ROOT,
  "crates/agent-kernel-x86_64/src/boot_agent_images/network_driver.rs"
)

def command_path(environment_name, candidates)
  configured = ENV[environment_name]
  return configured if configured && File.executable?(configured)

  candidates.find { |candidate| File.executable?(candidate) }
end

def run(*command)
  output, error, status = Open3.capture3(*command)
  abort("#{command.first} failed:\n#{output}#{error}") unless status.success?
  output
end

def rust_bytes(bytes)
  bytes.bytes.each_slice(16).map do |row|
    "    #{row.map { |byte| format("0x%02x", byte) }.join(", ")},"
  end.join("\n")
end

clang = command_path(
  "CLANG",
  ["/opt/homebrew/opt/llvm/bin/clang", "/usr/local/opt/llvm/bin/clang"]
)
objcopy = command_path(
  "LLVM_OBJCOPY",
  ["/opt/homebrew/opt/llvm/bin/llvm-objcopy", "/usr/local/opt/llvm/bin/llvm-objcopy"]
)
nm = command_path(
  "LLVM_NM",
  ["/opt/homebrew/opt/llvm/bin/llvm-nm", "/usr/local/opt/llvm/bin/llvm-nm"]
)
abort("LLVM clang, objcopy, and nm are required") unless clang && objcopy && nm

Dir.mktmpdir("agent-kernel-network-driver") do |directory|
  object = File.join(directory, "network-driver.o")
  binary = File.join(directory, "network-driver.bin")
  run(clang, "--target=x86_64-unknown-none", "-c", SOURCE, "-o", object)
  run(objcopy, "--only-section=.text", "-O", "binary", object, binary)
  code = File.binread(binary)
  symbols = run(nm, "--defined-only", "--numeric-sort", object).lines.to_h do |line|
    address, _kind, name = line.split
    [name, address.to_i(16)]
  end
  return_offsets = %w[
    network_driver_describe_return
    network_driver_inspect_return
    network_driver_acknowledge_return
    network_driver_submit_return
    network_driver_completion_return
  ].map { |name| symbols.fetch(name) }
  header = +"AGNTIMG\0".b
  header << [1, 1, 6, 0, 1, 1].pack("v6")
  header << [symbols.fetch("network_driver_entry"), code.bytesize, 0].pack("V3")
  capsule = header + code
  digest = Digest::SHA256.digest(capsule)

  source = <<~RUST
    //! Native Network Driver Capsule and exact Ring-3 transcript contract.
    //!
    //! Generated from `network_driver.S` by
    //! `scripts/regenerate-network-driver.rb`.

    use agent_kernel_core::AgentImageDigest;
    use agent_kernel_x86_64::agent_call::AgentCallOperation;

    const NONCE: u64 = 0xd81c_e030;
    const OPERATIONS: [AgentCallOperation; 5] = [
        AgentCallOperation::DescribeContext,
        AgentCallOperation::InspectDriverInvocation,
        AgentCallOperation::AcknowledgeDeviceEvent,
        AgentCallOperation::SubmitDriverCommand,
        AgentCallOperation::CompleteDriverInvocation,
    ];
    const RETURN_OFFSETS: [u32; 5] = #{return_offsets.inspect};

    static CAPSULE: [u8; #{capsule.bytesize}] = [
    #{rust_bytes(capsule)}
    ];

    const DIGEST: AgentImageDigest = AgentImageDigest::new([
    #{rust_bytes(digest)}
    ]);

    #[derive(Copy, Clone)]
    pub(crate) struct BootNetworkDriverImage;

    impl BootNetworkDriverImage {
        pub(crate) const fn bytes(self) -> &'static [u8] {
            &CAPSULE
        }

        pub(crate) const fn digest(self) -> AgentImageDigest {
            DIGEST
        }

        pub(crate) const fn nonce(self) -> u64 {
            NONCE
        }

        pub(crate) const fn expected_operations(self) -> [AgentCallOperation; 5] {
            OPERATIONS
        }

        pub(crate) const fn expected_return_offsets(self) -> [u32; 5] {
            RETURN_OFFSETS
        }
    }

    pub(crate) const fn network_driver() -> BootNetworkDriverImage {
        BootNetworkDriverImage
    }
  RUST
  File.write(OUTPUT, source)
end
