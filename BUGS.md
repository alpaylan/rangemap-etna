# rangemap — Injected Bugs

ETNA workload for the Rust `rangemap` crate. Each variant re-introduces
one historical bug-fix into a fresh patched branch and pairs it with a
framework-neutral property, four PBT adapters, and a deterministic
witness test.

Total mutations: 4

## Bug Index

| # | Variant | Name | Location | Injection | Fix Commit |
|---|---------|------|----------|-----------|------------|
| 1 | `coalesce_contiguous_d1999f4_1` | `coalesce_contiguous` | `src/map.rs:284` | `marauders` | `d1999f48003b43ec7ed598c9497b859a8302b897` |
| 2 | `inclusive_equality_a6cdac3_1` | `inclusive_equality` | `src/inclusive_map.rs:73` | `marauders` | `a6cdac3e99e747c9ec80b4ef238e1480e63927fb` |
| 3 | `overlapping_backwards_6df612f_1` | `overlapping_backwards` | `src/map.rs:832` | `marauders` | `6df612f45d6023cab8eeec69e10fe794c70eacd8` |
| 4 | `partialeq_map_b3a59e6_1` | `partialeq_map` | `src/map.rs:60` | `marauders` | `b3a59e6641a9e3869791f781abbae98f828f91c9` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `coalesce_contiguous_d1999f4_1` | `CoalesceNoAdjacentSameValue` | `witness_coalesce_no_adjacent_same_value_case_replace_middle` |
| `inclusive_equality_a6cdac3_1` | `InclusiveEqMatchesIterEq` | `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` |
| `overlapping_backwards_6df612f_1` | `OverlappingReversible` | `witness_overlapping_reversible_case_trailing_non_overlap` |
| `partialeq_map_b3a59e6_1` | `PartialEqMatchesIterEq` | `witness_partial_eq_matches_iter_eq_case_same_start_different_end` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `CoalesceNoAdjacentSameValue` | ✓ | ✓ | ✓ | ✓ |
| `InclusiveEqMatchesIterEq` | ✓ | ✓ | ✓ | ✓ |
| `OverlappingReversible` | ✓ | ✓ | ✓ | ✓ |
| `PartialEqMatchesIterEq` | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. coalesce_contiguous

- **Variant**: `coalesce_contiguous_d1999f4_1`
- **Location**: `src/map.rs:284`
- **Property**: `CoalesceNoAdjacentSameValue`
- **Witness(es)**:
  - `witness_coalesce_no_adjacent_same_value_case_replace_middle`
- **Source**: Fix coalescing of contiguous ranges
  > The insert path used `.take(2)` when scanning preceding ranges for coalescing; the buggy `.take(1)` only considers the single range immediately to the left, so when an earlier insert has split an older range into two pieces, the coalescing step misses the older-older piece and leaves two adjacent ranges with the same value.
- **Fix commit**: `d1999f48003b43ec7ed598c9497b859a8302b897` — Fix coalescing of contiguous ranges
- **Invariant violated**: After any sequence of `insert` calls the map must not contain two adjacent ranges that share the same value — the data structure's core coalescing invariant.
- **How the mutation triggers**: the fix changed `.take(2)` to `.take(1)` on the reverse-iteration candidate window. With `.take(1)` the insert only ever considers the single range immediately to the left of the new one. If a preceding insert has split an older range into two pieces, the mutation misses the older-older piece, and a freshly-replacing insert that makes the trailing piece share a value with the piece two steps back leaves two uncoalesced adjacent ranges. The witness inserts `1..3 => 0`, `3..5 => 1`, then overwrites `3..5 => 0`; the base coalesces into `1..5 => 0`, the mutation leaves both `1..3 => 0` and `3..5 => 0`.

### 2. inclusive_equality

- **Variant**: `inclusive_equality_a6cdac3_1`
- **Location**: `src/inclusive_map.rs:73` (inside `impl PartialEq for RangeInclusiveMap`)
- **Property**: `InclusiveEqMatchesIterEq`
- **Witness(es)**:
  - `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end`
