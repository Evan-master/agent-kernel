#!/usr/bin/env ruby
# frozen_string_literal: true

# Runs the V26 native ATA + TPM writer, kills QEMU after durable commit, then
# cold-boots a recovery kernel against the exact same raw disk.

require "digest"
require "fileutils"
require "open3"
require "optparse"
require "rbconfig"
require "timeout"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
TARGET = "x86_64-unknown-none"
TPM_HANDLE = "0x81010001"
ROOT_RESOURCE = 1
STORAGE_RESOURCE = 12
STATE_SIGNER_AGENT = 11
ARCHIVE_AUTHORITY = 36
STORAGE_AUTHORITY = 37
NONCE = "0xa17ce017"
THROUGH_SEQUENCE = 64
CALL_DATA_GENERATION = 1
POLICY_GENERATION = 1
RECOVERY_EVENT_LAST = 516
WRITER_MARKER = "AGENT_KERNEL_QEMU_DURABLE_COMMIT_OK"
RECOVERY_MARKER = "SUPERVISOR_HANDOFF_READY"
TIMEOUT_SECONDS = 180
SIGNER_DOMAIN = "AGENT-KERNEL-DURABLE-STATE-SIGNER-V2\0".b

@live_pids = []

def fail_with(message, log_path = nil)
  warn("QEMU durable power-loss proof failed: #{message}")
  if log_path && File.file?(log_path)
    warn("--- serial tail ---")
    warn(File.readlines(log_path).last(80).join)
  end
  raise RuntimeError, message
end

def command_path(name, candidates = [])
  configured = ENV[name]
  return configured if configured && File.executable?(configured)

  candidates.each do |candidate|
    return candidate if candidate.include?(File::SEPARATOR) && File.executable?(candidate)
    next if candidate.include?(File::SEPARATOR)

    output, status = Open3.capture2("which", candidate)
    return output.strip if status.success?
  end
  nil
end

def run!(environment, *command, chdir: ROOT)
  success = system(environment, *command, chdir: chdir)
  fail_with("command failed: #{command.join(' ')}") unless success
end

def capture!(environment, *command, chdir: ROOT)
  output, error, status = Open3.capture3(environment, *command, chdir: chdir)
  fail_with("#{command.join(' ')}\n#{output}#{error}") unless status.success?
  output
end

def parse_evidence(output)
  output.lines.each_with_object({}) do |line, values|
    key, value = line.strip.split("=", 2)
    values[key] = value if key && value
  end
end

def process_alive?(pid)
  Process.kill(0, pid)
  true
rescue Errno::ESRCH
  false
end

def stop_process(pid, signal = "TERM")
  return unless pid && process_alive?(pid)

  Process.kill(signal, pid)
  100.times do
    break unless process_alive?(pid)
    sleep(0.02)
  end
  Process.kill("KILL", pid) if process_alive?(pid)
rescue Errno::ESRCH
  nil
ensure
  @live_pids.delete(pid)
end

at_exit do
  @live_pids.reverse_each { |pid| stop_process(pid, "KILL") }
end

def wait_for_file(path)
  deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 10
  until File.exist?(path)
    fail_with("timed out waiting for #{path}") if
      Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
    sleep(0.02)
  end
end

def start_swtpm(swtpm, state, directory, mode)
  data_socket = File.join(directory, "data.sock")
  control_socket = File.join(directory, "ctrl.sock")
  pidfile = File.join(directory, "swtpm.pid")
  logfile = File.join(directory, "swtpm.log")
  [data_socket, control_socket, pidfile].each { |path| FileUtils.rm_f(path) }
  command = [
    swtpm,
    "socket",
    "--tpm2",
    "--tpmstate", "dir=#{state}",
    "--ctrl", "type=unixio,path=#{control_socket}",
    "--flags", "not-need-init,startup-none,disable-auto-shutdown",
    "--daemon",
    "--pid", "file=#{pidfile}",
    "--log", "file=#{logfile},level=20"
  ]
  command.concat(["--server", "type=unixio,path=#{data_socket},disconnect"]) if mode == :provision
  run!({}, *command)
  wait_for_file(pidfile)
  wait_for_file(mode == :provision ? data_socket : control_socket)
  pid = Integer(File.read(pidfile).strip, 10)
  @live_pids << pid
  {
    pid: pid,
    data_socket: data_socket,
    control_socket: control_socket,
    log: logfile
  }
