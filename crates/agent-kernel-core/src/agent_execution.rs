//! Kernel-owned agent execution context records.
//!
//! This module belongs to `agent-kernel-core`. It defines the fixed-capacity
//! no_std execution context attached to each registered agent. It stores only
//! deterministic kernel runtime state, not host runtime handles or model data.

use crate::{AgentId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentExecutionState {
    Idle,
    Running,
    Waiting,
    Faulted,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentExecutionContext {
    pub agent: AgentId,
    pub state: AgentExecutionState,
    pub task: Option<TaskId>,
    pub run_ticks: u64,
    pub quantum_remaining: u64,
}

impl AgentExecutionContext {
    pub(crate) const fn empty() -> Self {
        Self::idle(AgentId::new(0))
    }

    pub(crate) const fn idle(agent: AgentId) -> Self {
        Self {
            agent,
            state: AgentExecutionState::Idle,
            task: None,
            run_ticks: 0,
            quantum_remaining: 0,
        }
    }

    pub(crate) const fn running(
        agent: AgentId,
        task: TaskId,
        run_ticks: u64,
        quantum_remaining: u64,
    ) -> Self {
        Self {
            agent,
            state: AgentExecutionState::Running,
            task: Some(task),
            run_ticks,
            quantum_remaining,
        }
    }

    pub(crate) const fn waiting(agent: AgentId, task: TaskId) -> Self {
        Self {
            agent,
            state: AgentExecutionState::Waiting,
            task: Some(task),
            run_ticks: 0,
            quantum_remaining: 0,
        }
    }

    pub(crate) const fn faulted(agent: AgentId, task: TaskId) -> Self {
        Self {
            agent,
            state: AgentExecutionState::Faulted,
            task: Some(task),
            run_ticks: 0,
            quantum_remaining: 0,
        }
    }
}
