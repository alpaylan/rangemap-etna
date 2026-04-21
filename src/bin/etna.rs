// ETNA workload runner for rangemap.
//
// Usage: cargo run --release --bin etna -- <tool> <property>
//   tool:     etna | proptest | quickcheck | crabcheck | hegel
//   property: CoalesceNoAdjacentSameValue
//           | PartialEqMatchesIterEq
//           | InclusiveEqMatchesIterEq
//           | OverlappingReversible
//           | All
//
// Every invocation prints exactly one JSON line on stdout and exits 0
// (except argv parsing, which exits 2). Adapters drive their own framework
// crate directly — no subprocess dispatch.

use rangemap::etna::{
    property_coalesce_no_adjacent_same_value, property_inclusive_eq_matches_iter_eq,
    property_overlapping_reversible, property_partial_eq_matches_iter_eq, PropertyResult,
};

use crabcheck::quickcheck as crabcheck_qc;
use crabcheck::quickcheck::Arbitrary as CcArbitrary;
use hegel::{generators as hgen, HealthCheck, Hegel, Settings as HegelSettings, TestCase};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestError};
use quickcheck_etna::{Arbitrary as QcArbitrary, Gen, QuickCheck, ResultStatus, TestResult};
use rand_etna::Rng;

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Default, Clone, Copy)]
struct Metrics {
    inputs: u64,
    elapsed_us: u128,
}

impl Metrics {
    fn combine(self, other: Metrics) -> Metrics {
        Metrics {
            inputs: self.inputs + other.inputs,
            elapsed_us: self.elapsed_us + other.elapsed_us,
        }
    }
}

type Outcome = (Result<(), String>, Metrics);

fn to_err(r: PropertyResult) -> Result<(), String> {
    match r {
        PropertyResult::Pass | PropertyResult::Discard => Ok(()),
        PropertyResult::Fail(m) => Err(m),
    }
}

const ALL_PROPERTIES: &[&str] = &[
    "CoalesceNoAdjacentSameValue",
    "PartialEqMatchesIterEq",
    "InclusiveEqMatchesIterEq",
    "OverlappingReversible",
];

fn cases_budget() -> u64 {
    std::env::var("ETNA_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200)
}

fn run_all<F: FnMut(&str) -> Outcome>(mut f: F) -> Outcome {
    let mut total = Metrics::default();
    for p in ALL_PROPERTIES {
        let (r, m) = f(p);
        total = total.combine(m);
        if let Err(e) = r {
            return (Err(e), total);
        }
    }
    (Ok(()), total)
}

// ============================================================================
// Input wrappers
// ============================================================================

#[derive(Clone)]
struct InsertsInput {
    items: Vec<(u32, u32, i32)>,
}

impl fmt::Debug for InsertsInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.items)
    }
}

impl fmt::Display for InsertsInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone)]
struct TwoMapsInput {
    a: Vec<(u32, u32, i32)>,
    b: Vec<(u32, u32, i32)>,
}

impl fmt::Debug for TwoMapsInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a={:?} b={:?}", self.a, self.b)
    }
}

impl fmt::Display for TwoMapsInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone)]
struct OverlappingInput {
    items: Vec<(u32, u32, i32)>,
    qs: u32,
    qe: u32,
}

impl fmt::Debug for OverlappingInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "items={:?} q={}..{}", self.items, self.qs, self.qe)
    }
}

impl fmt::Display for OverlappingInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// ============================================================================
// Canonical witness inputs — used by `tool=etna`. Must match witness tests
// in tests/etna_witnesses.rs.
// ============================================================================

fn canonical_coalesce() -> InsertsInput {
    InsertsInput {
        items: vec![(1, 3, 0), (3, 5, 1), (3, 5, 0)],
    }
}

fn canonical_partial_eq() -> TwoMapsInput {
    TwoMapsInput {
        a: vec![(1, 3, 0)],
        b: vec![(1, 4, 0)],
    }
}

fn canonical_inclusive_eq() -> TwoMapsInput {
    TwoMapsInput {
        a: vec![(0, 5, 0)],
        b: vec![(0, 2, 0)],
    }
}

fn canonical_overlapping() -> OverlappingInput {
    OverlappingInput {
        items: vec![(0, 5, 1), (10, 15, 2)],
        qs: 0,
        qe: 7,
    }
}

