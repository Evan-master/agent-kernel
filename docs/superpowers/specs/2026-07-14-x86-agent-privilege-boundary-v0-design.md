# X86 Agent Privilege Boundary V0 Design

## Purpose

Agent CPU Context V0 gives one Worker a real native stack and asynchronous
preemption, but both the Agent and kernel execute at CPL0. This milestone moves
that Worker to CPL3 and makes the CPU enforce the authority boundary.

The proof sequence is:

    install kernel/user GDT segments and one long-mode TSS
      -> point TSS RSP0 at a dedicated privileged entry stack
      -> map fixed user code, signal, and guarded stack pages
      -> dispatch the Worker through iretq at CPL3
      -> PIT IRQ0 switches to RSP0 and saves the privilege frame
      -> validate user CS, SS, RIP, RSP, and the kernel entry stack
      -> expire and redispatch the semantic task
      -> resume the exact user frame through iretq
      -> user code invokes the DPL3 Agent call gate
      -> validate the second privilege frame and record TaskYielded

V0 remains one CPU, one Agent, and one active address space. The new page
region separates user-accessible Agent memory from supervisor-only kernel
memory, but it does not yet allocate a distinct CR3.

## References

- Intel 64 and IA-32 Software Developer Manuals:
  https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html
- x86_64 OffsetPageTable 0.15.5:
  https://docs.rs/x86_64/0.15.5/x86_64/structures/paging/mapper/struct.OffsetPageTable.html
- bootloader_api 0.11.15 BootInfo:
  https://docs.rs/bootloader_api/0.11.15/bootloader_api/info/struct.BootInfo.html

The x86_64 dependency is limited to the architecture binary. It is no_std,
already designed for the target architecture, and replaces error-prone ad hoc
page-table walking. Architecture-neutral descriptor encodings remain local and
host-testable.

## Descriptor Contract

The permanent GDT contains:

1. null,
2. ring-0 64-bit code,
3. ring-0 data,
4. ring-3 data,
5. ring-3 64-bit code,
6. the low half of one available 64-bit TSS descriptor,
7. the high half of that TSS descriptor.

The resulting selectors are 0x08, 0x10, 0x1b, 0x23, and 0x28. The user
selectors include RPL3. Installation reloads CS and the kernel data segments,
then loads TR. The TSS I/O-map base equals the 104-byte TSS size, so CPL3 has no
I/O permission bitmap and cannot execute port I/O.

## Privileged Entry Stack

The architecture binary owns one 32 KiB, page-aligned RSP0 stack with a bottom
canary. TSS RSP0 points to its exclusive upper bound. A PIT interrupt or Agent
call from CPL3 causes the processor to abandon the user RSP, load this RSP0,
and push SS, RSP, RFLAGS, CS, and RIP before entering the gate.

The interrupt assembly then pushes all 15 integer registers. The portable
PrivilegeInterruptStackFrame is therefore 160 bytes. The kernel validates the
whole frame against the RSP0 bounds before reading it.

## User Memory Region

The bootloader maps physical memory at a fixed supervisor-only offset. A
bounded frame allocator consumes only regions marked Usable in BootInfo. An
OffsetPageTable adds a fixed Agent region in an otherwise unused P4 slot:

- one present, user, executable, read-only code page,
- one present, user, read-only, non-executable signal page,
- one intentionally unmapped guard page,
- four present, user, writable, non-executable stack pages.

The code page contains a fixed position-independent Agent proof program. It
first pushes and pops one register to prove the user stack mapping, then polls
one byte in the signal page. The byte starts at zero, so PIT must preempt the
Agent. After semantic redispatch, the kernel writes one through the supervisor
physical mapping. The resumed Agent then invokes interrupt 0x90.

The kernel image, descriptor tables, event stores, physical-memory mapping, and
privileged stack remain supervisor-only. V0 never maps a kernel pointer into
the Agent region.

