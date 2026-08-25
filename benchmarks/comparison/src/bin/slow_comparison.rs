use std::{env, fs, time::Instant};

use anyhow::{ensure, Context, Result};
use lean_multisig_comparison::{
    BenchmarkReport, ComparisonReport, FixtureSet, RunConfig, SampleSummary, SupplementalReport,
};

const LIGHTHOUSE_REVISION: &str = "e423a66763bb1bd780492d635123f208d80c3538";
const SEMANTIC_WARNING: &str = "WARNING: leanMultisig aggregation generates zkVM proofs, while Lighthouse BLS aggregation combines elliptic-curve points; these operations do not have identical security semantics.";

fn main() -> Result<()> {
    let config =
        RunConfig::parse_from(env::args_os()).context("failed to parse runner arguments")?;
    run(config)
}

fn run(config: RunConfig) -> Result<()> {
    lean_multisig::setup();

    let mut comparisons = Vec::with_capacity(config.sizes.len() * 4);
    let mut supplemental = Vec::with_capacity(config.sizes.len());

    for &size in &config.sizes {
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
        comparisons,
        supplemental,
    };

    println!("{SEMANTIC_WARNING}");
    println!("Pinned Lighthouse revision: {LIGHTHOUSE_REVISION}");
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
) -> Result<(
    ComparisonReport,
    lean_multisig::Signature,
    lighthouse_bls::AggregateSignature,
)> {
    const WORKLOAD: &str = "same_claim_aggregate";
    let claim = fixtures.lean_claims()[0];
    let bls_message = fixtures.bls_messages()[0];

    let bls_warmup = aggregate_bls(fixtures.bls_signatures());
    ensure!(
        bls_warmup.fast_aggregate_verify(bls_message, bls_public_keys),
        "{WORKLOAD} (input size {size}): Lighthouse warm-up aggregate failed verification"
    );

    let lean_inputs = prepare_lean_inputs(fixtures.lean_signatures(), samples);
    let mut lean_durations = Vec::with_capacity(samples);
    let mut retained_lean = None;
    for (sample, signatures) in lean_inputs.into_iter().enumerate() {
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
    }

    let mut lighthouse_durations = Vec::with_capacity(samples);
    let mut retained_lighthouse = None;
    for sample in 0..samples {
        let start = Instant::now();
        let aggregate = aggregate_bls(fixtures.bls_signatures());
        let duration = start.elapsed();
        ensure!(
            aggregate.fast_aggregate_verify(bls_message, bls_public_keys),
            "{WORKLOAD} (input size {size}, sample {sample}): Lighthouse aggregate failed post-timing verification"
        );
        lighthouse_durations.push(duration);
        retained_lighthouse.get_or_insert(aggregate);
    }

    let retained_lean = retained_lean.with_context(|| {
        format!("{WORKLOAD} (input size {size}): retained no Lean aggregate proof")
    })?;
    let retained_lighthouse = retained_lighthouse.with_context(|| {
        format!("{WORKLOAD} (input size {size}): retained no Lighthouse aggregate signature")
    })?;
    let lean_artifact_bytes = retained_lean.to_bytes().len();
    let lighthouse_artifact_bytes = retained_lighthouse.serialize().len();
    let report = ComparisonReport::new(
        WORKLOAD,
        size,
        summarize(lean_durations, WORKLOAD, size, "Lean")?,
        summarize(lighthouse_durations, WORKLOAD, size, "Lighthouse")?,
        lean_artifact_bytes,
        lighthouse_artifact_bytes,
    )
    .with_context(|| format!("failed to assemble {WORKLOAD} report for input size {size}"))?;

    Ok((report, retained_lean, retained_lighthouse))
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
    ensure!(
        bls_aggregate.fast_aggregate_verify(bls_message, bls_public_keys),
        "{WORKLOAD} (input size {size}): Lighthouse warm-up verification failed"
    );

    let mut lean_durations = Vec::with_capacity(samples);
    for sample in 0..samples {
        let start = Instant::now();
        let result = lean_multisig::verify(lean_aggregate, lean_signers, claim);
        let duration = start.elapsed();
        result.with_context(|| {
            format!("{WORKLOAD} (input size {size}, sample {sample}): Lean verification failed")
        })?;
        lean_durations.push(duration);
    }

    let mut lighthouse_durations = Vec::with_capacity(samples);
    for sample in 0..samples {
        let start = Instant::now();
        let valid = bls_aggregate.fast_aggregate_verify(bls_message, bls_public_keys);
        let duration = start.elapsed();
        ensure!(
            valid,
            "{WORKLOAD} (input size {size}, sample {sample}): Lighthouse verification failed"
        );
        lighthouse_durations.push(duration);
    }

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
) -> Result<(
    ComparisonReport,
    lean_multisig::MultiClaimProof,
    lighthouse_bls::AggregateSignature,
)> {
    const WORKLOAD: &str = "distinct_claim_aggregate";

    let bls_warmup = aggregate_bls(fixtures.bls_signatures());
    ensure!(
        bls_warmup.aggregate_verify(bls_messages, bls_public_keys),
        "{WORKLOAD} (input size {size}): Lighthouse warm-up aggregate failed verification"
    );

    let lean_inputs = prepare_lean_inputs(fixtures.lean_signatures(), samples);
    let mut lean_durations = Vec::with_capacity(samples);
    let mut retained_lean = None;
    for (sample, signatures) in lean_inputs.into_iter().enumerate() {
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
    }

    let mut lighthouse_durations = Vec::with_capacity(samples);
    let mut retained_lighthouse = None;
    for sample in 0..samples {
        let start = Instant::now();
        let aggregate = aggregate_bls(fixtures.bls_signatures());
        let duration = start.elapsed();
        ensure!(
            aggregate.aggregate_verify(bls_messages, bls_public_keys),
            "{WORKLOAD} (input size {size}, sample {sample}): Lighthouse aggregate failed post-timing verification"
        );
        lighthouse_durations.push(duration);
        retained_lighthouse.get_or_insert(aggregate);
    }

    let retained_lean = retained_lean.with_context(|| {
        format!("{WORKLOAD} (input size {size}): retained no Lean aggregate proof")
    })?;
    let retained_lighthouse = retained_lighthouse.with_context(|| {
        format!("{WORKLOAD} (input size {size}): retained no Lighthouse aggregate signature")
    })?;
    let report = ComparisonReport::new(
        WORKLOAD,
        size,
        summarize(lean_durations, WORKLOAD, size, "Lean")?,
        summarize(lighthouse_durations, WORKLOAD, size, "Lighthouse")?,
        retained_lean.to_bytes().len(),
        retained_lighthouse.serialize().len(),
    )
    .with_context(|| format!("failed to assemble {WORKLOAD} report for input size {size}"))?;

    Ok((report, retained_lean, retained_lighthouse))
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
    ensure!(
        bls_aggregate.aggregate_verify(bls_messages, bls_public_keys),
        "{WORKLOAD} (input size {size}): Lighthouse warm-up verification failed"
    );

    let mut lean_durations = Vec::with_capacity(samples);
    for sample in 0..samples {
        let start = Instant::now();
        let result = lean_multisig::verify_claims(lean_aggregate, lean_expected);
        let duration = start.elapsed();
        result.with_context(|| {
            format!("{WORKLOAD} (input size {size}, sample {sample}): Lean verification failed")
        })?;
        lean_durations.push(duration);
    }

    let mut lighthouse_durations = Vec::with_capacity(samples);
    for sample in 0..samples {
        let start = Instant::now();
        let valid = bls_aggregate.aggregate_verify(bls_messages, bls_public_keys);
        let duration = start.elapsed();
        ensure!(
            valid,
            "{WORKLOAD} (input size {size}, sample {sample}): Lighthouse verification failed"
        );
        lighthouse_durations.push(duration);
    }

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
    ensure!(
        lighthouse_bls::verify_signature_sets(signature_sets.iter()),
        "{WORKLOAD} (input size {size}): Lighthouse warm-up verification failed"
    );

    let mut durations = Vec::with_capacity(samples);
    for sample in 0..samples {
        let start = Instant::now();
        let valid = lighthouse_bls::verify_signature_sets(signature_sets.iter());
        let duration = start.elapsed();
        ensure!(
            valid,
            "{WORKLOAD} (input size {size}, sample {sample}): Lighthouse verification failed"
        );
        durations.push(duration);
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

fn prepare_lean_inputs(
    signatures: &[lean_multisig::Signature],
    samples: usize,
) -> Vec<Vec<lean_multisig::Signature>> {
    std::iter::repeat_with(|| signatures.to_vec())
        .take(samples)
        .collect()
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
    use super::*;

    #[test]
    fn prepares_one_owned_lean_input_per_sample() {
        let fixtures = FixtureSet::same_claim(2).unwrap();

        let prepared = prepare_lean_inputs(fixtures.lean_signatures(), 3);

        assert_eq!(prepared.len(), 3);
        assert!(prepared.iter().all(|input| input.len() == 2));
    }
}
