//! Immutable native Worker Capsule for reclaimed address-space execution.
//!
//! This bare-metal image child owns a compact three-call payload and its exact
//! digest, nonce, result, and return-offset transcript. The adjacent assembly
//! file remains the auditable byte source.

use agent_kernel_core::{AgentImageDigest, TaskResult};
use agent_kernel_x86_64::agent_call::AgentCallOperation;

const NONCE: u64 = 0xe77c_e007;
const RESULT: TaskResult = TaskResult {
    code: 0x0e07,
    value: 0xe77c_0007,
};
const OPERATIONS: [AgentCallOperation; 3] = [
    AgentCallOperation::DescribeContext,
    AgentCallOperation::SubmitTaskResult,
    AgentCallOperation::CompleteTask,
];
const RETURN_OFFSETS: [u32; 3] = [46, 67, 76];

const CAPSULE: [u8; 110] = [
    b'A', b'G', b'N', b'T', b'I', b'M', b'G', 0, // magic
    1, 0, 1, 0, 1, 0, 0, 0, // format, architecture, kind, flags
    1, 0, 1, 0, 0, 0, 0, 0, // ABI, entry version, entry offset
    78, 0, 0, 0, 0, 0, 0, 0, // code length, reserved
    0x53, 0x5b, 0x48, 0xb8, 0x00, 0x10, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x80, 0x38, 0x00, 0x74,
    0xfb, 0x48, 0xb8, 0x41, 0x47, 0x4e, 0x54, 0x43, 0x41, 0x4c, 0x4c, 0xbb, 0x01, 0x00, 0x00, 0x00,
    0xb9, 0x01, 0x00, 0x00, 0x00, 0x31, 0xd2, 0xbe, 0x07, 0xe0, 0x7c, 0xe7, 0xcd, 0x90, 0xb9, 0x04,
    0x00, 0x00, 0x00, 0x31, 0xd2, 0x41, 0xba, 0x07, 0x0e, 0x00, 0x00, 0x41, 0xbb, 0x07, 0x00, 0x7c,
    0xe7, 0xcd, 0x90, 0xb9, 0x03, 0x00, 0x00, 0x00, 0x31, 0xd2, 0xcd, 0x90, 0xeb, 0xfe,
];

const DIGEST: AgentImageDigest = AgentImageDigest::new([
    0x99, 0x42, 0x91, 0x66, 0x3b, 0x15, 0x05, 0x74, 0x48, 0x3b, 0x98, 0x7d, 0x77, 0x33, 0xd1, 0xb1,
    0x39, 0x88, 0x02, 0xab, 0x73, 0xd5, 0x86, 0x5c, 0x66, 0xfa, 0x9b, 0x4c, 0xf0, 0xf0, 0x6d, 0xf0,
]);

#[derive(Copy, Clone)]
pub(crate) struct BootReuseWorkerImage;

impl BootReuseWorkerImage {
    pub(crate) const fn bytes(self) -> &'static [u8] {
        &CAPSULE
    }

    pub(crate) const fn digest(self) -> AgentImageDigest {
        DIGEST
    }

    pub(crate) const fn nonce(self) -> u64 {
        NONCE
    }

    pub(crate) const fn result(self) -> TaskResult {
        RESULT
    }

    pub(crate) const fn expected_operations(self) -> [AgentCallOperation; 3] {
        OPERATIONS
    }

    pub(crate) const fn expected_return_offsets(self) -> [u32; 3] {
        RETURN_OFFSETS
    }
}

pub(crate) const fn reuse_worker() -> BootReuseWorkerImage {
    BootReuseWorkerImage
}
