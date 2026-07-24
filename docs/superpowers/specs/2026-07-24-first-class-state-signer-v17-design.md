# First-Class State Signer V17 Design

## Goal

V17 gives durable signing its own kernel identity and native image contract:

```text
AgentImageKind::StateSigner
        -> AgentEntryKind::StateSigner
        -> signed Package v3 / image kind 5
        -> durable preflight
        -> provider ABI
        -> Agent Calls 54 and 55
```

The State Signer no longer depends on the Supervisor role. A Supervisor keeps
Store administration and runtime admission authority. A State Signer receives
only the capabilities required for archive preparation, storage commit, and
its configured signing provider.

## Core Identity

Core adds `StateSigner` to `AgentImageKind` and `AgentEntryKind`.

The launch contract requires an exact kind match. Event archive encoding assigns
the new image kind a stable tag without changing existing tags.

Durable archive preflight accepts only a launched State Signer. Bootstrap,
Supervisor, Worker, Verifier, Fault Handler, and Driver entries fail before
storage access.

## Image Trust

`AgentImageKindScope` reserves one independent bit for State Signer images.
Signer records may trust this kind alone or include it in `all()`.

The x86 image format reserves numeric kind `5`. Capsule v1 and Package v2/v3
parsers accept the value, while metadata verification requires
`AgentImageKind::StateSigner`.

Existing kind values and signer-scope bits remain unchanged.

## Native Package

The repository provides an auditable State Signer entry source and a package
builder. The builder accepts:

- an externally supplied Agent-image signing key;
- an externally supplied provider object;
- non-secret root, storage, authority, generation, and signer policy values;
- an output path outside the source tree.

The provider object exports one fixed x86_64 System V symbol:

```text
state_signer_provider_sign(
    manifest_ptr: *const u8,   // 285 bytes
    signature_ptr: *mut u8,    // 64 bytes
    policy_generation: u64,
    agent: u64,
    task: u64,
    image: u64
) -> u32                       // 0 = success
```

The final three register arguments were added in V19. Existing providers may
ignore them. The kernel-mediated TPM provider uses them to authenticate Agent
Call 56 without issuing a duplicate Describe call.

The package entry:

1. authenticates its Agent Call context;
2. invokes `PrepareDurableArchive(54)`;
3. validates the staged request against immutable package policy;
4. invokes the provider;
5. verifies that only the signature window changed;
6. invokes `CommitDurableArchiveFromMemory(55)`;
7. submits a bounded result and completes its task.

The final ELF fixes code at `0x4000_0000_0000` and read-only policy at
`0x4000_0001_0000`. Its only non-empty loadable sections are `.text` and
`.rodata`; writable provider state fails the link. Package v3 therefore carries
two segments and zero relocation records.

The image key and provider object must grant no group or other access. The
builder rejects path, symbolic-link, and hard-link aliases, writes outside the
source tree, and atomically installs the package with mode `0600`.

The provider owns key access. Kernel and package-building code perform no
durable-state private-key operation.

## Failure Semantics

- A non-State-Signer entry cannot acquire a durable preflight.
- Image kind and signer scope mismatches fail before mapping or execution.
- Provider failure prevents call 55.
- A zero signature prevents call 55.
- Provider writes outside bytes `317..381` are detected before call 55.
- Package output never overwrites either supplied key or provider object.
- Provider objects with writable loadable state cannot produce a package.
- Build commands never print private-key material.

## Verification

- Core tests freeze kind matching, event tags, and exclusive preflight identity.
- Trust tests freeze the new scope bit and preserve all earlier bits.
- x86 tests freeze numeric kind `5` across Capsule v1 and signed Package v3.
- Package-builder tests use temporary development keys and provider objects.
- The generated package passes the existing structural and Ed25519 verifier.
- Workspace tests, strict Clippy, Supervisor replay, and bare target checks pass.

## Exclusions

V17 does not ship a production signing provider, tracked private key, TPM/HSM
driver, enabled ATA boot profile, QEMU storage image, or emulator execution.
