//! Benchmark support for comparing lean-multisig with Lighthouse BLS.
//!
//! This crate supports the repository's comparison benchmarks and is not a
//! supported public API.

use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::PathBuf,
    time::Duration,
};

use anyhow::{anyhow, ensure, Context, Result};
use serde::{Deserialize, Serialize};

pub const MAX_DISTINCT_CLAIMS: usize = lean_multisig::MAX_CLAIMS;
pub const LIGHTHOUSE_REVISION: &str = "e423a66763bb1bd780492d635123f208d80c3538";

/// Returns deterministic, nonzero 32-byte key material for a fixture signer.
pub fn deterministic_key_material(index: usize) -> Result<[u8; 32]> {
    indexed_bytes(index)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SampleSummary {
    pub samples_ns: Vec<u64>,
    pub median_ns: u64,
    pub operations_per_second: f64,
}

impl SampleSummary {
    pub fn from_durations(durations: impl IntoIterator<Item = Duration>) -> Result<Self> {
        let samples_ns = durations
            .into_iter()
            .map(|duration| {
                u64::try_from(duration.as_nanos())
                    .context("sample duration exceeds the supported nanosecond range")
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(!samples_ns.is_empty(), "at least one sample is required");
        ensure!(
            samples_ns.iter().all(|sample| *sample > 0),
            "sample durations must be greater than zero"
        );

        let mut sorted = samples_ns.clone();
        sorted.sort_unstable();
        let midpoint = sorted.len() / 2;
        let median_ns = if sorted.len() % 2 == 0 {
            let lower = sorted[midpoint - 1];
            let upper = sorted[midpoint];
            lower / 2 + upper / 2 + (lower % 2 + upper % 2) / 2
        } else {
            sorted[midpoint]
        };

        Ok(Self {
            samples_ns,
            median_ns,
            operations_per_second: 1_000_000_000.0 / median_ns as f64,
        })
    }

    pub fn ratio_to(&self, baseline: &Self) -> Result<f64> {
        ensure!(
            baseline.median_ns > 0,
            "baseline median must be greater than zero"
        );
        Ok(self.median_ns as f64 / baseline.median_ns as f64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonReport {
    pub workload: String,
    pub input_size: usize,
    pub lean: SampleSummary,
    pub lighthouse: SampleSummary,
    pub lean_over_lighthouse: f64,
    pub lean_artifact_bytes: usize,
    pub lighthouse_artifact_bytes: usize,
}

impl ComparisonReport {
    pub fn new(
        workload: impl Into<String>,
        input_size: usize,
        lean: SampleSummary,
        lighthouse: SampleSummary,
        lean_artifact_bytes: usize,
        lighthouse_artifact_bytes: usize,
    ) -> Result<Self> {
        let workload = workload.into();
        ensure!(!workload.is_empty(), "workload must not be empty");
        ensure!(input_size > 0, "input size must be greater than zero");
        ensure!(
            input_size <= MAX_DISTINCT_CLAIMS,
            "input size {input_size} exceeds maximum {MAX_DISTINCT_CLAIMS}"
        );
        ensure!(lean.median_ns > 0, "lean median must be greater than zero");
        ensure!(
            lean_artifact_bytes > 0,
            "lean artifact size must be greater than zero"
        );
        ensure!(
            lighthouse_artifact_bytes > 0,
            "Lighthouse artifact size must be greater than zero"
        );
        let lean_over_lighthouse = lean.ratio_to(&lighthouse)?;

        Ok(Self {
            workload,
            input_size,
            lean,
            lighthouse,
            lean_over_lighthouse,
            lean_artifact_bytes,
            lighthouse_artifact_bytes,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SupplementalReport {
    pub workload: String,
    pub input_size: usize,
    pub lighthouse: SampleSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkReport {
    pub lighthouse_revision: String,
    pub samples: usize,
    pub proof_warmup: bool,
    pub comparisons: Vec<ComparisonReport>,
    pub supplemental: Vec<SupplementalReport>,
}

impl BenchmarkReport {
    #[must_use]
    pub fn to_table(&self) -> String {
        let mut rows = vec![
            "Workload | Size | Lean median | Lighthouse median | Lean/BLS | Lean ops/s | Lighthouse ops/s | Lean bytes | Lighthouse bytes".to_owned(),
            "--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---:".to_owned(),
        ];

        for comparison in &self.comparisons {
            rows.push(format!(
                "{} | {} | {} | {} | {:.2}x | {:.2} ops/s | {:.2} ops/s | {} | {}",
                comparison.workload,
                comparison.input_size,
                format_duration_ns(comparison.lean.median_ns),
                format_duration_ns(comparison.lighthouse.median_ns),
                comparison.lean_over_lighthouse,
                comparison.lean.operations_per_second,
                comparison.lighthouse.operations_per_second,
                comparison.lean_artifact_bytes,
                comparison.lighthouse_artifact_bytes,
            ));
        }

        for supplemental in &self.supplemental {
            rows.push(format!(
                "{} (Lighthouse-only) | {} | - | {} | - | - | {:.2} ops/s | - | -",
                supplemental.workload,
                supplemental.input_size,
                format_duration_ns(supplemental.lighthouse.median_ns),
                supplemental.lighthouse.operations_per_second,
            ));
        }

        rows.join("\n")
    }
}

fn format_duration_ns(nanoseconds: u64) -> String {
    match nanoseconds {
        0..=999 => format!("{nanoseconds} ns"),
        1_000..=999_999 => format!("{:.3} us", nanoseconds as f64 / 1_000.0),
        1_000_000..=999_999_999 => {
            format!("{:.3} ms", nanoseconds as f64 / 1_000_000.0)
        }
        _ => format!("{:.3} s", nanoseconds as f64 / 1_000_000_000.0),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    pub samples: usize,
    pub sizes: Vec<usize>,
    pub json_path: Option<PathBuf>,
    pub warmup_proofs: bool,
}

impl RunConfig {
    pub fn parse_from<I, T>(arguments: I) -> Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        let mut arguments = arguments.into_iter().map(Into::into);
        arguments.next().context("missing program name")?;

        let mut samples = None;
        let mut sizes = None;
        let mut json_path = None;
        let mut warmup_proofs = false;

        while let Some(argument) = arguments.next() {
            match argument.as_os_str() {
                argument if argument == OsStr::new("--samples") => {
                    ensure!(samples.is_none(), "--samples may only be supplied once");
                    let value = option_value("--samples", arguments.next())?;
                    let text = value
                        .to_str()
                        .context("--samples value must be valid UTF-8")?;
                    let parsed = text
                        .parse::<usize>()
                        .with_context(|| format!("invalid --samples value `{text}`"))?;
                    ensure!(parsed > 0, "--samples must be greater than zero");
                    samples = Some(parsed);
                }
                argument if argument == OsStr::new("--sizes") => {
                    ensure!(sizes.is_none(), "--sizes may only be supplied once");
                    let value = option_value("--sizes", arguments.next())?;
                    let text = value
                        .to_str()
                        .context("--sizes value must be valid UTF-8")?;
                    sizes = Some(parse_sizes(text)?);
                }
                argument if argument == OsStr::new("--json") => {
                    ensure!(json_path.is_none(), "--json may only be supplied once");
                    let value = option_value("--json", arguments.next())?;
                    ensure!(!value.is_empty(), "--json path must not be empty");
                    json_path = Some(PathBuf::from(value));
                }
                argument if argument == OsStr::new("--warmup-proofs") => {
                    ensure!(!warmup_proofs, "--warmup-proofs may only be supplied once");
                    warmup_proofs = true;
                }
                _ => {
                    return Err(anyhow!(
                        "unknown argument `{}`; expected --samples, --sizes, --json, or --warmup-proofs",
                        argument.to_string_lossy()
                    ));
                }
            }
        }

        Ok(Self {
            samples: samples.unwrap_or(3),
            sizes: sizes.unwrap_or_else(|| vec![1, 8, 16]),
            json_path,
            warmup_proofs,
        })
    }
}

fn option_value(option: &str, value: Option<OsString>) -> Result<OsString> {
    let value = value.with_context(|| format!("{option} requires a value"))?;
    ensure!(
        !value.to_string_lossy().starts_with("--"),
        "{option} requires a value"
    );
    Ok(value)
}

fn parse_sizes(value: &str) -> Result<Vec<usize>> {
    ensure!(!value.is_empty(), "--sizes requires a comma-separated list");

    let mut sizes = Vec::new();
    for component in value.split(',') {
        ensure!(
            !component.is_empty(),
            "--sizes contains an empty list entry"
        );
        let size = component
            .parse::<usize>()
            .with_context(|| format!("invalid --sizes entry `{component}`"))?;
        ensure!(size > 0, "--sizes entries must be greater than zero");
        ensure!(
            size <= MAX_DISTINCT_CLAIMS,
            "--sizes entry {size} exceeds maximum {MAX_DISTINCT_CLAIMS}"
        );
        ensure!(
            !sizes.contains(&size),
            "--sizes contains duplicate entry {size}"
        );
        sizes.push(size);
    }

    Ok(sizes)
}

pub struct FixtureSet {
    lean_claims: Vec<lean_multisig::Claim>,
    lean_keys: Vec<lean_multisig::SecretKey>,
    lean_signatures: Vec<lean_multisig::Signature>,
    lean_public_keys: Vec<lean_multisig::PublicKey>,
    bls_messages: Vec<lighthouse_bls::Hash256>,
    bls_keys: Vec<lighthouse_bls::SecretKey>,
    bls_signatures: Vec<lighthouse_bls::Signature>,
    bls_public_keys: Vec<lighthouse_bls::PublicKey>,
}

impl FixtureSet {
    pub fn same_claim(count: usize) -> Result<Self> {
        Self::build(count, false)
    }

    pub fn distinct_claims(count: usize) -> Result<Self> {
        if count > MAX_DISTINCT_CLAIMS {
            return Err(anyhow!(
                "distinct-claim fixture count {count} exceeds maximum {MAX_DISTINCT_CLAIMS}"
            ));
        }

        Self::build(count, true)
    }

    fn build(count: usize, distinct_claims: bool) -> Result<Self> {
        ensure!(count > 0, "fixture count must be greater than zero");

        let mut lean_claims = Vec::with_capacity(count);
        let mut lean_keys = Vec::with_capacity(count);
        let mut lean_signatures = Vec::with_capacity(count);
        let mut lean_public_keys = Vec::with_capacity(count);
        let mut bls_messages = Vec::with_capacity(count);
        let mut bls_keys = Vec::with_capacity(count);
        let mut bls_signatures = Vec::with_capacity(count);
        let mut bls_public_keys = Vec::with_capacity(count);

        for index in 0..count {
            let message_index = if distinct_claims { index } else { 0 };
            let message = indexed_bytes(message_index)
                .with_context(|| format!("failed to create message for signer {index}"))?;
            let slot = if distinct_claims {
                u32::try_from(index)
                    .with_context(|| format!("slot does not fit in u32 for signer {index}"))?
            } else {
                0
            };
            let claim = lean_multisig::Claim::new(message, slot);

            let lean_seed = indexed_bytes(index)
                .with_context(|| format!("failed to create XMSS seed for signer {index}"))?;
            let lean_key =
                lean_multisig::SecretKey::from_seed(lean_seed, 0..=MAX_DISTINCT_CLAIMS as u32)
                    .with_context(|| format!("failed to create XMSS key for signer {index}"))?;
            let lean_public_key = lean_key.public_key();
            let lean_signature = lean_key
                .sign(&claim)
                .with_context(|| format!("failed to create XMSS signature for signer {index}"))?;

            let bls_secret = indexed_bytes(index)
                .with_context(|| format!("failed to create BLS secret for signer {index}"))?;
            let bls_key = lighthouse_bls::SecretKey::deserialize(&bls_secret).map_err(|error| {
                anyhow!("failed to deserialize BLS key for signer {index}: {error:?}")
            })?;
            let bls_public_key = bls_key.public_key();
            let bls_message = lighthouse_bls::Hash256::from(message);
            let bls_signature = bls_key.sign(bls_message);

            lean_claims.push(claim);
            lean_keys.push(lean_key);
            lean_signatures.push(lean_signature);
            lean_public_keys.push(lean_public_key);
            bls_messages.push(bls_message);
            bls_keys.push(bls_key);
            bls_signatures.push(bls_signature);
            bls_public_keys.push(bls_public_key);
        }

        let fixtures = Self {
            lean_claims,
            lean_keys,
            lean_signatures,
            lean_public_keys,
            bls_messages,
            bls_keys,
            bls_signatures,
            bls_public_keys,
        };
        fixtures.validate_raw()?;
        Ok(fixtures)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lean_keys.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lean_keys.is_empty()
    }

    #[must_use]
    pub fn lean_claims(&self) -> &[lean_multisig::Claim] {
        &self.lean_claims
    }

    #[must_use]
    pub fn lean_keys(&self) -> &[lean_multisig::SecretKey] {
        &self.lean_keys
    }

    #[must_use]
    pub fn lean_signatures(&self) -> &[lean_multisig::Signature] {
        &self.lean_signatures
    }

    #[must_use]
    pub fn lean_public_keys(&self) -> &[lean_multisig::PublicKey] {
        &self.lean_public_keys
    }

    #[must_use]
    pub fn bls_messages(&self) -> &[lighthouse_bls::Hash256] {
        &self.bls_messages
    }

    #[must_use]
    pub fn bls_keys(&self) -> &[lighthouse_bls::SecretKey] {
        &self.bls_keys
    }

    #[must_use]
    pub fn bls_signatures(&self) -> &[lighthouse_bls::Signature] {
        &self.bls_signatures
    }

    #[must_use]
    pub fn bls_public_keys(&self) -> &[lighthouse_bls::PublicKey] {
        &self.bls_public_keys
    }

    #[must_use]
    pub fn bls_aggregate(&self) -> lighthouse_bls::AggregateSignature {
        let mut aggregate = lighthouse_bls::AggregateSignature::infinity();
        for signature in &self.bls_signatures {
            aggregate.add_assign(signature);
        }
        aggregate
    }

    #[must_use]
    pub fn verify_bls_same_claim_aggregate(
        &self,
        aggregate: &lighthouse_bls::AggregateSignature,
    ) -> bool {
        let public_keys = self.bls_public_keys.iter().collect::<Vec<_>>();
        aggregate.fast_aggregate_verify(self.bls_messages[0], &public_keys)
    }

    #[must_use]
    pub fn verify_bls_distinct_claim_aggregate(
        &self,
        aggregate: &lighthouse_bls::AggregateSignature,
    ) -> bool {
        let public_keys = self.bls_public_keys.iter().collect::<Vec<_>>();
        aggregate.aggregate_verify(&self.bls_messages, &public_keys)
    }

    #[must_use]
    pub fn bls_signature_sets(&self) -> Vec<lighthouse_bls::SignatureSet<'_>> {
        self.bls_signatures
            .iter()
            .zip(&self.bls_public_keys)
            .zip(&self.bls_messages)
            .map(|((signature, public_key), message)| {
                lighthouse_bls::SignatureSet::single_pubkey(
                    signature,
                    Cow::Borrowed(public_key),
                    *message,
                )
            })
            .collect()
    }

    pub fn validate_raw(&self) -> Result<()> {
        let expected_len = self.len();
        ensure!(
            [
                self.lean_claims.len(),
                self.lean_signatures.len(),
                self.lean_public_keys.len(),
                self.bls_messages.len(),
                self.bls_keys.len(),
                self.bls_signatures.len(),
                self.bls_public_keys.len(),
            ]
            .into_iter()
            .all(|len| len == expected_len),
            "fixture vectors have inconsistent lengths"
        );

        for index in 0..expected_len {
            lean_multisig::verify(
                &self.lean_signatures[index],
                std::slice::from_ref(&self.lean_public_keys[index]),
                &self.lean_claims[index],
            )
            .with_context(|| format!("XMSS signature failed validation for signer {index}"))?;

            ensure!(
                self.bls_signatures[index]
                    .verify(&self.bls_public_keys[index], self.bls_messages[index]),
                "BLS signature failed validation for signer {index}"
            );
        }

        Ok(())
    }
}

fn indexed_bytes(index: usize) -> Result<[u8; 32]> {
    let value = u64::try_from(index)
        .ok()
        .and_then(|index| index.checked_add(1))
        .with_context(|| format!("signer index {index} cannot be encoded as index + 1"))?;
    let mut bytes = [0; 32];
    bytes[24..].copy_from_slice(&value.to_be_bytes());
    Ok(bytes)
}