fn check_coalesce_no_adjacent_same_value() -> Result<(), String> {
    let v = canonical_coalesce();
    to_err(property_coalesce_no_adjacent_same_value(v.items))
}

fn check_partial_eq_matches_iter_eq() -> Result<(), String> {
    let v = canonical_partial_eq();
    to_err(property_partial_eq_matches_iter_eq(v.a, v.b))
}

fn check_inclusive_eq_matches_iter_eq() -> Result<(), String> {
    let v = canonical_inclusive_eq();
    to_err(property_inclusive_eq_matches_iter_eq(v.a, v.b))
}

fn check_overlapping_reversible() -> Result<(), String> {
    let v = canonical_overlapping();
    to_err(property_overlapping_reversible(v.items, v.qs, v.qe))
}

fn run_etna_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_etna_property);
    }
    let t0 = Instant::now();
    let result = match property {
        "CoalesceNoAdjacentSameValue" => check_coalesce_no_adjacent_same_value(),
        "PartialEqMatchesIterEq" => check_partial_eq_matches_iter_eq(),
        "InclusiveEqMatchesIterEq" => check_inclusive_eq_matches_iter_eq(),
        "OverlappingReversible" => check_overlapping_reversible(),
        _ => {
            return (
                Err(format!("Unknown property for etna: {property}")),
                Metrics::default(),
            );
        }
    };
    (
        result,
        Metrics {
            inputs: 1,
            elapsed_us: t0.elapsed().as_micros(),
        },
    )
}

// ============================================================================
// quickcheck Arbitrary
// ============================================================================

fn qc_gen_small_inserts(g: &mut Gen) -> Vec<(u32, u32, i32)> {
    let n: usize = <usize as QcArbitrary>::arbitrary(g) % 8;
    (0..n)
        .map(|_| {
            let s: u32 = <u32 as QcArbitrary>::arbitrary(g) % 16;
            let span: u32 = 1 + (<u32 as QcArbitrary>::arbitrary(g) % 8);
            let v: i32 = <i32 as QcArbitrary>::arbitrary(g) % 4;
            (s, s.saturating_add(span), v)
        })
        .collect()
}

impl QcArbitrary for InsertsInput {
    fn arbitrary(g: &mut Gen) -> Self {
        InsertsInput {
            items: qc_gen_small_inserts(g),
        }
    }
}

fn perturb_inserts_qc(a: &[(u32, u32, i32)], g: &mut Gen) -> Vec<(u32, u32, i32)> {
    let mut b: Vec<(u32, u32, i32)> = a.to_vec();
    let mode: u8 = <u8 as QcArbitrary>::arbitrary(g) % 4;
    match mode {
        0 => {}
        1 if !b.is_empty() => {
            let idx = (<usize as QcArbitrary>::arbitrary(g)) % b.len();
            let delta: u32 = 1 + (<u32 as QcArbitrary>::arbitrary(g) % 4);
            let (s, e, v) = b[idx];
            b[idx] = (s, e.saturating_add(delta), v);
        }
        2 if !b.is_empty() => {
            let idx = (<usize as QcArbitrary>::arbitrary(g)) % b.len();
            let (s, e, _v) = b[idx];
            let nv: i32 = <i32 as QcArbitrary>::arbitrary(g) % 4;
            b[idx] = (s, e, nv);
        }
        _ => {
            let s: u32 = <u32 as QcArbitrary>::arbitrary(g) % 16;
            let span: u32 = 1 + (<u32 as QcArbitrary>::arbitrary(g) % 8);
            let v: i32 = <i32 as QcArbitrary>::arbitrary(g) % 4;
            b.push((s, s.saturating_add(span), v));
        }
    }
    b
}

impl QcArbitrary for TwoMapsInput {
    fn arbitrary(g: &mut Gen) -> Self {
        let a = qc_gen_small_inserts(g);
        let b = perturb_inserts_qc(&a, g);
        TwoMapsInput { a, b }
    }
}

impl QcArbitrary for OverlappingInput {
    fn arbitrary(g: &mut Gen) -> Self {
        let items = qc_gen_small_inserts(g);
        let qs: u32 = <u32 as QcArbitrary>::arbitrary(g) % 16;
        let qspan: u32 = 1 + (<u32 as QcArbitrary>::arbitrary(g) % 16);
        OverlappingInput {
            items,
            qs,
            qe: qs.saturating_add(qspan),
        }
    }
}

