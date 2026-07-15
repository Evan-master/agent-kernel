# X86 Agent Address Space V0 Plan

- [x] Add host-failing contracts for the dedicated Agent P4 slot, aligned and
  distinct CR3 roots, control-bit preservation, and root classification.
- [x] Add QEMU-failing expectations for isolated page tables and two verified
  CR3 round trips.
- [x] Add the pure `address_space` architecture contract and export it from the
  host-testable x86_64 library.
- [x] Allocate a fresh Agent P4, inherit only supervisor kernel entries, and map
  Agent pages exclusively into that root.
- [x] Store the kernel/Agent CR3 pair in the CPU runtime and extend initial
  entry, resume, PIT, and Agent-call assembly with explicit switches.
- [x] Clear initial Agent GPR state and validate observed/current roots after
  each transition.
- [x] Publish the two QEMU proof markers without changing the 40-event semantic
  trace.
- [x] Run focused tests, workspace tests, no_std and bare-metal checks, Clippy,
  formatting, debug/release QEMU, and release disassembly inspection.
- [x] Update README and prepare the verified milestone commit.
- [ ] Publish the milestone when GitHub credentials are available.
