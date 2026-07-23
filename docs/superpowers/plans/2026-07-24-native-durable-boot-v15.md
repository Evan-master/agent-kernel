# Native Durable Boot V15 Plan

- [x] Audit boot configuration, Resource authority, Event archive calls, V13
  recovery, V14 ATA ownership, and call-data memory.
- [x] Freeze the Core recovery proof, recovered boot, request wire format,
  buffer ownership, ATA session, and failure semantics.
- [x] Add failing Core and facade recovery handoff tests.
- [x] Implement one-shot recovered-head import and Event sequence continuation.
- [x] Add failing recovered boot constructor tests.
- [x] Implement the recovered boot path without changing genesis behavior.
- [x] Add failing 384-byte signed request contract tests.
- [x] Implement canonical signed request decoding.
- [x] Add failing ATA boot-session and commit orchestration tests.
- [x] Implement ATA initialization, genesis/recovered binding, and signed
  transaction orchestration.
- [x] Expose an explicit disabled/ATA bare boot profile and compile both paths.
- [x] Pass formatting, workspace tests, strict Clippy, Supervisor replay, and
  `x86_64-unknown-none` compilation.
- [x] Update bilingual public documentation.
- [x] Commit and publish V15.
