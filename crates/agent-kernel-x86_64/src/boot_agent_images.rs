//! Immutable native Worker capsules used by the BIOS boot proof.
//!
//! This x86 boot-layer module owns two independently identified payloads. The
//! precomputed digests model manifest input; the runtime loader recomputes each
//! digest before mapping either code page.

use agent_kernel_core::{AgentImageDigest, TaskResult};

const WORKER_A_NONCE: u64 = 0xa11c_e001;
const WORKER_B_NONCE: u64 = 0xb22c_e002;
const WORKER_A_RESULT: TaskResult = TaskResult {
    code: 0x0a01,
    value: 0xa11c_0001,
};
const WORKER_B_RESULT: TaskResult = TaskResult {
    code: 0x0b02,
    value: 0xb22c_0002,
};
const WORKER_A_DESCRIBE_RETURN_OFFSET: u32 = 46;
const WORKER_A_RESULT_RETURN_OFFSET: u32 = 67;
const WORKER_A_COMPLETION_RETURN_OFFSET: u32 = 76;
const WORKER_B_DESCRIBE_RETURN_OFFSET: u32 = 48;
const WORKER_B_RESULT_RETURN_OFFSET: u32 = 69;
const WORKER_B_COMPLETION_RETURN_OFFSET: u32 = 78;

const WORKER_A_CAPSULE: [u8; 110] = [
    b'A', b'G', b'N', b'T', b'I', b'M', b'G', 0, // magic
    1, 0, 1, 0, 1, 0, 0, 0, // format, architecture, kind, flags
    1, 0, 1, 0, 0, 0, 0, 0, // ABI, entry version, entry offset
    78, 0, 0, 0, 0, 0, 0, 0, // code length, reserved
    0x53, 0x5b, 0x48, 0xb8, 0x00, 0x10, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x80, 0x38, 0x00, 0x74,
    0xfb, 0x48, 0xb8, 0x41, 0x47, 0x4e, 0x54, 0x43, 0x41, 0x4c, 0x4c, 0xbb, 0x01, 0x00, 0x00, 0x00,
    0xb9, 0x01, 0x00, 0x00, 0x00, 0x31, 0xd2, 0xbe, 0x01, 0xe0, 0x1c, 0xa1, 0xcd, 0x90, 0xb9, 0x04,
    0x00, 0x00, 0x00, 0x31, 0xd2, 0x41, 0xba, 0x01, 0x0a, 0x00, 0x00, 0x41, 0xbb, 0x01, 0x00, 0x1c,
    0xa1, 0xcd, 0x90, 0xb9, 0x03, 0x00, 0x00, 0x00, 0x31, 0xd2, 0xcd, 0x90, 0xeb, 0xfe,
];

const WORKER_B_CAPSULE: [u8; 112] = [
    b'A', b'G', b'N', b'T', b'I', b'M', b'G', 0, // magic
    1, 0, 1, 0, 1, 0, 0, 0, // format, architecture, kind, flags
    1, 0, 1, 0, 0, 0, 0, 0, // ABI, entry version, entry offset
    80, 0, 0, 0, 0, 0, 0, 0, // code length, reserved
    0x90, 0x90, 0x53, 0x5b, 0x48, 0xb8, 0x00, 0x10, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x80, 0x38,
    0x00, 0x74, 0xfb, 0x48, 0xb8, 0x41, 0x47, 0x4e, 0x54, 0x43, 0x41, 0x4c, 0x4c, 0xbb, 0x01, 0x00,
    0x00, 0x00, 0xb9, 0x01, 0x00, 0x00, 0x00, 0x31, 0xd2, 0xbe, 0x02, 0xe0, 0x2c, 0xb2, 0xcd, 0x90,
    0xb9, 0x04, 0x00, 0x00, 0x00, 0x31, 0xd2, 0x41, 0xba, 0x02, 0x0b, 0x00, 0x00, 0x41, 0xbb, 0x02,
    0x00, 0x2c, 0xb2, 0xcd, 0x90, 0xb9, 0x03, 0x00, 0x00, 0x00, 0x31, 0xd2, 0xcd, 0x90, 0xeb, 0xfe,
];

const WORKER_A_DIGEST: AgentImageDigest = AgentImageDigest::new([
    0x96, 0x23, 0xe0, 0xfa, 0xfe, 0x25, 0x06, 0xb6, 0xc8, 0xa5, 0xf7, 0xa6, 0x50, 0x5e, 0x3a, 0x7f,
    0x7f, 0x85, 0xed, 0x70, 0x14, 0x7f, 0x50, 0x8b, 0x75, 0xeb, 0xd5, 0x5c, 0x6b, 0x1c, 0x15, 0x8b,
]);

const WORKER_B_DIGEST: AgentImageDigest = AgentImageDigest::new([
    0x4f, 0x1d, 0x92, 0x5c, 0x76, 0xdc, 0x10, 0xe4, 0xe4, 0xd5, 0x05, 0x57, 0x62, 0x2b, 0x5c, 0xd2,
    0x4d, 0xe5, 0xfe, 0x9b, 0x18, 0xa5, 0x41, 0x02, 0x9d, 0x70, 0x81, 0x22, 0x41, 0xd9, 0xe7, 0x17,
]);

#[derive(Copy, Clone)]
pub(super) struct BootAgentImage {
    bytes: &'static [u8],
    digest: AgentImageDigest,
    nonce: u64,
    result: TaskResult,
    expected_describe_return_offset: u32,
    expected_result_return_offset: u32,
    expected_completion_return_offset: u32,
}

impl BootAgentImage {
    pub(super) const fn bytes(self) -> &'static [u8] {
        self.bytes
    }

    pub(super) const fn digest(self) -> AgentImageDigest {
        self.digest
    }

    pub(super) const fn nonce(self) -> u64 {
        self.nonce
    }

    pub(super) const fn result(self) -> TaskResult {
        self.result
    }

    pub(super) const fn expected_describe_return_offset(self) -> u32 {
        self.expected_describe_return_offset
    }

    pub(super) const fn expected_result_return_offset(self) -> u32 {
        self.expected_result_return_offset
    }

    pub(super) const fn expected_completion_return_offset(self) -> u32 {
        self.expected_completion_return_offset
    }
}

pub(super) const fn worker_a() -> BootAgentImage {
    BootAgentImage {
        bytes: &WORKER_A_CAPSULE,
        digest: WORKER_A_DIGEST,
        nonce: WORKER_A_NONCE,
        result: WORKER_A_RESULT,
        expected_describe_return_offset: WORKER_A_DESCRIBE_RETURN_OFFSET,
        expected_result_return_offset: WORKER_A_RESULT_RETURN_OFFSET,
        expected_completion_return_offset: WORKER_A_COMPLETION_RETURN_OFFSET,
    }
}

pub(super) const fn worker_b() -> BootAgentImage {
    BootAgentImage {
        bytes: &WORKER_B_CAPSULE,
        digest: WORKER_B_DIGEST,
        nonce: WORKER_B_NONCE,
        result: WORKER_B_RESULT,
        expected_describe_return_offset: WORKER_B_DESCRIBE_RETURN_OFFSET,
        expected_result_return_offset: WORKER_B_RESULT_RETURN_OFFSET,
        expected_completion_return_offset: WORKER_B_COMPLETION_RETURN_OFFSET,
    }
}
