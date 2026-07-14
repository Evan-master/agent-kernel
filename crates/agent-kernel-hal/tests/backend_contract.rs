use agent_kernel_core::{
    AgentId, DriverBindingId, DriverCommandId, DriverCommandKind, DriverCommandPayload,
    DriverCommandRequest, DriverCommandResult, ResourceId,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};

struct RecordingBackend {
    executions: u64,
}

impl DriverBackend for RecordingBackend {
    fn execute(&mut self, request: DriverCommandRequest) -> DriverCommandOutcome {
        self.executions += 1;
        if request.kind == DriverCommandKind::Write {
            DriverCommandOutcome::Completed(DriverCommandResult {
                code: 0,
                value: request.payload.value,
            })
        } else {
            DriverCommandOutcome::Failed(DriverCommandResult { code: 1, value: 0 })
        }
    }
}

fn request(kind: DriverCommandKind) -> DriverCommandRequest {
    DriverCommandRequest {
        command: DriverCommandId::new(1),
        binding: DriverBindingId::new(1),
        resource: ResourceId::new(1),
        driver: AgentId::new(1),
        cause: None,
        invocation: None,
        kind,
        payload: DriverCommandPayload {
            opcode: 3,
            value: 11,
        },
    }
}

#[test]
fn backend_executes_immutable_request_and_returns_terminal_outcome() {
    let mut backend = RecordingBackend { executions: 0 };

    let completed = backend.execute(request(DriverCommandKind::Write));
    let failed = backend.execute(request(DriverCommandKind::Read));

    assert_eq!(backend.executions, 2);
    assert_eq!(
        completed,
        DriverCommandOutcome::Completed(DriverCommandResult { code: 0, value: 11 })
    );
    assert_eq!(
        completed.result(),
        DriverCommandResult { code: 0, value: 11 }
    );
    assert_eq!(
        failed,
        DriverCommandOutcome::Failed(DriverCommandResult { code: 1, value: 0 })
    );
}
