//! Host-testable classification of one native ring-3 return boundary.
//!
//! This architecture-library child accepts only fixed-width evidence captured
//! after a single run. It distinguishes the Agent Call gate, PIT expiry, and
//! one supported Agent exception; frame validation and scheduler mutation
//! remain in the bare-metal adapter.

pub const INVALID_OPCODE_VECTOR: u8 = 6;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NativeAgentFault {
    InvalidOpcode,
}

impl NativeAgentFault {
    pub const fn vector(self) -> u8 {
        match self {
            Self::InvalidOpcode => INVALID_OPCODE_VECTOR,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NativeRunBoundary {
    AgentCall,
    QuantumExpired,
    AgentFault(NativeAgentFault),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NativeRunBoundaryError {
    InvalidEvidence,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NativeRunBoundaryEvidence {
    agent_call_count: u8,
    timer_irq_count: u8,
    agent_fault_count: u8,
    agent_call_seen: bool,
    timer_irq_seen: bool,
    preempted: bool,
    agent_fault_seen: bool,
    agent_fault_vector: u8,
}

impl NativeRunBoundaryEvidence {
    pub const fn new(
        agent_call_count: u8,
        timer_irq_count: u8,
        agent_fault_count: u8,
        agent_call_seen: bool,
        timer_irq_seen: bool,
        preempted: bool,
        agent_fault_seen: bool,
        agent_fault_vector: u8,
    ) -> Self {
        Self {
            agent_call_count,
            timer_irq_count,
            agent_fault_count,
            agent_call_seen,
            timer_irq_seen,
            preempted,
            agent_fault_seen,
            agent_fault_vector,
        }
    }

    pub const fn classify(self) -> Result<NativeRunBoundary, NativeRunBoundaryError> {
        if self.agent_call_count == 1
            && self.timer_irq_count == 0
            && self.agent_fault_count == 0
            && self.agent_call_seen
            && !self.timer_irq_seen
            && !self.preempted
            && !self.agent_fault_seen
            && self.agent_fault_vector == 0
        {
            Ok(NativeRunBoundary::AgentCall)
        } else if self.agent_call_count == 0
            && self.timer_irq_count == 1
            && self.agent_fault_count == 0
            && !self.agent_call_seen
            && self.timer_irq_seen
            && self.preempted
            && !self.agent_fault_seen
            && self.agent_fault_vector == 0
        {
            Ok(NativeRunBoundary::QuantumExpired)
        } else if self.agent_call_count == 0
            && self.timer_irq_count == 0
            && self.agent_fault_count == 1
            && !self.agent_call_seen
            && !self.timer_irq_seen
            && !self.preempted
            && self.agent_fault_seen
            && self.agent_fault_vector == INVALID_OPCODE_VECTOR
        {
            Ok(NativeRunBoundary::AgentFault(
                NativeAgentFault::InvalidOpcode,
            ))
        } else {
            Err(NativeRunBoundaryError::InvalidEvidence)
        }
    }
}
