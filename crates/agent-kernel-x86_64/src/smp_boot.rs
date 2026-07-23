//! BSP firmware topology and Local APIC identity bootstrap.
//!
//! This architecture-binary module consumes bootloader physical mapping data,
//! strictly loads ACPI topology, cross-checks CPUID and IA32_APIC_BASE, and
//! freezes the initial CPU Registry before Agent execution begins. AP startup
//! extends this owner without moving firmware parsing into kernel core state.

pub(crate) mod ap_worker;
mod delay;
mod interrupts;
mod memory;
mod startup;
mod tlb_ipi;
mod trampoline;

use core::{
    arch::{asm, x86_64::__cpuid},
    hint::spin_loop,
};

use agent_kernel_x86_64::{
    acpi_topology::{
        load_acpi_topology, AcpiMachineTopology, AcpiTopologyError, DirectAcpiHandler,
    },
    apic::{
        ApicBaseMsr, CpuidApicIdentity, LocalApicBase, LocalApicMmio, VolatileMmio,
        APIC_SPURIOUS_VECTOR, APIC_TIMER_VECTOR, APIC_TLB_SHOOTDOWN_VECTOR,
    },
    cpu::{CpuIndex, CpuLifecycleState, CpuRegistry, MAX_CPU_COUNT},
    tlb::{TlbAddressSpace, TlbFlushScope, TlbShootdownCoordinator},
};
use bootloader_api::BootInfo;

use crate::{agent_memory::PHYSICAL_MEMORY_OFFSET, exception_runtime};

use self::memory::ApicMappingError;
use self::startup::ApStartError;
use self::trampoline::{TrampolineError, TrampolinePage};

const IA32_APIC_BASE: u32 = 0x1b;
const CR3_ROOT_MASK: u64 = 0x000f_ffff_ffff_f000;
const CR4_PCIDE: u64 = 1 << 17;
const TLB_ACK_WAIT_LIMIT: usize = 100_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SmpBootError {
    MissingPhysicalMap,
    UnexpectedPhysicalOffset,
    MissingRsdp,
    PhysicalAddressOverflow,
    ApicFeatureMissing,
    InvalidApicBaseMsr,
    BootProcessorBitClear,
    X2ApicModeActive,
    Acpi(AcpiTopologyError),
    ApicBaseMismatch { msr: LocalApicBase, madt: u64 },
    ApicMapping(ApicMappingError),
    ApicControllerAlreadyPrepared,
    InvalidLocalApicMapping,
    LocalApicIdentityMismatch,
    InvalidLocalApicVersion,
    LocalApicTimerCalibrationFailed,
    SpuriousGateInstallFailed,
    IpiGateInstallFailed,
    PcideModeActive,
    Trampoline(TrampolineError),
    TrampolineAlreadyPrepared,
    ApplicationProcessorStartup(ApStartError),
    TlbShootdownFailed,
}

pub(crate) struct SmpBootstrap {
    topology: AcpiMachineTopology<MAX_CPU_COUNT>,
    registry: CpuRegistry<MAX_CPU_COUNT>,
    local_apic_base: LocalApicBase,
    local_apic: Option<LocalApicMmio<VolatileMmio>>,
    local_apic_quantum_count: Option<u32>,
    trampoline: Option<TrampolinePage>,
    tlb_coordinator: TlbShootdownCoordinator,
}

impl SmpBootstrap {
    pub(crate) fn discover(boot_info: &BootInfo) -> Result<Self, SmpBootError> {
        let physical_offset = boot_info
            .physical_memory_offset
            .into_option()
            .ok_or(SmpBootError::MissingPhysicalMap)?;
        if physical_offset != PHYSICAL_MEMORY_OFFSET {
            return Err(SmpBootError::UnexpectedPhysicalOffset);
        }
        let rsdp_address = boot_info
            .rsdp_addr
            .into_option()
            .ok_or(SmpBootError::MissingRsdp)?;
        let physical_length = boot_info
            .memory_regions
            .iter()
            .map(|region| region.end)
            .max()
            .ok_or(SmpBootError::MissingPhysicalMap)?;
        let physical_offset =
            usize::try_from(physical_offset).map_err(|_| SmpBootError::PhysicalAddressOverflow)?;
        let physical_length =
            usize::try_from(physical_length).map_err(|_| SmpBootError::PhysicalAddressOverflow)?;
        let rsdp_address =
            usize::try_from(rsdp_address).map_err(|_| SmpBootError::PhysicalAddressOverflow)?;

        let cpuid_leaf1 = __cpuid(1);
        let cpuid = CpuidApicIdentity::from_leaf1(cpuid_leaf1.ebx, cpuid_leaf1.edx)
            .ok_or(SmpBootError::ApicFeatureMissing)?;
        let apic_msr = ApicBaseMsr::from_raw(read_msr(IA32_APIC_BASE))
            .ok_or(SmpBootError::InvalidApicBaseMsr)?;
        if !apic_msr.boot_processor() {
            return Err(SmpBootError::BootProcessorBitClear);
        }
        if apic_msr.x2apic_enabled() {
            return Err(SmpBootError::X2ApicModeActive);
        }

        // SAFETY: the bootloader's fixed direct map covers every physical
        // region up to the maximum firmware memory-map endpoint.
        let handler = unsafe { DirectAcpiHandler::new(physical_offset, physical_length) };
        // SAFETY: bootloader_api supplies a firmware-validated physical RSDP
        // address inside the direct physical mapping domain.
        let topology = unsafe {
            load_acpi_topology::<_, MAX_CPU_COUNT>(handler, rsdp_address, cpuid.initial_apic_id())
        }
        .map_err(SmpBootError::Acpi)?;
        if topology.local_apic_address() != apic_msr.base().physical() {
            return Err(SmpBootError::ApicBaseMismatch {
                msr: apic_msr.base(),
                madt: topology.local_apic_address(),
            });
        }
        let registry = CpuRegistry::new(topology.cpus().clone());
        Ok(Self {
            topology,
            registry,
            local_apic_base: apic_msr.base(),
            local_apic: None,
            local_apic_quantum_count: None,
            trampoline: None,
            tlb_coordinator: TlbShootdownCoordinator::new(),
        })
    }

