# Native PCI Inventory V21 Design

**Status:** implemented and verified at the scripted configuration boundary

## Objective

V21 gives the native x86_64 kernel a bounded inventory of PCI functions before
Agent execution begins. The inventory is the common discovery foundation for
network, display, storage, and USB controller drivers.

The architecture boundary owns configuration-space access. Ring-3 Agents
receive no configuration address, configuration data, BAR, or raw port
authority.

## Standards Basis

- [UEFI PI 1.8A glossary](https://uefi.org/specs/PI/1.8A/V5_Introduction.html)
- [UEFI PI 1.8A PCI Configuration PPI](https://uefi.org/specs/PI/1.8A/V1_Additional_PPIs.html)
- [PCI Code and ID Assignment 1.17](https://pcisig.com/PCIExpress/Spec/Base/CodeandIDAssignment_1.17)

V21 uses PCI Configuration Access Mechanism 1 on x86_64:

```text
CONFIG_ADDRESS  I/O port 0x0cf8  32-bit
CONFIG_DATA     I/O port 0x0cfc  32-bit
```

The address latch contains:

```text
31       enable
23:16    bus
15:11    device
10:8     function
7:2      DWORD register
1:0      zero
```

## Scope

V21 provides:

- validated segment-zero bus/device/function addresses;
- validated DWORD-aligned registers in the first 256 configuration bytes;
- an exclusively owned Configuration Mechanism 1 adapter;
- a non-destructive address-latch probe with state restoration;
- read-only discovery across 256 buses and 32 devices;
- function 1 through 7 probing only when function 0 declares multifunction;
- fixed-capacity immutable function records;
- deterministic ascending bus/device/function order;
- raw class tuple helpers for network, display, and USB controllers;
- retained inventory ownership in the BSP bootstrap layer;
- host contract tests and freestanding compilation.

V21 does not size or assign BARs, enable bus mastering, configure MSI/MSI-X,
write device configuration, claim a function for a Driver Agent, or support
PCI Express ECAM and nonzero segments. Those operations require separate
authority and rollback contracts.

## Typed Configuration Boundary

```rust
pub struct PciFunctionAddress {
    bus: u8,
    device: u8,
    function: u8,
}

pub struct PciConfigRegister(u8);

pub trait PciConfigIo {
    fn read_address(&mut self) -> u32;
    fn write_address(&mut self, value: u32);
    fn read_data(&mut self) -> u32;
}

pub trait PciConfigAccess {
    fn read_u32(
        &mut self,
        address: PciFunctionAddress,
        register: PciConfigRegister,
    ) -> u32;
}
```

`PciConfigMechanismOne` owns its `PciConfigIo` value. A read writes one
validated selector and immediately consumes the selected data DWORD. The
native implementation exposes only the fixed PCI ports through this trait.

The probe saves the existing address latch, writes one enabled aligned
selector, reads it back, and restores the saved value on both success and
failure.

## Function Record

Each present function contributes one immutable record:

| Field | Source |
| --- | --- |
| address | scan coordinates |
| vendor and device ID | offset `0x00` |
| command and status | offset `0x04` |
| revision and class tuple | offset `0x08` |
| header type and multifunction flag | offset `0x0c` |

Vendor ID `0xffff` means absent. Unknown vendor IDs, class codes, and header
types remain visible as raw values.

The class tuple helpers match:

```text
network   base class 0x02
display   base class 0x03
USB       base class 0x0c / subclass 0x03
```

## Discovery

The scanner visits addresses in ascending BDF order:

```text
bus 0..255
  device 0..31
    function 0
    function 1..7 when multifunction
```

Every present function must fit in the caller-selected inventory capacity.
Overflow returns `InventoryFull` without publishing a partial inventory.
An all-absent scan returns `NoFunctions`.

The bare-metal binary retains a 256-entry inventory in `SmpBootstrap`.
Discovery runs on the BSP while interrupts remain disabled and before
application processors start.

## Authority Boundary

```text
NativePortIo
  -> fixed PCI address/data ports
  -> PciConfigMechanismOne
  -> PciInventory
  -> SmpBootstrap ownership
```

The inventory performs no device mutation. Future Driver Agent work will bind
a selected function to a kernel `ResourceId`, size its BARs under a reversible
transaction, and install a capability-authorized endpoint.

## Failure Semantics

- Invalid device, function, or register values cannot be constructed.
- Address-latch probe failure restores the previous latch and stops boot.
- Inventory overflow stops boot before any partial device authority is used.
- An empty inventory stops boot.
- Discovery writes no configuration-data register.
- No fallback grants raw configuration access to an Agent.

## Verification

V21 requires:

- exact selector encoding tests;
- address and register boundary tests;
- address-latch restoration tests;
- absent, single-function, and multifunction fixtures;
- cross-bus deterministic ordering;
- exact ID, command/status, class, and header decoding;
- network, display, and USB classification tests;
- empty and capacity failure tests;
- native bootstrap retention;
- workspace formatting, tests, strict Clippy, supervisor, package, image, and
  freestanding gates.

## Implementation Record

```text
types        validated BDF / aligned common-header register / raw class tuple
transport    exclusive 32-bit 0x0cf8 selector + 0x0cfc data reads
discovery    256 buses / multifunction-aware / stable BDF ordering
inventory    256 fixed records retained by SmpBootstrap
boot gate    latch probe + nonempty inventory + readiness requirement
evidence     focused contracts + workspace + strict bare-metal Clippy
```

Physical PCI execution remains a machine-validation step after the scripted
configuration model and freestanding binary proof.
