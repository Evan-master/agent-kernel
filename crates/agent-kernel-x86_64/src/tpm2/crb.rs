//! TPM 2.0 Command Response Buffer transport.
//!
//! The x86 machine layer owns locality-zero state, bounded polling, descriptor
//! validation, and cleanup. Callers see only the allocation-free HAL contract.

use agent_kernel_hal::TpmCommandTransport;
use core::sync::atomic::{compiler_fence, Ordering};

mod descriptor;
mod registers;
use descriptor::{BufferDescriptor, BufferDescriptorError, LOCALITY_BYTES};
use registers::*;

/// Minimal volatile-I/O contract used by the CRB state machine.
pub trait CrbIo {
    type Error;

    fn read_u32(&mut self, address: u64) -> Result<u32, Self::Error>;
    fn write_u32(&mut self, address: u64, value: u32) -> Result<(), Self::Error>;
    fn read_bytes(&mut self, address: u64, bytes: &mut [u8]) -> Result<(), Self::Error>;
    fn write_bytes(&mut self, address: u64, bytes: &[u8]) -> Result<(), Self::Error>;
}

/// A bounded, polling CRB transport for locality zero.
pub struct CrbTransport<I> {
    io: I,
    control_area: u64,
    poll_budget: u32,
    poisoned: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CrbTransportError<E> {
    InvalidControlArea { address: u64 },
    InvalidPollBudget,
    InvalidCommand,
    ResponseBufferTooSmall,
    Io(E),
    UnsupportedInterface { interface_type: u8 },
    UnsupportedInterfaceVersion { interface_version: u8 },
    MissingCrbCapability,
    InvalidInterfaceIdentifier { value: u32 },
    FatalStatus,
    LocalitySeized,
    LocalityTimeout,
    ReadyTimeout,
    InvalidCommandBuffer,
    InvalidResponseBuffer,
    CommandTooLarge { length: usize, capacity: usize },
    CommandTimeout,
    DescriptorChanged,
    InvalidResponseHeader,
    ResponseTooLarge { declared: usize, capacity: usize },
    CleanupFailed,
    Poisoned,
}

impl<I: CrbIo> CrbTransport<I> {
    pub fn new(
        io: I,
        control_area: u64,
        poll_budget: u32,
    ) -> Result<Self, CrbTransportError<I::Error>> {
        let valid_control_area = control_area.is_multiple_of(4)
            && control_area
                .checked_sub(LOCALITY_OFFSET)
                .is_some_and(|base| {
                    base.is_multiple_of(LOCALITY_BYTES)
                        && base.checked_add(LOCALITY_BYTES).is_some()
                });
        if !valid_control_area {
            return Err(CrbTransportError::InvalidControlArea {
                address: control_area,
            });
        }
        if poll_budget == 0 {
            return Err(CrbTransportError::InvalidPollBudget);
        }
        Ok(Self {
            io,
            control_area,
            poll_budget,
            poisoned: false,
        })
    }

    pub fn into_io(self) -> I {
        self.io
    }

    fn locality_address(&self, offset: u64) -> u64 {
        self.control_area - LOCALITY_OFFSET + offset
    }

    fn control_address(&self, offset: u64) -> u64 {
        self.control_area + offset
    }

    fn read_locality(&mut self, offset: u64) -> Result<u32, CrbTransportError<I::Error>> {
        let address = self.locality_address(offset);
        self.io.read_u32(address).map_err(CrbTransportError::Io)
    }

    fn write_locality(
        &mut self,
        offset: u64,
        value: u32,
    ) -> Result<(), CrbTransportError<I::Error>> {
        let address = self.locality_address(offset);
        self.io
            .write_u32(address, value)
            .map_err(CrbTransportError::Io)
    }

    fn read_control(&mut self, offset: u64) -> Result<u32, CrbTransportError<I::Error>> {
        let address = self.control_address(offset);
        self.io.read_u32(address).map_err(CrbTransportError::Io)
    }

    fn write_control(
        &mut self,
        offset: u64,
        value: u32,
    ) -> Result<(), CrbTransportError<I::Error>> {
        let address = self.control_address(offset);
        self.io
            .write_u32(address, value)
            .map_err(CrbTransportError::Io)
    }

    fn ensure_interface(&mut self) -> Result<(), CrbTransportError<I::Error>> {
        let identifier = self.read_locality(INTF_ID)?;
        let interface_type = (identifier & INTF_TYPE_MASK) as u8;
        if u32::from(interface_type) != INTF_TYPE_CRB {
            return Err(CrbTransportError::UnsupportedInterface { interface_type });
        }
        let interface_version = ((identifier >> INTF_VERSION_SHIFT) & INTF_VERSION_MASK) as u8;
        if u32::from(interface_version) > INTF_VERSION_MAX_SUPPORTED {
            return Err(CrbTransportError::UnsupportedInterfaceVersion { interface_version });
        }
        if identifier & INTF_CAP_CRB == 0 {
            return Err(CrbTransportError::MissingCrbCapability);
        }
        if identifier & INTF_RESERVED_MASK != 0 {
            return Err(CrbTransportError::InvalidInterfaceIdentifier { value: identifier });
        }
        Ok(())
    }

