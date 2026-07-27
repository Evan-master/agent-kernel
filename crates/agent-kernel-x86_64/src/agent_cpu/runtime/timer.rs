//! Local APIC one-shot timer owned by one native Agent CPU runtime.
//!
//! The x86_64 runtime uses this wrapper to validate, arm, mask, and acknowledge
//! one physical scheduling quantum without exposing APIC mutation to ring 3.

use agent_kernel_x86_64::{
    apic::{LocalApicBase, LocalApicMmio, VolatileMmio, APIC_RESCHEDULE_VECTOR},
    native_runtime::NativeRunBoundary,
};

#[derive(Copy, Clone)]
pub(super) struct CpuQuantumTimer {
    base: LocalApicBase,
    physical_offset: u64,
    initial_count: u32,
}

impl CpuQuantumTimer {
    pub(super) fn new(
        base: LocalApicBase,
        physical_offset: u64,
        initial_count: u32,
    ) -> Option<Self> {
        if initial_count == 0 {
            return None;
        }
        LocalApicMmio::new(base, physical_offset, VolatileMmio)?;
        Some(Self {
            base,
            physical_offset,
            initial_count,
        })
    }

    pub(super) fn arm(self) -> Option<()> {
        LocalApicMmio::new(self.base, self.physical_offset, VolatileMmio)?
            .arm_timer_one_shot(APIC_RESCHEDULE_VECTOR, self.initial_count)
    }

    pub(super) fn finish(self, boundary: Option<NativeRunBoundary>) {
        if let Some(mut apic) = LocalApicMmio::new(self.base, self.physical_offset, VolatileMmio) {
            apic.mask_timer(APIC_RESCHEDULE_VECTOR);
            if boundary == Some(NativeRunBoundary::QuantumExpired) {
                apic.end_of_interrupt();
            }
        }
    }
}
