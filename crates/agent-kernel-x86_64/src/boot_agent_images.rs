//! Immutable native Worker capsules used by the BIOS boot proof.
//!
//! This x86 boot-layer module owns two independently identified payloads. The
//! precomputed digests model manifest input; the runtime loader recomputes each
//! digest before mapping either code page.

use agent_kernel_core::AgentImageDigest;

const WORKER_A_CALL_RETURN_OFFSET: u32 = 19;
const WORKER_B_CALL_RETURN_OFFSET: u32 = 21;

const WORKER_A_CAPSULE: [u8; 53] = [
    b'A', b'G', b'N', b'T', b'I', b'M', b'G', 0, // magic
    1, 0, 1, 0, 1, 0, 0, 0, // format, architecture, kind, flags
    1, 0, 1, 0, 0, 0, 0, 0, // ABI, entry version, entry offset
    21, 0, 0, 0, 0, 0, 0, 0, // code length, reserved
    0x53, 0x5b, 0x48, 0xb8, 0x00, 0x10, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x80, 0x38, 0x00, 0x74,
    0xfb, 0xcd, 0x90, 0xeb, 0xfe,
];

const WORKER_B_CAPSULE: [u8; 55] = [
    b'A', b'G', b'N', b'T', b'I', b'M', b'G', 0, // magic
    1, 0, 1, 0, 1, 0, 0, 0, // format, architecture, kind, flags
    1, 0, 1, 0, 0, 0, 0, 0, // ABI, entry version, entry offset
    23, 0, 0, 0, 0, 0, 0, 0, // code length, reserved
    0x90, 0x90, 0x53, 0x5b, 0x48, 0xb8, 0x00, 0x10, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x80, 0x38,
    0x00, 0x74, 0xfb, 0xcd, 0x90, 0xeb, 0xfe,
];

const WORKER_A_DIGEST: AgentImageDigest = AgentImageDigest::new([
    0x83, 0x03, 0x05, 0x1d, 0xb2, 0x27, 0x4d, 0x8f, 0x80, 0x67, 0xd1, 0x16, 0x19, 0x2e, 0x38, 0x9d,
    0x13, 0x0d, 0xf1, 0x37, 0xf1, 0xd8, 0xe2, 0xde, 0xe4, 0xde, 0x2f, 0xde, 0x99, 0xbd, 0x2b, 0xc8,
]);

const WORKER_B_DIGEST: AgentImageDigest = AgentImageDigest::new([
    0xdd, 0xeb, 0x95, 0xc3, 0x0d, 0x79, 0x4d, 0xb4, 0x2e, 0x56, 0x4b, 0xdf, 0x7e, 0xd5, 0xb4, 0xc6,
    0x04, 0x07, 0x7e, 0xb4, 0x17, 0xda, 0xfb, 0xcb, 0x1c, 0x9c, 0xaa, 0x0b, 0x1f, 0xe3, 0x2a, 0xb8,
]);

#[derive(Copy, Clone)]
pub(super) struct BootAgentImage {
    bytes: &'static [u8],
    digest: AgentImageDigest,
    expected_call_return_offset: u32,
}

impl BootAgentImage {
    pub(super) const fn bytes(self) -> &'static [u8] {
        self.bytes
    }

    pub(super) const fn digest(self) -> AgentImageDigest {
        self.digest
    }

    pub(super) const fn expected_call_return_offset(self) -> u32 {
        self.expected_call_return_offset
    }
}

pub(super) const fn worker_a() -> BootAgentImage {
    BootAgentImage {
        bytes: &WORKER_A_CAPSULE,
        digest: WORKER_A_DIGEST,
        expected_call_return_offset: WORKER_A_CALL_RETURN_OFFSET,
    }
}

pub(super) const fn worker_b() -> BootAgentImage {
    BootAgentImage {
        bytes: &WORKER_B_CAPSULE,
        digest: WORKER_B_DIGEST,
        expected_call_return_offset: WORKER_B_CALL_RETURN_OFFSET,
    }
}