end

def build_kernel_image(role, profile, directory, release, image_builder, rustc)
  target_root = ENV["AGENT_KERNEL_QEMU_DURABLE_TARGET_ROOT"]
  target_dir = if target_root
                 File.join(File.expand_path(target_root), role)
               else
                 File.join(directory, "#{role}-target")
               end
  FileUtils.mkdir_p(target_dir)
  cargo_profile = release ? "release" : "debug"
  environment = {
    "PATH" => "#{File.join(Dir.home, ".cargo/bin")}:#{ENV.fetch("PATH")}",
    "RUSTC" => rustc,
    "CARGO_TARGET_DIR" => target_dir,
    "AGENT_KERNEL_QEMU_DURABLE_ROLE" => role,
    "AGENT_KERNEL_QEMU_DURABLE_PROFILE" => profile
  }
  command = [
    "rustup", "run", "nightly", "cargo", "build",
    "-p", "agent-kernel-x86_64",
    "--features", "bare-metal,qemu-durable-proof",
    "--target", TARGET
  ]
  command << "--release" if release
  run!(environment, *command)
  kernel = File.join(target_dir, TARGET, cargo_profile, "agent-kernel-x86_64")
  image = File.join(directory, "#{role}-#{cargo_profile}-bios.img")
  run!({}, image_builder, kernel, image)
  image
end

def qemu_arguments(qemu, boot_image, disk, pci_output, serial_output, tpm_socket = nil)
  arguments = [
    qemu,
    "-smp", "2",
    "-drive", "if=ide,index=0,media=disk,format=raw,file=#{boot_image}",
    "-drive", "if=ide,index=1,media=disk,format=raw,file=#{disk}",
    "-chardev", "file,id=agent_pci_serial,path=#{pci_output}",
    "-device", "pci-serial,chardev=agent_pci_serial,id=agent-pci-serial,addr=0x4",
    "-serial", "file:#{serial_output}",
    "-display", "none",
    "-no-reboot",
    "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"
  ]
  if tpm_socket
    arguments.concat([
      "-chardev", "socket,id=chrtpm,path=#{tpm_socket}",
      "-tpmdev", "emulator,id=tpm0,chardev=chrtpm",
      "-device", "tpm-crb,tpmdev=tpm0"
    ])
  end
  arguments
end

def spawn_qemu(arguments, qemu_log)
  log = File.open(qemu_log, "wb")
  pid = Process.spawn(*arguments, out: log, err: log)
  log.close
  @live_pids << pid
  pid
end

def wait_for_marker(pid, serial, marker, qemu_log)
  deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + TIMEOUT_SECONDS
  loop do
    return if File.file?(serial) && File.read(serial).include?(marker)

    waited = Process.waitpid2(pid, Process::WNOHANG)
    if waited
      @live_pids.delete(pid)
      evidence_log = File.size?(serial) ? serial : qemu_log
      fail_with("QEMU exited before #{marker}: #{waited[1]}", evidence_log)
    end
    fail_with("timed out waiting for #{marker}", serial) if
      Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
    sleep(0.05)
  end
end

def wait_for_exit(pid, serial)
  deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + TIMEOUT_SECONDS
  loop do
    waited = Process.waitpid2(pid, Process::WNOHANG)
    if waited
      @live_pids.delete(pid)
      return waited[1]
    end
    fail_with("timed out waiting for recovery QEMU", serial) if
      Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
    sleep(0.05)
  end
