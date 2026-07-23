//! Deterministic ISA IRQ to I/O APIC routing contracts.
//!
//! This architecture library module combines validated MADT topology with
//! hardware-reported redirection counts. It resolves one legacy ISA source to
//! exactly one controller entry without performing MMIO or owning route state.

use crate::acpi_topology::{
    AcpiMachineTopology, InterruptPolarity, InterruptTrigger, IoApicDescriptor,
};

use super::{IoApicPolarity, IoApicRedirectionIndex, IoApicTrigger, IoApicVersion};

const ISA_IRQ_COUNT: u8 = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoApicRoute {
    source_irq: u8,
    gsi: u32,
    controller: IoApicDescriptor,
    redirection_index: IoApicRedirectionIndex,
    polarity: IoApicPolarity,
    trigger: IoApicTrigger,
}

impl IoApicRoute {
    pub const fn source_irq(self) -> u8 {
        self.source_irq
    }

    pub const fn gsi(self) -> u32 {
        self.gsi
    }

    pub const fn controller(self) -> IoApicDescriptor {
        self.controller
    }

    pub const fn redirection_index(self) -> IoApicRedirectionIndex {
        self.redirection_index
    }

    pub const fn polarity(self) -> IoApicPolarity {
        self.polarity
    }

    pub const fn trigger(self) -> IoApicTrigger {
        self.trigger
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoApicRouteError {
    UnsupportedLegacyIrq(u8),
    VersionCountMismatch { controllers: usize, versions: usize },
    InvalidControllerVersion(u8),
    GsiRangeOverflow(u8),
    MissingGsi(u32),
    AmbiguousGsi(u32),
    InvalidRedirectionIndex(u32),
}

pub fn resolve_legacy_irq_route<const CPU_CAPACITY: usize>(
    topology: &AcpiMachineTopology<CPU_CAPACITY>,
    versions: &[IoApicVersion],
    source_irq: u8,
) -> Result<IoApicRoute, IoApicRouteError> {
    if source_irq >= ISA_IRQ_COUNT {
        return Err(IoApicRouteError::UnsupportedLegacyIrq(source_irq));
    }
    let controllers = topology.io_apics();
    if controllers.len() != versions.len() {
        return Err(IoApicRouteError::VersionCountMismatch {
            controllers: controllers.len(),
            versions: versions.len(),
        });
    }

    let source_override = topology
        .interrupt_overrides()
        .iter()
        .find(|entry| entry.source_irq() == source_irq);
    let gsi = source_override.map_or(u32::from(source_irq), |entry| entry.gsi());
    let polarity = match source_override.map(|entry| entry.polarity()) {
        None | Some(InterruptPolarity::SameAsBus | InterruptPolarity::ActiveHigh) => {
            IoApicPolarity::ActiveHigh
        }
        Some(InterruptPolarity::ActiveLow) => IoApicPolarity::ActiveLow,
    };
    let trigger = match source_override.map(|entry| entry.trigger()) {
        None | Some(InterruptTrigger::SameAsBus | InterruptTrigger::Edge) => IoApicTrigger::Edge,
        Some(InterruptTrigger::Level) => IoApicTrigger::Level,
    };

    let mut resolved = None;
    for (controller, version) in controllers.iter().copied().zip(versions.iter().copied()) {
        if version.version() == 0 {
            return Err(IoApicRouteError::InvalidControllerVersion(controller.id()));
        }
        let end = controller
            .gsi_base()
            .checked_add(u32::from(version.redirection_count()))
            .ok_or(IoApicRouteError::GsiRangeOverflow(controller.id()))?;
        if gsi < controller.gsi_base() || gsi >= end {
            continue;
        }
        if resolved.is_some() {
            return Err(IoApicRouteError::AmbiguousGsi(gsi));
        }
        let offset = gsi - controller.gsi_base();
        let index = u8::try_from(offset)
            .ok()
            .and_then(|index| IoApicRedirectionIndex::new(index, version))
            .ok_or(IoApicRouteError::InvalidRedirectionIndex(gsi))?;
        resolved = Some(IoApicRoute {
            source_irq,
            gsi,
            controller,
            redirection_index: index,
            polarity,
            trigger,
        });
    }
    resolved.ok_or(IoApicRouteError::MissingGsi(gsi))
}
