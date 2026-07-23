//! Validated ATA task-file endpoint configuration.

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AtaDrive {
    Master = 0,
    Slave = 1,
}

impl AtaDrive {
    pub(crate) const fn device_select(self) -> u8 {
        0x40 | ((self as u8) << 4)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AtaPioConfigError {
    ZeroPollBudget,
    ZeroCommandBase,
    ZeroControlBase,
    CommandSpanOverflow,
    ControlOverlapsCommandSpan,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AtaPioConfig {
    command_base: u16,
    control_base: u16,
    drive: AtaDrive,
    poll_budget: u32,
}

impl AtaPioConfig {
    pub const fn new(
        command_base: u16,
        control_base: u16,
        drive: AtaDrive,
        poll_budget: u32,
    ) -> Result<Self, AtaPioConfigError> {
        if poll_budget == 0 {
            return Err(AtaPioConfigError::ZeroPollBudget);
        }
        if command_base == 0 {
            return Err(AtaPioConfigError::ZeroCommandBase);
        }
        if control_base == 0 {
            return Err(AtaPioConfigError::ZeroControlBase);
        }
        let command_end = match command_base.checked_add(7) {
            Some(end) => end,
            None => return Err(AtaPioConfigError::CommandSpanOverflow),
        };
        if control_base >= command_base && control_base <= command_end {
            return Err(AtaPioConfigError::ControlOverlapsCommandSpan);
        }
        Ok(Self {
            command_base,
            control_base,
            drive,
            poll_budget,
        })
    }

    pub const fn command_base(self) -> u16 {
        self.command_base
    }

    pub const fn control_base(self) -> u16 {
        self.control_base
    }

    pub const fn drive(self) -> AtaDrive {
        self.drive
    }

    pub const fn poll_budget(self) -> u32 {
        self.poll_budget
    }

    pub(crate) const fn port(self, offset: u16) -> u16 {
        self.command_base + offset
    }
}
