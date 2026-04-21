//! ETNA witness tests — each `witness_<name>_case_<tag>` calls one property
//! function with frozen concrete inputs. On base, every witness passes. On
//! its paired variant branch (or under `M_<variant>=active`), the witness
//! fails because the mutation violates the property.

#![cfg(feature = "etna")]

use rangemap::etna::{
    property_coalesce_no_adjacent_same_value, property_inclusive_eq_matches_iter_eq,
    property_overlapping_reversible, property_partial_eq_matches_iter_eq, PropertyResult,
};

fn assert_pass(r: PropertyResult) {
    match r {
        PropertyResult::Pass | PropertyResult::Discard => {}
        PropertyResult::Fail(m) => panic!("property failed: {}", m),
    }
}

/// Triggers `coalesce_contiguous_d1999f4_1`. Insert `1..3 => 0`, then
/// `3..5 => 1` (splitting), then replace `3..5 => 0`. The fixed insert()
/// looks at two preceding candidates and coalesces the trailing `3..5 => 0`
/// with the adjacent `1..3 => 0`, leaving a single `1..5 => 0`. The buggy
/// insert() sees only the immediately-preceding candidate and leaves two
/// uncoalesced adjacent ranges with the same value.
#[test]
fn witness_coalesce_no_adjacent_same_value_case_replace_middle() {
    assert_pass(property_coalesce_no_adjacent_same_value(vec![
        (1, 3, 0),
        (3, 5, 1),
        (3, 5, 0),
    ]));
}

/// Triggers `partialeq_map_b3a59e6_1`. Two `RangeMap`s with identical range
/// *starts* but different *ends* must not compare equal. The buggy `eq`
/// delegates to the inner `BTreeMap` whose keys order by start only, so
/// `{1..3 => 0}` equals `{1..4 => 0}`.
#[test]
fn witness_partial_eq_matches_iter_eq_case_same_start_different_end() {
    assert_pass(property_partial_eq_matches_iter_eq(
        vec![(1, 3, 0)],
        vec![(1, 4, 0)],
    ));
}

/// Triggers `inclusive_equality_a6cdac3_1`. Two `RangeInclusiveMap`s with
/// identical starts but different ends must not compare equal. Same bug
/// class as the `RangeMap` case above.
#[test]
fn witness_inclusive_eq_matches_iter_eq_case_same_start_different_end() {
    assert_pass(property_inclusive_eq_matches_iter_eq(
        vec![(0, 5, 0)],
        vec![(0, 2, 0)],
    ));
}

/// Triggers `overlapping_backwards_6df612f_1`. Stored ranges `0..5` and
/// `10..15`; query range `0..7`. Forward iteration of `overlapping(&q)`
/// yields `(0..5)`. The fixed reverse iterator skips over the trailing
/// `10..15` (whose start is past the query end) and returns `(0..5)`; the
/// buggy reverse iterator returns `None` immediately on the first mismatch
/// and loses the earlier overlapping range.
#[test]
fn witness_overlapping_reversible_case_trailing_non_overlap() {
    assert_pass(property_overlapping_reversible(
        vec![(0, 5, 1), (10, 15, 2)],
        0,
        7,
    ));
}
