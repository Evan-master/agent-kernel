use agent_kernel_core::{AgentId, MemoryCellId, ResourceId};
use agent_kernel_x86_64::runtime_frame_pool::{
    RuntimeFramePoolLedger, MAX_RUNTIME_REGION_PAGES, RUNTIME_FRAME_POOL_CAPACITY,
};

#[test]
fn frame_pool_reservations_are_atomic_and_cross_agent_disjoint() {
    assert_eq!(RUNTIME_FRAME_POOL_CAPACITY, 16);
    assert_eq!(MAX_RUNTIME_REGION_PAGES, 4);

    let mut pool = RuntimeFramePoolLedger::new();
    let first = pool
        .reserve(AgentId::new(1), ResourceId::new(10), 4)
        .unwrap();
    let second = pool
        .reserve(AgentId::new(2), ResourceId::new(20), 3)
        .unwrap();

    assert_eq!(first.page_count(), 4);
    assert_eq!(second.page_count(), 3);
    assert_eq!(pool.available_frame_count(), 9);
    for left in 0..first.page_count() {
        for right in 0..second.page_count() {
            assert_ne!(first.frame_index(left), second.frame_index(right));
        }
    }

    assert!(pool.commit_mapping(first, MemoryCellId::new(1), 1));
    assert!(pool.commit_mapping(second, MemoryCellId::new(2), 7));
    assert!(!pool.agent_is_clear(AgentId::new(1)));
    assert!(!pool.agent_is_clear(AgentId::new(2)));

    let third = pool
        .reserve(AgentId::new(3), ResourceId::new(30), 4)
        .unwrap();
    let fourth = pool
        .reserve(AgentId::new(4), ResourceId::new(40), 4)
        .unwrap();
    assert_eq!(pool.available_frame_count(), 1);
    assert!(pool
        .reserve(AgentId::new(5), ResourceId::new(50), 2)
        .is_none());
    assert_eq!(pool.available_frame_count(), 1);
    assert!(pool.cancel(third));
    assert!(pool.cancel(fourth));
    assert_eq!(pool.available_frame_count(), 9);
}

#[test]
fn frame_pool_rejects_stale_tokens_and_reuses_released_frames() {
    let agent = AgentId::new(8);
    let resource = ResourceId::new(3);
    let cell = MemoryCellId::new(1);
    let mut pool = RuntimeFramePoolLedger::new();

    assert!(pool.reserve(AgentId::new(0), resource, 1).is_none());
    assert!(pool.reserve(agent, ResourceId::new(0), 1).is_none());
    assert!(pool.reserve(agent, resource, 0).is_none());
    assert!(pool
        .reserve(agent, resource, MAX_RUNTIME_REGION_PAGES + 1)
        .is_none());

    let stale = pool.reserve(agent, resource, 3).unwrap();
    let first_indices = indices(stale);
    assert!(pool.cancel(stale));

    let current = pool.reserve(agent, resource, 3).unwrap();
    assert_eq!(indices(current), first_indices);
    assert!(!pool.cancel(stale));
    assert!(!pool.commit_mapping(stale, cell, 1));
    assert!(!pool.commit_mapping(current, MemoryCellId::new(0), 1));
    assert!(!pool.commit_mapping(current, cell, 0));
    assert!(pool.commit_mapping(current, cell, 1));
    assert!(pool.contains_memory_cell(cell));
    assert!(!pool.contains_memory_cell(MemoryCellId::new(0)));
    assert!(!pool.contains_memory_cell(MemoryCellId::new(2)));

    let binding = pool.binding(agent, resource, cell, 1).unwrap();
    assert_eq!(binding.page_count(), 3);
    assert_eq!(indices(binding), first_indices);
    assert!(pool
        .prepare_release(agent, ResourceId::new(4), cell, 1)
        .is_none());
    let release = pool.prepare_release(agent, resource, cell, 1).unwrap();
    assert_eq!(release.page_count(), 3);
    assert_eq!(release.generation(), 1);
    assert!(pool.commit_release(release));
    assert!(!pool.contains_memory_cell(cell));
    assert!(!pool.commit_release(release));
    assert!(pool.agent_is_clear(agent));
    assert!(pool.all_available());

    let reused = pool
        .reserve(AgentId::new(9), ResourceId::new(4), 3)
        .unwrap();
    assert_eq!(indices(reused), first_indices);
}

fn indices(value: impl FrameSet + Copy) -> [usize; MAX_RUNTIME_REGION_PAGES] {
    let mut result = [usize::MAX; MAX_RUNTIME_REGION_PAGES];
    for (page, slot) in result.iter_mut().enumerate().take(value.page_count()) {
        *slot = value.frame_index(page).unwrap();
    }
    result
}

trait FrameSet {
    fn page_count(self) -> usize;
    fn frame_index(self, page: usize) -> Option<usize>;
}

impl FrameSet for agent_kernel_x86_64::runtime_frame_pool::RuntimeFrameReservation {
    fn page_count(self) -> usize {
        self.page_count()
    }

    fn frame_index(self, page: usize) -> Option<usize> {
        self.frame_index(page)
    }
}

impl FrameSet for agent_kernel_x86_64::runtime_frame_pool::RuntimeFrameBinding {
    fn page_count(self) -> usize {
        self.page_count()
    }

    fn frame_index(self, page: usize) -> Option<usize> {
        self.frame_index(page)
    }
}
