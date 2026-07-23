//! Single in-flight BSP-to-AP native Agent execution mailbox.

use core::{
    arch::asm,
    cell::UnsafeCell,
    hint::spin_loop,
    mem::MaybeUninit,
    sync::atomic::{AtomicU16, AtomicU8, Ordering},
};

use agent_kernel_x86_64::cpu::CpuIndex;

use crate::agent_cpu::{AgentCpuRuntime, AgentRunOutcome, PreemptedAgentCpu, PreparedAgentCpu};

const SLOT_IDLE: u8 = 0;
const SLOT_WRITING: u8 = 1;
const SLOT_READY: u8 = 2;
const SLOT_RUNNING: u8 = 3;
const SLOT_COMPLETE: u8 = 4;
const SLOT_FAILED: u8 = 5;
const PROOF_PREPARED_PENDING: u8 = 0;
const PROOF_CALL_PENDING: u8 = 1;
const PROOF_COMPLETE: u8 = 2;
const PROOF_FAILED: u8 = 3;
const PROOF_CPU: CpuIndex = match CpuIndex::new(1) {
    Some(cpu) => cpu,
    None => CpuIndex::BSP,
};
const WAIT_LIMIT: usize = 200_000_000;

// One fixed mailbox owns the larger variant directly; heap indirection would
// weaken the allocator-free boot and execution contract.
#[allow(clippy::large_enum_variant)]
enum ApWorkInput {
    Prepared(PreparedAgentCpu),
    Preempted(PreemptedAgentCpu),
}

impl ApWorkInput {
    const fn fallback_runtime(&self) -> AgentCpuRuntime {
        match self {
            Self::Prepared(cpu) => cpu.runtime(),
            Self::Preempted(cpu) => cpu.runtime(),
        }
    }

    fn run(self, runtime: AgentCpuRuntime) -> Option<AgentRunOutcome> {
        match self {
            Self::Prepared(cpu) => cpu.rebind_runtime(runtime)?.run_until_boundary(),
            Self::Preempted(cpu) => cpu.rebind_runtime(runtime)?.resume_until_boundary(),
        }
    }
}

struct ApWorkSlot {
    state: AtomicU8,
    target: AtomicU16,
    input: UnsafeCell<MaybeUninit<ApWorkInput>>,
    output: UnsafeCell<MaybeUninit<AgentRunOutcome>>,
}

impl ApWorkSlot {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(SLOT_IDLE),
            target: AtomicU16::new(0),
            input: UnsafeCell::new(MaybeUninit::uninit()),
            output: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
}

// SAFETY: state transitions grant exclusive payload ownership to one BSP
// producer or the exact target AP consumer. No payload location is accessed in
// two states concurrently, and Release/Acquire publishes every ownership edge.
unsafe impl Sync for ApWorkSlot {}

static WORK_SLOT: ApWorkSlot = ApWorkSlot::new();
static PROOF_STATE: AtomicU8 = AtomicU8::new(PROOF_PREPARED_PENDING);
static PROOF_WORKER_READY: AtomicU8 = AtomicU8::new(0);

pub(crate) fn wants_prepared_execution() -> bool {
    PROOF_STATE.load(Ordering::Acquire) == PROOF_PREPARED_PENDING
}

pub(crate) fn wants_preempted_execution() -> bool {
    PROOF_STATE.load(Ordering::Acquire) == PROOF_CALL_PENDING
}

pub(crate) fn proof_complete() -> bool {
    PROOF_STATE.load(Ordering::Acquire) == PROOF_COMPLETE
}

pub(crate) fn execute_prepared(cpu: PreparedAgentCpu) -> Option<AgentRunOutcome> {
    execute(ApWorkInput::Prepared(cpu), PROOF_PREPARED_PENDING)
}

pub(crate) fn execute_preempted(cpu: PreemptedAgentCpu) -> Option<AgentRunOutcome> {
    execute(ApWorkInput::Preempted(cpu), PROOF_CALL_PENDING)
}

