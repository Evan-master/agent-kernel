# Native Driver Agent Call V24 Implementation Plan

**Status:** complete

## 1. Freeze Contracts

- [x] Define the Driver execution context and unchanged Task ABI.
- [x] Assign Agent Call operations `57..60`.
- [x] Define semantic-only ring-3 payloads and ring-0 physical authority.
- [x] Define native preemption, completion, and frame-reclamation ownership.

## 2. Drive With Tests

- [x] Add failing Driver image-kind loader tests.
- [x] Add failing Driver Agent Call decode and reply tests.
- [x] Add failing Task/Driver authentication-isolation tests.
- [x] Run the focused tests and confirm intended failures.

## 3. Implement Driver ABI

- [x] Add Driver image-kind wire value `6`.
- [x] Add Driver-scoped `AgentCallContext`.
- [x] Add inspect, acknowledge, submit, and complete requests and replies.
- [x] Extend transcript operation identities without changing `1..56`.

## 4. Execute In Ring 3

- [x] Add the auditable PCI serial Driver assembly and Capsule.
- [x] Admit the verified image from the reclaimed address-space pool.
- [x] Dispatch and resume the Driver CPU from Core Invocation state.
- [x] Execute the immutable HAL command and return its typed result.
- [x] Complete the Invocation and reclaim the exact address space.

## 5. Integrate Evidence

- [x] Replace ring-0 command selection in the V23 flow.
- [x] Prove the exact five-call ring-3 transcript.
- [x] Freeze the new exact Event suffix and QEMU markers.
- [x] Update image audit counts and release-ELF uniqueness checks.
- [x] Update the bilingual README and V24 status documents.

## 6. Verify And Publish

- [x] Run focused and full workspace tests.
- [x] Run host and freestanding strict Clippy.
- [x] Run supervisor, shell, package, and image audits.
- [x] Run QEMU debug and release with exact Event and `0x50` proofs.
- [x] Commit V24, push its branch, and verify the remote SHA.