// ============================================================================
// crabcheck Arbitrary
// ============================================================================

fn cc_gen_small_inserts<R: Rng>(rng: &mut R) -> Vec<(u32, u32, i32)> {
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

impl<R: Rng> CcArbitrary<R> for InsertsInput {
    fn generate(rng: &mut R, _n: usize) -> Self {
        InsertsInput {
            items: cc_gen_small_inserts(rng),
        }
    }
}

fn perturb_inserts_cc<R: Rng>(a: &[(u32, u32, i32)], rng: &mut R) -> Vec<(u32, u32, i32)> {
    let mut b: Vec<(u32, u32, i32)> = a.to_vec();
    let mode: u8 = (rng.random::<u32>() % 4) as u8;
    match mode {
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
            let nv: i32 = rng.random::<i32>() % 4;
            b[idx] = (s, e, nv);
        }
        _ => {
            let s: u32 = rng.random::<u32>() % 16;
            let span: u32 = 1 + (rng.random::<u32>() % 8);
            let v: i32 = rng.random::<i32>() % 4;
            b.push((s, s.saturating_add(span), v));
        }
    }
    b
}

impl<R: Rng> CcArbitrary<R> for TwoMapsInput {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let a = cc_gen_small_inserts(rng);
        let b = perturb_inserts_cc(&a, rng);
        TwoMapsInput { a, b }
    }
}

impl<R: Rng> CcArbitrary<R> for OverlappingInput {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let items = cc_gen_small_inserts(rng);
        let qs: u32 = rng.random::<u32>() % 16;
        let qspan: u32 = 1 + (rng.random::<u32>() % 16);
        OverlappingInput {
            items,
            qs,
            qe: qs.saturating_add(qspan),
        }
    }
}

// ============================================================================
// proptest strategies
// ============================================================================

fn inserts_strategy() -> BoxedStrategy<InsertsInput> {
    proptest::collection::vec((0u32..16, 1u32..9, -2i32..=2), 0..8usize)
        .prop_map(|raw| InsertsInput {
            items: raw
                .into_iter()
                .map(|(s, span, v)| (s, s.saturating_add(span), v))
                .collect(),
        })
        .boxed()
}

fn two_maps_strategy() -> BoxedStrategy<TwoMapsInput> {
    (
        proptest::collection::vec((0u32..16, 1u32..9, -2i32..=2), 0..8usize),
        0u8..4,
        0usize..16,
        1u32..5,
        -2i32..=2,
        0u32..16,
        1u32..9,
    )
        .prop_map(|(raw, mode, idx_seed, delta, new_v, new_s, new_span)| {
            let a: Vec<(u32, u32, i32)> = raw
                .into_iter()
                .map(|(s, span, v)| (s, s.saturating_add(span), v))
                .collect();
            let mut b = a.clone();
            match mode {
                0 => {}
                1 if !b.is_empty() => {
                    let i = idx_seed % b.len();
                    let (s, e, v) = b[i];
                    b[i] = (s, e.saturating_add(delta), v);
                }
                2 if !b.is_empty() => {
                    let i = idx_seed % b.len();
                    let (s, e, _v) = b[i];
                    b[i] = (s, e, new_v);
                }
                _ => {
                    b.push((new_s, new_s.saturating_add(new_span), new_v));
                }
            }
            TwoMapsInput { a, b }
        })
        .boxed()
}

fn overlapping_strategy() -> BoxedStrategy<OverlappingInput> {
    (
        proptest::collection::vec((0u32..16, 1u32..9, -2i32..=2), 0..8usize),
        0u32..16,
        1u32..17,
    )
        .prop_map(|(raw, qs, qspan)| OverlappingInput {
            items: raw
                .into_iter()
                .map(|(s, span, v)| (s, s.saturating_add(span), v))
                .collect(),
            qs,
            qe: qs.saturating_add(qspan),
        })
        .boxed()
}

// ============================================================================
// proptest adapter
// ============================================================================

