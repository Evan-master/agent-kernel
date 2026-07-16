//! Host-testable classification of one native ring-3 return boundary.
//!
//! This architecture-library child accepts only fixed-width evidence captured
//! after a single run. It distinguishes the Agent Call gate from PIT expiry;
//! frame validation and scheduler mutation remain in the bare-metal adapter.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NativeRunBoundary {
    AgentCall,
    QuantumExpired,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NativeRunBoundaryError {
    InvalidEvidence,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NativeRunBoundaryEvidence {
    agent_call_count: u8,
    timer_irq_count: u8,
    agent_call_seen: bool,
    timer_irq_seen: bool,
    preempted: bool,
}

impl NativeRunBoundaryEvidence {
    pub const fn new(
        agent_call_count: u8,
        timer_irq_count: u8,
        agent_call_seen: bool,
        timer_irq_seen: bool,
        preempted: bool,
    ) -> Self {
        Self {
            agent_call_count,
            timer_irq_count,
            agent_call_seen,
            timer_irq_seen,
            preempted,
        }
    }

    pub const fn classify(self) -> Result<NativeRunBoundary, NativeRunBoundaryError> {
        if self.agent_call_count == 1
            && self.timer_irq_count == 0
            && self.agent_call_seen
            && !self.timer_irq_seen
            && !self.preempted
        {
            Ok(NativeRunBoundary::AgentCall)
        } else if self.agent_call_count == 0
            && self.timer_irq_count == 1
            && !self.agent_call_seen
            && self.timer_irq_seen
            && self.preempted
        {
            Ok(NativeRunBoundary::QuantumExpired)
        } else {
            Err(NativeRunBoundaryError::InvalidEvidence)
        }
    }
}
