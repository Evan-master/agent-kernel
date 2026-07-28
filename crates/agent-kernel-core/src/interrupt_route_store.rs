//! Fixed-capacity Interrupt Route authority and lifecycle.
//!
//! This Core module creates Device-child route Resources, enforces live vector
//! uniqueness, and records two-phase activation and revocation. Architecture
//! code performs PCI and interrupt-controller mutation between transitions.

use crate::{
    AgentId, CapabilityId, Event, EventKind, InterruptMode, InterruptRouteRecord,
    InterruptRouteStatus, InterruptTarget, KernelCore, KernelError, Operation, OperationSet,
    ResourceCreateOutcome, ResourceId, ResourceKind,
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
    #[allow(clippy::too_many_arguments)]
    pub fn create_interrupt_route(
        &mut self,
        agent: AgentId,
        device_capability: CapabilityId,
        device: ResourceId,
        mode: InterruptMode,
        target: InterruptTarget,
        operations: OperationSet,
    ) -> Result<ResourceCreateOutcome, KernelError> {
        self.ensure_agent_active(agent)?;
        if self.find_resource(device)?.kind != ResourceKind::Device {
            return Err(KernelError::ResourceKindMismatch);
        }
        self.ensure_authorized(agent, device_capability, device, Operation::Act)?;
        for route in self
            .interrupt_routes()
            .iter()
            .filter(|route| route.occupies_route())
        {
            if route.device() == device && same_mode_slot(route.mode(), mode) {
                return Err(KernelError::InterruptRouteAlreadyExists);
            }
            if route.target() == target {
                return Err(KernelError::InterruptVectorInUse);
            }
        }
        if self.interrupt_route_len >= RESOURCES {
            return Err(KernelError::InterruptRouteStoreFull);
        }
        self.ensure_event_slots(3)?;

        let outcome = self.create_resource(
            agent,
            ResourceKind::InterruptRoute,
            Some((device, device_capability)),
            operations,
        )?;
        self.interrupt_routes[self.interrupt_route_len] =
            InterruptRouteRecord::new(outcome.resource, device, mode, target);
        self.interrupt_route_len += 1;
        self.record_interrupt_route_event(
            EventKind::InterruptRouteReserved,
            agent,
            outcome.capability,
            outcome.resource,
            Operation::Act,
        )?;
        Ok(outcome)
    }

    pub fn activate_interrupt_route(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        route: ResourceId,
    ) -> Result<Event, KernelError> {
        self.transition_interrupt_route(
            agent,
            capability,
            route,
            InterruptRouteStatus::Reserved,
            InterruptRouteStatus::Active,
            EventKind::InterruptRouteActivated,
            Operation::Act,
        )
    }

    pub fn begin_interrupt_route_revoke(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        route: ResourceId,
    ) -> Result<Event, KernelError> {
        self.transition_interrupt_route(
            agent,
            capability,
            route,
            InterruptRouteStatus::Active,
            InterruptRouteStatus::Revoking,
            EventKind::InterruptRouteRevoking,
            Operation::Rollback,
        )
    }

    pub fn complete_interrupt_route_revoke(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        route: ResourceId,
    ) -> Result<Event, KernelError> {
        self.transition_interrupt_route(
            agent,
            capability,
            route,
            InterruptRouteStatus::Revoking,
            InterruptRouteStatus::Released,
            EventKind::InterruptRouteReleased,
            Operation::Rollback,
        )
    }

    pub fn interrupt_routes(&self) -> &[InterruptRouteRecord] {
        &self.interrupt_routes[..self.interrupt_route_len]
    }

    pub fn interrupt_route(
        &self,
        resource: ResourceId,
    ) -> Result<InterruptRouteRecord, KernelError> {
        self.interrupt_routes()
            .iter()
            .find(|route| route.resource() == resource)
            .copied()
            .ok_or(KernelError::InterruptRouteNotFound)
    }

    pub(crate) fn ensure_interrupt_resource_quiescent(
        &self,
        resource: ResourceId,
    ) -> Result<(), KernelError> {
        if self.interrupt_routes().iter().any(|route| {
            route.occupies_route() && (route.resource() == resource || route.device() == resource)
        }) {
            Err(KernelError::InterruptResourceBusy)
        } else {
            Ok(())
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn transition_interrupt_route(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        route: ResourceId,
        expected: InterruptRouteStatus,
        next: InterruptRouteStatus,
        event_kind: EventKind,
        operation: Operation,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let record = self.interrupt_route(route)?;
        self.ensure_authorized(agent, capability, route, operation)?;
        if record.status() != expected {
            return Err(KernelError::InterruptRouteStatusMismatch);
        }
        self.ensure_event_slots(1)?;
        let slot = self.interrupt_routes[..self.interrupt_route_len]
            .iter_mut()
            .find(|record| record.resource() == route)
            .ok_or(KernelError::InterruptRouteNotFound)?;
        slot.set_status(next);
        self.record_interrupt_route_event(event_kind, agent, capability, route, operation)
    }

    fn record_interrupt_route_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
    ) -> Result<Event, KernelError> {
        let mut event = Event::empty();
        event.agent = agent;
        event.kind = kind;
        event.resource = Some(resource);
        event.capability = Some(capability);
        event.operation = Some(operation);
        self.record(event)
    }
}

const fn same_mode_slot(first: InterruptMode, second: InterruptMode) -> bool {
    match (first, second) {
        (InterruptMode::Msi, InterruptMode::Msi) => true,
        (
            InterruptMode::MsiX { table_entry: first },
            InterruptMode::MsiX {
                table_entry: second,
            },
        ) => first == second,
        _ => false,
    }
}
