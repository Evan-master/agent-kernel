//! BSP interrupt-controller ownership and runtime route lifecycle.
//!
//! This architecture-binary child maps and calibrates Local APIC hardware,
//! initializes masked I/O APIC routing, permanently closes the 8259 path, and
//! owns the one-shot BSP UART route. Every mutation requires BSP context with
//! IF clear.

use core::arch::asm;

use agent_kernel_x86_64::{
    apic::{LocalApicBase, LocalApicMmio, VolatileMmio, APIC_SPURIOUS_VECTOR, APIC_TIMER_VECTOR},
    cpu::{CpuIndex, CpuLifecycleState},
};
use bootloader_api::BootInfo;

use crate::{agent_memory::PHYSICAL_MEMORY_OFFSET, exception_runtime, pic};

use super::{delay, interrupts, io_apic::IoApicRouting, memory, SmpBootError, SmpBootstrap};

impl SmpBootstrap {
    pub(crate) fn prepare_apic_mmio(
        &mut self,
        boot_info: &mut BootInfo,
    ) -> Result<(), SmpBootError> {
        if self.local_apic.is_some() {
            return Err(SmpBootError::ApicControllerAlreadyPrepared);
        }
        // SAFETY: the BSP owns shared IDT and page-table mutation before any
        // application processor receives a startup IPI.
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
        let io_apic_routing = IoApicRouting::prepare(
            &self.topology,
            self.topology.cpus().bsp().processor().apic_id(),
        )
        .map_err(SmpBootError::IoApicRouting)?;
        if self.topology.supports_legacy_pic() {
            // SAFETY: every I/O APIC entry is masked or preconfigured, IF is
            // clear, and this is the final ownership transfer from 8259 mode.
            unsafe {
                pic::mask_all();
            }
        }
        self.local_apic = Some(local_apic);
        self.local_apic_quantum_count = Some(quantum_count);
        self.io_apic_routing = Some(io_apic_routing);
        self.legacy_pic_disabled = true;
        Ok(())
    }

    pub(crate) fn bsp_quantum_timer(&self) -> Option<(LocalApicBase, u64, u32)> {
        self.local_apic.as_ref()?;
        Some((
            self.local_apic_base,
            PHYSICAL_MEMORY_OFFSET,
            self.local_apic_quantum_count?,
        ))
    }

    pub(crate) fn arm_uart_irq(&mut self) -> Result<(), SmpBootError> {
        self.io_apic_routing
            .as_mut()
            .ok_or(SmpBootError::InvalidLocalApicMapping)?
            .arm_uart()
            .map_err(SmpBootError::IoApicRouting)
    }

    pub(crate) fn complete_uart_irq(&mut self, delivered: bool) -> Result<(), SmpBootError> {
        self.io_apic_routing
            .as_mut()
            .ok_or(SmpBootError::InvalidLocalApicMapping)?
            .mask_uart()
            .map_err(SmpBootError::IoApicRouting)?;
        if delivered {
            self.local_apic
                .as_mut()
                .ok_or(SmpBootError::InvalidLocalApicMapping)?
                .end_of_interrupt();
        }
        Ok(())
    }

    pub(crate) fn ready_for_agent_boot(&self) -> bool {
        self.registry.state(CpuIndex::BSP) == Some(CpuLifecycleState::Online)
            && self.registry.online_mask().count() == 1
            && self.topology.cpus().bsp().index() == CpuIndex::BSP
            && self.topology.local_apic_address() == self.local_apic_base.physical()
            && !self.topology.io_apics().is_empty()
            && self.local_apic.is_some()
            && self.local_apic_quantum_count.is_some()
            && self
                .io_apic_routing
                .as_ref()
                .is_some_and(IoApicRouting::uart_masked)
            && self.legacy_pic_disabled
            && self.trampoline.is_some()
    }
}
