use std::{
    env, fs,
    hint::black_box,
    time::{Duration, Instant},
};

use anyhow::{bail, ensure, Context, Result};
use lean_multisig_comparison::{
    BenchmarkReport, ComparisonReport, FixtureSet, RunConfig, SampleSummary, SupplementalReport,
    LIGHTHOUSE_REVISION,
};

const SEMANTIC_WARNING: &str = "WARNING: leanMultisig aggregation generates zkVM proofs, while Lighthouse BLS aggregation combines elliptic-curve points; these operations do not have identical security semantics.";
const MIN_LIGHTHOUSE_BATCH_DURATION: Duration = Duration::from_millis(10);

fn main() -> Result<()> {
    let config =
        RunConfig::parse_from(env::args_os()).context("failed to parse runner arguments")?;
    run(config)
}

fn run(config: RunConfig) -> Result<()> {
    lean_multisig::setup();

    let mut comparisons =
        Vec::with_capacity((config.same_sizes.len() + config.distinct_sizes.len()) * 2);
    let mut supplemental = Vec::with_capacity(config.distinct_sizes.len());

    for &size in &config.same_sizes {
        let same_claim = FixtureSet::same_claim(size).with_context(|| {
            format!("failed to build same-claim fixtures for input size {size}")
        })?;
        let same_claim_value = same_claim.lean_claims()[0];
        let same_claim_signers = same_claim.lean_public_keys().to_vec();
        let same_claim_bls_message = same_claim.bls_messages()[0];
        let same_claim_bls_public_keys = same_claim.bls_public_keys().iter().collect::<Vec<_>>();

        let (aggregate_row, lean_aggregate, bls_aggregate) = measure_same_claim_aggregate(
            &same_claim,
            &same_claim_signers,
            &same_claim_bls_public_keys,
            config.samples,
            size,
            config.warmup_proofs,
        )?;
        comparisons.push(aggregate_row);
        comparisons.push(measure_same_claim_verify(
            &lean_aggregate,
            &bls_aggregate,
            &same_claim_value,
            &same_claim_signers,
            same_claim_bls_message,
            &same_claim_bls_public_keys,
            config.samples,
            size,
        )?);
    }

    for &size in &config.distinct_sizes {
        let distinct_claims = FixtureSet::distinct_claims(size).with_context(|| {
            format!("failed to build distinct-claim fixtures for input size {size}")
        })?;
        let distinct_expected = distinct_claims
            .lean_claims()
            .iter()
            .zip(distinct_claims.lean_public_keys())
            .map(|(claim, public_key)| lean_multisig::ClaimSigners {
                claim: *claim,
                signers: vec![*public_key],
            })
            .collect::<Vec<_>>();
        let distinct_bls_messages = distinct_claims.bls_messages().to_vec();
        let distinct_bls_public_keys = distinct_claims.bls_public_keys().iter().collect::<Vec<_>>();

        let (aggregate_row, lean_aggregate, bls_aggregate) = measure_distinct_claim_aggregate(
            &distinct_claims,
            &distinct_expected,
            &distinct_bls_messages,
            &distinct_bls_public_keys,
            config.samples,
            size,
            config.warmup_proofs,
        )?;
        comparisons.push(aggregate_row);
        comparisons.push(measure_distinct_claim_verify(
            &lean_aggregate,
            &bls_aggregate,
            &distinct_expected,
            &distinct_bls_messages,
            &distinct_bls_public_keys,
            config.samples,
            size,
        )?);
        supplemental.push(measure_signature_sets_verify(
            &distinct_claims,
            config.samples,
            size,
        )?);
    }

    let report = BenchmarkReport {
        lighthouse_revision: LIGHTHOUSE_REVISION.to_owned(),
        samples: config.samples,
        proof_warmup: config.warmup_proofs,
        comparisons,
        supplemental,
    };

    println!("{SEMANTIC_WARNING}");
    println!("Pinned Lighthouse revision: {LIGHTHOUSE_REVISION}");
    if config.warmup_proofs {
        println!(
            "PROOF TIMING MODE: steady-state (--warmup-proofs; one verified untimed Lean proof per aggregation workload and size)."
        );
    } else {
        println!(
            "PROOF TIMING MODE: first-use-inclusive (default; no untimed Lean proof generation)."
        );
    }
    if config.samples == 1 {
        println!("NOTE: a one-sample smoke run is not a publishable benchmark result.");
    }
    println!();
    println!("{}", report.to_table());

    if let Some(path) = config.json_path {
        let json = serde_json::to_string_pretty(&report)
            .context("failed to serialize benchmark report as JSON")?;
        fs::write(&path, json)
            .with_context(|| format!("failed to write JSON report to {}", path.display()))?;
    }

    Ok(())
}

