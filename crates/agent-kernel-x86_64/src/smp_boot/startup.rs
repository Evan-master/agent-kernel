//! Serial INIT/SIPI startup and AP-side runtime installation.

use core::{arch::asm, hint::spin_loop};

use agent_kernel_x86_64::{
    apic::{
        ApicVector, IcrCommand, LocalApicBase, LocalApicMmio, VolatileMmio, APIC_SPURIOUS_VECTOR,
    },
    cpu::{
        ApStartupDescriptor, ApStartupEvidence, ApStartupHandoff, ApStartupStatus, CpuIndex,
        CpuRegistry, MAX_CPU_COUNT,
    },
};

use crate::{
    agent_cpu::{self, AgentCpuRuntime},
    exception_runtime, halt_forever,
    privilege_runtime::{self, PrivilegeBoundary},
};

use super::{ap_worker, delay, trampoline::TrampolinePage};

const INIT_ASSERT_MICROS: u32 = 10_000;
const INIT_DEASSERT_MICROS: u32 = 10_000;
const SIPI_GAP_MICROS: u32 = 200;
const ONLINE_POLL_MICROS: u32 = 1_000;
const ONLINE_POLL_COUNT: usize = 100;
const ICR_POLL_LIMIT: usize = 2_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApStartError {
    NoApplicationProcessor,
    UnknownProcessor,
    InvalidDescriptor,
    RegistryTransition,
    HandoffTransition,
    IcrCommand,
    IcrTimeout,
    Delay,
    ApplicationProcessorFailed,
    ApplicationProcessorTimeout,
    IncompleteOnlineSet,
}

pub(super) fn start_all(
    registry: &mut CpuRegistry<MAX_CPU_COUNT>,
    local_apic: &mut LocalApicMmio<VolatileMmio>,
    trampoline: &TrampolinePage,
    local_apic_base: LocalApicBase,
    local_apic_quantum_count: u32,
) -> Result<usize, ApStartError> {
    let cpu_count = registry.topology().len();
    if cpu_count < 2 {
        return Err(ApStartError::NoApplicationProcessor);
    }
    if trampoline.physical_address() != trampoline.vector().address() {
        return Err(ApStartError::InvalidDescriptor);
    }
    let kernel_cr3 = current_cr3();
    for raw_index in 1..cpu_count {
        let cpu =
            CpuIndex::new(u16::try_from(raw_index).map_err(|_| ApStartError::UnknownProcessor)?)
                .ok_or(ApStartError::UnknownProcessor)?;
        let processor = registry
            .topology()
            .get(cpu)
            .ok_or(ApStartError::UnknownProcessor)?
            .processor();
        let generation = raw_index as u64;
        registry
            .request_startup(cpu, generation)
            .map_err(|_| ApStartError::RegistryTransition)?;
        let descriptor = privilege_runtime::startup_stack_top(cpu).and_then(|stack_top| {
            ApStartupDescriptor::new(
                cpu,
                processor.apic_id(),
                generation,
                kernel_cr3,
                stack_top,
                agent_kernel_ap_entry as *const () as usize as u64,
                local_apic_base.physical(),
                local_apic_quantum_count,
                crate::agent_memory::PHYSICAL_MEMORY_OFFSET,
            )
            .ok()
        });
        let Some(descriptor) = descriptor else {
            registry
                .fail_startup(cpu, generation)
                .map_err(|_| ApStartError::RegistryTransition)?;
            return Err(ApStartError::InvalidDescriptor);
        };
        if trampoline.handoff().prepare(descriptor).is_err() {
            registry
                .fail_startup(cpu, generation)
                .map_err(|_| ApStartError::RegistryTransition)?;
            return Err(ApStartError::HandoffTransition);
        }

        let result = start_one(local_apic, trampoline, descriptor);
        match result {
            Ok(evidence) => {
                if evidence.cpu != cpu
                    || evidence.apic_id != processor.apic_id()
                    || evidence.generation != generation
                {
                    return Err(ApStartError::HandoffTransition);
                }
                registry
                    .acknowledge_online(cpu, generation)
                    .map_err(|_| ApStartError::RegistryTransition)?;
                trampoline
                    .handoff()
                    .reset_terminal()
                    .map_err(|_| ApStartError::HandoffTransition)?;
            }
            Err(error) => {
                registry
                    .fail_startup(cpu, generation)
                    .map_err(|_| ApStartError::RegistryTransition)?;
                return Err(error);
            }
        }
    }
    let online = usize::from(registry.online_mask().count());
    if online != cpu_count {
        return Err(ApStartError::IncompleteOnlineSet);
    }
    Ok(online)
}