fn run_proptest_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_proptest_property);
    }
    let counter = Arc::new(AtomicU64::new(0));
    let t0 = Instant::now();
    let cfg = ProptestConfig {
        cases: cases_budget().min(u32::MAX as u64) as u32,
        max_shrink_iters: 32,
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let c = counter.clone();
    let result: Result<(), String> = match property {
        "CoalesceNoAdjacentSameValue" => runner
            .run(&inserts_strategy(), move |v| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({:?})", v);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_coalesce_no_adjacent_same_value(v.items.clone())
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        "PartialEqMatchesIterEq" => runner
            .run(&two_maps_strategy(), move |v| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({:?})", v);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_partial_eq_matches_iter_eq(v.a.clone(), v.b.clone())
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        "InclusiveEqMatchesIterEq" => runner
            .run(&two_maps_strategy(), move |v| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({:?})", v);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_inclusive_eq_matches_iter_eq(v.a.clone(), v.b.clone())
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        "OverlappingReversible" => runner
            .run(&overlapping_strategy(), move |v| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({:?})", v);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_overlapping_reversible(v.items.clone(), v.qs, v.qe)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        _ => {
            return (
                Err(format!("Unknown property for proptest: {property}")),
                Metrics::default(),
            );
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = counter.load(Ordering::Relaxed);
    (result, Metrics { inputs, elapsed_us })
}

// ============================================================================
// quickcheck adapter (fork with `etna` feature — fn-pointer API)
// ============================================================================

static QC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn qc_coalesce(v: InsertsInput) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        property_coalesce_no_adjacent_same_value(v.items)
    }));
    match out {
        Ok(PropertyResult::Pass) => TestResult::passed(),
        Ok(PropertyResult::Discard) => TestResult::discard(),
        Ok(PropertyResult::Fail(_)) | Err(_) => TestResult::failed(),
    }
}

fn qc_partial_eq(v: TwoMapsInput) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        property_partial_eq_matches_iter_eq(v.a, v.b)
    }));
    match out {
        Ok(PropertyResult::Pass) => TestResult::passed(),
        Ok(PropertyResult::Discard) => TestResult::discard(),
        Ok(PropertyResult::Fail(_)) | Err(_) => TestResult::failed(),
    }
}

fn qc_inclusive_eq(v: TwoMapsInput) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        property_inclusive_eq_matches_iter_eq(v.a, v.b)
    }));
    match out {
        Ok(PropertyResult::Pass) => TestResult::passed(),
        Ok(PropertyResult::Discard) => TestResult::discard(),
        Ok(PropertyResult::Fail(_)) | Err(_) => TestResult::failed(),
    }
}

fn qc_overlapping(v: OverlappingInput) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        property_overlapping_reversible(v.items, v.qs, v.qe)
    }));
    match out {
        Ok(PropertyResult::Pass) => TestResult::passed(),
        Ok(PropertyResult::Discard) => TestResult::discard(),
        Ok(PropertyResult::Fail(_)) | Err(_) => TestResult::failed(),
    }
}