fn measure_same_claim_aggregate(
    fixtures: &FixtureSet,
    lean_signers: &[lean_multisig::PublicKey],
    bls_public_keys: &[&lighthouse_bls::PublicKey],
    samples: usize,
    size: usize,
    warmup_proofs: bool,
) -> Result<(
    ComparisonReport,
    lean_multisig::Signature,
    lighthouse_bls::AggregateSignature,
)> {
    const WORKLOAD: &str = "same_claim_aggregate";
    let claim = fixtures.lean_claims()[0];
    let bls_message = fixtures.bls_messages()[0];

    let warmup_lean = run_optional_warmup(warmup_proofs, || {
        let aggregate = lean_multisig::aggregate(fixtures.lean_signatures().to_vec(), &claim)
            .with_context(|| {
                format!("{WORKLOAD} (input size {size}): Lean proof warm-up failed")
            })?;
        lean_multisig::verify(&aggregate, lean_signers, &claim).with_context(|| {
            format!("{WORKLOAD} (input size {size}): Lean proof warm-up failed verification")
        })?;
        Ok(aggregate)
    })?;

    let lighthouse_iterations = calibrate_batch_iterations(
        MIN_LIGHTHOUSE_BATCH_DURATION,
        |iterations| {
            let (elapsed, aggregate) = time_last_value_batch(iterations, || {
                aggregate_bls(black_box(fixtures.bls_signatures()))
            })?;
            ensure!(
                aggregate.fast_aggregate_verify(bls_message, bls_public_keys),
                "{WORKLOAD} (input size {size}): Lighthouse calibration aggregate failed verification"
            );
            Ok(elapsed)
        },
    )
    .with_context(|| {
        format!("{WORKLOAD} (input size {size}): Lighthouse calibration failed")
    })?;

    let mut lean_durations = Vec::with_capacity(samples);
    let mut lighthouse_durations = Vec::with_capacity(samples);
    let mut retained_lean = None;
    let mut retained_lighthouse = None;
    for_each_paired_sample(
        samples,
        |sample| {
            let signatures = fixtures.lean_signatures().to_vec();
            let start = Instant::now();
            let aggregate = lean_multisig::aggregate(signatures, &claim);
            let duration = start.elapsed();
            let aggregate = aggregate.with_context(|| {
                format!("{WORKLOAD} (input size {size}, sample {sample}): Lean aggregation failed")
            })?;
            lean_multisig::verify(&aggregate, lean_signers, &claim).with_context(|| {
                format!(
                    "{WORKLOAD} (input size {size}, sample {sample}): Lean aggregate failed post-timing verification"
                )
            })?;
            lean_durations.push(duration);
            retained_lean.get_or_insert(aggregate);
            Ok(())
        },
        |sample| {
            let aggregate = record_batched_sample(
                &mut lighthouse_durations,
                lighthouse_iterations,
                |iterations| {
                    time_last_value_batch(iterations, || {
                        aggregate_bls(black_box(fixtures.bls_signatures()))
                    })
                },
                |aggregate| {
                    ensure!(
                        aggregate.fast_aggregate_verify(bls_message, bls_public_keys),
                        "{WORKLOAD} (input size {size}, sample {sample}): Lighthouse aggregate failed post-timing verification"
                    );
                    Ok(())
                },
            )?;
            retained_lighthouse.get_or_insert(aggregate);
            Ok(())
        },
    )?;

    let timed_lean = retained_lean.with_context(|| {
        format!("{WORKLOAD} (input size {size}): retained no Lean aggregate proof")
    })?;
    let timed_lighthouse = retained_lighthouse.with_context(|| {
        format!("{WORKLOAD} (input size {size}): retained no Lighthouse aggregate signature")
    })?;
    let lean_artifact_bytes = timed_lean.to_bytes().len();
    let lighthouse_artifact_bytes = timed_lighthouse.serialize().len();
    let report = ComparisonReport::new(
        WORKLOAD,
        size,
        summarize(lean_durations, WORKLOAD, size, "Lean")?,
        summarize(lighthouse_durations, WORKLOAD, size, "Lighthouse")?,
        lean_artifact_bytes,
        lighthouse_artifact_bytes,
    )
    .with_context(|| format!("failed to assemble {WORKLOAD} report for input size {size}"))?;

    let verification_lean = warmup_lean.unwrap_or_else(|| timed_lean.clone());
    Ok((report, verification_lean, timed_lighthouse))
}

