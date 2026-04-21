# rangemap — Injected Bugs

Total mutations: 4

## Bug Index

| # | Name | Variant | File | Injection | Fix Commit |
|---|------|---------|------|-----------|------------|
| 1 | `coalesce_contiguous`   | `coalesce_contiguous_d1999f4_1`  | `src/map.rs:284`           | `marauders` | `d1999f48003b43ec7ed598c9497b859a8302b897` |
| 2 | `partialeq_map`         | `partialeq_map_b3a59e6_1`        | `src/map.rs:60`            | `marauders` | `b3a59e6641a9e3869791f781abbae98f828f91c9` |
| 3 | `inclusive_equality`    | `inclusive_equality_a6cdac3_1`   | `src/inclusive_map.rs:73`  | `marauders` | `a6cdac3e99e747c9ec80b4ef238e1480e63927fb` |
| 4 | `overlapping_backwards` | `overlapping_backwards_6df612f_1`| `src/map.rs:832`           | `marauders` | `6df612f45d6023cab8eeec69e10fe794c70eacd8` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `coalesce_contiguous_d1999f4_1`   | `property_coalesce_no_adjacent_same_value` | `witness_coalesce_no_adjacent_same_value_case_replace_middle` |
| `partialeq_map_b3a59e6_1`         | `property_partial_eq_matches_iter_eq`      | `witness_partial_eq_matches_iter_eq_case_same_start_different_end` |
| `inclusive_equality_a6cdac3_1`    | `property_inclusive_eq_matches_iter_eq`    | `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` |
| `overlapping_backwards_6df612f_1` | `property_overlapping_reversible`          | `witness_overlapping_reversible_case_trailing_non_overlap` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `property_coalesce_no_adjacent_same_value` | ✓ | ✓ | ✓ | ✓ |
| `property_partial_eq_matches_iter_eq`      | ✓ | ✓ | ✓ | ✓ |
| `property_inclusive_eq_matches_iter_eq`    | ✓ | ✓ | ✓ | ✓ |
| `property_overlapping_reversible`          | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. coalesce_contiguous

- **Variant**: `coalesce_contiguous_d1999f4_1`
- **Location**: `src/map.rs:284` (inside `RangeMap::insert`, the step that gathers candidate preceding ranges)
- **Property**: `property_coalesce_no_adjacent_same_value`
- **Witness**: `witness_coalesce_no_adjacent_same_value_case_replace_middle`
- **Fix commit**: `d1999f48003b43ec7ed598c9497b859a8302b897` — `Fix coalescing of contiguous ranges`
- **Invariant violated**: After any sequence of `insert` calls the map must not contain two adjacent ranges that share the same value — the data structure's core coalescing invariant.
- **How the mutation triggers**: the fix changed `.take(2)` to `.take(1)` on the reverse-iteration candidate window. With `.take(1)` the insert only ever considers the single range immediately to the left of the new one. If a preceding insert has split an older range into two pieces, the mutation misses the older-older piece, and a freshly-replacing insert that makes the trailing piece share a value with the piece two steps back leaves two uncoalesced adjacent ranges. The witness inserts `1..3 => 0`, `3..5 => 1`, then overwrites `3..5 => 0`; the base coalesces into `1..5 => 0`, the mutation leaves both `1..3 => 0` and `3..5 => 0`.

### 2. partialeq_map

- **Variant**: `partialeq_map_b3a59e6_1`
- **Location**: `src/map.rs:60` (inside `impl PartialEq for RangeMap`)
- **Property**: `property_partial_eq_matches_iter_eq`
- **Witness**: `witness_partial_eq_matches_iter_eq_case_same_start_different_end`
- **Fix commit**: `b3a59e6641a9e3869791f781abbae98f828f91c9` — `Fix PartialEq implementation for RangeMap`
- **Invariant violated**: `a == b` must iff `a.iter().eq(b.iter())`. Two maps that differ in any stored range or value must not compare equal.
- **How the mutation triggers**: the pre-b3a59e6 impl delegated to `self.btm == other.btm`. The inner `BTreeMap` keys are `RangeStartWrapper` which orders solely by `range.start`, so `{1..3 => 0}` and `{1..4 => 0}` hash to identical keys and the BTreeMap equality coincidentally also compares values, *but* for the *keys* themselves BTreeMap only consults `PartialEq` of the wrapper — which in turn compares only the start. End/value mismatches silently slip through. The fix replaces this with `self.iter().eq(other.iter())`.

### 3. inclusive_equality

- **Variant**: `inclusive_equality_a6cdac3_1`
- **Location**: `src/inclusive_map.rs:73` (inside `impl PartialEq for RangeInclusiveMap`)
- **Property**: `property_inclusive_eq_matches_iter_eq`
- **Witness**: `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end`
- **Fix commit**: `a6cdac3e99e747c9ec80b4ef238e1480e63927fb` — `Fix RangeInclusiveMap equality comparison`
- **Invariant violated**: identical to `partialeq_map` but for the inclusive-range variant: `a == b` iff `a.iter().eq(b.iter())`.
- **How the mutation triggers**: same mechanism — pre-a6cdac3 the impl reads `self.btm == other.btm` and compares BTreeMap keys via a start-only wrapper. The witness uses `{0..=5 => 0}` vs. `{0..=2 => 0}` which share a start but differ in end and therefore in iter-equality but not in the buggy `==`.

### 4. overlapping_backwards

- **Variant**: `overlapping_backwards_6df612f_1`
- **Location**: `src/map.rs:832` (inside `impl DoubleEndedIterator for Overlapping::next_back`)
- **Property**: `property_overlapping_reversible`
- **Witness**: `witness_overlapping_reversible_case_trailing_non_overlap`
- **Fix commit**: `6df612f45d6023cab8eeec69e10fe794c70eacd8` — `Fixes overlapping backwards iterator`
- **Invariant violated**: reverse iteration of `Overlapping` must yield the same set of ranges as forward iteration. In particular, if the inner BTreeMap's reverse cursor lands on a range whose start is past the query end, `next_back` must *skip* that range and keep walking backwards — not bail out.
- **How the mutation triggers**: the pre-6df612f impl uses a single `if let Some(_) = next_back()` followed by a start-bound check; if the check fails it returns `None` and the whole iterator terminates. The fix turns this into `while let Some(_)` so non-overlapping trailing ranges are skipped over. The witness stores `0..5` and `10..15`, queries `0..7`, and calls `next_back()`: forward yields `(0..5)`, base's reverse also yields `(0..5)` (skipping the trailing `10..15`), the mutation returns `None` immediately because the first reverse candidate is `10..15` whose start exceeds the query end.
