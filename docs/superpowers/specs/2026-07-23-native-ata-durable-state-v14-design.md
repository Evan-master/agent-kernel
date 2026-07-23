# Native ATA Durable State V14 Design

## Goal

V14 binds the V13 signed durable-state protocol to a native x86 block device.
The architecture backend translates semantic slot operations into bounded ATA
PIO commands while preserving the existing Core, facade, HAL, capsule, and
recovery contracts.

The first hardware profile deliberately stays narrow:

- one dedicated ATA device in legacy task-file mode;
- 512-byte logical sectors;
- LBA48 `READ SECTORS EXT`, `WRITE SECTORS EXT`, and `FLUSH CACHE EXT`;
- one explicitly reserved, aligned range containing slots `A` and `B`;
- polling with finite budgets and precise transport failures;
- caller-owned fixed-capacity staging memory;
- no filesystem, partition parser, DMA, allocator, or background worker.

## Layer Ownership

| Layer | V14 responsibility |
| :--- | :--- |
| `agent-kernel-core` | unchanged archive authority, proposal, receipt, and release rules |
| `agent-kernel` | unchanged syscall and verifier boundary |
| `agent-kernel-hal` | unchanged semantic write, flush, and readback contract |
| `agent-kernel-x86_64::ata` | ATA identity, task-file transport, sector I/O, slot mapping, and flush epochs |
| `agent-supervisor` | existing signed transaction and crash model |

ATA registers, commands, LBAs, and sector sizes remain architecture-private.
Agents continue to address a storage `ResourceId` and never receive raw device
authority.

## Device Profile

The transport owns one command block base, one control block base, one selected
device, and one nonzero polling budget. Construction validates that the command
register span fits in `u16`.

Initialization issues `IDENTIFY DEVICE` and accepts a device only when:

- the device responds with an ATA signature;
- LBA48 is advertised;
- the reported LBA48 capacity is nonzero;
- the logical sector size is exactly 512 bytes.

Each data command transfers exactly one sector. A small command size keeps every
loop bounded, simplifies fault attribution, and avoids the special zero sector
count encoding used for 65,536-sector ATA transfers.

### Frozen Task-File Contract

| Register | Command-block offset |
| :--- | ---: |
| Data | `0` |
| Error / Features | `1` |
| Sector Count | `2` |
| LBA Low | `3` |
| LBA Mid | `4` |
| LBA High | `5` |
| Device | `6` |
| Status / Command | `7` |

The alternate-status register is at the configured control-block base. Device
selection is followed by four alternate-status reads. LBA48 task-file values
are written high-order bytes first, then low-order bytes. The sector count is
always one.

| Command | Opcode |
| :--- | ---: |
| `IDENTIFY DEVICE` | `0xEC` |
| `READ SECTORS EXT` | `0x24` |
| `WRITE SECTORS EXT` | `0x34` |
| `FLUSH CACHE EXT` | `0xEA` |

Polling treats `BSY`, `DRQ`, `DF`, and `ERR` as protocol state. Zero and `0xFF`
status values report an absent device. Every wait consumes a finite poll budget.
Timeout, device fault, unsupported identity, and capacity errors remain distinct
at the transport boundary.

## Reserved Slot Layout

One durable storage binding contains:

```text
base_lba
  + 0 .. +127   slot A (64 KiB)
  +128 .. +255  slot B (64 KiB)
```

The base LBA is aligned to 128 sectors and the complete 256-sector range must
fit within the identified device capacity. The storage `ResourceId` is nonzero.
Platform setup owns the stronger guarantee that this range belongs exclusively
to the selected Resource. The initial runtime profile uses a dedicated second
disk so boot payloads and durable state cannot overlap.

## Semantic Write Translation

The backend borrows one 64 KiB staging buffer from its trusted caller. This
avoids heap allocation and avoids hiding a large stack object inside
construction.

### Prepared Header

1. Validate the bound storage Resource, recovered head, expected next
   generation, target parity, and idle phase.
2. Zero staging and copy the 64-byte header.
3. Write the first sector containing the header.
4. Write the final sector with a zero commit footer.
5. Mark the header dirty.

The next HAL flush issues `FLUSH CACHE EXT`. Only a successful device completion
advances the backend epoch and marks the header flushed.

### Body

1. Require the same target and a flushed header.
2. Copy the bounded body into staging.
3. Write all 128 sectors, including the prepared header and zero footer.
4. Mark the body dirty.

The next flush persists the complete prepared capsule. V13 then reads and
verifies the full slot before it can submit a commit footer.

### Commit Footer

1. Require the same target and a flushed body.
2. Copy the 64-byte footer into staging.
3. Write the final sector, preserving the body tail already in staging.
4. Mark the footer dirty.

The final flush persists the commit marker, advances the active generation, and
returns a monotonic nonzero epoch. A complete-slot readback still goes through
128 native sector reads and V13 cryptographic verification.

## Recovery Binding

A freshly constructed backend starts with an unbound archive head. It permits
slot readback so V13 recovery can inspect both slots, but rejects every write
and flush.

The device has no persisted V13 flush counter. Before the first flush in a new
boot, readback reports recovery-observation epoch `1`: bytes visible after
power-on already crossed a durability boundary in an earlier boot. Current-boot
flushes remain monotonic from epoch `1`. Recovery uses this value only after
capsule, digest, footer, and signature verification.

After recovery, trusted setup binds exactly one baseline:

- `Genesis`, with expected next generation `1`; or
- `Recovered(generation)`, with expected next generation
  `generation + 1`.

The baseline can be bound once. Overflow rejects further commits. A successful
footer flush advances the in-memory baseline. Generation parity selects the
opposite slot naturally.

## Failure Semantics

- Invalid resources, generations, phase order, and buffers fail before I/O.
- ATA timeouts map to HAL `Interrupted`.
- ATA device, identity, and capacity failures map to HAL `StorageFault`.
- Failed writes leave the phase unchanged.
- Failed flushes leave the epoch and active generation unchanged.
- Failed readback reports no successful byte count.
- Epoch exhaustion stops commits before returning an ambiguous receipt.
- A reboot discards phase and epoch state; signed dual-slot recovery rebuilds
  the trusted head from device bytes.

The V13 capsule parser remains the final authority for prepared and committed
state. ATA command completion alone never authorizes Event release.

## Verification Gates

- Register-contract tests freeze LBA48 byte order, command opcodes, data width,
  status polling, timeout, and error behavior.
- Backend tests freeze A/B LBA mapping and semantic write-to-sector translation.
- Full V13 transaction tests run through a sector-backed device double and
  recover the resulting signed head.
- Injected sector and flush failures preserve phase, epoch, and active head.
- Workspace tests, strict Clippy, formatting, and
  `x86_64-unknown-none` compilation pass.
- Native QEMU execution remains deferred until the user enables emulator runs
  and a dedicated ATA image is attached.

## Exclusions

V14 excludes ATA DMA, AHCI, NVMe, ATAPI, hot-plug, multiple devices, 4 KiB
logical sectors, partition discovery, wear management, encryption, RAID,
filesystems, and automatic boot-disk range selection.

## Normative References

- INCITS T13, ATA8-ACS and ACS-2 working drafts:
  <https://www.t13.org/project-working-drafts>
- UEFI 2.10A, ATA Pass Thru command and status block model:
  <https://uefi.org/specs/UEFI/2.10_A/13_Protocols_Media_Access.html>
