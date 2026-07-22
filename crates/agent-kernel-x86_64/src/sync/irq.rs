//! Ticket locking with exact local interrupt-state restoration.
//!
//! The control implementation belongs to the active CPU. Guard destruction
//! publishes and releases protected state before restoring the IF state
//! observed on entry; the guard is intentionally non-transferable across CPUs.

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use super::{TicketGuard, TicketLock, TicketLockError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct InterruptState {
    was_enabled: bool,
}

impl InterruptState {
    pub const fn new(was_enabled: bool) -> Self {
        Self { was_enabled }
    }

    pub const fn was_enabled(self) -> bool {
        self.was_enabled
    }
}

/// Local interrupt control used around a per-CPU critical section.
///
/// # Safety
///
/// Implementations must inspect and mutate only the current CPU's interrupt
/// state. `restore` must reinstate exactly the state returned by `disable` and
/// must remain valid after the protected lock has published its writes.
pub unsafe trait LocalInterruptControl: Sync {
    fn disable(&self) -> InterruptState;

    /// # Safety
    ///
    /// `state` must originate from the matching control instance on the same
    /// CPU, with no intervening restoration.
    unsafe fn restore(&self, state: InterruptState);
}

pub struct IrqTicketLock<T, C> {
    inner: TicketLock<T>,
    control: C,
}

impl<T, C: LocalInterruptControl> IrqTicketLock<T, C> {
    pub const fn new(value: T, control: C) -> Self {
        Self {
            inner: TicketLock::new(value),
            control,
        }
    }

    pub fn lock(&self) -> Result<IrqTicketGuard<'_, T, C>, TicketLockError> {
        let interrupt_state = self.control.disable();
        match self.inner.lock() {
            Ok(guard) => Ok(IrqTicketGuard::new(guard, &self.control, interrupt_state)),
            Err(error) => {
                // SAFETY: restoration matches the immediately preceding
                // disable on this control and no guard was created.
                unsafe { self.control.restore(interrupt_state) };
                Err(error)
            }
        }
    }

    pub fn try_lock(&self) -> Result<Option<IrqTicketGuard<'_, T, C>>, TicketLockError> {
        let interrupt_state = self.control.disable();
        match self.inner.try_lock() {
            Ok(Some(guard)) => Ok(Some(IrqTicketGuard::new(
                guard,
                &self.control,
                interrupt_state,
            ))),
            Ok(None) => {
                // SAFETY: restoration matches the immediately preceding
                // disable and ticket reservation did not occur.
                unsafe { self.control.restore(interrupt_state) };
                Ok(None)
            }
            Err(error) => {
                // SAFETY: restoration matches the immediately preceding
                // disable and ticket reservation failed.
                unsafe { self.control.restore(interrupt_state) };
                Err(error)
            }
        }
    }

    pub const fn control(&self) -> &C {
        &self.control
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut()
    }
}

pub struct IrqTicketGuard<'a, T, C: LocalInterruptControl> {
    guard: Option<TicketGuard<'a, T>>,
    control: &'a C,
    interrupt_state: InterruptState,
    _local_cpu: PhantomData<*mut ()>,
}

impl<'a, T, C: LocalInterruptControl> IrqTicketGuard<'a, T, C> {
    fn new(guard: TicketGuard<'a, T>, control: &'a C, interrupt_state: InterruptState) -> Self {
        Self {
            guard: Some(guard),
            control,
            interrupt_state,
            _local_cpu: PhantomData,
        }
    }

    pub fn ticket(&self) -> u64 {
        self.guard.as_ref().map_or(0, TicketGuard::ticket)
    }
}

impl<T, C: LocalInterruptControl> Deref for IrqTicketGuard<'_, T, C> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard
            .as_ref()
            .expect("live IRQ ticket guard lost its inner ticket")
    }
}

impl<T, C: LocalInterruptControl> DerefMut for IrqTicketGuard<'_, T, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard
            .as_mut()
            .expect("live IRQ ticket guard lost its inner ticket")
    }
}

impl<T, C: LocalInterruptControl> Drop for IrqTicketGuard<'_, T, C> {
    fn drop(&mut self) {
        drop(self.guard.take());
        // SAFETY: the guard cannot move across CPUs, this state came from the
        // matching disable, and lock release has already published mutations.
        unsafe { self.control.restore(self.interrupt_state) };
    }
}
