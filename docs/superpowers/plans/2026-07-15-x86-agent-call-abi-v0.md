# X86 Agent Call ABI V0 Plan

- [x] Add host-failing contracts for the register envelope, operation parsing,
  DescribeContext reply encoding, and trusted Yield context matching.
- [x] Add QEMU-failing expectations for ABI decode/return markers and preserve
  the exact 55-event semantic trace.
- [x] Implement the no_std Agent Call ABI types and explicit decode errors.
- [x] Bind each prepared CPU context to its admitted Agent/Task/Image tuple.
- [x] Split call capture from operation handling and remove assembly's implicit
  yield classification.
- [x] Resume an owned DescribeContext reply frame before accepting Yield.
- [x] Replace both boot Capsule payloads, digests, nonces, and expected call
  return offsets.
- [x] Prove two calls and four CR3 transitions for both isolated Workers.
- [x] Run focused tests, workspace tests, supervisor flow, no_std checks,
  Clippy, formatting, debug/release QEMU, and release disassembly inspection.
- [x] Update README and prepare the verified local commit.
- [ ] Publish the branch when GitHub credentials permit.
