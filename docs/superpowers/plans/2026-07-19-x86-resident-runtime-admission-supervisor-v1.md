# X86 Resident Runtime Admission Supervisor V1 Plan

- [x] Audit scheduler wait/wake, Mailbox, admission broker, runtime ownership,
  and reclamation boundaries.
- [x] Define the resident lifecycle, Worker notification contract, physical
  ownership, exact event tail, capacities, and validation evidence.
- [x] Change the strict QEMU contract first and observe the expected failure.
- [x] Extend the Worker Capsule with an authenticated completion notification.
- [x] Extend the Supervisor Capsule with blocking wait and two receive/ack pairs.
- [x] Keep the Supervisor CPU and address space resident across both broker
  admissions and Worker execution.
- [x] Validate FIFO notifications, exact transcripts, three-owner reclamation,
  and deterministic capacities.
- [x] Run the full validation matrix, audit release artifacts, update bilingual
  README files, publish public `main`, and keep the complete Agent Kernel goal
  active.
