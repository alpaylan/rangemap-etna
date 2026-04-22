# rangemap — ETNA Tasks

Total tasks: 16

## Task Index

| Task | Variant | Framework | Property | Witness |
|------|---------|-----------|----------|---------|
| 001 | `coalesce_contiguous_d1999f4_1` | proptest | `CoalesceNoAdjacentSameValue` | `witness_coalesce_no_adjacent_same_value_case_replace_middle` |
| 002 | `coalesce_contiguous_d1999f4_1` | quickcheck | `CoalesceNoAdjacentSameValue` | `witness_coalesce_no_adjacent_same_value_case_replace_middle` |
| 003 | `coalesce_contiguous_d1999f4_1` | crabcheck | `CoalesceNoAdjacentSameValue` | `witness_coalesce_no_adjacent_same_value_case_replace_middle` |
| 004 | `coalesce_contiguous_d1999f4_1` | hegel | `CoalesceNoAdjacentSameValue` | `witness_coalesce_no_adjacent_same_value_case_replace_middle` |
| 005 | `inclusive_equality_a6cdac3_1` | proptest | `InclusiveEqMatchesIterEq` | `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` |
| 006 | `inclusive_equality_a6cdac3_1` | quickcheck | `InclusiveEqMatchesIterEq` | `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` |
| 007 | `inclusive_equality_a6cdac3_1` | crabcheck | `InclusiveEqMatchesIterEq` | `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` |
| 008 | `inclusive_equality_a6cdac3_1` | hegel | `InclusiveEqMatchesIterEq` | `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` |
| 009 | `overlapping_backwards_6df612f_1` | proptest | `OverlappingReversible` | `witness_overlapping_reversible_case_trailing_non_overlap` |
| 010 | `overlapping_backwards_6df612f_1` | quickcheck | `OverlappingReversible` | `witness_overlapping_reversible_case_trailing_non_overlap` |
| 011 | `overlapping_backwards_6df612f_1` | crabcheck | `OverlappingReversible` | `witness_overlapping_reversible_case_trailing_non_overlap` |
| 012 | `overlapping_backwards_6df612f_1` | hegel | `OverlappingReversible` | `witness_overlapping_reversible_case_trailing_non_overlap` |
| 013 | `partialeq_map_b3a59e6_1` | proptest | `PartialEqMatchesIterEq` | `witness_partial_eq_matches_iter_eq_case_same_start_different_end` |
| 014 | `partialeq_map_b3a59e6_1` | quickcheck | `PartialEqMatchesIterEq` | `witness_partial_eq_matches_iter_eq_case_same_start_different_end` |
| 015 | `partialeq_map_b3a59e6_1` | crabcheck | `PartialEqMatchesIterEq` | `witness_partial_eq_matches_iter_eq_case_same_start_different_end` |
| 016 | `partialeq_map_b3a59e6_1` | hegel | `PartialEqMatchesIterEq` | `witness_partial_eq_matches_iter_eq_case_same_start_different_end` |

## Witness Catalog

- `witness_coalesce_no_adjacent_same_value_case_replace_middle` — base passes, variant fails
- `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` — base passes, variant fails
- `witness_overlapping_reversible_case_trailing_non_overlap` — base passes, variant fails
- `witness_partial_eq_matches_iter_eq_case_same_start_different_end` — base passes, variant fails
