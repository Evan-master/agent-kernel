# Segmented Agent Package V9 Design

Status: Implemented and verified on 2026-07-21

## Objective

Replace the flat executable-only image ceiling with a deterministic Agent
Package that carries separately protected code and immutable data. V9 adds a
canonical Package v2 format, bounded base relocations, exact physical ownership
for both segments, and a native Resource Manager proof that consumes relocated
read-only data at ring 3.

Capsule v1 remains accepted while the boot inventory migrates. Package v2 is
the forward format and all new package features are defined only for v2.

## Package v2 Header

Package v2 keeps the eight-byte `AGNTIMG\0` magic and uses format version `2`.
All integers are little-endian.

| Offset | Bytes | Field | Canonical value |
| ---: | ---: | :--- | :--- |
| `0` | 8 | magic | `AGNTIMG\0` |
| `8` | 2 | format version | `2` |
| `10` | 2 | architecture | `1` / x86_64 |
| `12` | 2 | image kind | `1..4` |
| `14` | 2 | package flags | `0` |
| `16` | 2 | ABI version | nonzero |
| `18` | 2 | entry version | nonzero |
| `20` | 2 | entry segment | `0` / code |
| `22` | 2 | reserved | `0` |
| `24` | 4 | entry offset | inside code |
| `28` | 2 | segment count | `2` |
| `30` | 2 | relocation count | `0..64` |
| `32` | 4 | segment table offset | `48` |
| `36` | 4 | relocation table offset | `96` |
| `40` | 4 | package length | exact byte length |
| `44` | 4 | reserved | `0` |

The segment table contains two 24-byte descriptors in canonical order.

| Index | Kind | Flags | Alignment | File bytes | Memory bytes |
| ---: | :--- | :--- | ---: | :--- | :--- |
| `0` | code | `R + X` | `4096` | `1..65,536` | exact file length |
| `1` | rodata | `R` | `4096` | `1..65,536` | exact file length |

Each descriptor stores kind, flags, alignment, file offset, file length, memory
length, and a zero reserved word. Payload bytes are packed directly after the
relocation table: code first, then rodata. Overlap, gaps, trailing bytes,
unknown flags, and nonzero reserved values are rejected.

## Relocation Contract

Each 24-byte relocation has:

| Field | V9 rule |
| :--- | :--- |
| target segment | code / index `0` |
| kind | `ABS64` / value `1` |
| symbol segment | rodata / index `1` |
| target offset | eight bytes wholly inside one code page |
| addend | nonnegative byte offset inside declared rodata |
| reserved fields | zero |

Relocations are sorted by target offset and may not overlap. Every target word
in the immutable package is zero. The loader verifies the complete package
digest first, copies code and rodata into private zeroed frames, then replaces
each target word with `rodata_virtual_base + addend`. No writable alias remains
reachable through the Agent page table.

## Fixed Virtual Layout

| Region | Start | Pages | Ring-3 policy |
| :--- | ---: | ---: | :--- |
| Code | `0x0000400000000000` | 16 | user RX, active prefix only |
| Rodata | `0x0000400000010000` | 16 | user R, NX, active prefix only |
| Signal | `0x0000400000020000` | 1 | user R, NX |
| Guard | `0x0000400000021000` | 1 | unmapped |
| Stack | `0x0000400000022000` | 4 | user RW, NX |
| Lazy data | `0x0000400000026000` | 1 | initially unmapped |
| Runtime page | `0x0000400000027000` | 1 | initially unmapped |
| Runtime region | `0x0000400000028000` | 8 | initially unmapped |
| Call data | `0x0000400000030000` | 1 | user RW, NX |

Every unused code and rodata page remains unmapped. The complete layout remains
inside dedicated P4 slot 128. Capsule v1 machine code is rebuilt because all
fixed non-image addresses move by `0x10000`.

## Physical Identity

Frame order is canonical:

~~~text
private page tables  4
active code          1..16
active rodata        0..16
signal               1
stack                4
lazy-data backing    1
call data            1
owned total          12..43
~~~

Only active segment prefixes participate in alias checks, clearing, transfer,
reclamation, and reuse. Inactive bounded storage is canonical zero.

The initial six-Agent inventory contains five V1 one-page identities and one
V2 Resource Manager identity with five code pages and one rodata page:

~~~text
5 * 12 + 1 * 17 = 77 frames
~~~

## Native Proof

Resource Manager Package v2 stores an eight-byte
`AGENT_KERNEL_PACKAGE_V2_RODATA` proof word in rodata. Its first instruction
contains a zero `movabs` immediate covered by one `ABS64` relocation. Ring 3
loads and validates the word before its existing 43-call transcript.

Successful QEMU completion proves:

- Package v2 structural and digest verification;
- one deterministic relocation into private code;
- five RX code mappings and one R+NX rodata mapping;
- no Agent-writable alias for either immutable segment;
- unchanged Resource, Capability, Memory, Namespace, and Event semantics;
- exact 17-frame Resource Manager reclamation into a sealed 77-frame pool.

The runtime emits `AGENT_KERNEL_NATIVE_SEGMENTED_PACKAGE_OK`,
`AGENT_KERNEL_NATIVE_RODATA_NX_OK`, and
`AGENT_KERNEL_NATIVE_RELOCATION_OK` only after execution evidence and mapping
evidence agree.

## Verification Gates

- Package parser accepts canonical boundary lengths and rejects malformed
  tables, flags, offsets, overlap, addends, placeholders, and trailing bytes.
- V1 parser behavior and exact digest identity remain locked.
- Layout tests lock all code, rodata, and shifted runtime addresses.
- Identity tests prove `12..43` bounded ownership with independent segment
  counts and canonical inactive storage.
- Pool tests resize between V1 and maximum V2 identities atomically.
- Page-table validation proves RX code, R+NX rodata, unmapped inactive pages,
  and complete kernel exclusion.
- Every native image digest, fixed address, and expected return offset is
  regenerated and audited.
- Focused tests, Workspace tests, Supervisor, five `no_std` targets, strict
  Clippy, debug and Release QEMU, and Release ELF uniqueness audits pass.

## Implemented Profile

| Measurement | V9 result |
| :--- | :--- |
| Resource Manager package | `16,634` bytes / SHA-256 `14f09265ccbbadfc09f2bb0ae12cd18a6d3a514d340e9cff49e351ad6db7b646` |
| Active segments | `16,483` code bytes / 5 RX pages; `31` rodata bytes / 1 R+NX page |
| Relocation | 1 canonical `ABS64` target at code offset `2` |
| Resource Manager identity | 17 frames: 4 tables + 5 code + 1 rodata + 7 fixed |
| Initial frame inventory | 77 sealed frames across six boot Agents |
| Native transcript | Events `1..409`, next sequence `410`, Supervisor handoff ready |

Verified gates:

- six focused Package v2 and relocation contracts;
- complete `cargo test --workspace` and Supervisor flow;
- five-library freestanding check and bare-metal binary check;
- host and bare-metal strict Clippy;
- debug and Release QEMU transcripts;
- eight-image digest audit, four-source assembly audit, Release ELF uniqueness.

## Deferred Work

- Ed25519 package signatures and signer trust policy;
- multiple read-only data segments and writable initialized data;
- richer relocation kinds and imported capability symbols;
- a stable public package-builder CLI;
- demand-backed package pages and SMP instruction-TLB synchronization.
