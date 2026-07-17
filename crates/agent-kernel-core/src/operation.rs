//! Agent-native operation model.
//!
//! This module owns the operation vocabulary and fixed-width operation set used
//! by capabilities. It deliberately avoids POSIX-style permission bits.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Observe,
    Act,
    Verify,
    Checkpoint,
    Rollback,
    Delegate,
}

impl Operation {
    const fn bit(self) -> u16 {
        match self {
            Self::Observe => 1 << 0,
            Self::Act => 1 << 1,
            Self::Verify => 1 << 2,
            Self::Checkpoint => 1 << 3,
            Self::Rollback => 1 << 4,
            Self::Delegate => 1 << 5,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OperationSet(u16);

impl OperationSet {
    const KNOWN_BITS: u16 = (1 << 6) - 1;

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn only(operation: Operation) -> Self {
        Self(operation.bit())
    }

    pub const fn with(self, operation: Operation) -> Self {
        Self(self.0 | operation.bit())
    }

    pub const fn allows(self, operation: Operation) -> bool {
        self.0 & operation.bit() != 0
    }

    pub const fn is_subset_of(self, other: Self) -> bool {
        self.0 & !other.0 == 0
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn from_bits(bits: u16) -> Option<Self> {
        if bits & !Self::KNOWN_BITS == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }
}
