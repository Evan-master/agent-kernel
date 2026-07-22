//! Fair ticket lock with explicit Acquire and Release publication.
//!
//! Ticket reservation never wraps. The lock contains no allocator or host
//! runtime dependency and spins only after a caller has successfully reserved a
//! place in FIFO order.

use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TicketLockError {
    TicketExhausted,
}

#[repr(align(64))]
pub struct TicketLock<T> {
    next: AtomicU64,
    serving: AtomicU64,
    value: UnsafeCell<T>,
}

impl<T> TicketLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            next: AtomicU64::new(0),
            serving: AtomicU64::new(0),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> Result<TicketGuard<'_, T>, TicketLockError> {
        let ticket = self.reserve_ticket()?;
        while self.serving.load(Ordering::Acquire) != ticket {
            spin_loop();
        }
        Ok(TicketGuard { lock: self, ticket })
    }

    pub fn try_lock(&self) -> Result<Option<TicketGuard<'_, T>>, TicketLockError> {
        let serving = self.serving.load(Ordering::Acquire);
        let next = self.next.load(Ordering::Relaxed);
        if serving != next {
            return Ok(None);
        }
        let following = next
            .checked_add(1)
            .ok_or(TicketLockError::TicketExhausted)?;
        match self
            .next
            .compare_exchange(next, following, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => Ok(Some(TicketGuard {
                lock: self,
                ticket: next,
            })),
            Err(_) => Ok(None),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.next.load(Ordering::Relaxed) != self.serving.load(Ordering::Acquire)
    }

    pub fn next_ticket(&self) -> u64 {
        self.next.load(Ordering::Relaxed)
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    fn reserve_ticket(&self) -> Result<u64, TicketLockError> {
        let mut next = self.next.load(Ordering::Relaxed);
        loop {
            let following = next
                .checked_add(1)
                .ok_or(TicketLockError::TicketExhausted)?;
            match self.next.compare_exchange_weak(
                next,
                following,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(next),
                Err(observed) => next = observed,
            }
        }
    }

    fn release(&self, ticket: u64) {
        debug_assert_eq!(self.serving.load(Ordering::Relaxed), ticket);
        self.serving.store(ticket + 1, Ordering::Release);
    }
}

// SAFETY: ownership of `T` may move with the lock when `T` is Send.
unsafe impl<T: Send> Send for TicketLock<T> {}

// SAFETY: all shared access to `T` is serialized by ticket ownership.
unsafe impl<T: Send> Sync for TicketLock<T> {}

pub struct TicketGuard<'a, T> {
    lock: &'a TicketLock<T>,
    ticket: u64,
}

impl<T> TicketGuard<'_, T> {
    pub const fn ticket(&self) -> u64 {
        self.ticket
    }
}

impl<T> Deref for TicketGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: this guard exclusively owns the currently served ticket.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for TicketGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: this guard exclusively owns the currently served ticket.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for TicketGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.release(self.ticket);
    }
}
