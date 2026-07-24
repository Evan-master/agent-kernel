use agent_kernel_hal::TpmCommandTransport;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum TransportError {
    ResponseTooSmall,
}

struct EchoTransport {
    executions: u8,
}

impl TpmCommandTransport for EchoTransport {
    type Error = TransportError;

    fn execute(&mut self, command: &[u8], response: &mut [u8]) -> Result<usize, Self::Error> {
        self.executions += 1;
        if response.len() < command.len() {
            return Err(TransportError::ResponseTooSmall);
        }
        response[..command.len()].copy_from_slice(command);
        Ok(command.len())
    }
}

#[test]
fn transport_uses_caller_owned_bounded_buffers() {
    let mut transport = EchoTransport { executions: 0 };
    let command = [0x80, 0x01, 0, 0, 0, 10, 0, 0, 1, 0x7b];
    let mut response = [0; 16];

    let response_len = transport.execute(&command, &mut response).unwrap();

    assert_eq!(response_len, command.len());
    assert_eq!(&response[..response_len], command);
    assert_eq!(transport.executions, 1);
}

#[test]
fn transport_reports_capacity_failure_without_allocating() {
    let mut transport = EchoTransport { executions: 0 };
    let mut response = [0; 2];

    assert_eq!(
        transport.execute(&[1, 2, 3], &mut response),
        Err(TransportError::ResponseTooSmall)
    );
    assert_eq!(transport.executions, 1);
}
