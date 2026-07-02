//! Agent Kernel resource descriptors.
//!
//! This module owns resource identity, classification, and parent linkage. It
//! does not perform lookup or authorization; `KernelCore` owns those stores.

use crate::ResourceId;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    Workspace,
    Memory,
    File,
    Process,
    Service,
    Network,
    Device,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Resource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub parent: Option<ResourceId>,
}
