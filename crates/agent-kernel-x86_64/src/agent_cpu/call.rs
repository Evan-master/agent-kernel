//! Capture and resume mechanics for structured Agent calls.
//!
//! This child module owns one-call mailbox validation and saved-frame transfer.
//! It decodes only after the complete CPL3 frame, CR3, RSP0 bounds, and kernel
//! continuation are valid; semantic task transitions remain outside this layer.

use core::sync::atomic::Ordering;

use agent_kernel_x86_64::{
    address_space::AddressSpaceRoots, agent_call::AgentCallRequest, context::SavedAgentFrame,
    native_runtime::NativeRunBoundary, user_memory::UserMemoryLayout,
};

use super::{assembly, storage, validation};
use crate::privilege_runtime::PrivilegedStackBounds;

pub(super) struct CapturedAgentCall {
    frame: SavedAgentFrame,
    request: AgentCallRequest,
    return_offset: u32,
}

impl CapturedAgentCall {
    pub(super) const fn request(&self) -> AgentCallRequest {
        self.request
    }

    pub(super) const fn return_offset(&self) -> u32 {
        self.return_offset
    }

    pub(super) fn into_frame(self) -> SavedAgentFrame {
        self.frame
    }
}

pub(super) fn capture(
    stack: PrivilegedStackBounds,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
) -> Option<CapturedAgentCall> {
    let frame_rsp = storage::AGENT_KERNEL_AGENT_CALL_RSP.load(Ordering::Acquire);
    let frame_rip = storage::AGENT_KERNEL_AGENT_CALL_RIP.load(Ordering::Acquire);
    let frame = validation::read_frame(frame_rsp, stack)?;
    if storage::run_boundary()? != NativeRunBoundary::AgentCall
        || storage::AGENT_KERNEL_HOST_CONTEXT_RSP.load() == 0
        || storage::AGENT_KERNEL_AGENT_CALL_CR3.load(Ordering::Acquire) != roots.agent_cr3()
        || frame.rip != frame_rip
        || !validation::user_frame_valid(&frame, layout)
        || !validation::kernel_boundary_valid(stack, roots.kernel_cr3())
    {
        return None;
    }
    let return_offset = u32::try_from(frame.rip.checked_sub(layout.code_start())?).ok()?;
    let request = AgentCallRequest::decode(&frame).ok()?;
    Some(CapturedAgentCall {
        frame: SavedAgentFrame::new(frame),
        request,
        return_offset,
    })
}

pub(super) fn resume_owned(
    frame: &mut SavedAgentFrame,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
) -> Option<()> {
    if storage::AGENT_KERNEL_HOST_CONTEXT_RSP.load() != 0
        || !validation::saved_frame_valid(frame, layout)
    {
        return None;
    }
    let frame_rsp = frame.as_mut_ptr() as usize as u64;
    // SAFETY: the complete owned privilege frame remains live until the call
    // gate restores this invocation's host continuation.
    unsafe {
        assembly::resume_interrupted_user(
            storage::AGENT_KERNEL_HOST_CONTEXT_RSP.pointer(),
            frame_rsp,
            roots.agent_cr3(),
        );
    }
    Some(())
}
