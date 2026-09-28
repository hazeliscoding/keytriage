use super::*;
use Confidence::*;

#[test]
fn s01_wilson_floor_matches_the_closed_form() {
    for (k, n, want) in [
        (14, 100, 85),
        (3, 100, 10),
        (2, 100, 5),
        (10, 100, 55),
        (9, 100, 48),
        (3, 30, 34),
        (0, 100, 0),
        (100, 100, 963),
        (6, 90, 30),
        (5, 60, 36),
        (4, 90, 17),
    ] {
        assert_eq!(wilson_floor_permille(k, n), want, "{k}/{n}");
    }
}

#[test]
fn s02_wilson_edges() {
    assert!(wilson_at_least(0, 0, 0, 1000));
    assert!(!wilson_at_least(5, 0, 1, 1000));
    assert!(!wilson_at_least(10, 10, 1000, 1000));
    assert!(!wilson_at_least(11, 10, 1, 1000));
}

#[test]
fn s03_rule_of_three() {
    assert_eq!(rule_of_three_permille(100), 30);
    assert_eq!(rule_of_three_permille(30), 100);
    assert_eq!(rule_of_three_permille(0), 1000);
}

#[test]
fn s04_chatter_confidence_boundaries() {
    for ((k, n, with, of), want) in [
        ((1, 100, 1, 3), None),
        ((2, 100, 1, 3), Some(Low)),
        ((2, 100, 2, 3), Some(Medium)),
        ((3, 100, 1, 3), Some(Medium)),
        ((3, 301, 1, 3), Some(Low)),
        ((3, 100, 2, 3), Some(Medium)),
        ((5, 100, 3, 3), Some(High)),
        ((6, 100, 3, 3), Some(High)),
        ((9, 100, 3, 3), Some(High)),
        ((10, 100, 3, 3), Some(VeryHigh)),
        ((14, 100, 2, 3), Some(High)),
        ((14, 100, 3, 3), Some(VeryHigh)),
        ((10, 100, 2, 2), Some(High)),
    ] {
        assert_eq!(
            chatter_confidence(k, n, with, of),
            want,
            "{k}/{n} in {with}/{of}"
        );
    }
}

#[test]
fn s05_wilson_never_overflows() {
    assert!(wilson_at_least(1 << 31, 1 << 31, 1, (1 << 31) + 1));
    assert!(wilson_at_least(u32::MAX, u32::MAX, 1, u32::MAX));
}
