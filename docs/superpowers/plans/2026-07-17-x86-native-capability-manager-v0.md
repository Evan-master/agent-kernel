# X86 Native Capability Manager V0 Plan

- [x] Add and observe red tests for authenticated direct-child revocation,
  facade exposure, Agent Call decoding/authentication, and canonical replies.
- [x] Implement `sys_revoke_derived_capability` with source ownership,
  `Delegate`, root scope, direct-parent, and atomic event checks.
- [x] Add Agent Call operations 12 and 13 through the host-testable ABI and the
  public no-std facade only.
- [x] Rebuild the kind-4 Manager Capsule for create, derive, revoke, retire,
  result, and completion calls; bind its exact digest and transcript.
- [x] Raise capability capacity to twelve and require the two new ordered
  events, preserving the existing physical scheduling evidence.
- [x] Update bilingual README capability proof and deterministic counts.
- [x] Run formatting, full tests, Supervisor, no-std checks, scoped Clippy,
  debug/release QEMU, digest extraction, and release disassembly inspection.
- [x] Commit, merge, publish main, clean the feature branch, and close only this
  milestone while keeping the complete Agent Kernel goal active.
