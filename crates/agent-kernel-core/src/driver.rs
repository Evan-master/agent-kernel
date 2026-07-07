//! Agent driver binding records.
//!
//! This module owns the compact no_std record that names which agent is
//! responsible for a device-like resource. It stores authority decisions only;
//! it never performs hardware I/O or grants capabilities.

use crate::{AgentId, DriverBindingId, ResourceId, ResourceKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverBindingRecord {
    pub id: DriverBindingId,
    pub installer: AgentId,
    pub resource: ResourceId,
    pub resource_kind: ResourceKind,
    pub driver: AgentId,
}

impl DriverBindingRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: DriverBindingId::new(0),
            installer: AgentId::new(0),
            resource: ResourceId::new(0),
            resource_kind: ResourceKind::Device,
            driver: AgentId::new(0),
        }
    }
}
