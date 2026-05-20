use std::fmt;

use crabcheck::profiling::quickcheck;
use crabcheck::quickcheck::{Arbitrary, Mutate};
use rand_etna::Rng;
use rangemap::etna::{
    property_coalesce_no_adjacent_same_value, property_inclusive_eq_matches_iter_eq,
    property_overlapping_reversible, property_partial_eq_matches_iter_eq, PropertyResult,
};

#[derive(Clone)]
struct Inserts(Vec<(u32, u32, i32)>);
impl fmt::Debug for Inserts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}

#[derive(Clone)]
struct TwoInserts { a: Vec<(u32, u32, i32)>, b: Vec<(u32, u32, i32)> }
impl fmt::Debug for TwoInserts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a={:?} b={:?}", self.a, self.b)
    }
}

#[derive(Clone)]
struct OverlappingInput { inserts: Vec<(u32, u32, i32)>, qs: u32, qe: u32 }
impl fmt::Debug for OverlappingInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "inserts={:?} qs={} qe={}", self.inserts, self.qs, self.qe)
    }
}

fn gen_triples<R: Rng>(rng: &mut R, _max: usize) -> Vec<(u32, u32, i32)> {
    // Mirror existing crabcheck adapter: small ranges (s in 0..16, span 1..8,
    // v in -3..=3). Critical for the partialeq/inclusive bugs which need
    // structurally similar ranges to compare.
    let n: usize = (rng.random::<u32>() as usize) % 8;
    (0..n).map(|_| {
        let s: u32 = rng.random::<u32>() % 16;
        let span: u32 = 1 + (rng.random::<u32>() % 8);
        let v: i32 = rng.random::<i32>() % 4;
        (s, s.saturating_add(span), v)
    }).collect()
}

// Perturb-based generator for related (a, b) pairs — mirrors
// perturb_inserts_cc in src/bin/etna.rs. The partialeq/inclusive bugs only
// fire when b is structurally derived from a (one slot perturbed), not when
// a and b are independent random vectors.
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
    fn generate(rng: &mut R, _n: usize) -> Self { Inserts(gen_triples(rng, 8)) }
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
        },
        1 if out.len() < max => {
            let s = rng.random_range(0u32..32);
            let len = rng.random_range(1u32..16);
            out.push((s, s + len, rng.random_range(-4i32..=4)));
        },
        _ if !out.is_empty() => { out.pop(); },
        _ => {},
    }
    out
}

impl<R: Rng> Mutate<R> for Inserts {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self { Inserts(mutate_triples(rng, &self.0, 8)) }
}
impl<R: Rng> Mutate<R> for TwoInserts {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut out = self.clone();
        if rng.random_bool(0.5) { out.a = mutate_triples(rng, &out.a, 6); }
        else { out.b = mutate_triples(rng, &out.b, 6); }
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

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 { return; }
    let result = match (args[1].as_str(), args[2].as_str()) {
        ("crabcheck", "CoalesceNoAdjacentSameValue") => {
            quickcheck(|Inserts(v)| to_opt(property_coalesce_no_adjacent_same_value(v)))
        },
        ("crabcheck", "PartialEqMatchesIterEq") => {
            quickcheck(|t: TwoInserts| to_opt(property_partial_eq_matches_iter_eq(t.a, t.b)))
        },
        ("crabcheck", "InclusiveEqMatchesIterEq") => {
            quickcheck(|t: TwoInserts| to_opt(property_inclusive_eq_matches_iter_eq(t.a, t.b)))
        },
        ("crabcheck", "OverlappingReversible") => {
            quickcheck(|t: OverlappingInput| to_opt(property_overlapping_reversible(t.inserts, t.qs, t.qe)))
        },
        (a, b) => panic!("Unknown: {a} {b}"),
    };
    println!("Result: {:?}", result);
}
