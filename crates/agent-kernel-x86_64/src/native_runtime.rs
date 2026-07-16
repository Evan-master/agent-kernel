//! Fixed-capacity ownership registry for native Agent runtime objects.
//!
//! This x86_64 architecture-library module maps kernel-selected Agent IDs to
//! non-Copy physical runtime values. It owns deterministic insert/take and
//! guarded selection; scheduler policy, CPU instructions, and semantic state
//! stay in the kernel core and bare-metal adapter. Failed insertion returns the
//! value so callers never lose an unregistered CPU or address-space ownership
//! token.

use agent_kernel_core::AgentId;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NativeAgentRuntimeError {
    InvalidAgent,
    AgentAlreadyRegistered,
    AgentNotFound,
    ContextMismatch,
    StoreFull,
}

struct NativeAgentRuntimeSlot<T> {
    agent: AgentId,
    value: T,
}

pub struct NativeAgentRuntimeStore<T, const CAPACITY: usize> {
    slots: [Option<NativeAgentRuntimeSlot<T>>; CAPACITY],
    len: usize,
}

impl<T, const CAPACITY: usize> NativeAgentRuntimeStore<T, CAPACITY> {
    pub fn new() -> Self {
        Self {
            slots: core::array::from_fn(|_| None),
            len: 0,
        }
    }

    pub fn insert(&mut self, agent: AgentId, value: T) -> Result<(), (NativeAgentRuntimeError, T)> {
        if agent.raw() == 0 {
            return Err((NativeAgentRuntimeError::InvalidAgent, value));
        }
        if self.contains(agent) {
            return Err((NativeAgentRuntimeError::AgentAlreadyRegistered, value));
        }
        if self.len >= CAPACITY {
            return Err((NativeAgentRuntimeError::StoreFull, value));
        }

        self.slots[self.len] = Some(NativeAgentRuntimeSlot { agent, value });
        self.len += 1;
        Ok(())
    }

    pub fn take(&mut self, agent: AgentId) -> Result<T, NativeAgentRuntimeError> {
        let index = self.slots[..self.len]
            .iter()
            .position(|slot| matches!(slot, Some(slot) if slot.agent == agent))
            .ok_or(NativeAgentRuntimeError::AgentNotFound)?;
        let removed = self.slots[index]
            .take()
            .ok_or(NativeAgentRuntimeError::AgentNotFound)?;
        let last = self.len - 1;
        let mut cursor = index;
        while cursor < last {
            self.slots[cursor] = self.slots[cursor + 1].take();
            cursor += 1;
        }
        self.slots[last] = None;
        self.len = last;
        Ok(removed.value)
    }

    pub fn take_matching(
        &mut self,
        agent: AgentId,
        matches: impl FnOnce(&T) -> bool,
    ) -> Result<T, NativeAgentRuntimeError> {
        if !matches(self.get(agent)?) {
            return Err(NativeAgentRuntimeError::ContextMismatch);
        }
        self.take(agent)
    }

    pub fn contains_matching(&self, agent: AgentId, matches: impl FnOnce(&T) -> bool) -> bool {
        match self.get(agent) {
            Ok(value) => matches(value),
            Err(_) => false,
        }
    }

    pub fn get(&self, agent: AgentId) -> Result<&T, NativeAgentRuntimeError> {
        self.slots[..self.len]
            .iter()
            .find_map(|slot| match slot {
                Some(slot) if slot.agent == agent => Some(&slot.value),
                _ => None,
            })
            .ok_or(NativeAgentRuntimeError::AgentNotFound)
    }

    pub fn contains(&self, agent: AgentId) -> bool {
        self.get(agent).is_ok()
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T, const CAPACITY: usize> Default for NativeAgentRuntimeStore<T, CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}