## Entry And PIT Preemption

The kernel arms PIT with IF clear. Entry assembly saves a kernel continuation,
constructs a five-word privilege return frame, and executes iretq. The return
frame sets IF but clears IOPL, NT, and TF.

IRQ0 enters through an interrupt gate, so IF is cleared. Its top half saves all
integer registers, records the frame address, masks the PIC, sends EOI, and
returns to the suspended kernel continuation rather than to CPL3. It calls no
Rust and mutates no Agent Kernel store.

Normal kernel code proves:

- current CPL is zero,
- one PIT interrupt occurred,
- the frame is wholly inside the RSP0 stack,
- saved CS and SS are the exact ring-3 selectors,
- saved RIP lies in the Agent code page,
- saved RSP lies in the mapped Agent stack,
- saved RFLAGS had IF set,
- the privileged-stack canary remains intact.

Only then may the semantic adapter record TaskQuantumExpired.

## Redispatch And Agent Call

Event 29 redispatches the same task before CPU resume. The architecture runtime
then sets the user signal byte and restores all registers plus the five-word
privilege frame through iretq.

IDT vector 0x90 is an interrupt gate with DPL3. The resumed Agent invokes it
without direct kernel memory or I/O access. Its top half saves a second complete
frame and returns to the saved kernel continuation. Normal code validates the
gate frame, exact selectors, user addresses, and one-call count before exposing
a YieldedAgentCpu token. The semantic adapter consumes that token to produce
event 30 TaskYielded.

Vector 0x90 is an Agent-specific call gate, not a POSIX syscall ABI. V0 defines
only one operation: yield the currently admitted Agent task. Capability and
task identity remain owned by the kernel-side type-state flow, not by untrusted
register arguments.

## Unsafe Boundary Audit

The final release ELF exposes five standalone assembly boundaries:

- agent_kernel_enter_user at address 0x120eb,
- agent_kernel_resume_interrupted_user at address 0x12118,
- agent_kernel_agent_timer_irq_stub at address 0x12149,
- agent_kernel_agent_call_stub at address 0x121a7,
- agent_kernel_load_privilege_tables at address 0x121f7.

Release disassembly proves that the table loader executes lgdt, reloads CS
through a far return, reloads data segments, and executes ltr. User entry saves
the kernel continuation, pushes SS, RSP, sanitized RFLAGS, CS, and RIP, then
executes iretq. The RFLAGS mask clears TF, IOPL, and NT before setting IF.

Both privilege ingresses push all 15 integer registers and read saved RIP from
offset 0x78. The PIT top half masks port 0x21, sends EOI through port 0x20, and
restores the saved kernel stack. The Agent-call top half performs no device I/O.
Neither top half calls Rust. Resume restores all 15 integer registers in inverse
order and executes iretq; its host trampoline executes cli before Rust regains
control.

The other unsafe boundary is page-table installation. OffsetPageTable receives
the active CR3 table and the bootloader-provided fixed physical offset. Every
mapped frame is first removed from a BootInfo Usable region. Runtime checks
prove the code page is user-readable/executable but not writable, the signal
page is user-readable and NX but not writable, the stack is user-writable and
NX, and the guard address has no translation. Kernel mutation of the signal
uses only its supervisor physical alias.

## Failure And Event Policy

Every descriptor, allocation, mapping, frame, selector, count, or canary
failure exits QEMU before any success marker or supervisor handoff. Physical
frame allocation and page-table installation are boot implementation details,
so they intentionally add no semantic event. Task expiry, redispatch, and
yield remain replayable events 28 through 30.

## Scope Limit

V0 does not add a second CR3, multiple Agent contexts, arbitrary executable
loading, a general Agent-call dispatcher, return values, copy-in/copy-out,
SMEP, SMAP, PCID, guard pages for RSP0, IST, XSAVE, TLS, SMP, APIC, or demand
paging. These remain explicit follow-up milestones.
