# X86 Native Blocking Mailbox V0 Plan

- [x] Add failing core and facade receive-or-wait contracts.
- [x] Add failing waiter atomicity and send-wakeup capacity contracts.
- [x] Extend waiters with Signal/Mailbox kinds and mailbox wait events.
- [x] Implement atomic receive-or-wait and send-triggered wakeup.
- [x] Add BootedKernel waiter capacity without breaking default callers.
- [x] Add the retained x86 ReceiveMessage CPU type state.
- [x] Reverse the physical Worker schedule and bind wait/wake semantic states.
- [x] Update QEMU markers and the exact 82-event sequence.
- [x] Update README and milestone status.
- [x] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [x] Commit, merge, and publish main.
