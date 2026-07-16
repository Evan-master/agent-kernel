# X86 Agent Page Fault Repair V1 Plan

- [x] Add and observe host red tests for the lazy page, seven-frame identity,
  not-present #PF(6), and packed semantic detail.
- [x] Allocate and retain one zeroed private lazy frame per Agent while keeping
  its initial PTE absent from both kernel and Agent translations.
- [x] Add exact one-way 4 KiB user/writable/NX leaf activation with no
  fault-time allocation.
- [x] Preserve call progress and normalized frame ownership in terminal fault
  objects, then consume the exact #PF(6) into a resumable CPU context.
- [x] Add a distinct recovered-fault runtime variant and bootstrap-authorized
  semantic recovery/queue flow.
- [x] Assemble and bind the five-path Fault Worker Capsule with exact fault and
  call offsets, SHA-256 digest, and lazy-byte readback.
- [x] Prove four ordered fault records, same-RIP retry, restart generation 3,
  retained run ticks, and terminal private byte `0x5a`.
- [x] Require eighteen dispatches, eight expiries, four faults, one repaired
  dispatch, four completed contexts, and empty queues.
- [x] Update the QEMU marker and exact event contract from 119 to 123 events.
- [x] Update README architecture, boot flow, runtime evidence, and non-goals.
- [x] Run formatting, full tests, Supervisor, no_std checks, scoped Clippy,
  debug/release QEMU, and release disassembly inspection.
- [x] Commit, merge, publish main, clean the worktree, and close the milestone.
