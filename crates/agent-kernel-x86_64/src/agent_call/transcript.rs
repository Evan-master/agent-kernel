//! Fixed-capacity evidence for one native Agent Call session.
//!
//! This architecture-library module records decoded operation order and user
//! return offsets without owning CPU frames or semantic authority. Appends are
//! deterministic and leave the transcript unchanged on every failure.

use super::AgentCallOperation;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentCallTranscriptError {
    DescribeRequired,
    DuplicateDescribe,
    InvalidReturnOffset,
    Full,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentCallTranscript<const CAPACITY: usize> {
    operations: [AgentCallOperation; CAPACITY],
    return_offsets: [u32; CAPACITY],
    len: usize,
}

impl<const CAPACITY: usize> AgentCallTranscript<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            operations: [AgentCallOperation::DescribeContext; CAPACITY],
            return_offsets: [0; CAPACITY],
            len: 0,
        }
    }

    pub fn record(
        &mut self,
        operation: AgentCallOperation,
        return_offset: u32,
    ) -> Result<(), AgentCallTranscriptError> {
        if return_offset == 0 {
            return Err(AgentCallTranscriptError::InvalidReturnOffset);
        }
        if self.len == 0 && operation != AgentCallOperation::DescribeContext {
            return Err(AgentCallTranscriptError::DescribeRequired);
        }
        if self.len != 0 && operation == AgentCallOperation::DescribeContext {
            return Err(AgentCallTranscriptError::DuplicateDescribe);
        }
        if self.len == CAPACITY {
            return Err(AgentCallTranscriptError::Full);
        }

        self.operations[self.len] = operation;
        self.return_offsets[self.len] = return_offset;
        self.len += 1;
        Ok(())
    }

    pub const fn call_count(&self) -> usize {
        self.len
    }

    pub const fn address_space_switch_count(&self) -> usize {
        self.len * 2
    }

    pub fn operations(&self) -> &[AgentCallOperation] {
        &self.operations[..self.len]
    }

    pub fn return_offsets(&self) -> &[u32] {
        &self.return_offsets[..self.len]
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const CAPACITY: usize> Default for AgentCallTranscript<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}
