use agent_kernel_hal::TpmCommandTransport;
use agent_kernel_x86_64::tpm2::{CrbIo, CrbTransport, CrbTransportError};

const CONTROL: u64 = 0xfed4_0040;
const LOCALITY: u64 = CONTROL - 0x40;
const COMMAND_BUFFER: u64 = 0xfed4_0080;
const RESPONSE_BUFFER: u64 = COMMAND_BUFFER;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum BusError {
    InvalidAddress,
}

struct ScriptedCrb {
    interface_identifier: u32,
    command_address: u64,
    response_address: u64,
    locality_state: u32,
    locality_status: u32,
    request: u32,
    status: u32,
    start: u32,
    command: [u8; 128],
    command_len: usize,
    response: [u8; 128],
    response_len: usize,
    grant_locality: bool,
    complete_command: bool,
    change_descriptor_after_start: bool,
    release_locality: bool,
    cancel_writes: u8,
    idle_writes: u8,
    relinquish_writes: u8,
}

impl ScriptedCrb {
    fn completing(response: &[u8]) -> Self {
        let mut scripted = Self {
            interface_identifier: 1 | (2 << 4) | (1 << 14),
            command_address: COMMAND_BUFFER,
            response_address: RESPONSE_BUFFER,
            locality_state: 1 << 7,
            locality_status: 0,
            request: 0,
            status: 1 << 1,
            start: 0,
            command: [0; 128],
            command_len: 0,
            response: [0; 128],
            response_len: response.len(),
            grant_locality: true,
            complete_command: true,
            change_descriptor_after_start: false,
            release_locality: true,
            cancel_writes: 0,
            idle_writes: 0,
            relinquish_writes: 0,
        };
        scripted.response[..response.len()].copy_from_slice(response);
        scripted
    }

    fn register(&self, address: u64) -> Result<u32, BusError> {
        let value = match address {
            a if a == LOCALITY => self.locality_state,
            a if a == LOCALITY + 0x0c => self.locality_status,
            a if a == LOCALITY + 0x30 => self.interface_identifier,
            a if a == LOCALITY + 0x34 => 0,
            a if a == CONTROL => self.request,
            a if a == CONTROL + 0x04 => self.status,
            a if a == CONTROL + 0x0c => self.start,
            a if a == CONTROL + 0x18 => 128,
            a if a == CONTROL + 0x1c => self.command_address as u32,
            a if a == CONTROL + 0x20 => (self.command_address >> 32) as u32,
            a if a == CONTROL + 0x24 => 128,
            a if a == CONTROL + 0x28 => {
                let changed = self.change_descriptor_after_start && self.command_len != 0;
                self.response_address as u32 + u32::from(changed) * 0x100
            }
            a if a == CONTROL + 0x2c => (self.response_address >> 32) as u32,
            _ => return Err(BusError::InvalidAddress),
        };
        Ok(value)
    }
}

impl CrbIo for ScriptedCrb {
    type Error = BusError;

    fn read_u32(&mut self, address: u64) -> Result<u32, Self::Error> {
        self.register(address)
    }

    fn write_u32(&mut self, address: u64, value: u32) -> Result<(), Self::Error> {
        match address {
            a if a == LOCALITY + 0x08 && value == 1 => {
                if self.grant_locality {
                    self.locality_state = (1 << 7) | (1 << 1);
                    self.locality_status = 1;
                }
            }
            a if a == LOCALITY + 0x08 && value == 2 => {
                self.relinquish_writes += 1;
                if self.release_locality {
                    self.locality_state = 1 << 7;
                    self.locality_status = 0;
                }
            }
            a if a == CONTROL && value == 1 => {
                self.request = 0;
                self.status &= !(1 << 1);
            }
            a if a == CONTROL && value == 2 => {
                self.idle_writes += 1;
                self.request = 0;
                self.status |= 1 << 1;
            }
            a if a == CONTROL + 0x08 => {
                self.cancel_writes += 1;
                if value == 1 {
                    self.start = 0;
                }
            }
            a if a == CONTROL + 0x0c && value == 1 => {
                self.start = u32::from(!self.complete_command);
            }
            _ => return Err(BusError::InvalidAddress),
        }
        Ok(())
    }

