//! Validated xAPIC and I/O APIC register contracts.
//!
//! This architecture layer owns interrupt vectors, ICR command encoding,
//! redirection entries, and MMIO offsets. Bare-metal drivers execute only these
//! validated values and keep volatile access outside the pure contract types.

mod icr;
mod identity;
mod io;
mod local;
mod mmio;
mod route;
mod vector;

pub use icr::{IcrCommand, IcrError};
pub use identity::{ApicBaseMsr, CpuidApicIdentity};
pub use io::{
    IoApicPolarity, IoApicRedirectionEntry, IoApicRedirectionIndex, IoApicTrigger, IoApicVersion,
};
pub use local::{LocalApicBase, LocalApicRegister};
pub use mmio::{ApicMmioError, IoApicMmio, LocalApicMmio, Mmio32, VolatileMmio};
pub use route::{resolve_legacy_irq_route, IoApicRoute, IoApicRouteError};
pub use vector::{
    ApicVector, StartupVector, APIC_RESCHEDULE_VECTOR, APIC_SPURIOUS_VECTOR,
    APIC_STARTUP_ERROR_VECTOR, APIC_TIMER_VECTOR, APIC_TLB_SHOOTDOWN_VECTOR,
};
