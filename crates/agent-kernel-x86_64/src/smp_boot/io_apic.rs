//! BSP-owned I/O APIC route installation for the SMP boot profile.
//!
//! This architecture-binary child masks every discovered redirection entry,
//! resolves the ISA UART source through MADT overrides, and permits only a
//! bounded one-shot IRQ4 route. Callers keep IF clear around every mutation.

use agent_kernel_x86_64::{
    acpi_topology::{AcpiMachineTopology, MAX_IO_APICS},
    apic::{
        resolve_legacy_irq_route, resolve_pci_intx_route, ApicVector, IoApicMmio, IoApicPolarity,
        IoApicRedirectionEntry, IoApicRedirectionIndex, IoApicRoute, IoApicRouteError,
        IoApicTrigger, IoApicVersion, VolatileMmio, APIC_SPURIOUS_VECTOR,
    },
    cpu::{ApicId, MAX_CPU_COUNT},
    interrupt::{PCI_INTX_IRQ_VECTOR, UART_IRQ_LINE, UART_IRQ_VECTOR},
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
    InvalidPciIntxVector,
    PciIntxRouteAlreadyPrepared,
    UnexpectedUartRouteState,
    UnexpectedPciIntxRouteState,
}

pub(super) struct IoApicRouting {
    versions: [IoApicVersion; MAX_IO_APICS],
    controller_count: usize,
    uart_route: IoApicRoute,
    uart_entry: IoApicRedirectionEntry,
    uart_masked: bool,
    pci_intx_route: Option<IoApicRoute>,
    pci_intx_entry: Option<IoApicRedirectionEntry>,
    pci_intx_masked: bool,
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
            versions,
            controller_count: controllers.len(),
            uart_route,
            uart_entry,
            uart_masked: true,
            pci_intx_route: None,
            pci_intx_entry: None,
            pci_intx_masked: true,
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

    pub(super) fn prepare_pci_intx(
        &mut self,
        topology: &AcpiMachineTopology<MAX_CPU_COUNT>,
        interrupt_line: u8,
    ) -> Result<(), IoApicRoutingError> {
        if self.pci_intx_route.is_some() {
            return Err(IoApicRoutingError::PciIntxRouteAlreadyPrepared);
        }
        let route = resolve_pci_intx_route(
            topology,
            &self.versions[..self.controller_count],
            interrupt_line,
        )
        .map_err(IoApicRoutingError::Route)?;
        let vector =
            ApicVector::new(PCI_INTX_IRQ_VECTOR).ok_or(IoApicRoutingError::InvalidPciIntxVector)?;
        let entry = IoApicRedirectionEntry::fixed(
            vector,
            self.uart_entry.destination(),
            route.polarity(),
            route.trigger(),
            true,
        );
        write_route(route, entry, true)?;
        self.pci_intx_route = Some(route);
        self.pci_intx_entry = Some(entry);
        self.pci_intx_masked = true;
        Ok(())
    }

    pub(super) fn arm_pci_intx(&mut self) -> Result<(), IoApicRoutingError> {
        if self.pci_intx_route.is_none() || !self.pci_intx_masked {
            return Err(IoApicRoutingError::UnexpectedPciIntxRouteState);
        }
        self.write_pci_intx(false)
    }

    pub(super) fn mask_pci_intx(&mut self) -> Result<(), IoApicRoutingError> {
        if self.pci_intx_route.is_none() || self.pci_intx_masked {
            return Err(IoApicRoutingError::UnexpectedPciIntxRouteState);
        }
        self.write_pci_intx(true)
    }

    pub(super) const fn pci_intx_masked(&self) -> bool {
        self.pci_intx_route.is_some() && self.pci_intx_masked
    }

    fn write_uart(&mut self, masked: bool) -> Result<(), IoApicRoutingError> {
        write_route(self.uart_route, self.uart_entry, masked)?;
        self.uart_masked = masked;
        Ok(())
    }

    fn write_pci_intx(&mut self, masked: bool) -> Result<(), IoApicRoutingError> {
        let route = self
            .pci_intx_route
            .ok_or(IoApicRoutingError::UnexpectedPciIntxRouteState)?;
        let entry = self
            .pci_intx_entry
            .ok_or(IoApicRoutingError::UnexpectedPciIntxRouteState)?;
        write_route(route, entry, masked)?;
        self.pci_intx_masked = masked;
        Ok(())
    }
}

fn write_route(
    route: IoApicRoute,
    entry: IoApicRedirectionEntry,
    masked: bool,
) -> Result<(), IoApicRoutingError> {
    let descriptor = route.controller();
    let mut controller =
        IoApicMmio::new(descriptor.address(), PHYSICAL_MEMORY_OFFSET, VolatileMmio)
            .ok_or(IoApicRoutingError::InvalidMapping(descriptor.id()))?;
    controller.write_redirection(route.redirection_index(), entry.with_masked(masked));
    Ok(())
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
