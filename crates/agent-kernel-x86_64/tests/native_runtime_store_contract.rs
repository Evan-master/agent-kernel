use agent_kernel_core::AgentId;
use agent_kernel_x86_64::native_runtime::{NativeAgentRuntimeError, NativeAgentRuntimeStore};

#[derive(Debug, PartialEq, Eq)]
struct OwnedRuntime(&'static str);

#[test]
fn runtime_store_transfers_one_non_copy_agent_value() {
    let mut store = NativeAgentRuntimeStore::<OwnedRuntime, 3>::new();
    store
        .insert(AgentId::new(3), OwnedRuntime("worker-a"))
        .unwrap();
    store
        .insert(AgentId::new(4), OwnedRuntime("worker-b"))
        .unwrap();
    store
        .insert(AgentId::new(5), OwnedRuntime("verifier"))
        .unwrap();

    assert_eq!(store.len(), 3);
    assert_eq!(store.get(AgentId::new(4)), Ok(&OwnedRuntime("worker-b")));
    assert_eq!(store.take(AgentId::new(4)), Ok(OwnedRuntime("worker-b")));
    assert_eq!(store.len(), 2);
    assert!(store.contains(AgentId::new(3)));
    assert!(store.contains(AgentId::new(5)));
    assert!(!store.contains(AgentId::new(4)));
    assert_eq!(store.take(AgentId::new(3)), Ok(OwnedRuntime("worker-a")));
    assert_eq!(store.take(AgentId::new(5)), Ok(OwnedRuntime("verifier")));
    assert!(store.is_empty());
}

#[test]
fn runtime_store_rejects_invalid_duplicate_and_full_insertions_with_ownership() {
    let mut store = NativeAgentRuntimeStore::<OwnedRuntime, 1>::new();

    let (error, rejected) = store
        .insert(AgentId::new(0), OwnedRuntime("invalid"))
        .unwrap_err();
    assert_eq!(error, NativeAgentRuntimeError::InvalidAgent);
    assert_eq!(rejected, OwnedRuntime("invalid"));

    store
        .insert(AgentId::new(3), OwnedRuntime("worker-a"))
        .unwrap();
    let (error, rejected) = store
        .insert(AgentId::new(3), OwnedRuntime("duplicate"))
        .unwrap_err();
    assert_eq!(error, NativeAgentRuntimeError::AgentAlreadyRegistered);
    assert_eq!(rejected, OwnedRuntime("duplicate"));
    let (error, rejected) = store
        .insert(AgentId::new(4), OwnedRuntime("full"))
        .unwrap_err();
    assert_eq!(error, NativeAgentRuntimeError::StoreFull);
    assert_eq!(rejected, OwnedRuntime("full"));
    assert_eq!(store.len(), 1);
}

#[test]
fn runtime_store_compaction_reuses_capacity_and_missing_take_is_atomic() {
    let mut store = NativeAgentRuntimeStore::<OwnedRuntime, 2>::new();
    store
        .insert(AgentId::new(3), OwnedRuntime("worker-a"))
        .unwrap();
    store
        .insert(AgentId::new(4), OwnedRuntime("worker-b"))
        .unwrap();

    assert_eq!(
        store.take(AgentId::new(9)),
        Err(NativeAgentRuntimeError::AgentNotFound)
    );
    assert_eq!(
        store.get(AgentId::new(9)),
        Err(NativeAgentRuntimeError::AgentNotFound)
    );
    assert_eq!(store.len(), 2);
    assert_eq!(store.take(AgentId::new(3)), Ok(OwnedRuntime("worker-a")));
    store
        .insert(AgentId::new(5), OwnedRuntime("verifier"))
        .expect("taken capacity should be reusable");
    assert_eq!(store.take(AgentId::new(4)), Ok(OwnedRuntime("worker-b")));
    assert_eq!(store.take(AgentId::new(5)), Ok(OwnedRuntime("verifier")));
}

#[test]
fn guarded_take_rejects_context_mismatch_without_transferring_ownership() {
    let mut store = NativeAgentRuntimeStore::<OwnedRuntime, 2>::new();
    store
        .insert(AgentId::new(3), OwnedRuntime("preempted-a"))
        .unwrap();
    store
        .insert(AgentId::new(4), OwnedRuntime("waiting-b"))
        .unwrap();

    assert_eq!(
        store.take_matching(AgentId::new(3), |runtime| runtime.0 == "waiting-a"),
        Err(NativeAgentRuntimeError::ContextMismatch)
    );
    assert_eq!(store.len(), 2);
    assert_eq!(store.get(AgentId::new(3)), Ok(&OwnedRuntime("preempted-a")));
    assert_eq!(
        store.take_matching(AgentId::new(4), |runtime| runtime.0 == "waiting-b"),
        Ok(OwnedRuntime("waiting-b"))
    );
    assert_eq!(store.take(AgentId::new(3)), Ok(OwnedRuntime("preempted-a")));
    assert!(store.is_empty());
}

#[test]
fn matching_preflight_is_read_only_for_match_mismatch_and_missing_agent() {
    let mut store = NativeAgentRuntimeStore::<OwnedRuntime, 2>::new();
    store
        .insert(AgentId::new(3), OwnedRuntime("prepared-a"))
        .unwrap();
    store
        .insert(AgentId::new(4), OwnedRuntime("preempted-b"))
        .unwrap();

    assert!(store.contains_matching(AgentId::new(3), |runtime| runtime.0 == "prepared-a"));
    assert!(!store.contains_matching(AgentId::new(4), |runtime| runtime.0 == "waiting-b"));
    assert!(!store.contains_matching(AgentId::new(9), |_| true));
    assert_eq!(store.len(), 2);
    assert_eq!(store.take(AgentId::new(3)), Ok(OwnedRuntime("prepared-a")));
    assert_eq!(store.take(AgentId::new(4)), Ok(OwnedRuntime("preempted-b")));
}