fn run_quickcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_quickcheck_property);
    }
    QC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let budget = cases_budget();
    let mut qc = QuickCheck::new()
        .tests(budget)
        .max_tests(budget.saturating_mul(4))
        .max_time(Duration::from_secs(86_400));
    let result = match property {
        "CoalesceNoAdjacentSameValue" => qc.quicktest(qc_coalesce as fn(InsertsInput) -> TestResult),
        "PartialEqMatchesIterEq" => {
            qc.quicktest(qc_partial_eq as fn(TwoMapsInput) -> TestResult)
        }
        "InclusiveEqMatchesIterEq" => {
            qc.quicktest(qc_inclusive_eq as fn(TwoMapsInput) -> TestResult)
        }
        "OverlappingReversible" => {
            qc.quicktest(qc_overlapping as fn(OverlappingInput) -> TestResult)
        }
        _ => {
            return (
                Err(format!("Unknown property for quickcheck: {property}")),
                Metrics::default(),
            );
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = QC_COUNTER.load(Ordering::Relaxed);
    let status = match result.status {
        ResultStatus::Finished => Ok(()),
        ResultStatus::Failed { arguments } => Err(format!("({})", arguments.join(" "))),
        ResultStatus::Aborted { err } => Err(format!("quickcheck aborted: {err:?}")),
        ResultStatus::TimedOut => Err("quickcheck timed out".to_string()),
        ResultStatus::GaveUp => Err(format!(
            "quickcheck gave up after {} tests",
            result.n_tests_passed
        )),
    };
    (status, Metrics { inputs, elapsed_us })
}

// ============================================================================
// crabcheck adapter (fn-pointer API)
// ============================================================================

static CC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn cc_coalesce(v: InsertsInput) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_coalesce_no_adjacent_same_value(v.items) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn cc_partial_eq(v: TwoMapsInput) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_partial_eq_matches_iter_eq(v.a, v.b) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn cc_inclusive_eq(v: TwoMapsInput) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_inclusive_eq_matches_iter_eq(v.a, v.b) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn cc_overlapping(v: OverlappingInput) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_overlapping_reversible(v.items, v.qs, v.qe) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn run_crabcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_crabcheck_property);
    }
    CC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let cc_config = crabcheck_qc::Config {
        tests: cases_budget(),
    };
    let result = match property {
        "CoalesceNoAdjacentSameValue" => {
            crabcheck_qc::quickcheck_with_config(cc_config, cc_coalesce)
        }
        "PartialEqMatchesIterEq" => {
            crabcheck_qc::quickcheck_with_config(cc_config, cc_partial_eq)
        }
        "InclusiveEqMatchesIterEq" => {
            crabcheck_qc::quickcheck_with_config(cc_config, cc_inclusive_eq)
        }
        "OverlappingReversible" => {
            crabcheck_qc::quickcheck_with_config(cc_config, cc_overlapping)
        }
        _ => {
            return (
                Err(format!("Unknown property for crabcheck: {property}")),
                Metrics::default(),
            );
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = CC_COUNTER.load(Ordering::Relaxed);
    let status = match result.status {
        crabcheck_qc::ResultStatus::Finished => Ok(()),
        crabcheck_qc::ResultStatus::Failed { arguments } => {
            Err(format!("({})", arguments.join(" ")))
        }
        crabcheck_qc::ResultStatus::TimedOut => Err("crabcheck timed out".to_string()),
        crabcheck_qc::ResultStatus::GaveUp => Err(format!(
            "crabcheck gave up: passed={}, discarded={}",
            result.passed, result.discarded
        )),
        crabcheck_qc::ResultStatus::Aborted { error } => {
            Err(format!("crabcheck aborted: {error}"))
        }
    };
    (status, Metrics { inputs, elapsed_us })
}

// ============================================================================
// hegel adapter (real hegeltest 0.3.7 — panic-on-cex API)
// ============================================================================

static HG_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hegel_settings() -> HegelSettings {
    HegelSettings::new()
        .test_cases(cases_budget())
        .suppress_health_check(HealthCheck::all())
}

fn hg_draw_u32_range(tc: &TestCase, max: u32) -> u32 {
    tc.draw(hgen::integers::<u32>().min_value(0).max_value(max))
}

fn hg_draw_i32_range(tc: &TestCase, range: i32) -> i32 {
    tc.draw(hgen::integers::<i32>().min_value(-range).max_value(range))
}

fn hg_draw_inserts(tc: &TestCase) -> Vec<(u32, u32, i32)> {
    let n = hg_draw_u32_range(tc, 7) as usize;
    (0..n)
        .map(|_| {
            let s = hg_draw_u32_range(tc, 15);
            let span = 1 + hg_draw_u32_range(tc, 7);
            let v = hg_draw_i32_range(tc, 2);
            (s, s.saturating_add(span), v)
        })
        .collect()
}

fn hg_draw_two_maps(tc: &TestCase) -> (Vec<(u32, u32, i32)>, Vec<(u32, u32, i32)>) {
    let a = hg_draw_inserts(tc);
    let mut b = a.clone();
    let mode = hg_draw_u32_range(tc, 3);
    match mode {
        0 => {}
        1 if !b.is_empty() => {
            let idx = hg_draw_u32_range(tc, (b.len() - 1) as u32) as usize;
            let delta = 1 + hg_draw_u32_range(tc, 3);
            let (s, e, v) = b[idx];
            b[idx] = (s, e.saturating_add(delta), v);
        }
        2 if !b.is_empty() => {
            let idx = hg_draw_u32_range(tc, (b.len() - 1) as u32) as usize;
            let (s, e, _v) = b[idx];
            let nv = hg_draw_i32_range(tc, 2);
            b[idx] = (s, e, nv);
        }
        _ => {
            let s = hg_draw_u32_range(tc, 15);
            let span = 1 + hg_draw_u32_range(tc, 7);
            let v = hg_draw_i32_range(tc, 2);
            b.push((s, s.saturating_add(span), v));
        }
    }
    (a, b)
}

fn run_hegel_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_hegel_property);
    }
    HG_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let settings = hegel_settings();
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match property {
        "CoalesceNoAdjacentSameValue" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let items = hg_draw_inserts(&tc);
                let cex = format!("(items={:?})", items);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_coalesce_no_adjacent_same_value(items.clone())
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{}", cex),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "PartialEqMatchesIterEq" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let (a, b) = hg_draw_two_maps(&tc);
                let cex = format!("(a={:?} b={:?})", a, b);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_partial_eq_matches_iter_eq(a.clone(), b.clone())
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{}", cex),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "InclusiveEqMatchesIterEq" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let (a, b) = hg_draw_two_maps(&tc);
                let cex = format!("(a={:?} b={:?})", a, b);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_inclusive_eq_matches_iter_eq(a.clone(), b.clone())
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{}", cex),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "OverlappingReversible" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let items = hg_draw_inserts(&tc);
                let qs = hg_draw_u32_range(&tc, 15);
                let qspan = 1 + hg_draw_u32_range(&tc, 15);
                let qe = qs.saturating_add(qspan);
                let cex = format!("(items={:?} q={}..{})", items, qs, qe);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_overlapping_reversible(items.clone(), qs, qe)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{}", cex),
                }
            })
            .settings(settings.clone())
            .run();
        }
        _ => panic!("__unknown_property:{}", property),
    }));
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = HG_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match run_result {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "hegel panicked with non-string payload".to_string()
            };
            if let Some(rest) = msg.strip_prefix("__unknown_property:") {
                return (
                    Err(format!("Unknown property for hegel: {rest}")),
                    Metrics::default(),
                );
            }
            Err(msg
                .strip_prefix("Property test failed: ")
                .unwrap_or(&msg)
                .to_string())
        }
    };
    (status, metrics)
}

