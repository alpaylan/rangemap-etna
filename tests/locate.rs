//! End-to-end fault-localization tests for `rangemap` properties.
//!
//! Each `#[test]` runs `crabcheck::quickcheck_with_locate!` on one property
//! from `etna-faultloc.rs`. Tests never panic — they print the LocateResult
//! and emit one `@@LOCATE@@ <json>` line per property so a harness can
//! collect machine-readable suspect summaries.

#![cfg(feature = "etna")]

use std::fmt;

use crabcheck::quickcheck::{Arbitrary, Mutate};
use rand_etna::Rng;
use rangemap::etna::{
    property_coalesce_no_adjacent_same_value, property_inclusive_eq_matches_iter_eq,
    property_overlapping_reversible, property_partial_eq_matches_iter_eq, PropertyResult,
};

#[derive(Clone)]
struct Inserts(Vec<(u32, u32, i32)>);
impl fmt::Debug for Inserts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone)]
struct TwoInserts {
    a: Vec<(u32, u32, i32)>,
    b: Vec<(u32, u32, i32)>,
}
impl fmt::Debug for TwoInserts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a={:?} b={:?}", self.a, self.b)
    }
}

#[derive(Clone)]
struct OverlappingInput {
    inserts: Vec<(u32, u32, i32)>,
    qs: u32,
    qe: u32,
}
impl fmt::Debug for OverlappingInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "inserts={:?} qs={} qe={}", self.inserts, self.qs, self.qe)
    }
}

fn gen_triples<R: Rng>(rng: &mut R, _max: usize) -> Vec<(u32, u32, i32)> {
    let n: usize = (rng.random::<u32>() as usize) % 8;
    (0..n)
        .map(|_| {
            let s: u32 = rng.random::<u32>() % 16;
            let span: u32 = 1 + (rng.random::<u32>() % 8);
            let v: i32 = rng.random::<i32>() % 4;
            (s, s.saturating_add(span), v)
        })
        .collect()
}

fn perturb_triples<R: Rng>(a: &[(u32, u32, i32)], rng: &mut R) -> Vec<(u32, u32, i32)> {
    let mut b: Vec<(u32, u32, i32)> = a.to_vec();
    match (rng.random::<u32>() % 4) as u8 {
        0 => {}
        1 if !b.is_empty() => {
            let idx = (rng.random::<u32>() as usize) % b.len();
            let delta: u32 = 1 + (rng.random::<u32>() % 4);
            let (s, e, v) = b[idx];
            b[idx] = (s, e.saturating_add(delta), v);
        }
        2 if !b.is_empty() => {
            let idx = (rng.random::<u32>() as usize) % b.len();
            let (s, e, _v) = b[idx];
            b[idx] = (s, e, rng.random::<i32>() % 4);
        }
        _ => {
            let s: u32 = rng.random::<u32>() % 16;
            let span: u32 = 1 + (rng.random::<u32>() % 8);
            b.push((s, s.saturating_add(span), rng.random::<i32>() % 4));
        }
    }
    b
}

impl<R: Rng> Arbitrary<R> for Inserts {
    fn generate(rng: &mut R, _n: usize) -> Self {
        Inserts(gen_triples(rng, 8))
    }
}
impl<R: Rng> Arbitrary<R> for TwoInserts {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let a = gen_triples(rng, 6);
        let b = perturb_triples(&a, rng);
        TwoInserts { a, b }
    }
}
impl<R: Rng> Arbitrary<R> for OverlappingInput {
    fn generate(rng: &mut R, _n: usize) -> Self {
        OverlappingInput {
            inserts: gen_triples(rng, 6),
            qs: rng.random_range(0u32..40),
            qe: rng.random_range(0u32..40),
        }
    }
}

fn mutate_triples<R: Rng>(rng: &mut R, v: &[(u32, u32, i32)], max: usize) -> Vec<(u32, u32, i32)> {
    let mut out = v.to_vec();
    match rng.random_range(0u8..3) {
        0 if !out.is_empty() => {
            let i = rng.random_range(0..out.len());
            let (s, e, val) = out[i];
            match rng.random_range(0u8..3) {
                0 => out[i] = (s.wrapping_add(1), e, val),
                1 => out[i] = (s, e.wrapping_add(1), val),
                _ => out[i] = (s, e, val.wrapping_add(1)),
            }
        }
        1 if out.len() < max => {
            let s = rng.random_range(0u32..32);
            let len = rng.random_range(1u32..16);
            out.push((s, s + len, rng.random_range(-4i32..=4)));
        }
        _ if !out.is_empty() => {
            out.pop();
        }
        _ => {}
    }
    out
}

