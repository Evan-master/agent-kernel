//! Generation-bound x86_64 TLB shootdown protocol.
//!
//! This architecture layer owns allocator-free request values and the
//! deterministic coordinator used around page-table mutation. Hardware IPI and
//! invalidation code consumes the protocol but cannot bypass its reuse gate.

mod coordinator;
mod types;

pub use coordinator::{
    TlbShootdownCompletion, TlbShootdownCoordinator, TlbShootdownError, TlbShootdownProgress,
};
pub use types::{
    TlbAddressSpace, TlbFlushKind, TlbFlushScope, TlbShootdownRequest, TlbShootdownStatus,
    MAX_TLB_RANGE_PAGES,
};
