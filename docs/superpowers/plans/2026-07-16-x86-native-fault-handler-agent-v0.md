# X86 Native Fault Handler Agent V0 Plan

- [x] Add and observe red tests for the FaultHandler role, structured fault
  receive reply, blocking-handler wake, atomic capacity failure, and boot
  capacity forwarding.
- [x] Add first-class FaultHandler image/entry identity and x86 Capsule kind 3.
- [x] Make direct and policy fault routing atomically wake a native mailbox
  waiter while preserving no-waiter event compatibility.
- [x] Expose resource, fault, and intent IDs in bounded ReceiveMessage reply
  registers while rejecting unsupported payload fields.
- [x] Build and digest the five-call immutable Fault Handler Capsule.
- [x] Admit Agent 7 with private memory/CPU ownership and install the exact
  ExecutionTrap handler plus RouteToHandler policy.
- [x] Run the Handler to blocking receive, route #PF(6), validate its
  acknowledgement/result/transcript, and gate page repair on an opaque approval.
- [x] Require twenty-one dispatches, two waiting boundaries, five completions,
  the new markers, and exactly 149 ordered events.
- [x] Update README architecture, boot flow, runtime evidence, and non-goals.
- [x] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [x] Commit, merge, publish main, clean the branch, and close the milestone.