impl<R: Rng> Mutate<R> for Inserts {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        Inserts(mutate_triples(rng, &self.0, 8))
    }
}
impl<R: Rng> Mutate<R> for TwoInserts {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut out = self.clone();
        if rng.random_bool(0.5) {
            out.a = mutate_triples(rng, &out.a, 6);
        } else {
            out.b = mutate_triples(rng, &out.b, 6);
        }
        out
    }
}
impl<R: Rng> Mutate<R> for OverlappingInput {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut out = self.clone();
        match rng.random_range(0u8..3) {
            0 => out.inserts = mutate_triples(rng, &out.inserts, 6),
            1 => out.qs = (out.qs.wrapping_add(1)) % 40,
            _ => out.qe = (out.qe.wrapping_add(1)) % 40,
        }
        out
    }
}

fn to_opt(r: PropertyResult) -> Option<bool> {
    match r {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn coalesce_no_adjacent_same_value_wrapper(Inserts(v): Inserts) -> Option<bool> {
    to_opt(property_coalesce_no_adjacent_same_value(v))
}

fn partial_eq_matches_iter_eq_wrapper(t: TwoInserts) -> Option<bool> {
    to_opt(property_partial_eq_matches_iter_eq(t.a, t.b))
}

fn inclusive_eq_matches_iter_eq_wrapper(t: TwoInserts) -> Option<bool> {
    to_opt(property_inclusive_eq_matches_iter_eq(t.a, t.b))
}

fn overlapping_reversible_wrapper(t: OverlappingInput) -> Option<bool> {
    to_opt(property_overlapping_reversible(t.inserts, t.qs, t.qe))
}

fn emit_locate_json(r: &crabcheck::profiling::LocateResult) {
    use crabcheck::quickcheck::ResultStatus;
    let status = match &r.run.status {
        ResultStatus::Failed { .. } => "Failed",
        ResultStatus::Finished => "Finished",
        ResultStatus::GaveUp => "GaveUp",
        ResultStatus::TimedOut => "TimedOut",
        ResultStatus::Aborted { .. } => "Aborted",
    };
    let top = if let Some(s) = r.top() {
        serde_json::json!({
            "rank": s.rank,
            "file": s.region.file,
            "function": s.region.function,
            "start_line": s.region.start_line,
            "end_line": s.region.end_line,
            "ochiai": s.region.suspiciousness.ochiai,
            "delta": s.region.delta,
            "panic_overlap": s.panic_overlap,
            "confidence": format!("{}", s.confidence),
            "confidence_rule": s.confidence_rule,
        })
    } else {
        serde_json::Value::Null
    };
    let top_5: Vec<_> = r
        .suspects
        .iter()
        .take(5)
        .map(|s| {
            serde_json::json!({
                "rank": s.rank,
                "file": s.region.file,
                "function": s.region.function,
                "start_line": s.region.start_line,
                "end_line": s.region.end_line,
                "confidence": format!("{}", s.confidence),
                "confidence_rule": s.confidence_rule,
                "panic_overlap": s.panic_overlap,
            })
        })
        .collect();
    let diags: Vec<_> = r.diagnostics.iter().map(|d| d.tag()).collect();
    let out = serde_json::json!({
        "status": status,
        "passed": r.run.passed,
        "discarded": r.run.discarded,
        "n_panics": r.n_panics,
        "n_suspects": r.suspects.len(),
        "top": top,
        "top_5": top_5,
        "diagnostics": diags,
    });
    println!("@@LOCATE@@ {}", out);
}

#[test]
fn locate_coalesce_no_adjacent_same_value() {
    let report = crabcheck::quickcheck_with_locate!(
        coalesce_no_adjacent_same_value_wrapper,
        "rangemap"
    );
    eprintln!("{report}");
    emit_locate_json(&report);
}

#[test]
fn locate_partial_eq_matches_iter_eq() {
    let report =
        crabcheck::quickcheck_with_locate!(partial_eq_matches_iter_eq_wrapper, "rangemap");
    eprintln!("{report}");
    emit_locate_json(&report);
}

#[test]
fn locate_inclusive_eq_matches_iter_eq() {
    let report =
        crabcheck::quickcheck_with_locate!(inclusive_eq_matches_iter_eq_wrapper, "rangemap");
    eprintln!("{report}");
    emit_locate_json(&report);
}

#[test]
fn locate_overlapping_reversible() {
    let report = crabcheck::quickcheck_with_locate!(overlapping_reversible_wrapper, "rangemap");
    eprintln!("{report}");
    emit_locate_json(&report);
}
