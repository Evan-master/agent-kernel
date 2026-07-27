//! Recovery-relative Event window used by native boot Agents.

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct BootEventWindow {
    first_sequence: u64,
    through_sequence: u64,
    count: u16,
}

impl BootEventWindow {
    pub const fn new(first_sequence: u64, count: u16) -> Option<Self> {
        if first_sequence == 0 || count == 0 {
            return None;
        }
        let Some(through_sequence) = first_sequence.checked_add(count as u64 - 1) else {
            return None;
        };
        Some(Self {
            first_sequence,
            through_sequence,
            count,
        })
    }

    pub const fn first_sequence(self) -> u64 {
        self.first_sequence
    }

    pub const fn through_sequence(self) -> u64 {
        self.through_sequence
    }

    pub const fn count(self) -> u16 {
        self.count
    }
}