- **Source**: Fix RangeInclusiveMap equality comparison
  > Same bug class as `partialeq_map` but for `RangeInclusiveMap`: the pre-fix impl compared inner `BTreeMap`s whose keys order solely on `range.start`, so inclusive maps with matching starts but different ends compared equal. The fix delegates to iterator equality.
- **Fix commit**: `a6cdac3e99e747c9ec80b4ef238e1480e63927fb` — Fix RangeInclusiveMap equality comparison
- **Invariant violated**: identical to `partialeq_map` but for the inclusive-range variant: `a == b` iff `a.iter().eq(b.iter())`.
- **How the mutation triggers**: same mechanism — pre-a6cdac3 the impl reads `self.btm == other.btm` and compares BTreeMap keys via a start-only wrapper. The witness uses `{0..=5 => 0}` vs. `{0..=2 => 0}` which share a start but differ in end and therefore in iter-equality but not in the buggy `==`.

### 3. overlapping_backwards

- **Variant**: `overlapping_backwards_6df612f_1`
- **Location**: `src/map.rs:832` (inside `impl DoubleEndedIterator for Overlapping::next_back`)
- **Property**: `OverlappingReversible`
- **Witness(es)**:
  - `witness_overlapping_reversible_case_trailing_non_overlap`
- **Source**: Fixes overlapping backwards iterator
  > `Overlapping::next_back` used a single `if let Some(_)` followed by a start-bound check, terminating the iterator when the first reverse candidate's start exceeded the query end. The fix replaces it with `while let Some(_)` so non-overlapping trailing ranges are skipped instead of ending iteration.
- **Fix commit**: `6df612f45d6023cab8eeec69e10fe794c70eacd8` — Fixes overlapping backwards iterator
- **Invariant violated**: reverse iteration of `Overlapping` must yield the same set of ranges as forward iteration. In particular, if the inner BTreeMap's reverse cursor lands on a range whose start is past the query end, `next_back` must *skip* that range and keep walking backwards — not bail out.
- **How the mutation triggers**: the pre-6df612f impl uses a single `if let Some(_) = next_back()` followed by a start-bound check; if the check fails it returns `None` and the whole iterator terminates. The fix turns this into `while let Some(_)` so non-overlapping trailing ranges are skipped over. The witness stores `0..5` and `10..15`, queries `0..7`, and calls `next_back()`: forward yields `(0..5)`, base's reverse also yields `(0..5)` (skipping the trailing `10..15`), the mutation returns `None` immediately because the first reverse candidate is `10..15` whose start exceeds the query end.

### 4. partialeq_map

- **Variant**: `partialeq_map_b3a59e6_1`
- **Location**: `src/map.rs:60` (inside `impl PartialEq for RangeMap`)
- **Property**: `PartialEqMatchesIterEq`
- **Witness(es)**:
  - `witness_partial_eq_matches_iter_eq_case_same_start_different_end`
- **Source**: Fix PartialEq implementation for RangeMap
  > `RangeMap::PartialEq` delegated to `self.btm == other.btm`. The inner `BTreeMap` keys are `RangeStartWrapper`, which orders and compares only on `range.start`, so maps with matching starts but different ends (e.g. `{1..3 => 0}` vs `{1..4 => 0}`) compared equal. The fix switches to `self.iter().eq(other.iter())`.
- **Fix commit**: `b3a59e6641a9e3869791f781abbae98f828f91c9` — Fix PartialEq implementation for RangeMap
- **Invariant violated**: `a == b` must iff `a.iter().eq(b.iter())`. Two maps that differ in any stored range or value must not compare equal.
- **How the mutation triggers**: the pre-b3a59e6 impl delegated to `self.btm == other.btm`. The inner `BTreeMap` keys are `RangeStartWrapper` which orders solely by `range.start`, so `{1..3 => 0}` and `{1..4 => 0}` hash to identical keys and the BTreeMap equality coincidentally also compares values, *but* for the *keys* themselves BTreeMap only consults `PartialEq` of the wrapper — which in turn compares only the start. End/value mismatches silently slip through. The fix replaces this with `self.iter().eq(other.iter())`.
