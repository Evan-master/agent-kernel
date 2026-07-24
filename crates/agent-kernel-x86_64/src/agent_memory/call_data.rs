//! Authenticated fixed-page snapshots for bounded Agent Call records.
//!
//! This bare-metal Agent-memory child validates the active kernel CR3 and the
//! supervisor physical alias before a volatile copy. Protocol decoders remain
//! pure architecture-library code, and ring 3 is stopped for the entire copy.

use x86_64::{structures::paging::PhysFrame, PhysAddr};

use agent_kernel_core::ResourceId;
use agent_kernel_x86_64::{
    durable_archive_request::{DurableArchiveRequest, DURABLE_ARCHIVE_REQUEST_BYTES},
    namespace_path_buffer::{NamespacePathBuffer, NAMESPACE_PATH_BUFFER_BYTES},
    typed_call_data::{CallDataMessage, CallDataMessageKind, TYPED_CALL_DATA_BYTES},
    user_memory::PAGE_BYTES,
};

use super::{physical_pointer, PreparedAgentMemory, PHYSICAL_MEMORY_OFFSET};

impl PreparedAgentMemory {
    pub(crate) fn stage_durable_archive_request(
        &mut self,
        request: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES],
    ) -> bool {
        self.stage_call_data(request)
    }

    pub(crate) fn snapshot_durable_archive_request(
        &self,
        expected_generation: u64,
    ) -> Option<[u8; DURABLE_ARCHIVE_REQUEST_BYTES]> {
        let bytes = self.snapshot_call_data::<DURABLE_ARCHIVE_REQUEST_BYTES>()?;
        DurableArchiveRequest::decode(&bytes, expected_generation).ok()?;
        Some(bytes)
    }

    pub(crate) fn replace_durable_archive_request_if_unchanged(
        &mut self,
        expected: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES],
        replacement: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES],
    ) -> bool {
        matches!(
            self.snapshot_call_data::<DURABLE_ARCHIVE_REQUEST_BYTES>(),
            Some(current) if current == *expected
        ) && self.stage_call_data(replacement)
    }

    pub(crate) fn snapshot_namespace_path(
        &self,
        expected_root: ResourceId,
        expected_generation: u64,
    ) -> Option<NamespacePathBuffer> {
        let bytes = self.snapshot_call_data::<NAMESPACE_PATH_BUFFER_BYTES>()?;
        NamespacePathBuffer::decode(&bytes, expected_root, expected_generation).ok()
    }

    pub(crate) fn snapshot_typed_call_data(
        &self,
        expected_kind: CallDataMessageKind,
        expected_generation: u64,
    ) -> Option<CallDataMessage> {
        let bytes = self.snapshot_call_data::<TYPED_CALL_DATA_BYTES>()?;
        CallDataMessage::decode(&bytes, expected_kind, expected_generation).ok()
    }

    fn snapshot_call_data<const BYTES: usize>(&self) -> Option<[u8; BYTES]> {
        if BYTES == 0
            || BYTES > agent_kernel_x86_64::user_memory::PAGE_BYTES as usize
            || !self.kernel_address_space_active()
        {
            return None;
        }
        let frame =
            PhysFrame::from_start_address(PhysAddr::new(self.identity.call_data_frame())).ok()?;
        if physical_pointer(PHYSICAL_MEMORY_OFFSET, frame)? != self.call_data_pointer {
            return None;
        }

        let mut bytes = [0; BYTES];
        for (offset, byte) in bytes.iter_mut().enumerate() {
            // SAFETY: ring 3 is stopped, the kernel CR3 is active, and this
            // physical alias names the Agent's exclusive call-data frame.
            *byte = unsafe { self.call_data_pointer.add(offset).read_volatile() };
        }
        Some(bytes)
    }

    fn stage_call_data<const BYTES: usize>(&mut self, bytes: &[u8; BYTES]) -> bool {
        if BYTES == 0 || BYTES > PAGE_BYTES as usize || !self.kernel_address_space_active() {
            return false;
        }
        let Ok(frame) =
            PhysFrame::from_start_address(PhysAddr::new(self.identity.call_data_frame()))
        else {
            return false;
        };
        if physical_pointer(PHYSICAL_MEMORY_OFFSET, frame) != Some(self.call_data_pointer) {
            return false;
        }

        for offset in 0..PAGE_BYTES as usize {
            // SAFETY: ring 3 is stopped, the kernel CR3 is active, and this
            // physical alias names the Agent's exclusive call-data frame.
            unsafe { self.call_data_pointer.add(offset).write_volatile(0) };
        }
        for (offset, byte) in bytes.iter().copied().enumerate() {
            // SAFETY: the validated fixed record fits within the same frame.
            unsafe { self.call_data_pointer.add(offset).write_volatile(byte) };
        }
        true
    }
}