end

options = { release: false }
OptionParser.new do |parser|
  parser.banner = "usage: scripts/run-qemu-durable-power-loss.rb [--release]"
  parser.on("--release", "build optimized writer and recovery kernels") do
    options[:release] = true
  end
  parser.on_tail("-h", "--help", "show this help") do
    puts(parser)
    exit
  end
end.parse!
fail_with("unexpected positional arguments") unless ARGV.empty?

swtpm = command_path("SWTPM", ["swtpm"])
swtpm_setup = command_path("SWTPM_SETUP", ["swtpm_setup"])
qemu = command_path("QEMU", ["qemu-system-x86_64"])
openssl = command_path(
  "OPENSSL",
  ["openssl", "/opt/homebrew/opt/openssl@3/bin/openssl", "/opt/homebrew/bin/openssl"]
)
ruby = command_path(
  "RUBY",
  ["/opt/homebrew/opt/ruby/bin/ruby", RbConfig.ruby]
)
[swtpm, swtpm_setup, qemu, openssl, ruby].each do |command|
  fail_with("required host tool is unavailable") unless command
end
rustc = capture!({}, "rustup", "which", "rustc", "--toolchain", "nightly").strip

Dir.mktmpdir("ak-v26-", "/private/tmp") do |directory|
  File.chmod(0o700, directory)
  state = File.join(directory, "tpm-state")
  FileUtils.mkdir_p(state, mode: 0o700)
  run!(
    {},
    swtpm_setup,
    "--tpm2",
    "--tpmstate", "dir://#{state}",
    "--pcr-banks", "sha256",
    "--overwrite"
  )

  provision_runtime = start_swtpm(swtpm, state, directory, :provision)
  provisioner = File.join(directory, "qemu-tpm-provision")
  run!(
    {},
    "go", "build", "-o", provisioner, ".",
    chdir: File.join(ROOT, "tools/qemu-tpm-provision")
  )
  durable_public_key = File.join(directory, "durable-state-public.pem")
  tpm_output = capture!(
    {},
    provisioner,
    "--socket", provision_runtime[:data_socket],
    "--handle", TPM_HANDLE,
    "--public-key-output", durable_public_key
  )
  tpm = parse_evidence(tpm_output)
  stop_process(provision_runtime[:pid])

  required_tpm = %w[
    tpm_handle tpm_name_hex state_public_key_sec1_hex
    pcr_selection_hex pcr_digest_hex policy_digest_hex
  ]
  fail_with("provisioner omitted public TPM evidence") unless
    required_tpm.all? { |key| tpm[key] && !tpm[key].empty? }
  signer_id = Digest::SHA256.hexdigest(
    SIGNER_DOMAIN + [2].pack("v") + [tpm.fetch("state_public_key_sec1_hex")].pack("H*")
  )

  image_key = File.join(directory, "state-signer-image-key.pem")
  run!({}, openssl, "genpkey", "-algorithm", "ED25519", "-out", image_key)
  File.chmod(0o600, image_key)
  package = File.join(directory, "state-signer.pkg")
  builder_output = capture!(
    {},
    ruby,
    File.join(ROOT, "scripts/build-state-signer-package.rb"),
    "--image-key", image_key,
    "--kernel-tpm-provider",
    "--output", package,
    "--nonce", NONCE,
    "--archive-authority", ARCHIVE_AUTHORITY.to_s,
    "--storage-authority", STORAGE_AUTHORITY.to_s,
    "--root", ROOT_RESOURCE.to_s,
    "--storage", STORAGE_RESOURCE.to_s,
    "--through-sequence", THROUGH_SEQUENCE.to_s,
    "--call-data-generation", CALL_DATA_GENERATION.to_s,
    "--policy-generation", POLICY_GENERATION.to_s,
    "--signature-algorithm", "ecdsa-p256-sha256",
    "--state-signer-id", signer_id
  )
  package_evidence = parse_evidence(builder_output)
  fail_with("StateSigner package omitted six return offsets") unless
    package_evidence.fetch("return_offsets", "").split(",").length == 6

  profile = File.join(directory, "durable.profile")
  File.write(
    profile,
    <<~PROFILE
      version=1
      root_resource=#{ROOT_RESOURCE}
      storage_resource=#{STORAGE_RESOURCE}
      base_lba=0
      policy_generation=#{POLICY_GENERATION}
      tpm_handle=#{tpm.fetch("tpm_handle")}
      tpm_command=sign-v184
      tpm_name_hex=#{tpm.fetch("tpm_name_hex")}
      state_public_key_sec1_hex=#{tpm.fetch("state_public_key_sec1_hex")}
      pcr_selection_hex=#{tpm.fetch("pcr_selection_hex")}
      pcr_digest_hex=#{tpm.fetch("pcr_digest_hex")}
      state_signer_package=#{package}
      state_signer_public_key_hex=#{package_evidence.fetch("public_key")}
      state_signer_agent=#{STATE_SIGNER_AGENT}
      archive_authority=#{ARCHIVE_AUTHORITY}
      storage_authority=#{STORAGE_AUTHORITY}
      state_signer_nonce=#{NONCE}
      through_sequence=#{THROUGH_SEQUENCE}
      call_data_generation=#{CALL_DATA_GENERATION}
      state_signer_return_offsets=#{package_evidence.fetch("return_offsets")}
    PROFILE
  )

  run!(
    { "PATH" => "#{File.join(Dir.home, ".cargo/bin")}:#{ENV.fetch("PATH")}" },
    "rustup", "run", "nightly", "cargo", "build", "-p", "agent-kernel-image"
  )
  image_builder = File.join(ROOT, "target/debug/agent-kernel-image")
  writer_image = build_kernel_image(
    "writer", profile, directory, options[:release], image_builder, rustc
  )
  recovery_image = build_kernel_image(
    "recovery", profile, directory, options[:release], image_builder, rustc
  )

  disk = File.join(directory, "durable-state.raw")
  File.open(disk, "wb") { |file| file.truncate(1024 * 1024) }
  writer_serial = File.join(directory, "writer.serial")
  writer_pci = File.join(directory, "writer.pci")
  writer_qemu_log = File.join(directory, "writer.qemu")
  writer_tpm = start_swtpm(swtpm, state, directory, :qemu)
  writer_pid = spawn_qemu(
    qemu_arguments(
      qemu,
      writer_image,
      disk,
      writer_pci,
      writer_serial,
      writer_tpm[:control_socket]
    ),
    writer_qemu_log
  )
  wait_for_marker(writer_pid, writer_serial, WRITER_MARKER, writer_qemu_log)
  writer_output = File.read(writer_serial)
  %w[
    AGENT_KERNEL_TPM_CRB_MMIO_OK
    AGENT_KERNEL_TPM_SIGNER_BINDING_OK
    AGENT_KERNEL_NATIVE_DURABLE_GENESIS_OK
    AGENT_KERNEL_NATIVE_DURABLE_RESOURCE_OK
    AGENT_KERNEL_DURABLE_ARCHIVE_PREPARED_OK
    AGENT_KERNEL_DURABLE_ARCHIVE_TPM_SIGNED_OK
    AGENT_KERNEL_DURABLE_ARCHIVE_COMMITTED_OK
    AGENT_KERNEL_NATIVE_STATE_SIGNER_OK
    AGENT_KERNEL_QEMU_DURABLE_COMMIT_OK
  ].each do |marker|
    fail_with("writer omitted #{marker}", writer_serial) unless writer_output.include?(marker)
  end
  Process.kill("KILL", writer_pid)
  Process.wait(writer_pid)
  @live_pids.delete(writer_pid)
  stop_process(writer_tpm[:pid], "KILL")

  inspector = capture!(
    {},
    ruby,
    File.join(ROOT, "scripts/inspect-qemu-durable-disk.rb"),
    "--disk", disk,
    "--public-key", durable_public_key,
    "--storage", STORAGE_RESOURCE.to_s,
    "--expect-generation", "1",
    "--expect-through-sequence", THROUGH_SEQUENCE.to_s
  )
  disk_evidence = parse_evidence(inspector)
  fail_with("disk actor does not match StateSigner") unless
    disk_evidence["actor"] == STATE_SIGNER_AGENT.to_s
  fail_with("disk archive authority does not match StateSigner") unless
    disk_evidence["archive_authority"] == ARCHIVE_AUTHORITY.to_s
  committed_disk_hash = Digest::SHA256.file(disk).hexdigest

  recovery_serial = File.join(directory, "recovery.serial")
  recovery_pci = File.join(directory, "recovery.pci")
  recovery_qemu_log = File.join(directory, "recovery.qemu")
  recovery_pid = spawn_qemu(
    qemu_arguments(qemu, recovery_image, disk, recovery_pci, recovery_serial),
    recovery_qemu_log
  )
  recovery_status = wait_for_exit(recovery_pid, recovery_serial)
  fail_with("recovery QEMU exit status is #{recovery_status}", recovery_serial) unless
    recovery_status.exitstatus == 33
  recovery_output = File.read(recovery_serial)
  %w[
    AGENT_KERNEL_NATIVE_DURABLE_RECOVERY_OK
    AGENT_KERNEL_NATIVE_DURABLE_RESOURCE_OK
    AGENT_KERNEL_NATIVE_EVENT_SNAPSHOT_HISTORY_OK
    AGENT_KERNEL_PCI_SERIAL_RING3_DRIVER_OK
    AGENT_KERNEL_SMP_HANDOFF_READY
    SUPERVISOR_HANDOFF_READY
  ].each do |marker|
    fail_with("recovery omitted #{marker}", recovery_serial) unless recovery_output.include?(marker)
  end
  %w[
    AGENT_KERNEL_TPM_CRB_MMIO_OK
    AGENT_KERNEL_DURABLE_ARCHIVE_COMMITTED_OK
    AGENT_KERNEL_QEMU_DURABLE_COMMIT_OK
  ].each do |marker|
    fail_with("recovery unexpectedly emitted #{marker}", recovery_serial) if
      recovery_output.include?(marker)
  end
  sequences = recovery_output.scan(/^event\[(\d+)\]/).flatten.map!(&:to_i)
  expected_sequences = (65..RECOVERY_EVENT_LAST).to_a
  fail_with(
    "recovery Event range is not contiguous 65..#{RECOVERY_EVENT_LAST}",
    recovery_serial
  ) unless
    sequences == expected_sequences
  fail_with("PCI serial output mismatch") unless File.binread(recovery_pci) == "P"
  fail_with("recovery mutated the durable disk") unless
    Digest::SHA256.file(disk).hexdigest == committed_disk_hash

  capture!(
    {},
    ruby,
    File.join(ROOT, "scripts/inspect-qemu-durable-disk.rb"),
    "--disk", disk,
    "--public-key", durable_public_key,
    "--storage", STORAGE_RESOURCE.to_s,
    "--expect-generation", "1",
    "--expect-through-sequence", THROUGH_SEQUENCE.to_s
  )

  puts("profile=#{options[:release] ? "release" : "debug"}")
  puts("writer_power_cut=SIGKILL")
  puts("generation=#{disk_evidence.fetch("generation")}")
  puts("through_sequence=#{disk_evidence.fetch("through_sequence")}")
  puts("recovery_event_range=65..#{RECOVERY_EVENT_LAST}")
  puts("pci_serial_byte=0x50")
  puts("disk_sha256=#{committed_disk_hash}")
  puts("AGENT_KERNEL_QEMU_DURABLE_POWER_LOSS_OK")
end