// ============================================================================
// dispatch + main
// ============================================================================

fn run(tool: &str, property: &str) -> Outcome {
    match tool {
        "etna" => run_etna_property(property),
        "proptest" => run_proptest_property(property),
        "quickcheck" => run_quickcheck_property(property),
        "crabcheck" => run_crabcheck_property(property),
        "hegel" => run_hegel_property(property),
        _ => (Err(format!("Unknown tool: {tool}")), Metrics::default()),
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_json(
    tool: &str,
    property: &str,
    status: &str,
    metrics: Metrics,
    counterexample: Option<&str>,
    error: Option<&str>,
) {
    let cex = counterexample.map_or("null".to_string(), json_str);
    let err = error.map_or("null".to_string(), json_str);
    println!(
        "{{\"status\":{},\"tests\":{},\"discards\":0,\"time\":{},\"counterexample\":{},\"error\":{},\"tool\":{},\"property\":{}}}",
        json_str(status),
        metrics.inputs,
        json_str(&format!("{}us", metrics.elapsed_us)),
        cex,
        err,
        json_str(tool),
        json_str(property),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <tool> <property>", args[0]);
        eprintln!("Tools: etna | proptest | quickcheck | crabcheck | hegel");
        eprintln!(
            "Properties: CoalesceNoAdjacentSameValue | PartialEqMatchesIterEq | InclusiveEqMatchesIterEq | OverlappingReversible | All"
        );
        std::process::exit(2);
    }
    let (tool, property) = (args[1].as_str(), args[2].as_str());

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(tool, property)));
    std::panic::set_hook(previous_hook);

    let (result, metrics) = match caught {
        Ok(outcome) => outcome,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "panic with non-string payload".to_string()
            };
            emit_json(tool, property, "aborted", Metrics::default(), None, Some(&msg));
            return;
        }
    };

    match result {
        Ok(()) => emit_json(tool, property, "passed", metrics, None, None),
        Err(e) => emit_json(tool, property, "failed", metrics, Some(&e), None),
    }
}
