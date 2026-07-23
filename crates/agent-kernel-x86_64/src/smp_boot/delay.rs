//! Bounded PIT channel-2 waits for architectural AP startup intervals.

use core::hint::spin_loop;

use crate::{inb, outb};

const PIT_HZ: u64 = 1_193_182;
const PIT_COMMAND_PORT: u16 = 0x43;
const PIT_CHANNEL2_PORT: u16 = 0x42;
const SPEAKER_CONTROL_PORT: u16 = 0x61;
const CHANNEL2_ONE_SHOT: u8 = 0xb0;
const CHANNEL2_GATE: u8 = 1 << 0;
const SPEAKER_DATA: u8 = 1 << 1;
const CHANNEL2_OUTPUT: u8 = 1 << 5;
const POLL_LIMIT: usize = 20_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DelayError {
    InvalidDuration,
    TimerExpiredWithoutOutput,
}

pub(super) fn wait_micros(micros: u32) -> Result<(), DelayError> {
    let ticks = ticks_for_micros(micros).ok_or(DelayError::InvalidDuration)?;
    let [low, high] = ticks.to_le_bytes();
    // SAFETY: SMP startup runs at CPL0 with IF clear and exclusively uses PIT
    // channel 2. Channel 0 remains untouched until the Agent timer is armed.
    let original = unsafe { inb(SPEAKER_CONTROL_PORT) };
    let gate_low = original & !(CHANNEL2_GATE | SPEAKER_DATA);
    unsafe {
        outb(SPEAKER_CONTROL_PORT, gate_low);
        outb(PIT_COMMAND_PORT, CHANNEL2_ONE_SHOT);
        outb(PIT_CHANNEL2_PORT, low);
        outb(PIT_CHANNEL2_PORT, high);
        outb(SPEAKER_CONTROL_PORT, gate_low | CHANNEL2_GATE);
    }
    for _ in 0..POLL_LIMIT {
        if unsafe { inb(SPEAKER_CONTROL_PORT) } & CHANNEL2_OUTPUT != 0 {
            unsafe {
                outb(SPEAKER_CONTROL_PORT, original);
            }
            return Ok(());
        }
        spin_loop();
    }
    unsafe {
        outb(SPEAKER_CONTROL_PORT, original);
    }
    Err(DelayError::TimerExpiredWithoutOutput)
}

fn ticks_for_micros(micros: u32) -> Option<u16> {
    let numerator = u64::from(micros)
        .checked_mul(PIT_HZ)?
        .checked_add(999_999)?;
    let ticks = numerator / 1_000_000;
    if ticks == 0 || ticks > u16::MAX as u64 {
        None
    } else {
        Some(ticks as u16)
    }
}
