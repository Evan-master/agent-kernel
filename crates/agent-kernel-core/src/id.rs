//! Typed identifiers for Agent Kernel resources.
//!
//! This module owns small copyable ID wrappers. It has no dependencies and is
//! safe to use from no_std kernel code, tests, and host simulators.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentId(u64);

impl AgentId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ResourceId(u64);

impl ResourceId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CapabilityId(u64);

impl CapabilityId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ActionId(u64);

impl ActionId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ObservationId(u64);

impl ObservationId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MessageId(u64);

impl MessageId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MemoryCellId(u64);

impl MemoryCellId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CheckpointId(u64);

impl CheckpointId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TaskId(u64);

impl TaskId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IntentId(u64);

impl IntentId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}
