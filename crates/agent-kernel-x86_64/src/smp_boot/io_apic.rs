//! BSP-owned I/O APIC route installation for the SMP boot profile.
//!
//! This architecture-binary child masks every discovered redirection entry,
//! resolves the ISA UART source through MADT overrides, and permits only a
//! bounded one-shot IRQ4 route. Callers keep IF clear around every mutation.

use agent_kernel_x86_64::{
    acpi_topology::{AcpiMachineTopology, MAX_IO_APICS},
    apic::{
        resolve_legacy_irq_route, ApicVector, IoApicMmio, IoApicPolarity, IoApicRedirectionEntry,
        IoApicRedirectionIndex, IoApicRoute, IoApicRouteError, IoApicTrigger, IoApicVersion,
        VolatileMmio, APIC_SPURIOUS_VECTOR,
    },
    cpu::{ApicId, MAX_CPU_COUNT},
    interrupt::{UART_IRQ_LINE, UART_IRQ_VECTOR},
};

use crate::agent_memory::PHYSICAL_MEMORY_OFFSET;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IoApicRoutingError {
    DestinationRequiresX2Apic(ApicId),
    InvalidMapping(u8),
    InvalidVersion(u8),
    InvalidRedirectionIndex { controller: u8, index: u16 },
    Route(IoApicRouteError),
    InvalidUartVector,
    UnexpectedUartRouteState,
}

pub(super) struct IoApicRouting {
    uart_route: IoApicRoute,
    uart_entry: IoApicRedirectionEntry,
    uart_masked: bool,
}

impl IoApicRouting {
    pub(super) fn prepare(
        topology: &AcpiMachineTopology<MAX_CPU_COUNT>,
        bsp_apic_id: ApicId,
    ) -> Result<Self, IoApicRoutingError> {
        let destination = u8::try_from(bsp_apic_id.get())
            .map_err(|_| IoApicRoutingError::DestinationRequiresX2Apic(bsp_apic_id))?;
        let controllers = topology.io_apics();
        let mut versions = [IoApicVersion::from_raw(0); MAX_IO_APICS];
        for (raw, descriptor) in controllers.iter().copied().enumerate() {
            let mut controller =
                IoApicMmio::new(descriptor.address(), PHYSICAL_MEMORY_OFFSET, VolatileMmio)
                    .ok_or(IoApicRoutingError::InvalidMapping(descriptor.id()))?;
            let version = controller.version();
            if version.version() == 0 {
                return Err(IoApicRoutingError::InvalidVersion(descriptor.id()));
            }
            mask_all(&mut controller, descriptor.id(), version, destination)?;
            versions[raw] = version;
        }

        let uart_route =
            resolve_legacy_irq_route(topology, &versions[..controllers.len()], UART_IRQ_LINE)
                .map_err(IoApicRoutingError::Route)?;
        let vector =
            ApicVector::new(UART_IRQ_VECTOR).ok_or(IoApicRoutingError::InvalidUartVector)?;
        let uart_entry = IoApicRedirectionEntry::fixed(
            vector,
            destination,
            uart_route.polarity(),
            uart_route.trigger(),
            true,
        );
        let mut routing = Self {
            uart_route,
            uart_entry,
            uart_masked: true,
        };
        routing.write_uart(true)?;
        Ok(routing)
    }

    pub(super) fn arm_uart(&mut self) -> Result<(), IoApicRoutingError> {
        if !self.uart_masked {
            return Err(IoApicRoutingError::UnexpectedUartRouteState);
        }
        self.write_uart(false)
    }

    pub(super) fn mask_uart(&mut self) -> Result<(), IoApicRoutingError> {
        if self.uart_masked {
            return Err(IoApicRoutingError::UnexpectedUartRouteState);
        }
        self.write_uart(true)
    }

    pub(super) const fn uart_masked(&self) -> bool {
        self.uart_masked
    }

    fn write_uart(&mut self, masked: bool) -> Result<(), IoApicRoutingError> {
        let descriptor = self.uart_route.controller();
        let mut controller =
            IoApicMmio::new(descriptor.address(), PHYSICAL_MEMORY_OFFSET, VolatileMmio)
                .ok_or(IoApicRoutingError::InvalidMapping(descriptor.id()))?;
        controller.write_redirection(
            self.uart_route.redirection_index(),
            self.uart_entry.with_masked(masked),
        );
        self.uart_masked = masked;
        Ok(())
    }
}

fn mask_all(
    controller: &mut IoApicMmio<VolatileMmio>,
    controller_id: u8,
    version: IoApicVersion,
    destination: u8,
) -> Result<(), IoApicRoutingError> {
    let entry = IoApicRedirectionEntry::fixed(
        APIC_SPURIOUS_VECTOR,
        destination,
        IoApicPolarity::ActiveHigh,
        IoApicTrigger::Edge,
        true,
    );
    for raw in 0..version.redirection_count() {
        let index = u8::try_from(raw)
            .ok()
            .and_then(|index| IoApicRedirectionIndex::new(index, version))
            .ok_or(IoApicRoutingError::InvalidRedirectionIndex {
                controller: controller_id,
                index: raw,
            })?;
        controller.write_redirection(index, entry);
    }
    Ok(())
}
