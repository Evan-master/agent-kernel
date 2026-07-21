# Expanded Agent Capsule V8 Design

Status: Implemented and verified on 2026-07-21

## Objective

Expand the native executable window from four pages to sixteen pages while
preserving Capsule v1, bounded parsing, exact SHA-256 identity, and V7
right-sized physical ownership. V8 raises the flat executable limit from
16 KiB to 64 KiB and proves instruction return from code page five.

## Stable Contracts

- Capsule v1 keeps its 32-byte header, architecture, image-kind, ABI, entry,
  reserved-field, and exact-length rules.
- `AGENT_REGION_BASE` remains `0x0000400000000000`.
- Code pages remain immutable user RX mappings.
- Every unused page in the sixteen-page code window remains unmapped.
- Each identity keeps four private page-table frames and seven fixed non-code
  content frames.
- Physical ownership remains `11 + code_page_count` frames.
- Restart, cancellation, reclamation, and reuse transfer only active frames.

## Capsule Bound

| Field | V8 contract |
| :--- | :--- |
| Header bytes | `32` |
| Code length | `1..65,536` |
| Code pages | `1..16` |
| Entry offset | less than code length |
| Digest | SHA-256 over exact header and code bytes |
| Final physical page | kernel-zeroed after declared code |

Capsules of 4,096, 4,097, 16,384, 16,385, and 65,536 bytes are valid.
65,537 bytes is rejected.

## Fixed Virtual Layout

| Region | Start | Pages | Ring-3 policy |
| :--- | ---: | ---: | :--- |
| Code | `0x0000400000000000` | 16 | user RX, active prefix only |
| Signal | `0x0000400000010000` | 1 | user R, NX |
| Guard | `0x0000400000011000` | 1 | unmapped |
| Stack | `0x0000400000012000` | 4 | user RW, NX |
| Lazy data | `0x0000400000016000` | 1 | initially unmapped |
| Runtime page | `0x0000400000017000` | 1 | initially unmapped |
| Runtime region | `0x0000400000018000` | 8 | initially unmapped |
| Call data | `0x0000400000020000` | 1 | user RW, NX |

The complete layout remains inside dedicated P4 slot 128. Every native Capsule
is rebuilt in the same milestone because signal, runtime, and call-data
addresses are part of its immutable machine code.

## Physical Identity

V7's variable identity widens its bounded storage without changing active
ordering:

~~~text
page tables       4
code              1..16
signal            1
stack             4
lazy-data backing 1
call-data         1
owned total       12..27
~~~

Inactive storage stays canonical zero and never enters mapping, zeroing, alias
checks, pool transfer, or frame-count evidence.

The six initial native Agents use five one-page identities and one five-page
Resource Manager identity. Their sealed inventory is 76 frames:

~~~text
5 * 12 + 1 * 16 = 76
~~~

## Native Page-five Proof

The Resource Manager keeps its 43-call semantic transcript and adds a
deterministic NOP span before result submission. Its final result and completion
return offsets land after `16,384`. QEMU therefore proves:

- parser and digest acceptance beyond four pages;
- allocation of exactly five physical code frames;
- five RX mappings followed by eleven unmapped code-window pages;
- instruction fetch across four page boundaries;
- Agent Call return-offset capture from page five;
- complete 16-frame reclamation into the sealed 76-frame inventory;
- unchanged Resource, Capability, Memory, Namespace, and Event semantics.

The runtime emits `AGENT_KERNEL_NATIVE_FIFTH_CODE_PAGE_OK` after the page-five
offsets and final semantic evidence agree.

## Verification Gates

- parser accepts the boundary lengths and rejects `65,537`;
- layout tests lock every shifted address and the dedicated P4 slot;
- identity tests prove `12..27` owned frames and canonical inactive storage;
- pool tests resize between one-page and sixteen-page identities;
- page-table validation rejects any mapping after the active code prefix;
- every static Capsule digest and expected return offset is regenerated;
- focused tests fail before implementation and pass afterward;
- Workspace tests, Supervisor simulation, five `no_std` targets, strict Clippy,
  debug and Release QEMU, and Release ELF uniqueness audits pass.

## Measured V8 Profile

| Evidence | Value |
| :--- | ---: |
| Initial Agent address spaces | 6 |
| One-page identities | 5 x 12 frames |
| Five-page identities | 1 x 16 frames |
| Sealed boot inventory | 76 frames |
| Resource Manager code | 16,448 bytes |
| Resource Manager Capsule | 16,480 bytes |
| Final page-five return offset | 16,446 |

Debug and Release QEMU completed Events `1..409`, restored all 76 frames,
replayed the Event archive, and reached `SUPERVISOR_HANDOFF_READY`.

Final gates:

| Gate | Evidence |
| :--- | :--- |
| Workspace | `216` result groups, `745` passed tests |
| Supervisor | Host simulation completed through Event archive checkpoint |
| Freestanding | Five `no_std` libraries plus the bare-metal binary passed |
| Lints | Workspace and bare-metal Clippy passed with warnings denied |
| Capsules | Eight headers, SHA-256 values, code bodies, and return tables matched |
| QEMU | Debug and Release produced the exact Events `1..409` transcript |
| ELF | Resource Manager and Admission Supervisor Capsule/code each occurred once |

## Deferred Work

- segmented executable and read-only data sections;
- relocation records and a production Capsule builder;
- signed package manifests;
- demand-backed executable pages;
- SMP instruction-TLB synchronization.
