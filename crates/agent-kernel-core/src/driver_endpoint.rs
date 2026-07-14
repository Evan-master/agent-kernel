//! Fixed-width architecture endpoint values for native driver resources.
//!
//! This core-layer module defines allocator-free descriptors and immutable
//! registration records. Validation and authorization live in the endpoint
//! store; descriptors never perform physical I/O themselves.

use crate::{AgentId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DriverEndpointKind {
    Virtual,
    Mmio,
    Port,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverEndpointDescriptor {
    pub kind: DriverEndpointKind,
    pub base: u64,
    pub span: u64,
}

impl DriverEndpointDescriptor {
    pub const fn virtual_channel(channel: u64) -> Self {
        Self {
            kind: DriverEndpointKind::Virtual,
            base: channel,
            span: 1,
        }
    }

    pub const fn mmio(base: u64, span: u64) -> Self {
        Self {
            kind: DriverEndpointKind::Mmio,
            base,
            span,
        }
    }

    pub const fn port(base: u64, span: u64) -> Self {
        Self {
            kind: DriverEndpointKind::Port,
            base,
            span,
        }
    }

    pub(crate) const fn end(self) -> Option<u64> {
        if self.span == 0 {
            None
        } else {
            self.base.checked_add(self.span - 1)
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverEndpointRecord {
    pub resource: ResourceId,
    pub installer: AgentId,
    pub descriptor: DriverEndpointDescriptor,
}

impl DriverEndpointRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            resource: ResourceId::new(0),
            installer: AgentId::new(0),
            descriptor: DriverEndpointDescriptor {
                kind: DriverEndpointKind::Virtual,
                base: 0,
                span: 0,
            },
        }
    }
}
