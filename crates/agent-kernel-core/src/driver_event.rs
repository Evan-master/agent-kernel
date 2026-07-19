//! Fixed-capacity driver binding store and events.
//!
//! This module belongs to `agent-kernel-core`. It binds active agents to
//! device-like resources under explicit delegation authority and records the
//! binding as a replayable kernel event. It performs no host I/O and does not
//! grant driver capabilities implicitly.

use crate::{
    AgentId, CapabilityId, DriverBindingId, DriverBindingRecord, Event, EventKind, KernelCore,
    KernelError, Operation, OperationSet, ResourceId, ResourceKind, VerificationRequirement,
};

impl<
        const AGENTS: usize,
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
        const MESSAGES: usize,
        const MEMORY_CELLS: usize,
        const NAMESPACE_ENTRIES: usize,
        const FAULTS: usize,
        const FAULT_HANDLERS: usize,
        const FAULT_POLICIES: usize,
        const WAITERS: usize,
        const AGENT_IMAGES: usize,
        const DRIVER_BINDINGS: usize,
        const DEVICE_EVENTS: usize,
        const DRIVER_COMMANDS: usize,
        const DRIVER_INVOCATIONS: usize,
        const RUNTIME_ADMISSIONS: usize,
    >
    KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
        MESSAGES,
        MEMORY_CELLS,
        NAMESPACE_ENTRIES,
        FAULTS,
        FAULT_HANDLERS,
        FAULT_POLICIES,
        WAITERS,
        AGENT_IMAGES,
        DRIVER_BINDINGS,
        DEVICE_EVENTS,
        DRIVER_COMMANDS,
        DRIVER_INVOCATIONS,
        RUNTIME_ADMISSIONS,
    >
{
    pub fn bind_driver(
        &mut self,
        installer: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        driver: AgentId,
    ) -> Result<DriverBindingId, KernelError> {
        self.ensure_agent_active(installer)?;
        self.ensure_agent_active(driver)?;
        let resource_record = self.find_resource(resource)?;
        Self::ensure_driver_resource(resource_record.kind)?;
        self.ensure_authorized(installer, capability, resource, Operation::Delegate)?;
        if self.find_driver_binding_for_resource(resource).is_ok() {
            return Err(KernelError::DriverBindingAlreadyExists);
        }
        if self.driver_binding_len >= DRIVER_BINDINGS {
            return Err(KernelError::DriverBindingStoreFull);
        }
        self.ensure_event_slots(1)?;

        let id = DriverBindingId::new(self.next_driver_binding);
        self.next_driver_binding += 1;
        self.driver_bindings[self.driver_binding_len] = DriverBindingRecord {
            id,
            installer,
            resource,
            resource_kind: resource_record.kind,
            driver,
        };
        self.driver_binding_len += 1;
        self.record_driver_bound_event(installer, capability, resource, id, driver)?;
        Ok(id)
    }

    pub fn driver_bindings(&self) -> &[DriverBindingRecord] {
        &self.driver_bindings[..self.driver_binding_len]
    }

    pub(crate) fn find_driver_binding_for_resource(
        &self,
        resource: ResourceId,
    ) -> Result<DriverBindingRecord, KernelError> {
        self.driver_bindings()
            .iter()
            .find(|binding| binding.resource == resource)
            .copied()
            .ok_or(KernelError::DriverBindingNotFound)
    }

    pub(crate) fn ensure_driver_resource(kind: ResourceKind) -> Result<(), KernelError> {
        match kind {
            ResourceKind::Device | ResourceKind::Network | ResourceKind::Service => Ok(()),
            ResourceKind::Workspace
            | ResourceKind::Memory
            | ResourceKind::File
            | ResourceKind::Process => Err(KernelError::ResourceKindMismatch),
        }
    }

    pub(crate) fn find_driver_binding(
        &self,
        id: DriverBindingId,
    ) -> Result<DriverBindingRecord, KernelError> {
        self.driver_bindings()
            .iter()
            .find(|binding| binding.id == id)
            .copied()
            .ok_or(KernelError::DriverBindingNotFound)
    }

    fn record_driver_bound_event(
        &mut self,
        installer: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        binding: DriverBindingId,
        driver: AgentId,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent: installer,
            kind: EventKind::DriverBound,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: Some(Operation::Delegate),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            runtime_admission: None,
            task_result: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            fault_policy: None,
            fault_policy_action: None,
            waiter: None,
            signal: None,
            target_agent: Some(driver),
            driver_binding: Some(binding),
            device_event: None,
            device_event_kind: None,
            device_event_payload: None,
            driver_command: None,
            driver_command_kind: None,
            driver_command_payload: None,
            driver_command_result: None,
            driver_invocation: None,
            driver_invocation_ticks: None,
            driver_invocation_quantum: None,
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }
}