    pub(crate) fn prepare_apic_mmio(
        &mut self,
        boot_info: &mut BootInfo,
    ) -> Result<(), SmpBootError> {
        if self.local_apic.is_some() {
            return Err(SmpBootError::ApicControllerAlreadyPrepared);
        }
        // SAFETY: all shared IDT and page-table mutation remains on the BSP
        // before any application processor receives a startup IPI.
        unsafe {
            asm!("cli", options(nomem, nostack));
        }
        memory::map_apic_pages(
            boot_info,
            self.local_apic_base.physical(),
            self.topology.io_apics(),
        )
        .map_err(SmpBootError::ApicMapping)?;
        // SAFETY: the IDT is live, IF is clear, and no AP can observe this
        // final Local APIC gate update yet.
        unsafe {
            exception_runtime::install_irq_gate(
                APIC_SPURIOUS_VECTOR.get(),
                interrupts::spurious_handler(),
            )
        }
        .ok_or(SmpBootError::SpuriousGateInstallFailed)?;

        let mut local_apic =
            LocalApicMmio::new(self.local_apic_base, PHYSICAL_MEMORY_OFFSET, VolatileMmio)
                .ok_or(SmpBootError::InvalidLocalApicMapping)?;
        if local_apic.id() != self.topology.cpus().bsp().processor().apic_id() {
            return Err(SmpBootError::LocalApicIdentityMismatch);
        }
        if local_apic.version_raw() & 0xff == 0 {
            return Err(SmpBootError::InvalidLocalApicVersion);
        }
        local_apic.enable(APIC_SPURIOUS_VECTOR);
        local_apic.begin_timer_calibration(APIC_TIMER_VECTOR);
        delay::wait_micros(10_000).map_err(|_| SmpBootError::LocalApicTimerCalibrationFailed)?;
        let current = local_apic.timer_current_count();
        local_apic.mask_timer(APIC_TIMER_VECTOR);
        let quantum_count = u32::MAX
            .checked_sub(current)
            .filter(|count| *count != 0)
            .ok_or(SmpBootError::LocalApicTimerCalibrationFailed)?;
        self.local_apic = Some(local_apic);
        self.local_apic_quantum_count = Some(quantum_count);
        Ok(())
    }

    pub(crate) fn prepare_trampoline(
        &mut self,
        boot_info: &mut BootInfo,
    ) -> Result<(), SmpBootError> {
        if self.local_apic.is_none() {
            return Err(SmpBootError::InvalidLocalApicMapping);
        }
        if self.trampoline.is_some() {
            return Err(SmpBootError::TrampolineAlreadyPrepared);
        }
        self.trampoline =
            Some(TrampolinePage::prepare(boot_info).map_err(SmpBootError::Trampoline)?);
        Ok(())
    }

    pub(crate) fn install_ipi_gate(&self) -> Result<(), SmpBootError> {
        if read_cr4() & CR4_PCIDE != 0 {
            return Err(SmpBootError::PcideModeActive);
        }
        tlb_ipi::configure(self.local_apic_base, PHYSICAL_MEMORY_OFFSET)
            .ok_or(SmpBootError::IpiGateInstallFailed)?;
        // SAFETY: the BSP still owns all IDT mutation and interrupts remain
        // disabled until every SMP gate has been installed and frozen.
        unsafe {
            exception_runtime::install_irq_gate(APIC_TLB_SHOOTDOWN_VECTOR.get(), tlb_ipi::handler())
        }
        .ok_or(SmpBootError::IpiGateInstallFailed)
    }

