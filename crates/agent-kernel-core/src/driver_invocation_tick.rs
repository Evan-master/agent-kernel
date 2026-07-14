//! Deterministic Driver Invocation tick accounting and preemption.
//!
//! This core module advances only a currently running invocation. Explicit
//! ticks update fixed-width counters and return expired work to queued status;
//! timer hardware and host clocks remain outside the kernel model.

use crate::{
    AgentId, DriverInvocationId, DriverInvocationStatus, Event, EventKind, KernelCore, KernelError,
    Operation,
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
    >
{
    pub fn tick_driver_invocation(
        &mut self,
        driver: AgentId,
        invocation: DriverInvocationId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(driver)?;
        let record = self.find_driver_invocation(invocation)?;
        if record.status != DriverInvocationStatus::Running || record.driver != driver {
            return Err(KernelError::DriverInvocationNotRunnable);
        }
        if record.quantum_remaining == 0 {
            return Err(KernelError::DriverInvocationQuantumInvalid);
        }
        self.find_resource(record.resource)?;
        self.ensure_agent_admitted_for_driver(driver, record.binding, record.resource)?;
        self.ensure_execution_context_running_driver(driver, invocation)?;
        self.ensure_event_slots(1)?;

        let ticks = record.run_ticks + 1;
        let quantum = record.quantum_remaining - 1;
        let kind = if quantum == 0 {
            EventKind::DriverInvocationQuantumExpired
        } else {
            EventKind::DriverInvocationTicked
        };
        let stored = self.find_driver_invocation_mut(invocation)?;
        stored.run_ticks = ticks;
        stored.quantum_remaining = quantum;
        if quantum == 0 {
            stored.status = DriverInvocationStatus::Queued;
            self.set_execution_context_idle(driver)?;
        } else {
            self.set_execution_context_running_driver(driver, invocation, ticks, quantum)?;
        }
        self.record_driver_invocation_event(
            kind,
            driver,
            None,
            invocation,
            Operation::Act,
            Some(ticks),
            Some(quantum),
        )
    }
}