#[allow(clippy::too_many_arguments)]
fn measure_same_claim_verify(
    lean_aggregate: &lean_multisig::Signature,
    bls_aggregate: &lighthouse_bls::AggregateSignature,
    claim: &lean_multisig::Claim,
    lean_signers: &[lean_multisig::PublicKey],
    bls_message: lighthouse_bls::Hash256,
    bls_public_keys: &[&lighthouse_bls::PublicKey],
    samples: usize,
    size: usize,
) -> Result<ComparisonReport> {
    const WORKLOAD: &str = "same_claim_verify";
    let lighthouse_iterations =
        calibrate_batch_iterations(MIN_LIGHTHOUSE_BATCH_DURATION, |iterations| {
            let (elapsed, all_valid) = time_bool_batch(iterations, || {
                black_box(bls_aggregate)
                    .fast_aggregate_verify(black_box(bls_message), black_box(bls_public_keys))
            })?;
            ensure!(
                all_valid,
                "{WORKLOAD} (input size {size}): Lighthouse calibration verification failed"
            );
            Ok(elapsed)
        })
        .with_context(|| {
            format!("{WORKLOAD} (input size {size}): Lighthouse calibration failed")
        })?;

    let mut lean_durations = Vec::with_capacity(samples);
    let mut lighthouse_durations = Vec::with_capacity(samples);
    for_each_paired_sample(
        samples,
        |sample| {
            let start = Instant::now();
            let result = lean_multisig::verify(lean_aggregate, lean_signers, claim);
            let duration = start.elapsed();
            result.with_context(|| {
                format!("{WORKLOAD} (input size {size}, sample {sample}): Lean verification failed")
            })?;
            lean_durations.push(duration);
            Ok(())
        },
        |sample| {
            record_batched_sample(
                &mut lighthouse_durations,
                lighthouse_iterations,
                |iterations| {
                    time_bool_batch(iterations, || {
                        black_box(bls_aggregate).fast_aggregate_verify(
                            black_box(bls_message),
                            black_box(bls_public_keys),
                        )
                    })
                },
                |all_valid| {
                    ensure!(
                        *all_valid,
                        "{WORKLOAD} (input size {size}, sample {sample}): Lighthouse verification batch contained an invalid result"
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;

    ComparisonReport::new(
        WORKLOAD,
        size,
        summarize(lean_durations, WORKLOAD, size, "Lean")?,
        summarize(lighthouse_durations, WORKLOAD, size, "Lighthouse")?,
        lean_aggregate.to_bytes().len(),
        bls_aggregate.serialize().len(),
    )
    .with_context(|| format!("failed to assemble {WORKLOAD} report for input size {size}"))
}

fn measure_distinct_claim_aggregate(
    fixtures: &FixtureSet,
    lean_expected: &[lean_multisig::ClaimSigners],
    bls_messages: &[lighthouse_bls::Hash256],
    bls_public_keys: &[&lighthouse_bls::PublicKey],
    samples: usize,
    size: usize,
    warmup_proofs: bool,
) -> Result<(
    ComparisonReport,
    lean_multisig::MultiClaimProof,
    lighthouse_bls::AggregateSignature,
)> {
    const WORKLOAD: &str = "distinct_claim_aggregate";

    let warmup_lean = run_optional_warmup(warmup_proofs, || {
        let aggregate = lean_multisig::merge_claims(fixtures.lean_signatures().to_vec())
            .with_context(|| {
                format!("{WORKLOAD} (input size {size}): Lean proof warm-up failed")
            })?;
        lean_multisig::verify_claims(&aggregate, lean_expected).with_context(|| {
            format!("{WORKLOAD} (input size {size}): Lean proof warm-up failed verification")
        })?;
        Ok(aggregate)
    })?;

    let lighthouse_iterations = calibrate_batch_iterations(
        MIN_LIGHTHOUSE_BATCH_DURATION,
        |iterations| {
            let (elapsed, aggregate) = time_last_value_batch(iterations, || {
                aggregate_bls(black_box(fixtures.bls_signatures()))
            })?;
            ensure!(
                aggregate.aggregate_verify(bls_messages, bls_public_keys),
                "{WORKLOAD} (input size {size}): Lighthouse calibration aggregate failed verification"
            );
            Ok(elapsed)
        },
    )
    .with_context(|| {
        format!("{WORKLOAD} (input size {size}): Lighthouse calibration failed")
    })?;

    let mut lean_durations = Vec::with_capacity(samples);
    let mut lighthouse_durations = Vec::with_capacity(samples);
    let mut retained_lean = None;
    let mut retained_lighthouse = None;
    for_each_paired_sample(
        samples,
        |sample| {
            let signatures = fixtures.lean_signatures().to_vec();
            let start = Instant::now();
            let aggregate = lean_multisig::merge_claims(signatures);
            let duration = start.elapsed();
            let aggregate = aggregate.with_context(|| {
                format!("{WORKLOAD} (input size {size}, sample {sample}): Lean aggregation failed")
            })?;
            lean_multisig::verify_claims(&aggregate, lean_expected).with_context(|| {
                format!(
                    "{WORKLOAD} (input size {size}, sample {sample}): Lean aggregate failed post-timing verification"
                )
            })?;
            lean_durations.push(duration);
            retained_lean.get_or_insert(aggregate);
            Ok(())
        },
        |sample| {
            let aggregate = record_batched_sample(
                &mut lighthouse_durations,
                lighthouse_iterations,
                |iterations| {
                    time_last_value_batch(iterations, || {
                        aggregate_bls(black_box(fixtures.bls_signatures()))
                    })
                },
                |aggregate| {
                    ensure!(
                        aggregate.aggregate_verify(bls_messages, bls_public_keys),
                        "{WORKLOAD} (input size {size}, sample {sample}): Lighthouse aggregate failed post-timing verification"
                    );
                    Ok(())
                },
            )?;
            retained_lighthouse.get_or_insert(aggregate);
            Ok(())
        },
    )?;

    let timed_lean = retained_lean.with_context(|| {
        format!("{WORKLOAD} (input size {size}): retained no Lean aggregate proof")
    })?;
    let timed_lighthouse = retained_lighthouse.with_context(|| {
        format!("{WORKLOAD} (input size {size}): retained no Lighthouse aggregate signature")
    })?;
    let report = ComparisonReport::new(
        WORKLOAD,
        size,
        summarize(lean_durations, WORKLOAD, size, "Lean")?,
        summarize(lighthouse_durations, WORKLOAD, size, "Lighthouse")?,
        timed_lean.to_bytes().len(),
        timed_lighthouse.serialize().len(),
    )
    .with_context(|| format!("failed to assemble {WORKLOAD} report for input size {size}"))?;

    let verification_lean = warmup_lean.unwrap_or_else(|| timed_lean.clone());
    Ok((report, verification_lean, timed_lighthouse))
}

fn measure_distinct_claim_verify(
    lean_aggregate: &lean_multisig::MultiClaimProof,
    bls_aggregate: &lighthouse_bls::AggregateSignature,
    lean_expected: &[lean_multisig::ClaimSigners],
    bls_messages: &[lighthouse_bls::Hash256],
    bls_public_keys: &[&lighthouse_bls::PublicKey],
    samples: usize,
    size: usize,
) -> Result<ComparisonReport> {
    const WORKLOAD: &str = "distinct_claim_verify";
    let lighthouse_iterations =
        calibrate_batch_iterations(MIN_LIGHTHOUSE_BATCH_DURATION, |iterations| {
            let (elapsed, all_valid) = time_bool_batch(iterations, || {
                black_box(bls_aggregate)
                    .aggregate_verify(black_box(bls_messages), black_box(bls_public_keys))
            })?;
            ensure!(
                all_valid,
                "{WORKLOAD} (input size {size}): Lighthouse calibration verification failed"
            );
            Ok(elapsed)
        })
        .with_context(|| {
            format!("{WORKLOAD} (input size {size}): Lighthouse calibration failed")
        })?;

    let mut lean_durations = Vec::with_capacity(samples);
    let mut lighthouse_durations = Vec::with_capacity(samples);
    for_each_paired_sample(
        samples,
        |sample| {
            let start = Instant::now();
            let result = lean_multisig::verify_claims(lean_aggregate, lean_expected);
            let duration = start.elapsed();
            result.with_context(|| {
                format!("{WORKLOAD} (input size {size}, sample {sample}): Lean verification failed")
            })?;
            lean_durations.push(duration);
            Ok(())
        },
        |sample| {
            record_batched_sample(
                &mut lighthouse_durations,
                lighthouse_iterations,
                |iterations| {
                    time_bool_batch(iterations, || {
                        black_box(bls_aggregate)
                            .aggregate_verify(black_box(bls_messages), black_box(bls_public_keys))
                    })
                },
                |all_valid| {
                    ensure!(
                        *all_valid,
                        "{WORKLOAD} (input size {size}, sample {sample}): Lighthouse verification batch contained an invalid result"
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;

    ComparisonReport::new(
        WORKLOAD,
        size,
        summarize(lean_durations, WORKLOAD, size, "Lean")?,
        summarize(lighthouse_durations, WORKLOAD, size, "Lighthouse")?,
        lean_aggregate.to_bytes().len(),
        bls_aggregate.serialize().len(),
    )
    .with_context(|| format!("failed to assemble {WORKLOAD} report for input size {size}"))
}

fn measure_signature_sets_verify(
    fixtures: &FixtureSet,
    samples: usize,
    size: usize,
) -> Result<SupplementalReport> {
    const WORKLOAD: &str = "verify_signature_sets";
    let signature_sets = fixtures.bls_signature_sets();
    let lighthouse_iterations =
        calibrate_batch_iterations(MIN_LIGHTHOUSE_BATCH_DURATION, |iterations| {
            let (elapsed, all_valid) = time_bool_batch(iterations, || {
                lighthouse_bls::verify_signature_sets(black_box(signature_sets.iter()))
            })?;
            ensure!(
                all_valid,
                "{WORKLOAD} (input size {size}): Lighthouse calibration verification failed"
            );
            Ok(elapsed)
        })
        .with_context(|| {
            format!("{WORKLOAD} (input size {size}): Lighthouse calibration failed")
        })?;

    let mut durations = Vec::with_capacity(samples);
    for sample in 0..samples {
        record_batched_sample(
            &mut durations,
            lighthouse_iterations,
            |iterations| {
                time_bool_batch(iterations, || {
                    lighthouse_bls::verify_signature_sets(black_box(signature_sets.iter()))
                })
            },
            |all_valid| {
                ensure!(
                    *all_valid,
                    "{WORKLOAD} (input size {size}, sample {sample}): Lighthouse verification batch contained an invalid result"
                );
                Ok(())
            },
        )?;
    }

    Ok(SupplementalReport {
        workload: WORKLOAD.to_owned(),
        input_size: size,
        lighthouse: summarize(durations, WORKLOAD, size, "Lighthouse")?,
    })
}

fn aggregate_bls(signatures: &[lighthouse_bls::Signature]) -> lighthouse_bls::AggregateSignature {
    let mut aggregate = lighthouse_bls::AggregateSignature::infinity();
    for signature in signatures {
        aggregate.add_assign(signature);
    }
    aggregate
}

fn calibrate_batch_iterations(
    minimum_duration: Duration,
    mut measure_batch: impl FnMut(usize) -> Result<Duration>,
) -> Result<usize> {
    const MAX_STEPS: usize = 8;
    ensure!(
        !minimum_duration.is_zero(),
        "minimum batch duration must be greater than zero"
    );

    let mut iterations = 1usize;
    for _ in 0..MAX_STEPS {
        let elapsed = measure_batch(iterations)?;
        ensure!(
            !elapsed.is_zero(),
            "calibration batch duration must be greater than zero"
        );
        if elapsed >= minimum_duration {
            return Ok(iterations);
        }

        let numerator = minimum_duration
            .as_nanos()
            .checked_mul(iterations as u128)
            .context("calibration iteration estimate overflowed")?;
        let denominator = elapsed.as_nanos();
        let estimated = numerator
            .checked_add(denominator - 1)
            .context("calibration iteration estimate overflowed")?
            / denominator;
        let estimated =
            usize::try_from(estimated).context("calibration iteration estimate exceeds usize")?;
        iterations = estimated.max(
            iterations
                .checked_add(1)
                .context("calibration iteration count overflowed")?,
        );
    }

    bail!("failed to calibrate a batch in {MAX_STEPS} steps")
}

fn normalize_batch_duration(elapsed: Duration, iterations: usize) -> Result<Duration> {
    ensure!(iterations > 0, "batch iterations must be greater than zero");
    ensure!(
        !elapsed.is_zero(),
        "batch duration must be greater than zero"
    );
    let nanoseconds = elapsed.as_nanos() / iterations as u128;
    ensure!(
        nanoseconds > 0,
        "normalized per-operation duration rounded to zero"
    );
    Ok(Duration::from_nanos(
        u64::try_from(nanoseconds).context("normalized duration exceeds u64 nanoseconds")?,
    ))
}

fn record_batched_sample<T>(
    durations: &mut Vec<Duration>,
    iterations: usize,
    run_batch: impl FnOnce(usize) -> Result<(Duration, T)>,
    validate: impl FnOnce(&T) -> Result<()>,
) -> Result<T> {
    let (elapsed, value) = run_batch(iterations)?;
    validate(&value)?;
    let duration = normalize_batch_duration(elapsed, iterations)?;
    durations.push(duration);
    Ok(value)
}

fn time_bool_batch(
    iterations: usize,
    mut operation: impl FnMut() -> bool,
) -> Result<(Duration, bool)> {
    ensure!(iterations > 0, "batch iterations must be greater than zero");
    let start = Instant::now();
    let mut all_valid = true;
    for _ in 0..iterations {
        all_valid &= black_box(operation());
    }
    Ok((start.elapsed(), all_valid))
}

fn time_last_value_batch<T>(
    iterations: usize,
    mut operation: impl FnMut() -> T,
) -> Result<(Duration, T)> {
    ensure!(iterations > 0, "batch iterations must be greater than zero");
    let start = Instant::now();
    let mut retained = None;
    for _ in 0..iterations {
        retained = Some(black_box(operation()));
    }
    let elapsed = start.elapsed();
    Ok((
        elapsed,
        retained.context("batch retained no operation result")?,
    ))
}

fn for_each_paired_sample(
    samples: usize,
    mut lean: impl FnMut(usize) -> Result<()>,
    mut lighthouse: impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    ensure!(samples > 0, "at least one paired sample is required");
    for sample in 0..samples {
        if sample % 2 == 0 {
            lighthouse(sample)?;
            lean(sample)?;
        } else {
            lean(sample)?;
            lighthouse(sample)?;
        }
    }
    Ok(())
}

fn run_optional_warmup<T>(enabled: bool, warmup: impl FnOnce() -> Result<T>) -> Result<Option<T>> {
    if enabled {
        warmup().map(Some)
    } else {
        Ok(None)
    }
}

fn summarize(
    durations: Vec<std::time::Duration>,
    workload: &str,
    size: usize,
    implementation: &str,
) -> Result<SampleSummary> {
    SampleSummary::from_durations(durations).with_context(|| {
        format!("failed to summarize {implementation} {workload} samples for input size {size}")
    })
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc, time::Duration};

    use super::*;

    #[test]
    fn calibration_scales_batch_and_counts_invocations() {
        let observed = RefCell::new(Vec::new());

        let iterations = calibrate_batch_iterations(Duration::from_millis(10), |iterations| {
            observed.borrow_mut().push(iterations);
            Ok(Duration::from_millis(iterations as u64))
        })
        .unwrap();

        assert_eq!(iterations, 10);
        assert_eq!(*observed.borrow(), vec![1, 10]);
    }

    #[test]
    fn calibration_rejects_zero_target_and_zero_elapsed() {
        assert!(
            calibrate_batch_iterations(Duration::ZERO, |_| { Ok(Duration::from_nanos(1)) })
                .is_err()
        );
        assert!(
            calibrate_batch_iterations(Duration::from_millis(10), |_| { Ok(Duration::ZERO) })
                .is_err()
        );
    }

    #[test]
    fn batch_duration_is_normalized_per_operation() {
        assert_eq!(
            normalize_batch_duration(Duration::from_millis(12), 4).unwrap(),
            Duration::from_millis(3)
        );
        assert!(normalize_batch_duration(Duration::ZERO, 1).is_err());
        assert!(normalize_batch_duration(Duration::from_nanos(1), 0).is_err());
        assert!(normalize_batch_duration(Duration::from_nanos(1), 2).is_err());
    }

    #[test]
    fn batch_recording_counts_invocations_and_excludes_invalid_result() {
        let mut durations = Vec::new();
        let mut invocations = 0;
        let value = record_batched_sample(
            &mut durations,
            4,
            |iterations| {
                invocations += iterations;
                Ok((Duration::from_millis(12), true))
            },
            |valid| {
                ensure!(*valid, "invalid batch");
                Ok(())
            },
        )
        .unwrap();

        assert!(value);
        assert_eq!(invocations, 4);
        assert_eq!(durations, vec![Duration::from_millis(3)]);

        let error = record_batched_sample(
            &mut durations,
            4,
            |_| Ok((Duration::from_millis(12), false)),
            |valid| {
                ensure!(*valid, "invalid batch");
                Ok(())
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("invalid batch"));
        assert_eq!(durations, vec![Duration::from_millis(3)]);
    }

    #[test]
    fn timed_bool_batch_invokes_every_operation_and_tracks_any_failure() {
        let mut invocations = 0;
        let (_, all_valid) = time_bool_batch(5, || {
            invocations += 1;
            invocations != 3
        })
        .unwrap();

        assert_eq!(invocations, 5);
        assert!(!all_valid);
    }

    #[test]
    fn timed_last_value_batch_invokes_every_operation_and_retains_last() {
        let mut invocations = 0;
        let (_, retained) = time_last_value_batch(4, || {
            invocations += 1;
            invocations
        })
        .unwrap();

        assert_eq!(invocations, 4);
        assert_eq!(retained, 4);
    }

    #[test]
    fn paired_samples_alternate_bls_and_lean_order() {
        let order = Rc::new(RefCell::new(Vec::new()));
        let lean_order = Rc::clone(&order);
        let lighthouse_order = Rc::clone(&order);

        for_each_paired_sample(
            3,
            |sample| {
                lean_order.borrow_mut().push(format!("lean-{sample}"));
                Ok(())
            },
            |sample| {
                lighthouse_order
                    .borrow_mut()
                    .push(format!("lighthouse-{sample}"));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(
            *order.borrow(),
            vec![
                "lighthouse-0",
                "lean-0",
                "lean-1",
                "lighthouse-1",
                "lighthouse-2",
                "lean-2",
            ]
        );
    }

    #[test]
    fn optional_proof_warmup_runs_exactly_once_only_when_enabled() {
        let mut invocations = 0;
        let disabled = run_optional_warmup(false, || {
            invocations += 1;
            Ok(1)
        })
        .unwrap();
        assert!(disabled.is_none());
        assert_eq!(invocations, 0);

        let enabled = run_optional_warmup(true, || {
            invocations += 1;
            Ok(2)
        })
        .unwrap();
        assert_eq!(enabled, Some(2));
        assert_eq!(invocations, 1);
    }
}
