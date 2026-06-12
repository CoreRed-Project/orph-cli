/// Unit tests for pure pet_service logic (no DB required).
use orph_cli::services::pet_service::calculate_decay;

#[test]
fn decay_zero_elapsed_no_change() {
    let (h, hap) = calculate_decay(20, 80, 0.0);
    assert_eq!(h, 20);
    assert_eq!(hap, 80);
}

#[test]
fn decay_one_hour() {
    // hunger: 20 + 5*1 = 25, happiness: 80 - 3*1 = 77
    let (h, hap) = calculate_decay(20, 80, 1.0);
    assert_eq!(h, 25);
    assert_eq!(hap, 77);
}

#[test]
fn decay_clamps_at_100_and_0() {
    // hunger maxes at 100, happiness floors at 0
    let (h, hap) = calculate_decay(95, 5, 2.0);
    assert_eq!(h, 100);
    assert_eq!(hap, 0);
}
