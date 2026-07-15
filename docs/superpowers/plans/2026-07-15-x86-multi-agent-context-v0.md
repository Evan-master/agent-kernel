# X86 Multi-Agent Context V0 Plan

- [x] Add host-failing contracts for disjoint Agent memory identities and
  by-value saved privilege frames.
- [x] Add QEMU-failing expectations for a second preempted Agent, private signal
  isolation, multi-context restoration, and 53 semantic events.
- [x] Record root/code/signal/stack physical identity and prepare two private
  Agent address spaces at the same virtual layout.
- [x] Separate one-time CPU runtime installation from per-dispatch mailbox reset.
- [x] Copy validated RSP0 frames into each preempted context and resume from the
  owned copies.
- [x] Admit two delegated Worker tasks and validate A/B round-robin queue state
  through two quantum expiries and two yields.
- [x] Publish multi-Agent QEMU proof markers and update the expected 53-event
  trace without changing Driver semantics.
- [x] Run focused tests, workspace tests, supervisor flow, no_std checks,
  Clippy, formatting, debug/release QEMU, and release disassembly inspection.
- [x] Update README and prepare the verified local commit.
- [ ] Publish the branch when GitHub credentials permit.