    fn locality_is_owned(&mut self) -> Result<bool, CrbTransportError<I::Error>> {
        let state = self.read_locality(LOC_STATE)?;
        let status = self.read_locality(LOC_STS)?;
        if status & LOC_STS_SEIZED != 0 {
            return Err(CrbTransportError::LocalitySeized);
        }
        Ok(state & LOC_STATE_VALID != 0
            && state & LOC_STATE_ASSIGNED != 0
            && state & LOC_STATE_ACTIVE_MASK == 0
            && status & LOC_STS_GRANTED != 0)
    }

    fn acquire_locality(&mut self) -> Result<(), CrbTransportError<I::Error>> {
        if self.locality_is_owned()? {
            return Ok(());
        }
        self.write_locality(LOC_CTRL, LOC_CTRL_REQUEST_ACCESS)?;
        for _ in 0..self.poll_budget {
            if self.locality_is_owned()? {
                return Ok(());
            }
        }
        Err(CrbTransportError::LocalityTimeout)
    }

    fn ensure_not_fatal(&mut self) -> Result<(), CrbTransportError<I::Error>> {
        if self.read_control(CTRL_STS)? & CTRL_STS_FATAL != 0 {
            return Err(CrbTransportError::FatalStatus);
        }
        Ok(())
    }

    fn enter_ready(&mut self) -> Result<(), CrbTransportError<I::Error>> {
        self.write_control(CTRL_REQ, CTRL_REQ_CMD_READY)?;
        for _ in 0..self.poll_budget {
            self.ensure_not_fatal()?;
            let request = self.read_control(CTRL_REQ)?;
            let status = self.read_control(CTRL_STS)?;
            if request & CTRL_REQ_CMD_READY == 0 && status & CTRL_STS_IDLE == 0 {
                return Ok(());
            }
        }
        Err(CrbTransportError::ReadyTimeout)
    }

    fn read_descriptor(&mut self) -> Result<BufferDescriptor, CrbTransportError<I::Error>> {
        let command_size = self.read_control(CTRL_CMD_SIZE)?;
        let command_low = u64::from(self.read_control(CTRL_CMD_LADDR)?);
        let command_high = u64::from(self.read_control(CTRL_CMD_HADDR)?);
        let response_size = self.read_control(CTRL_RSP_SIZE)?;
        let response_low = u64::from(self.read_control(CTRL_RSP_LADDR)?);
        let response_high = u64::from(self.read_control(CTRL_RSP_HADDR)?);
        BufferDescriptor::new(
            self.locality_address(0),
            command_size,
            command_low | (command_high << 32),
            response_size,
            response_low | (response_high << 32),
        )
        .map_err(|error| match error {
            BufferDescriptorError::InvalidCommand => CrbTransportError::InvalidCommandBuffer,
            BufferDescriptorError::InvalidResponse => CrbTransportError::InvalidResponseBuffer,
        })
    }

    fn wait_for_completion(&mut self) -> Result<(), CrbTransportError<I::Error>> {
        self.write_control(CTRL_START, CTRL_START_START)?;
        for _ in 0..self.poll_budget {
            self.ensure_not_fatal()?;
            if self.read_control(CTRL_START)? & CTRL_START_START == 0 {
                return Ok(());
            }
        }
        Err(CrbTransportError::CommandTimeout)
    }

    fn read_response(
        &mut self,
        descriptor: BufferDescriptor,
        response: &mut [u8],
    ) -> Result<usize, CrbTransportError<I::Error>> {
        let mut header = [0_u8; TPM_HEADER_SIZE];
        self.io
            .read_bytes(descriptor.response_address, &mut header)
            .map_err(CrbTransportError::Io)?;
        let tag = u16::from_be_bytes([header[0], header[1]]);
        let declared = u32::from_be_bytes([header[2], header[3], header[4], header[5]]) as usize;
        if !matches!(tag, TPM_ST_NO_SESSIONS | TPM_ST_SESSIONS) || declared < TPM_HEADER_SIZE {
            return Err(CrbTransportError::InvalidResponseHeader);
        }
        let capacity = response.len().min(descriptor.response_size);
        if declared > capacity {
            return Err(CrbTransportError::ResponseTooLarge { declared, capacity });
        }
        response[..TPM_HEADER_SIZE].copy_from_slice(&header);
        if declared > TPM_HEADER_SIZE {
            let body_address = descriptor
                .response_address
                .checked_add(TPM_HEADER_SIZE as u64)
                .ok_or(CrbTransportError::InvalidResponseBuffer)?;
            self.io
                .read_bytes(body_address, &mut response[TPM_HEADER_SIZE..declared])
                .map_err(CrbTransportError::Io)?;
        }
        Ok(declared)
    }

