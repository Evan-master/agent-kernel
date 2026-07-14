//! Fixed-capacity task wait and signal wakeup store.
//!
//! This module belongs to `agent-kernel-core`. It owns deterministic task
//! waiting, resource-scoped signal emission, and run queue wakeup behavior with
//! no allocation, host waiting, async runtime integration, or callbacks.

use crate::{
    AgentId, CapabilityId, EventKind, KernelCore, KernelError, Operation, ResourceId,
    RunQueueEntry, SignalKey, SignalOutcome, TaskId, TaskStatus, WaiterId, WaiterRecord,
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
    pub fn wait_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        resource: ResourceId,
        signal: SignalKey,
    ) -> Result<WaiterId, KernelError> {
        self.ensure_agent_active(agent)?;
        let task_record = self.find_task(task)?;
        if task_record.resource != resource {
            return Err(KernelError::ResourceMismatch);
        }
        self.ensure_authorized_for_task(agent, capability, resource, Operation::Act, task)?;
        if task_record.status != TaskStatus::Running {
            return Err(KernelError::TaskStatusMismatch);
        }
        if task_record.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        self.ensure_agent_admitted_for_task(agent, task)?;
        if self.waiter_len >= WAITERS {
            return Err(KernelError::WaiterStoreFull);
        }
        self.ensure_event_slots(1)?;

        let waiter = WaiterId::new(self.next_waiter);
        self.next_waiter += 1;
        self.waiters[self.waiter_len] = WaiterRecord {
            id: waiter,
            task,
            agent,
            resource,
            signal,
            active: true,
        };
        self.waiter_len += 1;
        self.find_task_mut(task)?.status = TaskStatus::Waiting;
        self.set_execution_context_waiting(agent, task)?;
        self.record_wait_signal_event(
            EventKind::TaskWaiting,
            agent,
            capability,
            resource,
            Some(task),
            Some(waiter),
            signal,
            None,
        )?;
        Ok(waiter)
    }

    pub fn emit_signal(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        signal: SignalKey,
    ) -> Result<SignalOutcome, KernelError> {
        self.ensure_agent_active(agent)?;
        self.ensure_authorized(agent, capability, resource, Operation::Act)?;

        let waiter_index = self.find_matching_waiter_index(resource, signal);
        let Some(waiter_index) = waiter_index else {
            self.ensure_event_slots(1)?;
            let signal_event = self.record_wait_signal_event(
                EventKind::SignalEmitted,
                agent,
                capability,
                resource,
                None,
                None,
                signal,
                None,
            )?;
            return Ok(SignalOutcome {
                signal_event,
                woken_task: None,
                wake_event: None,
            });
        };

        let waiter = self.waiters[waiter_index];
        self.ensure_agent_admitted_for_task(waiter.agent, waiter.task)?;
        self.ensure_run_queue_capacity()?;
        self.ensure_event_slots(2)?;

        let signal_event = self.record_wait_signal_event(
            EventKind::SignalEmitted,
            agent,
            capability,
            resource,
            Some(waiter.task),
            Some(waiter.id),
            signal,
            Some(waiter.agent),
        )?;
        self.waiters[waiter_index].active = false;
        self.find_task_mut(waiter.task)?.status = TaskStatus::Accepted;
        self.set_execution_context_idle(waiter.agent)?;
        self.run_queue[self.run_queue_len] = RunQueueEntry {
            task: waiter.task,
            agent: waiter.agent,
        };
        self.run_queue_len += 1;
        let wake_event = self.record_wait_signal_event(
            EventKind::TaskWoken,
            agent,
            capability,
            resource,
            Some(waiter.task),
            Some(waiter.id),
            signal,
            Some(waiter.agent),
        )?;
        Ok(SignalOutcome {
            signal_event,
            woken_task: Some(waiter.task),
            wake_event: Some(wake_event),
        })
    }

    pub fn waiters(&self) -> &[WaiterRecord] {
        &self.waiters[..self.waiter_len]
    }

    fn find_matching_waiter_index(&self, resource: ResourceId, signal: SignalKey) -> Option<usize> {
        let mut index = 0;
        while index < self.waiter_len {
            let waiter = self.waiters[index];
            if waiter.active && waiter.resource == resource && waiter.signal == signal {
                if let Ok(task) = self.find_task(waiter.task) {
                    if task.status == TaskStatus::Waiting {
                        return Some(index);
                    }
                }
            }
            index += 1;
        }
        None
    }
}
