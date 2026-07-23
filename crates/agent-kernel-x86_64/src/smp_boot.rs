//! BSP firmware topology and Local APIC identity bootstrap.
//!
//! This architecture-binary module consumes bootloader physical mapping data,
//! strictly loads ACPI topology, cross-checks CPUID and IA32_APIC_BASE, and
//! freezes the initial CPU Registry before Agent execution begins. AP startup
//! extends this owner without moving firmware parsing into kernel core state.

mod delay;
mod interrupts;
mod memory;
mod startup;
mod trampoline;

use core::arch::{asm, x86_64::__cpuid};

use agent_kernel_x86_64::{
    acpi_topology::{
        load_acpi_topology, AcpiMachineTopology, AcpiTopologyError, DirectAcpiHandler,
    },
    apic::{
        ApicBaseMsr, CpuidApicIdentity, LocalApicBase, LocalApicMmio, VolatileMmio,
        APIC_SPURIOUS_VECTOR,
    },
    cpu::{CpuIndex, CpuLifecycleState, CpuRegistry, MAX_CPU_COUNT},
};
use bootloader_api::BootInfo;

use crate::{agent_memory::PHYSICAL_MEMORY_OFFSET, exception_runtime};

use self::memory::ApicMappingError;
use self::startup::ApStartError;
use self::trampoline::{TrampolineError, TrampolinePage};

const IA32_APIC_BASE: u32 = 0x1b;

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
    SpuriousGateInstallFailed,
    Trampoline(TrampolineError),
    TrampolineAlreadyPrepared,
    ApplicationProcessorStartup(ApStartError),
}

pub(crate) struct SmpBootstrap {
    topology: AcpiMachineTopology<MAX_CPU_COUNT>,
    registry: CpuRegistry<MAX_CPU_COUNT>,
    local_apic_base: LocalApicBase,
    local_apic: Option<LocalApicMmio<VolatileMmio>>,
    trampoline: Option<TrampolinePage>,
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
            trampoline: None,
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
        self.local_apic = Some(local_apic);
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
        )
        .map_err(SmpBootError::ApplicationProcessorStartup)
    }

    pub(crate) fn ready_for_agent_boot(&self) -> bool {
        self.registry.state(CpuIndex::BSP) == Some(CpuLifecycleState::Online)
            && self.registry.online_mask().count() == 1
            && self.topology.cpus().bsp().index() == CpuIndex::BSP
            && self.topology.local_apic_address() == self.local_apic_base.physical()
            && !self.topology.io_apics().is_empty()
            && self.local_apic.is_some()
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
