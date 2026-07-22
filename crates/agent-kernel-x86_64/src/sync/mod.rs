//! Allocator-free synchronization for x86_64 kernel state.
//!
//! Ticket ordering provides bounded critical-section fairness, while the IRQ
//! wrapper preserves each CPU's prior interrupt-enable state. These primitives
//! own synchronization only; Agent authorization remains in kernel core state.

mod irq;
mod ticket;

pub use irq::{InterruptState, IrqTicketGuard, IrqTicketLock, LocalInterruptControl};
pub use ticket::{TicketGuard, TicketLock, TicketLockError};
