# SMP Runtime V12 Plan

- [x] Audit the single-CPU context, privilege, exception, interrupt, scheduler,
  and page-table mutation paths.
- [x] Freeze CPU identity, fixed capacity, lifecycle, ACPI, APIC, AP startup,
  per-CPU runtime, synchronization, shootdown, and QEMU proof contracts.
- [x] Add failing CPU mask, topology, and lifecycle contracts.
- [x] Implement the allocator-free CPU registry and deterministic BSP remap.
- [x] Add failing TLB shootdown coordinator contracts.
- [x] Implement generation-bound requests, target snapshots, acknowledgement,
  completion, and frame-reuse quarantine contracts.
- [x] Add the IRQ-safe ticket lock and publication tests.
- [x] Integrate allocator-free ACPI table discovery and MADT fixture tests.
- [x] Add Local APIC and I/O APIC register contracts.
- [ ] Replace legacy PIC routing in the SMP profile.
- [x] Replace global transition mailboxes and privilege tables with per-CPU
  runtime slots.
- [x] Add the AP startup trampoline and bounded startup handshake.
- [ ] Execute a native Agent on an AP and wire the TLB shootdown IPI path.
- [x] Pass SMP host contracts, freestanding Clippy, linking, trampoline layout,
  relocation, and formatting gates.
- [ ] Add guard pages to per-CPU privilege stacks.
- [ ] Pass debug and release dual-CPU QEMU gates.
- [ ] Update bilingual public documentation, commit, and publish the V12 branch.
