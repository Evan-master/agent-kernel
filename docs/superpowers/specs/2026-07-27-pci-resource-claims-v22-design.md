# PCI Resource Claims V22 Design

**Status:** implemented and verified

## Objective

V22 converts one discovered PCI function into an auditable Agent Kernel
resource tree. The native architecture layer sizes and restores the function's
BARs, then the deterministic kernel creates the function and region resources,
initial capabilities, and physical Driver Endpoints in one transaction.

The resulting authority path is:

```text
PCI BDF
  -> restored BAR inventory
  -> function Resource
  -> BAR region Resources
  -> owner Capabilities
  -> immutable Driver Endpoints
```

Ring-3 Agents continue to name `ResourceId` values. Configuration selectors,
BAR registers, physical coordinates, and direct configuration writes remain
inside the native Ring-0 owner.

## Standards Basis

- [UEFI PI 1.9 PCI Host Bridge](https://uefi.org/specs/PI/1.9/V5_PCI_HostBridge.html)
- [UEFI 2.9A PCI Bus Support](https://uefi.org/specs/UEFI/2.9_A/14_Protocols_PCI_Bus_Support.html)
- [UEFI PI 1.8A PCI Configuration PPI](https://uefi.org/specs/PI/1.8A/V1_Additional_PPIs.html)
- [ACPI 6.6 PCI BAR target regions](https://uefi.org/specs/ACPI/6.6/05_ACPI_Software_Programming_Model.html)

V22 adopts already assigned segment-zero BAR addresses. Host-bridge window
allocation and relocation remain a later enumeration phase.

## Configuration Mutation Boundary

Read-only discovery remains available through `PciConfigAccess`. Mutation
requires a second explicit trait:

```rust
pub trait PciConfigWriteIo: PciConfigIo {
    fn write_data(&mut self, value: u32);
}

pub trait PciConfigMutationAccess: PciConfigAccess {
    fn write_u32(
        &mut self,
        address: PciFunctionAddress,
        register: PciConfigRegister,
        value: u32,
    );
}
```

`PciConfigMechanismOne` implements mutation only when its owned I/O value
implements `PciConfigWriteIo`. A configuration write always emits one validated
selector immediately followed by one 32-bit data write.

## BAR Probe Transaction

V22 probes normal endpoint headers (`Header Type 0`) with six BAR slots.

For one function:

1. Read the original command register.
2. Clear I/O decode, memory decode, and bus mastering.
3. Verify that the three command bits are clear.
4. Save each BAR before probing it.
5. Write all ones, read the implemented address mask, and restore the saved BAR.
6. Verify each restored BAR before continuing.
7. Restore the original command bits with zeroes in the status halfword.
8. Verify the restored command bits.

Writing zeroes to the status halfword avoids clearing write-one-to-clear status
bits. A 64-bit BAR is probed and restored as one low/high pair. The high slot is
never published as a second region.

Every exit path restores the currently touched BAR and the command register.
Restore failure takes precedence over a decode or shape error because the
hardware state can no longer be proven.

## BAR Model

```rust
pub enum PciBarKind {
    Io,
    MemoryBelowOneMegabyte { prefetchable: bool },
    Memory32 { prefetchable: bool },
    Memory64 { prefetchable: bool },
}

pub struct PciBar {
    index: PciBarIndex,
    kind: PciBarKind,
    base: u64,
    size: u64,
}
```

The probe rejects reserved memory types, malformed 64-bit pairs, non-power-of-
two sizes, misaligned assigned bases, and any restoration mismatch.
Unimplemented BARs are omitted. Implemented BARs with base zero remain visible
as unassigned and cannot enter a claim.

## Resource Catalog

The BSP builds a fixed-capacity catalog for every discovered endpoint function.
Each entry retains:

- the immutable `PciFunction`;
- its restored `PciBarSet`;
- whether every implemented BAR has an assigned nonzero base;
- the exact Driver Endpoint descriptor for each claimable BAR.

Bridge and CardBus headers remain in the V21 function inventory and are skipped
by the V22 endpoint catalog.

Catalog order follows the existing ascending BDF order. One deterministic
claim candidate is the first endpoint function with at least one BAR and no
unassigned BAR.

## Atomic Driver Resource Tree

The architecture-neutral core gains a bounded `DriverResourceTreeSpec` with one
root and up to six endpoint regions:

```text
function Resource (Device or Network)
  + owner Capability
  |
  +-- region Resource (Device)
      + owner Capability
      + Driver Endpoint
```

Creation preflights:

- active owner Agent;
- `Act` authority on the optional parent;
- `Delegate` in the requested owner operation set;
- supported root kind;
- one through six region descriptors;
- descriptor validity;
- overlap against existing endpoints and sibling regions;
- Resource, Capability, Endpoint, and Event capacity.

Only after every check succeeds does the core append:

```text
root:   ResourceCreated / CapabilityGranted
region: ResourceCreated / CapabilityGranted / DriverEndpointRegistered
```

The result preserves region slots, resource IDs, capability IDs, and endpoint
descriptors. This lets the architecture layer bind each BAR index to one exact
kernel authority without exposing physical coordinates through Agent command
payloads.

## Native Boot Integration

V22 reserves seven additional Resource and Capability slots in the x86_64
runtime:

```text
1 function root + 6 BAR regions
```

Agent workflow evidence continues to use its existing ten Resource and thirty
Capability slots. The hardware reserve is consumed only by the BSP after the
current Event-history proof. Claim events remain in the live suffix and enter
the next durable archive.

The Disabled storage profile retains an authorized read-only snapshot of Events
1 through 64 and releases nothing. The durable ATA profile still requires a
verified commit receipt before Core records a checkpoint and releases the
archived prefix.

The BSP retains both the BAR catalog and the resulting `PciFunctionClaim`.
Agent boot readiness requires a completed catalog. A failed probe, missing
fully assigned candidate, or failed atomic claim stops boot.

## Failure Semantics

- Configuration mutation requires the dedicated write trait.
- Decode-disable verification failure restores command state and aborts.
- Every BAR is restored before its decoded value is accepted.
- A restoration mismatch aborts and is never converted into a claim.
- Empty and overlapping resource trees change no kernel store or event.
- Capacity failures allocate no IDs and append no events.
- Unassigned BARs cannot become physical endpoints.
- Ring-3 code receives no configuration mutation primitive.

## Verification

V22 requires:

- exact selector and data-write ordering;
- 32-bit memory, I/O, below-1-MiB, and 64-bit BAR sizing;
- command/status write discipline;
- BAR and command restoration on success and failure;
- malformed BAR rejection;
- deterministic catalog ordering and candidate selection;
- atomic Resource/Capability/Endpoint tree creation;
- overlap, authority, capacity, and event-order failure tests;
- architecture claim-to-resource mapping tests;
- native boot retention and marker checks;
- workspace tests, strict Clippy, supervisor flow, package and image audits, and
  freestanding compilation.

## Verified Result

- workspace tests and doc tests pass;
- host and `x86_64-unknown-none` strict Clippy pass with warnings denied;
- native image, assembly, State Signer, and TPM provisioning audits pass;
- QEMU debug and release both reach `SUPERVISOR_HANDOFF_READY`;
- QEMU records the exact Event history `1..417`;
- Events `413..417` create one function root, one BAR region, two owner
  Capabilities, and one physical Driver Endpoint;
- `PCI_BAR_CATALOG_OK`, `PCI_FUNCTION_CLAIM_OK`, and
  `PCI_CAPABILITY_BOUNDARY_OK` prove the boot boundary.

## Deferred Work

- host-bridge I/O and memory window allocation;
- assigning zero-base BARs;
- bridge-window programming;
- expansion ROM BARs;
- MSI and MSI-X;
- DMA/IOMMU domains and bus-master enable;
- endpoint detach and hardware release;
- PCI Express ECAM and nonzero segments;
- controller-specific Network, Display, and USB drivers.
