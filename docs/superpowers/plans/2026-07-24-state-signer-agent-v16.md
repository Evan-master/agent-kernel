# State Signer Agent V16 Plan

- [x] Audit the V15 durable session, disabled legacy archive call, Agent Call
  memory boundary, native executor, and image trust chain.
- [x] Freeze Core preflight, one-shot session preparation, request staging,
  signer-provider policy, ABI IDs, and failure semantics.
- [x] Add failing Core durable preflight tests.
- [x] Implement the mutation-free preflight record and shared commit checks.
- [x] Add failing unsigned-request and ATA preparation tests.
- [x] Implement canonical staging and the ready/prepared/faulted session.
- [x] Add the `no_std` `agent-state-signer` crate with failing policy tests.
- [x] Prove the complete host preflight/sign/ATA/release/recovery flow.
- [x] Add failing Agent Call 54/55 decode, authentication, and reply tests.
- [x] Wire call-data staging, snapshotting, native handlers, and ATA session
  ownership into the bare executor.
- [x] Pass formatting, workspace tests, strict Clippy, Supervisor replay, and
  `x86_64-unknown-none` compilation.
- [x] Update bilingual public documentation, commit, and publish V16.
