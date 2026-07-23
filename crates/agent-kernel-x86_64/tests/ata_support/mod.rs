use std::collections::VecDeque;

use agent_kernel_x86_64::ata::{
    AtaRegisterIo, ATA_COMMAND_IDENTIFY, ATA_COMMAND_READ_EXT, ATA_COMMAND_WRITE_EXT,
};

pub const COMMAND_BASE: u16 = 0x1f0;
pub const CONTROL_BASE: u16 = 0x3f6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegisterOperation {
    ReadU8(u16),
    WriteU8(u16, u8),
    ReadU16(u16),
    WriteU16(u16, u16),
}

pub struct RegisterDouble {
    default_status: u8,
    status_reads: VecDeque<u8>,
    data_reads: VecDeque<u16>,
    operations: Vec<RegisterOperation>,
    data_command_active: bool,
    data_words_transferred: usize,
}

impl RegisterDouble {
    pub fn ready() -> Self {
        Self {
            default_status: 0x40,
            status_reads: VecDeque::new(),
            data_reads: VecDeque::new(),
            operations: Vec::new(),
            data_command_active: false,
            data_words_transferred: 0,
        }
    }

    pub fn with_status(status: u8) -> Self {
        Self {
            default_status: status,
            status_reads: VecDeque::new(),
            data_reads: VecDeque::new(),
            operations: Vec::new(),
            data_command_active: false,
            data_words_transferred: 0,
        }
    }

    pub fn queue_statuses(&mut self, statuses: impl IntoIterator<Item = u8>) {
        self.status_reads.extend(statuses);
    }

    pub fn queue_data(&mut self, words: impl IntoIterator<Item = u16>) {
        self.data_reads.extend(words);
    }

    pub fn clear_operations(&mut self) {
        self.operations.clear();
    }

    pub fn writes_u8(&self) -> Vec<(u16, u8)> {
        self.operations
            .iter()
            .filter_map(|operation| match operation {
                RegisterOperation::WriteU8(port, value) => Some((*port, *value)),
                _ => None,
            })
            .collect()
    }

    pub fn writes_u16(&self) -> Vec<(u16, u16)> {
        self.operations
            .iter()
            .filter_map(|operation| match operation {
                RegisterOperation::WriteU16(port, value) => Some((*port, *value)),
                _ => None,
            })
            .collect()
    }
}

impl AtaRegisterIo for RegisterDouble {
    fn read_u8(&mut self, port: u16) -> u8 {
        self.operations.push(RegisterOperation::ReadU8(port));
        if port == COMMAND_BASE + 7 {
            let status = if self.data_command_active {
                0x48
            } else {
                self.default_status
            };
            self.status_reads.pop_front().unwrap_or(status)
        } else if port == CONTROL_BASE {
            self.default_status
        } else {
            0
        }
    }

    fn write_u8(&mut self, port: u16, value: u8) {
        self.operations
            .push(RegisterOperation::WriteU8(port, value));
        if port == COMMAND_BASE + 7
            && matches!(
                value,
                ATA_COMMAND_IDENTIFY | ATA_COMMAND_READ_EXT | ATA_COMMAND_WRITE_EXT
            )
        {
            self.data_command_active = true;
            self.data_words_transferred = 0;
        }
    }

    fn read_u16(&mut self, port: u16) -> u16 {
        self.operations.push(RegisterOperation::ReadU16(port));
        self.complete_data_word();
        self.data_reads.pop_front().unwrap_or(0)
    }

    fn write_u16(&mut self, port: u16, value: u16) {
        self.operations
            .push(RegisterOperation::WriteU16(port, value));
        self.complete_data_word();
    }
}

impl RegisterDouble {
    fn complete_data_word(&mut self) {
        if self.data_command_active {
            self.data_words_transferred += 1;
            if self.data_words_transferred == 256 {
                self.data_command_active = false;
            }
        }
    }
}

pub fn identify_words(sector_count: u64) -> [u16; 256] {
    let mut words = [0_u16; 256];
    words[83] = 1 << 10;
    words[100] = sector_count as u16;
    words[101] = (sector_count >> 16) as u16;
    words[102] = (sector_count >> 32) as u16;
    words[103] = (sector_count >> 48) as u16;
    words
}
