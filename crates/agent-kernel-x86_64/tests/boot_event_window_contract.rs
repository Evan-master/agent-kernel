use agent_kernel_x86_64::boot_event_window::BootEventWindow;

#[test]
fn genesis_and_recovery_windows_cover_exactly_sixty_four_events() {
    let genesis = BootEventWindow::new(1, 64).unwrap();
    assert_eq!(genesis.first_sequence(), 1);
    assert_eq!(genesis.through_sequence(), 64);
    assert_eq!(genesis.count(), 64);

    let recovered = BootEventWindow::new(65, 64).unwrap();
    assert_eq!(recovered.first_sequence(), 65);
    assert_eq!(recovered.through_sequence(), 128);
    assert_eq!(recovered.count(), 64);
}

#[test]
fn invalid_or_overflowing_windows_fail_closed() {
    assert!(BootEventWindow::new(0, 64).is_none());
    assert!(BootEventWindow::new(1, 0).is_none());
    assert!(BootEventWindow::new(u64::MAX, 2).is_none());
}
