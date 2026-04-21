# rangemap — ETNA Tasks

Total tasks: 16

ETNA tasks are **mutation/property/witness triplets**. Each row below is one runnable task.

## Task Index

| Task | Variant | Framework | Property | Witness | Command |
|------|---------|-----------|----------|---------|---------|
| 001  | `coalesce_contiguous_d1999f4_1`   | proptest   | `property_coalesce_no_adjacent_same_value` | `witness_coalesce_no_adjacent_same_value_case_replace_middle`        | `cargo run --release --features etna --bin etna -- proptest CoalesceNoAdjacentSameValue` |
| 002  | `coalesce_contiguous_d1999f4_1`   | quickcheck | `property_coalesce_no_adjacent_same_value` | `witness_coalesce_no_adjacent_same_value_case_replace_middle`        | `cargo run --release --features etna --bin etna -- quickcheck CoalesceNoAdjacentSameValue` |
| 003  | `coalesce_contiguous_d1999f4_1`   | crabcheck  | `property_coalesce_no_adjacent_same_value` | `witness_coalesce_no_adjacent_same_value_case_replace_middle`        | `cargo run --release --features etna --bin etna -- crabcheck CoalesceNoAdjacentSameValue` |
| 004  | `coalesce_contiguous_d1999f4_1`   | hegel      | `property_coalesce_no_adjacent_same_value` | `witness_coalesce_no_adjacent_same_value_case_replace_middle`        | `cargo run --release --features etna --bin etna -- hegel CoalesceNoAdjacentSameValue` |
| 005  | `partialeq_map_b3a59e6_1`         | proptest   | `property_partial_eq_matches_iter_eq`      | `witness_partial_eq_matches_iter_eq_case_same_start_different_end`   | `cargo run --release --features etna --bin etna -- proptest PartialEqMatchesIterEq` |
| 006  | `partialeq_map_b3a59e6_1`         | quickcheck | `property_partial_eq_matches_iter_eq`      | `witness_partial_eq_matches_iter_eq_case_same_start_different_end`   | `cargo run --release --features etna --bin etna -- quickcheck PartialEqMatchesIterEq` |
| 007  | `partialeq_map_b3a59e6_1`         | crabcheck  | `property_partial_eq_matches_iter_eq`      | `witness_partial_eq_matches_iter_eq_case_same_start_different_end`   | `cargo run --release --features etna --bin etna -- crabcheck PartialEqMatchesIterEq` |
| 008  | `partialeq_map_b3a59e6_1`         | hegel      | `property_partial_eq_matches_iter_eq`      | `witness_partial_eq_matches_iter_eq_case_same_start_different_end`   | `cargo run --release --features etna --bin etna -- hegel PartialEqMatchesIterEq` |
| 009  | `inclusive_equality_a6cdac3_1`    | proptest   | `property_inclusive_eq_matches_iter_eq`    | `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` | `cargo run --release --features etna --bin etna -- proptest InclusiveEqMatchesIterEq` |
| 010  | `inclusive_equality_a6cdac3_1`    | quickcheck | `property_inclusive_eq_matches_iter_eq`    | `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` | `cargo run --release --features etna --bin etna -- quickcheck InclusiveEqMatchesIterEq` |
| 011  | `inclusive_equality_a6cdac3_1`    | crabcheck  | `property_inclusive_eq_matches_iter_eq`    | `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` | `cargo run --release --features etna --bin etna -- crabcheck InclusiveEqMatchesIterEq` |
| 012  | `inclusive_equality_a6cdac3_1`    | hegel      | `property_inclusive_eq_matches_iter_eq`    | `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` | `cargo run --release --features etna --bin etna -- hegel InclusiveEqMatchesIterEq` |
| 013  | `overlapping_backwards_6df612f_1` | proptest   | `property_overlapping_reversible`          | `witness_overlapping_reversible_case_trailing_non_overlap`           | `cargo run --release --features etna --bin etna -- proptest OverlappingReversible` |
| 014  | `overlapping_backwards_6df612f_1` | quickcheck | `property_overlapping_reversible`          | `witness_overlapping_reversible_case_trailing_non_overlap`           | `cargo run --release --features etna --bin etna -- quickcheck OverlappingReversible` |
| 015  | `overlapping_backwards_6df612f_1` | crabcheck  | `property_overlapping_reversible`          | `witness_overlapping_reversible_case_trailing_non_overlap`           | `cargo run --release --features etna --bin etna -- crabcheck OverlappingReversible` |
| 016  | `overlapping_backwards_6df612f_1` | hegel      | `property_overlapping_reversible`          | `witness_overlapping_reversible_case_trailing_non_overlap`           | `cargo run --release --features etna --bin etna -- hegel OverlappingReversible` |

## Witness catalog

Each witness is a deterministic concrete test. Base build: passes. Variant-active build: fails.

- `witness_coalesce_no_adjacent_same_value_case_replace_middle` — insert `1..3 => 0`, then `3..5 => 1`, then overwrite `3..5 => 0` on a fresh `RangeMap<u32, i32>`. Base coalesces into a single `1..5 => 0`. The mutation leaves `1..3 => 0` and `3..5 => 0` uncoalesced because the candidate scan only considers the immediately-preceding range.
- `witness_partial_eq_matches_iter_eq_case_same_start_different_end` — compares `RangeMap::from([(1..3, 0)])` with `RangeMap::from([(1..4, 0)])`. Base: `a != b` and `iter().eq(iter())` both `false`. Mutation: delegates `==` to the inner `BTreeMap`, which orders keys by range start only, so `a == b` incorrectly reports `true`.
- `witness_inclusive_eq_matches_iter_eq_case_same_start_different_end` — compares `RangeInclusiveMap::from([(0..=5, 0)])` with `RangeInclusiveMap::from([(0..=2, 0)])`. Same bug class as `partialeq_map`; base reports `!=` iff `iter_eq` is `false`, mutation reports `==` because the start-only key ordering hides the end mismatch.
- `witness_overlapping_reversible_case_trailing_non_overlap` — insert `0..5 => 1` and `10..15 => 2`; call `overlapping(0..7).next_back()`. Base skips the non-overlapping `10..15` (whose start is past the query end) and returns `(0..5)`. Mutation returns `None` immediately because `next_back` gives up after the first non-overlapping candidate.

## Execution notes

- Commands above assume the `etna` cargo feature is activated (`--features etna`).
- To activate a specific variant, set the matching env var before running, e.g. `M_coalesce_contiguous_d1999f4_1=active cargo run --features etna --bin etna -- proptest CoalesceNoAdjacentSameValue`. Variants must first be converted to **functional** marauders form (`marauders convert --path src/... --to functional`).
- Runner exits 0 regardless of pass/fail and emits a single JSON line on stdout: `{"status":"passed|failed", "tests":N, "discards":0, "time":"Xus", "counterexample":"...", "error":null, "tool":"proptest|...", "property":"..."}`. Non-zero exit only on argv parsing errors.
- Budget is tunable via the `ETNA_CASES` env var; default 200.
