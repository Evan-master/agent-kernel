use std::thread;

use agent_kernel_x86_64::{
    cpu::{CpuIndex, CpuMask},
    tlb::{
        TlbAddressSpace, TlbFlushScope, TlbIpiError, TlbIpiMailbox, TlbIpiState,
        TlbShootdownCoordinator,
    },
};

fn request(targets: &[u16]) -> agent_kernel_x86_64::tlb::TlbShootdownRequest {
    let mut online = CpuMask::singleton(CpuIndex::BSP);
    let mut resident = CpuMask::singleton(CpuIndex::BSP);
    for raw in targets {
        let cpu = CpuIndex::new(*raw).unwrap();
        online.insert(cpu);
        resident.insert(cpu);
    }
    TlbShootdownCoordinator::new()
        .begin(
            CpuIndex::BSP,
            online,
            resident,
            TlbAddressSpace::new(0x4000, 9).unwrap(),
            TlbFlushScope::range(0x8000, 3).unwrap(),
        )
        .unwrap()
}

#[test]
fn publish_exposes_one_validated_request_to_exact_targets() {
    let mailbox = TlbIpiMailbox::new();
    let expected = request(&[1, 65]);
    mailbox.publish(expected).unwrap();
    assert_eq!(mailbox.state(), Ok(TlbIpiState::Active));
    let work = mailbox.work_for(CpuIndex::new(65).unwrap()).unwrap();
    assert_eq!(work.generation(), expected.generation());
    assert_eq!(work.address_space(), expected.address_space());
    assert_eq!(work.scope(), expected.scope());
    assert_eq!(work.targets(), expected.targets());
    assert_eq!(
        mailbox.work_for(CpuIndex::new(2).unwrap()),
        Err(TlbIpiError::CpuNotTargeted(CpuIndex::new(2).unwrap()))
    );
}

#[test]
fn exact_atomic_acknowledgements_are_required_before_finish() {
    let mailbox = TlbIpiMailbox::new();
    let expected = request(&[1, 65]);
    mailbox.publish(expected).unwrap();
    mailbox
        .acknowledge(CpuIndex::new(1).unwrap(), expected.generation())
        .unwrap();
    assert_eq!(
        mailbox.finish(expected.generation()),
        Err(TlbIpiError::Incomplete(CpuMask::singleton(
            CpuIndex::new(65).unwrap()
        )))
    );
    mailbox
        .acknowledge(CpuIndex::new(65).unwrap(), expected.generation())
        .unwrap();
    assert_eq!(
        mailbox.finish(expected.generation()).unwrap().targets(),
        expected.targets()
    );
    assert_eq!(mailbox.state(), Ok(TlbIpiState::Complete));
    mailbox.reset_complete().unwrap();
    assert_eq!(mailbox.state(), Ok(TlbIpiState::Idle));
}

#[test]
fn stale_duplicate_and_non_target_acknowledgements_do_not_complete() {
    let mailbox = TlbIpiMailbox::new();
    let expected = request(&[1]);
    mailbox.publish(expected).unwrap();
    let cpu = CpuIndex::new(1).unwrap();
    assert!(matches!(
        mailbox.acknowledge(cpu, expected.generation() + 1),
        Err(TlbIpiError::StaleGeneration { .. })
    ));
    assert_eq!(
        mailbox.acknowledge(CpuIndex::new(2).unwrap(), expected.generation()),
        Err(TlbIpiError::CpuNotTargeted(CpuIndex::new(2).unwrap()))
    );
    mailbox.acknowledge(cpu, expected.generation()).unwrap();
    assert_eq!(
        mailbox.acknowledge(cpu, expected.generation()),
        Err(TlbIpiError::DuplicateAcknowledgement(cpu))
    );
    mailbox.finish(expected.generation()).unwrap();
}

#[test]
fn release_acquire_transport_is_safe_across_host_threads() {
    let mailbox = TlbIpiMailbox::new();
    let expected = request(&[1]);
    thread::scope(|scope| {
        scope.spawn(|| {
            while mailbox.state() != Ok(TlbIpiState::Active) {
                thread::yield_now();
            }
            let cpu = CpuIndex::new(1).unwrap();
            let work = mailbox.work_for(cpu).unwrap();
            mailbox.acknowledge(cpu, work.generation()).unwrap();
        });
        mailbox.publish(expected).unwrap();
    });
    mailbox.finish(expected.generation()).unwrap();
}

#[test]
fn timeout_is_sticky_and_cannot_be_reset_for_frame_reuse() {
    let mailbox = TlbIpiMailbox::new();
    let expected = request(&[1]);
    mailbox.publish(expected).unwrap();
    assert_eq!(
        mailbox.mark_timed_out(expected.generation()).unwrap(),
        expected.targets()
    );
    assert_eq!(mailbox.state(), Ok(TlbIpiState::TimedOut));
    assert_eq!(mailbox.reset_complete(), Err(TlbIpiError::InvalidState));
}