    fn read_bytes(&mut self, address: u64, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let offset =
            usize::try_from(address - RESPONSE_BUFFER).map_err(|_| BusError::InvalidAddress)?;
        let end = offset + bytes.len();
        if end > self.response_len {
            return Err(BusError::InvalidAddress);
        }
        bytes.copy_from_slice(&self.response[offset..end]);
        Ok(())
    }

    fn write_bytes(&mut self, address: u64, bytes: &[u8]) -> Result<(), Self::Error> {
        if address != COMMAND_BUFFER || bytes.len() > self.command.len() {
            return Err(BusError::InvalidAddress);
        }
        self.command[..bytes.len()].copy_from_slice(bytes);
        self.command_len = bytes.len();
        Ok(())
    }
}

#[test]
fn crb_rejects_incompatible_interface_identifiers_before_locality_access() {
    let cases = [
        (
            (2 << 4) | (1 << 14),
            CrbTransportError::UnsupportedInterface { interface_type: 0 },
        ),
        (
            1 | (4 << 4) | (1 << 14),
            CrbTransportError::UnsupportedInterfaceVersion {
                interface_version: 4,
            },
        ),
        (1 | (2 << 4), CrbTransportError::MissingCrbCapability),
        (
            1 | (2 << 4) | (1 << 14) | (1 << 15),
            CrbTransportError::InvalidInterfaceIdentifier {
                value: 1 | (2 << 4) | (1 << 14) | (1 << 15),
            },
        ),
    ];

    for (identifier, expected) in cases {
        let mut io = ScriptedCrb::completing(&success_response());
        io.interface_identifier = identifier;
        let mut transport = CrbTransport::new(io, CONTROL, 3).unwrap();
        let mut output = [0; 16];

        assert_eq!(transport.execute(&command(), &mut output), Err(expected));
        let io = transport.into_io();
        assert_eq!(io.command_len, 0);
        assert_eq!(io.relinquish_writes, 0);
    }
}

#[test]
fn crb_constructor_rejects_a_misaligned_locality_window() {
    assert!(matches!(
        CrbTransport::new(ScriptedCrb::completing(&success_response()), CONTROL + 4, 3),
        Err(CrbTransportError::InvalidControlArea {
            address: value
        }) if value == CONTROL + 4
    ));
}

#[test]
fn crb_rejects_malformed_command_headers_before_locality_access() {
    for malformed in [
        [0x80, 0x00, 0, 0, 0, 10, 0, 0, 1, 0x73],
        [0x80, 0x01, 0, 0, 0, 11, 0, 0, 1, 0x73],
    ] {
        let mut transport =
            CrbTransport::new(ScriptedCrb::completing(&success_response()), CONTROL, 3).unwrap();
        let mut output = [0; 16];
        assert_eq!(
            transport.execute(&malformed, &mut output),
            Err(CrbTransportError::InvalidCommand)
        );
        assert_eq!(transport.into_io().relinquish_writes, 0);
    }
}

#[test]
fn crb_rejects_descriptors_that_overlap_control_registers() {
    let mut command_overlap = ScriptedCrb::completing(&success_response());
    command_overlap.command_address = CONTROL;
    let mut transport = CrbTransport::new(command_overlap, CONTROL, 3).unwrap();
    let mut output = [0; 16];
    assert_eq!(
        transport.execute(&command(), &mut output),
        Err(CrbTransportError::InvalidCommandBuffer)
    );
    let io = transport.into_io();
    assert_eq!(io.command_len, 0);
    assert_eq!(io.relinquish_writes, 1);

    let mut response_overlap = ScriptedCrb::completing(&success_response());
    response_overlap.response_address = CONTROL + 4;
    let mut transport = CrbTransport::new(response_overlap, CONTROL, 3).unwrap();
    assert_eq!(
        transport.execute(&command(), &mut output),
        Err(CrbTransportError::InvalidResponseBuffer)
    );
    assert_eq!(transport.into_io().command_len, 0);
}