    fn cleanup(&mut self) -> bool {
        let mut clean = true;
        let active = match self.read_control(CTRL_START) {
            Ok(start) => start & CTRL_START_START != 0,
            Err(_) => {
                clean = false;
                true
            }
        };
        if active {
            clean &= self.write_control(CTRL_CANCEL, CTRL_CANCEL_CANCEL).is_ok();
            let mut cancelled = false;
            for _ in 0..self.poll_budget {
                match self.read_control(CTRL_START) {
                    Ok(start) if start & CTRL_START_START == 0 => {
                        cancelled = true;
                        break;
                    }
                    Ok(_) => {}
                    Err(_) => {
                        clean = false;
                        break;
                    }
                }
            }
            clean &= cancelled;
        }
        clean &= self.write_control(CTRL_CANCEL, 0).is_ok();
        clean &= self.write_control(CTRL_REQ, CTRL_REQ_GO_IDLE).is_ok();
        let mut idle = false;
        for _ in 0..self.poll_budget {
            match (self.read_control(CTRL_REQ), self.read_control(CTRL_STS)) {
                (Ok(request), Ok(status))
                    if request & CTRL_REQ_GO_IDLE == 0 && status & CTRL_STS_IDLE != 0 =>
                {
                    idle = true;
                    break;
                }
                (Ok(_), Ok(_)) => {}
                _ => {
                    clean = false;
                    break;
                }
            }
        }
        clean &= idle;
        let relinquished = self.relinquish_locality();
        clean && relinquished
    }

    fn relinquish_locality(&mut self) -> bool {
        let mut clean = self.write_locality(LOC_CTRL, LOC_CTRL_RELINQUISH).is_ok();
        let mut relinquished = false;
        for _ in 0..self.poll_budget {
            match self.read_locality(LOC_STATE) {
                Ok(state) if state & LOC_STATE_ASSIGNED == 0 => {
                    relinquished = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => {
                    clean = false;
                    break;
                }
            }
        }
        clean && relinquished
    }

    fn transact_with_locality(
        &mut self,
        command: &[u8],
        response: &mut [u8],
    ) -> Result<usize, CrbTransportError<I::Error>> {
        self.ensure_not_fatal()?;
        self.enter_ready()?;
        let descriptor = self.read_descriptor()?;
        if command.len() > descriptor.command_size {
            return Err(CrbTransportError::CommandTooLarge {
                length: command.len(),
                capacity: descriptor.command_size,
            });
        }
        self.io
            .write_bytes(descriptor.command_address, command)
            .map_err(CrbTransportError::Io)?;
        compiler_fence(Ordering::SeqCst);
        self.wait_for_completion()?;
        compiler_fence(Ordering::SeqCst);
        if self.read_descriptor()? != descriptor {
            return Err(CrbTransportError::DescriptorChanged);
        }
        self.read_response(descriptor, response)
    }
}

impl<I: CrbIo> TpmCommandTransport for CrbTransport<I> {
    type Error = CrbTransportError<I::Error>;

    fn execute(&mut self, command: &[u8], response: &mut [u8]) -> Result<usize, Self::Error> {
        if self.poisoned {
            return Err(CrbTransportError::Poisoned);
        }
        if !command_header_is_valid(command) {
            return Err(CrbTransportError::InvalidCommand);
        }
        if response.len() < TPM_HEADER_SIZE {
            return Err(CrbTransportError::ResponseBufferTooSmall);
        }

        self.ensure_interface()?;
        if let Err(error) = self.acquire_locality() {
            if !self.relinquish_locality() {
                self.poisoned = true;
            }
            return Err(error);
        }
        let result = self.transact_with_locality(command, response);
        if !self.cleanup() {
            self.poisoned = true;
            if result.is_ok() {
                return Err(CrbTransportError::CleanupFailed);
            }
        }
        result
    }
}

fn command_header_is_valid(command: &[u8]) -> bool {
    if command.len() < TPM_HEADER_SIZE {
        return false;
    }
    let tag = u16::from_be_bytes([command[0], command[1]]);
    let declared = u32::from_be_bytes([command[2], command[3], command[4], command[5]]) as usize;
    matches!(tag, TPM_ST_NO_SESSIONS | TPM_ST_SESSIONS) && declared == command.len()
}
