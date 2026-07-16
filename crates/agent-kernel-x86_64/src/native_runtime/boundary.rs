//! Host-testable classification of one native ring-3 return boundary.
//!
//! This architecture-library child accepts only fixed-width evidence captured
//! after a single run. It distinguishes the Agent Call gate, PIT expiry, and
//! supported Agent exceptions; frame validation and scheduler mutation
//! remain in the bare-metal adapter.

pub const INVALID_OPCODE_VECTOR: u8 = 6;
pub const GENERAL_PROTECTION_VECTOR: u8 = 13;
pub const PAGE_FAULT_VECTOR: u8 = 14;
pub const PAGE_FAULT_MAX_ERROR_CODE: u64 = 0x0fff;
pub const LOWER_CANONICAL_USER_MAX: u64 = 0x0000_7fff_ffff_ffff;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NativeAgentFault {
    InvalidOpcode,
    GeneralProtection { error_code: u32 },
    PageFault { error_code: u16, address: u64 },
}

impl NativeAgentFault {
    pub const fn vector(self) -> u8 {
        match self {
            Self::InvalidOpcode => INVALID_OPCODE_VECTOR,
            Self::GeneralProtection { .. } => GENERAL_PROTECTION_VECTOR,
            Self::PageFault { .. } => PAGE_FAULT_VECTOR,
        }
    }

    pub const fn error_code(self) -> u32 {
        match self {
            Self::InvalidOpcode => 0,
            Self::GeneralProtection { error_code } => error_code,
            Self::PageFault { error_code, .. } => error_code as u32,
        }
    }

    pub const fn fault_address(self) -> Option<u64> {
        match self {
            Self::PageFault { address, .. } => Some(address),
            Self::InvalidOpcode | Self::GeneralProtection { .. } => None,
        }
    }

    pub const fn detail(self) -> u64 {
        match self {
            Self::PageFault {
                error_code,
                address,
            } => ((PAGE_FAULT_VECTOR as u64) << 60) | ((error_code as u64) << 48) | address,
            Self::InvalidOpcode | Self::GeneralProtection { .. } => {
                (self.vector() as u64) | (self.error_code() as u64) << 8
            }
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
    agent_fault_error_code: u64,
    agent_fault_address: u64,
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
        agent_fault_error_code: u64,
        agent_fault_address: u64,
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
            agent_fault_error_code,
            agent_fault_address,
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
            && self.agent_fault_error_code == 0
            && self.agent_fault_address == 0
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
            && self.agent_fault_error_code == 0
            && self.agent_fault_address == 0
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
            && self.agent_fault_error_code == 0
            && self.agent_fault_address == 0
        {
            Ok(NativeRunBoundary::AgentFault(
                NativeAgentFault::InvalidOpcode,
            ))
        } else if self.agent_call_count == 0
            && self.timer_irq_count == 0
            && self.agent_fault_count == 1
            && !self.agent_call_seen
            && !self.timer_irq_seen
            && !self.preempted
            && self.agent_fault_seen
            && self.agent_fault_vector == GENERAL_PROTECTION_VECTOR
            && self.agent_fault_error_code <= u32::MAX as u64
            && self.agent_fault_address == 0
        {
            Ok(NativeRunBoundary::AgentFault(
                NativeAgentFault::GeneralProtection {
                    error_code: self.agent_fault_error_code as u32,
                },
            ))
        } else if self.agent_call_count == 0
            && self.timer_irq_count == 0
            && self.agent_fault_count == 1
            && !self.agent_call_seen
            && !self.timer_irq_seen
            && !self.preempted
            && self.agent_fault_seen
            && self.agent_fault_vector == PAGE_FAULT_VECTOR
            && self.agent_fault_error_code <= PAGE_FAULT_MAX_ERROR_CODE
            && self.agent_fault_address <= LOWER_CANONICAL_USER_MAX
        {
            Ok(NativeRunBoundary::AgentFault(NativeAgentFault::PageFault {
                error_code: self.agent_fault_error_code as u16,
                address: self.agent_fault_address,
            }))
        } else {
            Err(NativeRunBoundaryError::InvalidEvidence)
        }
    }
}
