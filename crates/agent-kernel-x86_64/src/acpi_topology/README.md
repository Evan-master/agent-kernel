# ACPI Topology Boundary

This directory turns firmware-owned ACPI bytes into fixed-capacity x86_64
descriptors. Parsing stays allocation-free and performs no device MMIO.

## Discovery

```text
RSDP
  -> XSDT
  -> MADT  -> CPU and interrupt topology
  -> TPM2  -> TPM CRB descriptor
  -> DMAR  -> VT-d hardware units and PCI scopes
```

## Modules

| Module | Responsibility |
| --- | --- |
| `handler.rs` | bounded physical ACPI reads |
| `parser.rs` | RSDP, XSDT, and MADT parsing |
| `discover.rs` | unique-table selection and typed discovery errors |
| `types.rs` | immutable CPU and interrupt descriptors |
| `dmar.rs` | fixed-capacity DMAR model |
| `dmar/parser.rs` | strict DMAR, DRHD, scope, and PCI path parsing |

## Invariants

- Every SDT signature, declared length, checksum, and reserved field is checked.
- Duplicate required tables and capacity overflow fail closed.
- DRHD register bases are non-zero and page aligned.
- PCI requester selection prefers an exact endpoint scope before `include_all`.
- Firmware bytes never grant kernel authority by themselves.
