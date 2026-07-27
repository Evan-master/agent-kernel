# QEMU ATA Power-Loss V26 Plan

## 1. Freeze Recovery-Relative Event Semantics

- [x] Add a read-only boot Event base field to the Agent signal page.
- [x] Make the admission Supervisor request `base + 63`.
- [x] Rebuild the immutable Supervisor Capsule and return-offset table.
- [x] Generalize retained-snapshot evidence across genesis and recovery.
- [x] Preserve the exact V25 default transcript.

## 2. Add Generated Durable Profiles

- [x] Add the `qemu-durable-proof` feature.
- [x] Generate a disabled `OUT_DIR` profile by default.
- [x] Validate complete writer and recovery environment input.
- [x] Construct matching ATA and durable signer records.
- [x] Construct the TPM CRB profile only for the writer.
- [x] Embed the generated State Signer Package only for the writer.

## 3. Add the Native State Signer Flow

- [x] Delay Device Resource binding until Runtime Admission completes.
- [x] Trust the generated State Signer image key with exact kind scope.
- [x] Reuse Agent 11 for the State Signer task and least-authority Capabilities.
- [x] Verify and load the signed two-segment Package v3.
- [x] Execute Calls 54, 56, and 55 through the native runtime.
- [x] Retain the released Event prefix for final writer evidence.
- [x] Reclaim and verify every State Signer address-space frame.

## 4. Build the Emulator Harness

- [x] Provision a temporary `swtpm` P-256 PCR-policy key.
- [x] Generate the public kernel profile and ephemeral signed Agent package.
- [x] Create a dedicated raw ATA image.
- [x] Build separate writer and recovery BIOS images.
- [x] Attach boot master, durable slave, TPM CRB, and PCI serial devices.
- [x] Kill the writer QEMU process after the durable commit marker.
- [x] Restart the recovery image with the unchanged durable disk.

## 5. Inspect and Prove Durable Media

- [x] Add a strict dual-slot disk inspector.
- [x] Test valid, torn, corrupt, stale, and signature-invalid fixtures.
- [x] Require generation 1 and through sequence 64 after writer death.
- [x] Hash the durable image before and after recovery.
- [x] Require recovery marker and first new Event 65.
- [x] Require an ordered contiguous recovery transcript through Event 516.

## 6. Close the Milestone

- [x] Run focused contract tests.
- [x] Run workspace tests and strict host/bare Clippy.
- [x] Run formatting, Supervisor replay, package, and assembly audits.
- [x] Run default QEMU debug and release regressions.
- [x] Run the V26 abrupt-power proof in debug and release.
- [x] Update English and Chinese README status and roadmap.
- [x] Commit, push, and verify the remote branch SHA.
