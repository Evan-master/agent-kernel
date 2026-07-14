//! Fixed-capacity agent execution context store.
//!
//! This module belongs to `agent-kernel-core`. It owns read-only inspection and
//! scheduler-facing transition helpers for the one-context-per-agent runtime
//! model. Context mutations are always paired with existing agent or task
//! events in their caller.

use crate::{AgentExecutionContext, AgentExecutionState, AgentId, KernelCore, KernelError, TaskId};

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
    >
{
    pub fn execution_contexts(&self) -> &[AgentExecutionContext] {
        &self.execution_contexts[..self.agent_len]
    }

    pub fn execution_context(&self, agent: AgentId) -> Result<AgentExecutionContext, KernelError> {
        self.find_execution_context(agent)
    }

    pub(crate) fn ensure_execution_context_idle(&self, agent: AgentId) -> Result<(), KernelError> {
        let context = self.find_execution_context(agent)?;
        if context.state == AgentExecutionState::Idle {
            Ok(())
        } else {
            Err(KernelError::ExecutionContextBusy)
        }
    }

    pub(crate) fn set_execution_context_idle(&mut self, agent: AgentId) -> Result<(), KernelError> {
        *self.find_execution_context_mut(agent)? = AgentExecutionContext::idle(agent);
        Ok(())
    }

    pub(crate) fn set_execution_context_running(
        &mut self,
        agent: AgentId,
        task: TaskId,
        run_ticks: u64,
        quantum_remaining: u64,
    ) -> Result<(), KernelError> {
        *self.find_execution_context_mut(agent)? =
            AgentExecutionContext::running(agent, task, run_ticks, quantum_remaining);
        Ok(())
    }

    pub(crate) fn set_execution_context_waiting(
        &mut self,
        agent: AgentId,
        task: TaskId,
    ) -> Result<(), KernelError> {
        *self.find_execution_context_mut(agent)? = AgentExecutionContext::waiting(agent, task);
        Ok(())
    }

    pub(crate) fn set_execution_context_faulted(
        &mut self,
        agent: AgentId,
        task: TaskId,
    ) -> Result<(), KernelError> {
        *self.find_execution_context_mut(agent)? = AgentExecutionContext::faulted(agent, task);
        Ok(())
    }

    pub(crate) fn clear_execution_context_for_task(&mut self, task: TaskId) {
        for context in &mut self.execution_contexts[..self.agent_len] {
            if context.task == Some(task) {
                *context = AgentExecutionContext::idle(context.agent);
            }
        }
    }

    fn find_execution_context(&self, agent: AgentId) -> Result<AgentExecutionContext, KernelError> {
        self.execution_contexts()
            .iter()
            .find(|context| context.agent == agent)
            .copied()
            .ok_or(KernelError::AgentNotFound)
    }

    fn find_execution_context_mut(
        &mut self,
        agent: AgentId,
    ) -> Result<&mut AgentExecutionContext, KernelError> {
        self.execution_contexts[..self.agent_len]
            .iter_mut()
            .find(|context| context.agent == agent)
            .ok_or(KernelError::AgentNotFound)
    }
}
