# X86 Native Blocking Mailbox V0 Plan

- [ ] Add failing core and facade receive-or-wait contracts.
- [ ] Add failing waiter atomicity and send-wakeup capacity contracts.
- [ ] Extend waiters with Signal/Mailbox kinds and mailbox wait events.
- [ ] Implement atomic receive-or-wait and send-triggered wakeup.
- [ ] Add BootedKernel waiter capacity without breaking default callers.
- [ ] Add the retained x86 ReceiveMessage CPU type state.
- [ ] Reverse the physical Worker schedule and bind wait/wake semantic states.
- [ ] Update QEMU markers and the exact 82-event sequence.
- [ ] Update README and milestone status.
- [ ] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [ ] Commit, merge, and publish main.
