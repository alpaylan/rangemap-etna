//! ETNA framework-neutral property functions for the rangemap crate.
//!
//! Each `property_<name>` is a pure function taking concrete, owned inputs
//! and returning [`PropertyResult`]. Framework adapters in `src/bin/etna.rs`
//! and witness tests in `tests/etna_witnesses.rs` all call these functions
//! directly, so the same invariant is exercised by every PBT tool.

#![allow(missing_docs)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::ops::{Range, RangeInclusive};

use crate::{RangeInclusiveMap, RangeMap};

#[derive(Debug)]
pub enum PropertyResult {
    Pass,
    Fail(String),
    Discard,
}

/// Build a `RangeMap<u32, i32>` from a list of `(start, end, value)` inserts,
/// skipping any entry whose range is empty (so the caller can generate freely
/// without pre-filtering). Returns `None` if the entire input was invalid.
fn build_range_map(inserts: &[(u32, u32, i32)]) -> RangeMap<u32, i32> {
    let mut m = RangeMap::new();
    for &(s, e, v) in inserts {
        if s < e {
            m.insert(s..e, v);
        }
    }
    m
}

fn build_inclusive_map(inserts: &[(u32, u32, i32)]) -> RangeInclusiveMap<u32, i32> {
    let mut m = RangeInclusiveMap::new();
    for &(s, e, v) in inserts {
        if s <= e {
            m.insert(s..=e, v);
        }
    }
    m
}

/// Invariant for variant `coalesce_contiguous_d1999f4_1`.
///
/// After any sequence of insertions into a `RangeMap`, no two *consecutive*
/// stored ranges may (a) share the same value AND (b) be immediately
/// adjacent (i.e. `prev.end == next.start`). Such pairs are supposed to be
/// coalesced into a single range by `insert`. The historical bug made
/// `insert` look at only one candidate for coalescing at a time, so replacing
/// a middle range with a value matching a neighbour left an uncoalesced seam.
pub fn property_coalesce_no_adjacent_same_value(
    inserts: Vec<(u32, u32, i32)>,
) -> PropertyResult {
    if inserts.iter().all(|&(s, e, _)| s >= e) {
        return PropertyResult::Discard;
    }
    let m = build_range_map(&inserts);
    let collected: Vec<(Range<u32>, i32)> =
        m.iter().map(|(r, v)| (r.clone(), *v)).collect();
    for w in collected.windows(2) {
        let (ra, va) = &w[0];
        let (rb, vb) = &w[1];
        if ra.end == rb.start && va == vb {
            return PropertyResult::Fail(format!(
                "uncoalesced adjacent ranges with same value: {:?}={} then {:?}={}",
                ra, va, rb, vb
            ));
        }
    }
    PropertyResult::Pass
}

/// Invariant for variant `partialeq_map_b3a59e6_1`.
///
/// For two `RangeMap`s `a` and `b`, `a == b` iff `a.iter().eq(b.iter())`.
/// The historical bug delegated `PartialEq` to the inner `BTreeMap`, whose
/// key ordering uses only the range *start*, so maps with identical starts
/// but different ends were considered equal.
pub fn property_partial_eq_matches_iter_eq(
    inserts_a: Vec<(u32, u32, i32)>,
    inserts_b: Vec<(u32, u32, i32)>,
) -> PropertyResult {
    let a = build_range_map(&inserts_a);
    let b = build_range_map(&inserts_b);
    let eq_result = a == b;
    let iter_eq = a.iter().eq(b.iter());
    if eq_result == iter_eq {
        PropertyResult::Pass
    } else {
        PropertyResult::Fail(format!(
            "a == b = {}, but a.iter().eq(b.iter()) = {}; a.len={}, b.len={}",
            eq_result,
            iter_eq,
            a.len(),
            b.len()
        ))
    }
}

/// Invariant for variant `inclusive_equality_a6cdac3_1`.
///
/// Same shape as the `RangeMap` variant but for `RangeInclusiveMap`. The
/// historical bug delegated `PartialEq` (and `PartialOrd`/`Ord`) to the
/// inner `BTreeMap`, whose keys order by range start only, so
/// `{0..=5}` and `{0..=2}` compared equal.
pub fn property_inclusive_eq_matches_iter_eq(
    inserts_a: Vec<(u32, u32, i32)>,
    inserts_b: Vec<(u32, u32, i32)>,
) -> PropertyResult {
    let a = build_inclusive_map(&inserts_a);
    let b = build_inclusive_map(&inserts_b);
    let eq_result = a == b;
    let iter_eq: bool = {
        let va: Vec<(RangeInclusive<u32>, i32)> =
            a.iter().map(|(r, v)| (r.clone(), *v)).collect();
        let vb: Vec<(RangeInclusive<u32>, i32)> =
            b.iter().map(|(r, v)| (r.clone(), *v)).collect();
        va == vb
    };
    if eq_result == iter_eq {
        PropertyResult::Pass
    } else {
        PropertyResult::Fail(format!(
            "a == b = {}, but iter_eq = {}; a.len={}, b.len={}",
            eq_result,
            iter_eq,
            a.len(),
            b.len()
        ))
    }
}

/// Invariant for variant `overlapping_backwards_6df612f_1`.
///
/// For any `RangeMap` and any query range, iterating `overlapping(&q)`
/// forward must produce the same sequence as iterating it backward and
/// then reversing. The historical bug made the reverse iterator return
/// `None` as soon as it saw one range whose start was past the query end,
/// skipping valid earlier matches.
pub fn property_overlapping_reversible(
    inserts: Vec<(u32, u32, i32)>,
    query_start: u32,
    query_end: u32,
) -> PropertyResult {
    if query_start >= query_end {
        return PropertyResult::Discard;
    }
    let m = build_range_map(&inserts);
    let q: Range<u32> = query_start..query_end;
    let forward: Vec<(Range<u32>, i32)> =
        m.overlapping(&q).map(|(r, v)| (r.clone(), *v)).collect();
    let mut backward: Vec<(Range<u32>, i32)> =
        m.overlapping(&q).rev().map(|(r, v)| (r.clone(), *v)).collect();
    backward.reverse();
    if forward == backward {
        PropertyResult::Pass
    } else {
        PropertyResult::Fail(format!(
            "forward={:?} != rev-then-reverse={:?}",
            forward, backward
        ))
    }
}