    pub(crate) fn start_application_processors(&mut self) -> Result<usize, SmpBootError> {
        let local_apic = self
            .local_apic
            .as_mut()
            .ok_or(SmpBootError::InvalidLocalApicMapping)?;
        let trampoline = self
            .trampoline
            .as_ref()
            .ok_or(SmpBootError::TrampolineAlreadyPrepared)?;
        startup::start_all(
            &mut self.registry,
            local_apic,
            trampoline,
            self.local_apic_base,
            self.local_apic_quantum_count
                .ok_or(SmpBootError::LocalApicTimerCalibrationFailed)?,
        )
        .map_err(SmpBootError::ApplicationProcessorStartup)
    }

    pub(crate) fn prove_tlb_shootdown(&mut self) -> Result<(), SmpBootError> {
        let online = self.registry.online_mask();
        if online.count() < 2 {
            return Err(SmpBootError::TlbShootdownFailed);
        }
        let address_space = TlbAddressSpace::new(read_cr3() & CR3_ROOT_MASK, 1)
            .ok_or(SmpBootError::TlbShootdownFailed)?;
        let request = self
            .tlb_coordinator
            .begin(
                CpuIndex::BSP,
                online,
                online,
                address_space,
                TlbFlushScope::all_contexts(),
            )
            .map_err(|_| SmpBootError::TlbShootdownFailed)?;
        if tlb_ipi::publish(request).is_none() {
            let _ = self.tlb_coordinator.mark_timed_out(request.generation());
            return Err(SmpBootError::TlbShootdownFailed);
        }

        for raw_index in 1..self.registry.topology().len() {
            let cpu = CpuIndex::new(
                u16::try_from(raw_index).map_err(|_| SmpBootError::TlbShootdownFailed)?,
            )
            .ok_or(SmpBootError::TlbShootdownFailed)?;
            if !request.targets().contains(cpu) {
                continue;
            }
            let destination = self
                .registry
                .topology()
                .get(cpu)
                .ok_or(SmpBootError::TlbShootdownFailed)?
                .processor()
                .apic_id();
            let local_apic = self
                .local_apic
                .as_mut()
                .ok_or(SmpBootError::InvalidLocalApicMapping)?;
            startup::send_fixed_ipi(local_apic, destination, APIC_TLB_SHOOTDOWN_VECTOR)
                .map_err(SmpBootError::ApplicationProcessorStartup)?;
        }

        for _ in 0..TLB_ACK_WAIT_LIMIT {
            if tlb_ipi::acknowledged() == request.targets() {
                tlb_ipi::finish(request.generation()).ok_or(SmpBootError::TlbShootdownFailed)?;
                for raw_index in 1..self.registry.topology().len() {
                    let cpu = CpuIndex::new(
                        u16::try_from(raw_index).map_err(|_| SmpBootError::TlbShootdownFailed)?,
                    )
                    .ok_or(SmpBootError::TlbShootdownFailed)?;
                    if request.targets().contains(cpu) {
                        self.tlb_coordinator
                            .acknowledge(cpu, request.generation())
                            .map_err(|_| SmpBootError::TlbShootdownFailed)?;
                    }
                }
                self.tlb_coordinator
                    .finish(request.generation())
                    .map_err(|_| SmpBootError::TlbShootdownFailed)?;
                tlb_ipi::reset_complete().ok_or(SmpBootError::TlbShootdownFailed)?;
                return Ok(());
            }
            spin_loop();
        }

        let _ = tlb_ipi::mark_timed_out(request.generation());
        let _ = self.tlb_coordinator.mark_timed_out(request.generation());
        Err(SmpBootError::TlbShootdownFailed)
    }

    pub(crate) fn ready_for_agent_boot(&self) -> bool {
        self.registry.state(CpuIndex::BSP) == Some(CpuLifecycleState::Online)
            && self.registry.online_mask().count() == 1
            && self.topology.cpus().bsp().index() == CpuIndex::BSP
            && self.topology.local_apic_address() == self.local_apic_base.physical()
            && !self.topology.io_apics().is_empty()
            && self.local_apic.is_some()
            && self.local_apic_quantum_count.is_some()
            && self.trampoline.is_some()
    }

    pub(crate) const fn bsp_index(&self) -> CpuIndex {
        CpuIndex::BSP
    }
}

fn read_msr(register: u32) -> u64 {
    let low: u32;
    let high: u32;
    // SAFETY: kernel entry runs at CPL0 and CPUID already proved Local APIC
    // support. RDMSR reads the architectural IA32_APIC_BASE register only.
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") register,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
    (u64::from(high) << 32) | u64::from(low)
}

fn read_cr3() -> u64 {
    let value: u64;
    // SAFETY: kernel bootstrap reads the active page-table root only.
    unsafe {
        asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

fn read_cr4() -> u64 {
    let value: u64;
    // SAFETY: kernel bootstrap reads control state without mutation.
    unsafe {
        asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}