fn execute(input: ApWorkInput, expected_proof: u8) -> Option<AgentRunOutcome> {
    if PROOF_STATE.load(Ordering::Acquire) != expected_proof || !wait_for_worker() {
        return None;
    }
    let fallback_runtime = input.fallback_runtime();
    if fallback_runtime.cpu() != CpuIndex::BSP
        || WORK_SLOT
            .state
            .compare_exchange(SLOT_IDLE, SLOT_WRITING, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        return None;
    }
    WORK_SLOT.target.store(PROOF_CPU.get(), Ordering::Relaxed);
    // SAFETY: SLOT_WRITING gives the BSP exclusive input ownership.
    unsafe {
        (*WORK_SLOT.input.get()).write(input);
    }
    WORK_SLOT.state.store(SLOT_READY, Ordering::Release);

    for _ in 0..WAIT_LIMIT {
        match WORK_SLOT.state.load(Ordering::Acquire) {
            SLOT_COMPLETE => {
                // SAFETY: AP published one initialized output before Complete.
                let outcome = unsafe { (*WORK_SLOT.output.get()).assume_init_read() };
                WORK_SLOT.state.store(SLOT_IDLE, Ordering::Release);
                record_proof(expected_proof, &outcome);
                return outcome.rebind_runtime(fallback_runtime);
            }
            SLOT_FAILED => {
                PROOF_STATE.store(PROOF_FAILED, Ordering::Release);
                return None;
            }
            _ => spin_loop(),
        }
    }
    PROOF_STATE.store(PROOF_FAILED, Ordering::Release);
    None
}

fn wait_for_worker() -> bool {
    for _ in 0..WAIT_LIMIT {
        if PROOF_WORKER_READY.load(Ordering::Acquire) == 1 {
            return true;
        }
        spin_loop();
    }
    false
}

fn record_proof(expected: u8, outcome: &AgentRunOutcome) {
    let next = match (expected, outcome) {
        (PROOF_PREPARED_PENDING, AgentRunOutcome::Preempted(_)) => PROOF_CALL_PENDING,
        (PROOF_PREPARED_PENDING | PROOF_CALL_PENDING, AgentRunOutcome::Call(_)) => PROOF_COMPLETE,
        (PROOF_CALL_PENDING, AgentRunOutcome::Preempted(_)) => PROOF_CALL_PENDING,
        _ => PROOF_FAILED,
    };
    PROOF_STATE.store(next, Ordering::Release);
}

pub(super) fn run(cpu: CpuIndex, runtime: AgentCpuRuntime) -> ! {
    if cpu == PROOF_CPU && runtime.cpu() == cpu {
        PROOF_WORKER_READY.store(1, Ordering::Release);
    }
    // SAFETY: this AP has loaded its frozen IDT and enabled Local APIC before
    // entering the worker. Interrupts are reclaimed before context transfer.
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
    loop {
        if WORK_SLOT.state.load(Ordering::Acquire) != SLOT_READY
            || WORK_SLOT.target.load(Ordering::Relaxed) != cpu.get()
            || WORK_SLOT
                .state
                .compare_exchange(
                    SLOT_READY,
                    SLOT_RUNNING,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
        {
            spin_loop();
            continue;
        }
        // SAFETY: the AP exclusively owns the claimed input and transition
        // slot; Agent entry also requires IF clear until iretq.
        unsafe {
            asm!("cli", options(nomem, nostack));
        }
        // SAFETY: SLOT_RUNNING transfers the initialized input to this AP.
        let input = unsafe { (*WORK_SLOT.input.get()).assume_init_read() };
        match input.run(runtime) {
            Some(outcome) => {
                // SAFETY: this target AP exclusively owns output in Running.
                unsafe {
                    (*WORK_SLOT.output.get()).write(outcome);
                }
                WORK_SLOT.state.store(SLOT_COMPLETE, Ordering::Release);
            }
            None => WORK_SLOT.state.store(SLOT_FAILED, Ordering::Release),
        }
        // SAFETY: execution returned to the AP kernel CR3 and frozen IDT.
        unsafe {
            asm!("sti", options(nomem, nostack));
        }
    }
}