#[test]
fn crb_executes_one_bounded_command_and_releases_locality() {
    let response = [0x80, 0x01, 0, 0, 0, 12, 0, 0, 0, 0, 0xaa, 0xbb];
    let io = ScriptedCrb::completing(&response);
    let mut transport = CrbTransport::new(io, CONTROL, 8).unwrap();
    let command = [0x80, 0x01, 0, 0, 0, 10, 0, 0, 1, 0x73];
    let mut output = [0; 32];

    let length = transport.execute(&command, &mut output).unwrap();

    assert_eq!(length, response.len());
    assert_eq!(&output[..length], response);
    let io = transport.into_io();
    assert_eq!(&io.command[..io.command_len], command);
    assert_eq!(io.idle_writes, 1);
    assert_eq!(io.relinquish_writes, 1);
}

#[test]
fn crb_refuses_to_write_without_locality() {
    let mut io = ScriptedCrb::completing(&success_response());
    io.grant_locality = false;
    let mut transport = CrbTransport::new(io, CONTROL, 3).unwrap();
    let mut output = [0; 16];

    assert_eq!(
        transport.execute(&command(), &mut output),
        Err(CrbTransportError::LocalityTimeout)
    );
    let io = transport.into_io();
    assert_eq!(io.command_len, 0);
    assert_eq!(io.relinquish_writes, 1);
}

#[test]
fn command_timeout_cancels_idles_and_relinquishes() {
    let mut io = ScriptedCrb::completing(&success_response());
    io.complete_command = false;
    let mut transport = CrbTransport::new(io, CONTROL, 3).unwrap();
    let mut output = [0; 16];

    assert_eq!(
        transport.execute(&command(), &mut output),
        Err(CrbTransportError::CommandTimeout)
    );

    let io = transport.into_io();
    assert!(io.cancel_writes >= 2);
    assert_eq!(io.idle_writes, 1);
    assert_eq!(io.relinquish_writes, 1);
}

#[test]
fn crb_rejects_oversized_responses_and_descriptor_changes() {
    let oversized = [0x80, 0x01, 0, 0, 0, 32, 0, 0, 0, 0];
    let mut transport = CrbTransport::new(ScriptedCrb::completing(&oversized), CONTROL, 4).unwrap();
    let mut output = [0; 16];
    assert_eq!(
        transport.execute(&command(), &mut output),
        Err(CrbTransportError::ResponseTooLarge {
            declared: 32,
            capacity: 16
        })
    );

    let mut io = ScriptedCrb::completing(&success_response());
    io.change_descriptor_after_start = true;
    let mut transport = CrbTransport::new(io, CONTROL, 4).unwrap();
    assert_eq!(
        transport.execute(&command(), &mut output),
        Err(CrbTransportError::DescriptorChanged)
    );
}

#[test]
fn failed_cleanup_poisoning_blocks_every_later_command() {
    let mut io = ScriptedCrb::completing(&success_response());
    io.release_locality = false;
    let mut transport = CrbTransport::new(io, CONTROL, 3).unwrap();
    let mut output = [0; 16];

    assert_eq!(
        transport.execute(&command(), &mut output),
        Err(CrbTransportError::CleanupFailed)
    );
    assert_eq!(
        transport.execute(&command(), &mut output),
        Err(CrbTransportError::Poisoned)
    );
    assert_eq!(transport.into_io().command_len, command().len());
}

#[test]
fn seizure_and_fatal_status_fail_closed_and_relinquish_locality() {
    let mut seized = ScriptedCrb::completing(&success_response());
    seized.locality_status = 1 << 1;
    let mut transport = CrbTransport::new(seized, CONTROL, 3).unwrap();
    let mut output = [0; 16];
    assert_eq!(
        transport.execute(&command(), &mut output),
        Err(CrbTransportError::LocalitySeized)
    );
    let seized = transport.into_io();
    assert_eq!(seized.command_len, 0);
    assert_eq!(seized.relinquish_writes, 1);

    let mut fatal = ScriptedCrb::completing(&success_response());
    fatal.status |= 1;
    let mut transport = CrbTransport::new(fatal, CONTROL, 3).unwrap();
    assert_eq!(
        transport.execute(&command(), &mut output),
        Err(CrbTransportError::FatalStatus)
    );
    let fatal = transport.into_io();
    assert_eq!(fatal.command_len, 0);
    assert_eq!(fatal.relinquish_writes, 1);
}

fn command() -> [u8; 10] {
    [0x80, 0x01, 0, 0, 0, 10, 0, 0, 1, 0x73]
}

fn success_response() -> [u8; 10] {
    [0x80, 0x01, 0, 0, 0, 10, 0, 0, 0, 0]
}
