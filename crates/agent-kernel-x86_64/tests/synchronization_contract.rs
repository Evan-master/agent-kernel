use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use agent_kernel_x86_64::sync::{InterruptState, IrqTicketLock, LocalInterruptControl, TicketLock};

#[derive(Debug)]
struct TicketTrace {
    tickets: [u64; 16],
    len: usize,
}

impl TicketTrace {
    const fn new() -> Self {
        Self {
            tickets: [0; 16],
            len: 0,
        }
    }

    fn push(&mut self, ticket: u64) {
        self.tickets[self.len] = ticket;
        self.len += 1;
    }
}

#[test]
fn ticket_lock_enters_in_reserved_ticket_order() {
    let lock = Arc::new(TicketLock::new(TicketTrace::new()));
    let start = Arc::new(AtomicBool::new(false));
    let mut threads = Vec::new();
    for _ in 0..16 {
        let lock = Arc::clone(&lock);
        let start = Arc::clone(&start);
        threads.push(thread::spawn(move || {
            while !start.load(Ordering::Acquire) {
                core::hint::spin_loop();
            }
            let mut guard = lock.lock().unwrap();
            let ticket = guard.ticket();
            guard.push(ticket);
        }));
    }
    start.store(true, Ordering::Release);
    for thread in threads {
        thread.join().unwrap();
    }

    let guard = lock.lock().unwrap();
    assert_eq!(guard.len, 16);
    assert_eq!(
        guard.tickets,
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
    );
}

#[test]
fn ticket_lock_publishes_mutation_across_host_threads() {
    let lock = Arc::new(TicketLock::new(0usize));
    let mut threads = Vec::new();
    for _ in 0..8 {
        let lock = Arc::clone(&lock);
        threads.push(thread::spawn(move || {
            for _ in 0..2_000 {
                *lock.lock().unwrap() += 1;
            }
        }));
    }
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(*lock.lock().unwrap(), 16_000);
}

#[test]
fn try_lock_never_consumes_a_ticket_when_busy() {
    let lock = TicketLock::new(7u64);
    let guard = lock.lock().unwrap();
    assert_eq!(guard.ticket(), 0);
    assert!(lock.try_lock().unwrap().is_none());
    assert_eq!(lock.next_ticket(), 1);
    drop(guard);

    let guard = lock.try_lock().unwrap().unwrap();
    assert_eq!(guard.ticket(), 1);
    assert_eq!(*guard, 7);
}

struct FakeInterruptControl {
    enabled: AtomicBool,
    disables: AtomicUsize,
    restores: AtomicUsize,
}

impl FakeInterruptControl {
    fn enabled() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            disables: AtomicUsize::new(0),
            restores: AtomicUsize::new(0),
        }
    }
}

// SAFETY: tests use one control on one host thread; atomics model the local IF
// state and make all observations explicit.
unsafe impl LocalInterruptControl for FakeInterruptControl {
    fn disable(&self) -> InterruptState {
        self.disables.fetch_add(1, Ordering::Relaxed);
        InterruptState::new(self.enabled.swap(false, Ordering::AcqRel))
    }

    unsafe fn restore(&self, state: InterruptState) {
        self.restores.fetch_add(1, Ordering::Relaxed);
        self.enabled.store(state.was_enabled(), Ordering::Release);
    }
}

#[test]
fn irq_ticket_guard_restores_the_exact_previous_interrupt_state() {
    let lock = IrqTicketLock::new(11u64, FakeInterruptControl::enabled());
    {
        let mut guard = lock.lock().unwrap();
        assert!(!lock.control().enabled.load(Ordering::Acquire));
        *guard = 12;
        assert!(lock.try_lock().unwrap().is_none());
        assert!(!lock.control().enabled.load(Ordering::Acquire));
    }
    assert!(lock.control().enabled.load(Ordering::Acquire));
    assert_eq!(lock.control().disables.load(Ordering::Relaxed), 2);
    assert_eq!(lock.control().restores.load(Ordering::Relaxed), 2);
    assert_eq!(*lock.lock().unwrap(), 12);
}

#[test]
fn irq_guard_keeps_interrupts_clear_when_the_caller_arrived_clear() {
    let control = FakeInterruptControl::enabled();
    control.enabled.store(false, Ordering::Release);
    let lock = IrqTicketLock::new(0u8, control);
    drop(lock.lock().unwrap());
    assert!(!lock.control().enabled.load(Ordering::Acquire));
}
