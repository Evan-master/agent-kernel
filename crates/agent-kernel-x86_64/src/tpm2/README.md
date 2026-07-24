# x86_64 TPM 2.0 Boundary

This directory owns the allocation-free TPM path for the x86_64 machine
layer. Agent Kernel Core remains independent of TPM types and transport
details.

## Flow

```text
ACPI TPM2
  -> locality-zero CRB mapping
  -> bounded command transport
  -> ReadPublic provisioning check
  -> retained-manifest SHA-256
  -> SignDigest v185 | Sign v184
  -> verified low-S P-256 signature
```

## Modules

| Module | Responsibility |
| --- | --- |
| `acpi.rs` | strict `TPM2` SDT parsing |
| `crb.rs` | locality, ready, execute, cleanup, and poison state machine |
| `crb/descriptor.rs` | command/response data-window confinement |
| `crb/registers.rs` | PTP register offsets and masks |
| `mmio.rs` | volatile access to one boot-owned device page |
| `wire/` | fixed-capacity TPM command and response encoding |
| `public.rs` | immutable ECC signing-template verification |
| `signer.rs` | provisioned handle binding and manifest signing |
| `service.rs` | retained durable-request signing policy |

## Invariants

- Ring 3 receives no MMIO mapping, key blob, authorization secret, or raw TPM
  command channel.
- Only the manifest retained by durable archive preparation can reach the
  signing key.
- CRB identification requires active type 1, version 0 through 3, `CapCRB`,
  and zero reserved bits; RAM-buffer type 2 is rejected.
- Every poll is bounded.
- Command and response descriptors stay inside the locality-zero data window.
- Cleanup failure poisons the transport.
- Runtime transport, wire, and signature failures disable the signer instance.
- No software-key fallback exists.

## Tests

Contract tests live under `crates/agent-kernel-x86_64/tests/tpm2_*`. The
`native_tpm_durable_closed_loop` test carries a scripted TPM signature through
ATA commit, simulated power loss, and cold recovery.
