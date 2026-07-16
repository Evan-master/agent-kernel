# X86 Native Mailbox IPC V0 Plan

- [x] Add failing BootedKernel message-capacity and Agent Call ABI contracts.
- [x] Add failing QEMU markers and the exact 79-event sequence.
- [x] Add canonical SendMessage, ReceiveMessage, and AcknowledgeMessage ABI
  request/reply encoding.
- [x] Add the trailing boot message capacity without breaking default callers.
- [x] Implement owned sender and receiver CPU type-state flows.
- [x] Implement mailbox semantic transitions through facade syscalls.
- [x] Replace both Worker Capsules with physical IPC sequences and bind their
  new digests and return offsets.
- [x] Prove the acknowledged message, Worker terminal states, Verifier target,
  and unchanged Driver flow in QEMU.
- [x] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [x] Update README, commit, and merge locally.
- [ ] Publish local main when GitHub credentials permit.