fn start_one(
    local_apic: &mut LocalApicMmio<VolatileMmio>,
    trampoline: &TrampolinePage,
    descriptor: ApStartupDescriptor,
) -> Result<ApStartupEvidence, ApStartError> {
    send(
        local_apic,
        IcrCommand::init_assert(descriptor.apic_id()).map_err(|_| ApStartError::IcrCommand)?,
    )?;
    delay::wait_micros(INIT_ASSERT_MICROS).map_err(|_| ApStartError::Delay)?;
    send(
        local_apic,
        IcrCommand::init_deassert(descriptor.apic_id()).map_err(|_| ApStartError::IcrCommand)?,
    )?;
    delay::wait_micros(INIT_DEASSERT_MICROS).map_err(|_| ApStartError::Delay)?;
    let startup = IcrCommand::startup(descriptor.apic_id(), trampoline.vector())
        .map_err(|_| ApStartError::IcrCommand)?;
    send(local_apic, startup)?;
    delay::wait_micros(SIPI_GAP_MICROS).map_err(|_| ApStartError::Delay)?;
    send(local_apic, startup)?;

    for _ in 0..ONLINE_POLL_COUNT {
        match trampoline
            .handoff()
            .status()
            .map_err(|_| ApStartError::HandoffTransition)?
        {
            ApStartupStatus::Online => {
                return trampoline
                    .handoff()
                    .evidence()
                    .map_err(|_| ApStartError::HandoffTransition);
            }
            ApStartupStatus::Failed => return Err(ApStartError::ApplicationProcessorFailed),
            ApStartupStatus::Prepared => {}
            _ => return Err(ApStartError::HandoffTransition),
        }
        delay::wait_micros(ONLINE_POLL_MICROS).map_err(|_| ApStartError::Delay)?;
    }
    Err(ApStartError::ApplicationProcessorTimeout)
}

fn send(
    local_apic: &mut LocalApicMmio<VolatileMmio>,
    command: IcrCommand,
) -> Result<(), ApStartError> {
    wait_for_delivery(local_apic)?;
    local_apic
        .try_send(command)
        .map_err(|_| ApStartError::IcrCommand)?;
    wait_for_delivery(local_apic)
}

pub(super) fn send_fixed_ipi(
    local_apic: &mut LocalApicMmio<VolatileMmio>,
    destination: agent_kernel_x86_64::cpu::ApicId,
    vector: ApicVector,
) -> Result<(), ApStartError> {
    let command = IcrCommand::fixed(destination, vector).map_err(|_| ApStartError::IcrCommand)?;
    send(local_apic, command)
}

fn wait_for_delivery(local_apic: &LocalApicMmio<VolatileMmio>) -> Result<(), ApStartError> {
    for _ in 0..ICR_POLL_LIMIT {
        if !local_apic.delivery_pending() {
            return Ok(());
        }
        spin_loop();
    }
    Err(ApStartError::IcrTimeout)
}

fn current_cr3() -> u64 {
    let cr3: u64;
    // SAFETY: reading CR3 is side-effect free at CPL0.
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }
    cr3
}

#[no_mangle]
extern "C" fn agent_kernel_ap_entry(
    cpu_raw: u32,
    generation: u64,
    apic_raw: u32,
    handoff_address: u64,
) -> ! {
    // SAFETY: trampoline enters with IF clear; repeat the invariant before any
    // descriptor or CPU-local installation.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    let handoff = unsafe { &*(handoff_address as *const ApStartupHandoff) };
    match initialize_ap(cpu_raw, generation, apic_raw, handoff) {
        Ok(initialized) => {
            if handoff.acknowledge_online(initialized.evidence).is_ok() {
                ap_worker::run(initialized.evidence.cpu, initialized.runtime);
            }
        }
        Err((cpu, startup_generation)) => {
            let _ = handoff.fail(cpu, startup_generation);
        }
    }
    halt_forever()
}

struct InitializedAp {
    evidence: ApStartupEvidence,
    runtime: AgentCpuRuntime,
}

fn initialize_ap(
    cpu_raw: u32,
    generation: u64,
    apic_raw: u32,
    handoff: &ApStartupHandoff,
) -> Result<InitializedAp, (CpuIndex, u64)> {
    let cpu = u16::try_from(cpu_raw)
        .ok()
        .and_then(CpuIndex::new)
        .ok_or((CpuIndex::BSP, generation))?;
    let descriptor = handoff.descriptor().map_err(|_| (cpu, generation))?;
    if descriptor.cpu() != cpu
        || descriptor.generation() != generation
        || descriptor.apic_id().get() != apic_raw
        || current_cr3() != descriptor.cr3()
    {
        return Err((cpu, generation));
    }
    let privilege = PrivilegeBoundary::install(cpu).ok_or((cpu, generation))?;
    exception_runtime::load_for_current_cpu().ok_or((cpu, generation))?;
    let transition =
        agent_cpu::install_ap_transition_slot(descriptor.cr3(), cpu).ok_or((cpu, generation))?;
    let base = LocalApicBase::new(descriptor.local_apic_base()).ok_or((cpu, generation))?;
    let mut local_apic = LocalApicMmio::new(base, descriptor.physical_offset(), VolatileMmio)
        .ok_or((cpu, generation))?;
    if local_apic.id() != descriptor.apic_id() || local_apic.version_raw() & 0xff == 0 {
        return Err((cpu, generation));
    }
    local_apic.enable(APIC_SPURIOUS_VECTOR);
    let runtime = AgentCpuRuntime::attach_application_processor(
        &privilege,
        transition,
        descriptor.cr3(),
        cpu,
        base,
        descriptor.physical_offset(),
        descriptor.timer_initial_count(),
    )
    .ok_or((cpu, generation))?;
    let stack = privilege.stack_bounds();
    Ok(InitializedAp {
        evidence: ApStartupEvidence {
            cpu,
            apic_id: descriptor.apic_id(),
            generation,
            privileged_stack_start: stack.start as u64,
            privileged_stack_end: stack.end as u64,
            transition_slot: transition.as_ptr() as usize as u64,
        },
        runtime,
    })
}
