//! One-time binding of a cryptographically recovered archive generation.

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AtaDurableHead {
    Genesis,
    Recovered(u64),
}

impl AtaDurableHead {
    pub(crate) const fn next_generation(self) -> Option<u64> {
        match self {
            Self::Genesis => Some(1),
            Self::Recovered(generation) => generation.checked_add(1),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AtaDurableHeadBindError {
    AlreadyBound,
    ZeroRecoveredGeneration,
    GenerationExhausted,
}
