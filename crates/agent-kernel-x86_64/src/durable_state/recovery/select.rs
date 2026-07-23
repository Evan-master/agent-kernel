//! Deterministic selection of one verified dual-slot chain head.

use agent_kernel_core::{
    DurableAnchorMode, DurableRecoveredHead, DurableRecoveryError, DurableRecoveryGuarantee,
};

pub(super) fn select_recovered_head(
    slot_a: Option<DurableRecoveredHead>,
    slot_b: Option<DurableRecoveredHead>,
) -> Result<DurableRecoveredHead, DurableRecoveryError> {
    match (slot_a, slot_b) {
        (None, None) => Err(DurableRecoveryError::NoCommittedSlot),
        (Some(head), None) | (None, Some(head)) => select_single(head),
        (Some(first), Some(second)) => select_pair(first, second),
    }
}

fn select_single(head: DurableRecoveredHead) -> Result<DurableRecoveredHead, DurableRecoveryError> {
    if head.generation() == u64::MAX {
        return Err(DurableRecoveryError::GenerationExhausted);
    }
    if head.generation() == 1 || head.guarantee() == DurableRecoveryGuarantee::RollbackResistant {
        return Ok(head);
    }
    Err(DurableRecoveryError::DisconnectedHead {
        generation: head.generation(),
    })
}

fn select_pair(
    first: DurableRecoveredHead,
    second: DurableRecoveredHead,
) -> Result<DurableRecoveredHead, DurableRecoveryError> {
    if first.generation() == second.generation() {
        return Err(DurableRecoveryError::SplitBrain {
            generation: first.generation(),
        });
    }
    let (lower, higher) = if first.generation() < second.generation() {
        (first, second)
    } else {
        (second, first)
    };
    if higher.generation() == u64::MAX {
        return Err(DurableRecoveryError::GenerationExhausted);
    }

    let adjacent = lower.generation().checked_add(1) == Some(higher.generation());
    if adjacent {
        let sequence_link =
            lower.through_sequence().checked_add(1) == Some(higher.manifest().first_sequence());
        let digest_link = lower.archive_digest() == higher.previous_digest();
        if sequence_link && digest_link {
            return Ok(higher);
        }
        if higher.manifest().anchor().mode() == DurableAnchorMode::Trusted {
            return Err(DurableRecoveryError::AnchorMismatch {
                generation: higher.generation(),
            });
        }
        return Err(DurableRecoveryError::DisconnectedHead {
            generation: higher.generation(),
        });
    }

    if higher.manifest().anchor().mode() == DurableAnchorMode::Trusted {
        Ok(higher)
    } else {
        Err(DurableRecoveryError::DisconnectedHead {
            generation: higher.generation(),
        })
    }
}
