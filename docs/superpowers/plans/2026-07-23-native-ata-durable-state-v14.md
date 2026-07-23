# Native ATA Durable State V14 Plan

- [x] Audit the V13 HAL, capsule transaction, recovery, x86 port I/O, and bare
  target boundaries.
- [x] Freeze the native ATA profile, reserved range, task-file contract,
  staging ownership, recovery binding, and failure semantics.
- [x] Add failing ATA register and transport contract tests.
- [x] Implement bounded `IDENTIFY DEVICE`, LBA48 sector read/write, and cache
  flush transport.
- [x] Add failing A/B slot mapping and semantic backend contract tests.
- [x] Implement the caller-buffered ATA `DurableStateBackend`.
- [x] Exercise a complete signed V13 transaction and recovery through a
  sector-backed device double.
- [x] Cover transport timeout, device fault, write interruption, flush
  interruption, and generation conflict.
- [x] Pass formatting, workspace tests, strict Clippy, Supervisor replay, and
  `x86_64-unknown-none` compilation.
- [x] Update bilingual public documentation, commit, and publish V14.
