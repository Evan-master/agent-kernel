use agent_kernel_core::{Operation, OperationSet};

#[test]
fn operation_sets_round_trip_through_the_canonical_six_bit_encoding() {
    let operations = OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Checkpoint)
        .with(Operation::Rollback)
        .with(Operation::Delegate);

    assert_eq!(operations.bits(), 0b11_1111);
    assert_eq!(OperationSet::from_bits(operations.bits()), Some(operations));
    assert_eq!(OperationSet::from_bits(0), Some(OperationSet::empty()));
}

#[test]
fn operation_set_decoding_rejects_unknown_authority_bits() {
    assert_eq!(OperationSet::from_bits(1 << 6), None);
    assert_eq!(OperationSet::from_bits(u16::MAX), None);
}
