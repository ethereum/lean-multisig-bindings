//! Benchmark support for comparing lean-multisig with Lighthouse BLS.
//!
//! This crate supports the repository's comparison benchmarks and is not a
//! supported public API.

use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, ensure, Context, Result};
use serde::{Deserialize, Serialize};

pub const MAX_DISTINCT_CLAIMS: usize = lean_multisig::MAX_CLAIMS;
pub const MAX_SAME_CLAIM_SIGNERS: usize = 512;
pub const MIXED_CLAIM_SIGNATURES: usize = 512;
pub const MIXED_CLAIM_COUNTS: [usize; 2] = [8, 16];
pub const RECURSIVE_CHILD_SIGNERS: usize = 512;
pub const MAX_RECURSIVE_FAN_IN: usize = MAX_DISTINCT_CLAIMS;
pub const RECURSIVE_AGGREGATION_WORKLOAD: &str = "recursive_same_claim_aggregate";
pub const KEY_CREATION_SLOTS: usize = 1 << 20;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_count: Option<usize>,
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
        if workload == "key_creation" {
            ensure!(
                input_size == KEY_CREATION_SLOTS,
                "key_creation must measure exactly {KEY_CREATION_SLOTS} slots"
            );
        } else {
            ensure!(
                input_size <= MAX_SAME_CLAIM_SIGNERS,
                "input size {input_size} exceeds maximum {MAX_SAME_CLAIM_SIGNERS}"
            );
        }
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
            claim_count: None,
            lean,
            lighthouse,
            lean_over_lighthouse,
            lean_artifact_bytes,
            lighthouse_artifact_bytes,
        })
    }

    pub fn with_claim_count(mut self, claim_count: usize) -> Result<Self> {
        ensure!(
            self.workload.starts_with("mixed_claim_"),
            "claim count is only valid for mixed-claim workloads"
        );
        ensure!(claim_count > 1, "claim count must be greater than one");
        ensure!(
            claim_count <= MAX_DISTINCT_CLAIMS,
            "claim count {claim_count} exceeds maximum {MAX_DISTINCT_CLAIMS}"
        );
        ensure!(
            claim_count <= self.input_size,
            "claim count must not exceed the signature count"
        );
        ensure!(
            self.input_size.is_multiple_of(claim_count),
            "signature count must be divisible by claim count"
        );
        self.claim_count = Some(claim_count);
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SupplementalReport {
    pub workload: String,
    pub input_size: usize,
    pub lighthouse: SampleSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecursiveAggregationReport {
    pub workload: String,
    pub fan_in: usize,
    pub signers_per_child: usize,
    pub total_signers: usize,
    pub lean: SampleSummary,
    pub lean_artifact_bytes: usize,
}

impl RecursiveAggregationReport {
    pub fn new(
        fan_in: usize,
        signers_per_child: usize,
        lean: SampleSummary,
        lean_artifact_bytes: usize,
    ) -> Result<Self> {
        ensure!(fan_in >= 2, "recursive fan-in must be at least two");
        ensure!(
            fan_in <= MAX_RECURSIVE_FAN_IN,
            "recursive fan-in {fan_in} exceeds maximum {MAX_RECURSIVE_FAN_IN}"
        );
        ensure!(
            signers_per_child == RECURSIVE_CHILD_SIGNERS,
            "recursive child signer count must be exactly {RECURSIVE_CHILD_SIGNERS}"
        );
        let total_signers = fan_in
            .checked_mul(signers_per_child)
            .context("recursive total signer count overflowed")?;
        ensure!(
            lean.median_ns > 0,
            "LeanVM median must be greater than zero"
        );
        ensure!(
            lean_artifact_bytes > 0,
            "LeanVM artifact size must be greater than zero"
        );
        Ok(Self {
            workload: RECURSIVE_AGGREGATION_WORKLOAD.to_owned(),
            fan_in,
            signers_per_child,
            total_signers,
            lean,
            lean_artifact_bytes,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkReport {
    pub lighthouse_revision: String,
    pub samples: usize,
    pub proof_warmup: bool,
    pub comparisons: Vec<ComparisonReport>,
    #[serde(default)]
    pub recursive_aggregations: Vec<RecursiveAggregationReport>,
    pub supplemental: Vec<SupplementalReport>,
}

/// One entry in github-action-benchmark's `customSmallerIsBetter` schema.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ActionBenchmark {
    pub name: String,
    pub unit: String,
    pub value: f64,
    pub extra: String,
}

impl BenchmarkReport {
    #[must_use]
    pub fn to_table(&self) -> String {
        let mut rows = vec![
            "Workload | Size | Lean median | Lighthouse median | Lean/BLS | Lean ops/s | Lighthouse ops/s | Lean bytes | Lighthouse bytes".to_owned(),
            "--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---:".to_owned(),
        ];

        for comparison in &self.comparisons {
            let input = comparison.claim_count.map_or_else(
                || comparison.input_size.to_string(),
                |claim_count| {
                    format!(
                        "{} signatures / {claim_count} claims",
                        comparison.input_size
                    )
                },
            );
            rows.push(format!(
                "{} | {} | {} | {} | {:.2}x | {:.2} ops/s | {:.2} ops/s | {} | {}",
                comparison.workload,
                input,
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

        for recursive in &self.recursive_aggregations {
            rows.push(format!(
                "{} (LeanVM-only) | {} proofs x {} signers ({} total) | {} | - | - | {:.2} ops/s | - | {} | -",
                recursive.workload,
                recursive.fan_in,
                recursive.signers_per_child,
                recursive.total_signers,
                format_duration_ns(recursive.lean.median_ns),
                recursive.lean.operations_per_second,
                recursive.lean_artifact_bytes,
            ));
        }

        rows.join("\n")
    }

    pub fn to_action_benchmarks(&self) -> Result<Vec<ActionBenchmark>> {
        ensure!(
            !self.comparisons.is_empty()
                || !self.recursive_aggregations.is_empty()
                || !self.supplemental.is_empty(),
            "cannot export an empty benchmark report"
        );

        let warmup = if self.proof_warmup {
            "enabled"
        } else {
            "disabled"
        };
        let extra = format!(
            "samples={}; proof_warmup={warmup}; lighthouse_revision={}",
            self.samples, self.lighthouse_revision
        );
        let mut entries = Vec::with_capacity(
            self.comparisons.len() * 5
                + self.recursive_aggregations.len() * 2
                + self.supplemental.len(),
        );

        for comparison in &self.comparisons {
            let prefix = comparison.claim_count.map_or_else(
                || format!("{}/size-{}", comparison.workload, comparison.input_size),
                |claim_count| {
                    format!(
                        "{}/signatures-{}/claims-{claim_count}",
                        comparison.workload, comparison.input_size
                    )
                },
            );
            entries.extend([
                action_entry(
                    format!("{prefix}/lean/median"),
                    "ns",
                    comparison.lean.median_ns as f64,
                    &extra,
                ),
                action_entry(
                    format!("{prefix}/lighthouse/median"),
                    "ns",
                    comparison.lighthouse.median_ns as f64,
                    &extra,
                ),
                action_entry(
                    format!("{prefix}/lean-over-lighthouse"),
                    "ratio",
                    comparison.lean_over_lighthouse,
                    &extra,
                ),
                action_entry(
                    format!("{prefix}/lean/artifact"),
                    "bytes",
                    comparison.lean_artifact_bytes as f64,
                    &extra,
                ),
                action_entry(
                    format!("{prefix}/lighthouse/artifact"),
                    "bytes",
                    comparison.lighthouse_artifact_bytes as f64,
                    &extra,
                ),
            ]);
        }

        for recursive in &self.recursive_aggregations {
            let prefix = format!(
                "{}/fan-in-{}/child-signers-{}",
                recursive.workload, recursive.fan_in, recursive.signers_per_child
            );
            entries.extend([
                action_entry(
                    format!("{prefix}/lean/median"),
                    "ns",
                    recursive.lean.median_ns as f64,
                    &extra,
                ),
                action_entry(
                    format!("{prefix}/lean/artifact"),
                    "bytes",
                    recursive.lean_artifact_bytes as f64,
                    &extra,
                ),
            ]);
        }

        for supplemental in &self.supplemental {
            entries.push(action_entry(
                format!(
                    "{}/size-{}/lighthouse/median",
                    supplemental.workload, supplemental.input_size
                ),
                "ns",
                supplemental.lighthouse.median_ns as f64,
                &extra,
            ));
        }

        Ok(entries)
    }
}

fn action_entry(name: String, unit: &str, value: f64, extra: &str) -> ActionBenchmark {
    ActionBenchmark {
        name,
        unit: unit.to_owned(),
        value,
        extra: extra.to_owned(),
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
    pub same_sizes: Vec<usize>,
    pub distinct_sizes: Vec<usize>,
    pub mixed_claim_counts: Vec<usize>,
    pub recursive_fan_ins: Vec<usize>,
    pub json_path: Option<PathBuf>,
    pub action_json_path: Option<PathBuf>,
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
        let mut same_sizes = None;
        let mut distinct_sizes = None;
        let mut mixed_claim_counts = None;
        let mut recursive_fan_ins = None;
        let mut json_path = None;
        let mut action_json_path = None;
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
                    sizes = Some(parse_sizes("--sizes", text, MAX_DISTINCT_CLAIMS)?);
                }
                argument if argument == OsStr::new("--same-sizes") => {
                    ensure!(
                        same_sizes.is_none(),
                        "--same-sizes may only be supplied once"
                    );
                    let value = option_value("--same-sizes", arguments.next())?;
                    let text = value
                        .to_str()
                        .context("--same-sizes value must be valid UTF-8")?;
                    same_sizes = Some(parse_sizes("--same-sizes", text, MAX_SAME_CLAIM_SIGNERS)?);
                }
                argument if argument == OsStr::new("--distinct-sizes") => {
                    ensure!(
                        distinct_sizes.is_none(),
                        "--distinct-sizes may only be supplied once"
                    );
                    let value = option_value("--distinct-sizes", arguments.next())?;
                    let text = value
                        .to_str()
                        .context("--distinct-sizes value must be valid UTF-8")?;
                    distinct_sizes =
                        Some(parse_sizes("--distinct-sizes", text, MAX_DISTINCT_CLAIMS)?);
                }
                argument if argument == OsStr::new("--mixed-claim-counts") => {
                    ensure!(
                        mixed_claim_counts.is_none(),
                        "--mixed-claim-counts may only be supplied once"
                    );
                    let value = option_value("--mixed-claim-counts", arguments.next())?;
                    let text = value
                        .to_str()
                        .context("--mixed-claim-counts value must be valid UTF-8")?;
                    let parsed = parse_sizes("--mixed-claim-counts", text, MAX_DISTINCT_CLAIMS)?;
                    ensure!(
                        parsed.iter().all(|count| *count > 1),
                        "--mixed-claim-counts entries must be greater than one"
                    );
                    ensure!(
                        parsed
                            .iter()
                            .all(|count| MIXED_CLAIM_SIGNATURES.is_multiple_of(*count)),
                        "--mixed-claim-counts entries must divide {MIXED_CLAIM_SIGNATURES} signatures evenly"
                    );
                    mixed_claim_counts = Some(parsed);
                }
                argument if argument == OsStr::new("--recursive-fan-ins") => {
                    ensure!(
                        recursive_fan_ins.is_none(),
                        "--recursive-fan-ins may only be supplied once"
                    );
                    let value = option_value("--recursive-fan-ins", arguments.next())?;
                    let text = value
                        .to_str()
                        .context("--recursive-fan-ins value must be valid UTF-8")?;
                    let parsed = parse_sizes("--recursive-fan-ins", text, MAX_RECURSIVE_FAN_IN)?;
                    ensure!(
                        parsed.iter().all(|fan_in| *fan_in >= 2),
                        "--recursive-fan-ins entries must be at least two"
                    );
                    recursive_fan_ins = Some(parsed);
                }
                argument if argument == OsStr::new("--json") => {
                    ensure!(json_path.is_none(), "--json may only be supplied once");
                    let value = option_value("--json", arguments.next())?;
                    ensure!(!value.is_empty(), "--json path must not be empty");
                    json_path = Some(PathBuf::from(value));
                }
                argument if argument == OsStr::new("--action-json") => {
                    ensure!(
                        action_json_path.is_none(),
                        "--action-json may only be supplied once"
                    );
                    let value = option_value("--action-json", arguments.next())?;
                    ensure!(!value.is_empty(), "--action-json path must not be empty");
                    action_json_path = Some(PathBuf::from(value));
                }
                argument if argument == OsStr::new("--warmup-proofs") => {
                    ensure!(!warmup_proofs, "--warmup-proofs may only be supplied once");
                    warmup_proofs = true;
                }
                _ => {
                    return Err(anyhow!(
                        "unknown argument `{}`; expected --samples, --sizes, --same-sizes, --distinct-sizes, --mixed-claim-counts, --recursive-fan-ins, --json, --action-json, or --warmup-proofs",
                        argument.to_string_lossy()
                    ));
                }
            }
        }

        if sizes.is_some() {
            ensure!(
                same_sizes.is_none(),
                "--sizes cannot be combined with --same-sizes"
            );
            ensure!(
                distinct_sizes.is_none(),
                "--sizes cannot be combined with --distinct-sizes"
            );
        }

        ensure_distinct_output_paths(json_path.as_deref(), action_json_path.as_deref())?;

        let default_sizes = || vec![1, 8, 16];
        let (same_sizes, distinct_sizes) = if let Some(sizes) = sizes {
            (sizes.clone(), sizes)
        } else {
            (
                same_sizes.unwrap_or_else(default_sizes),
                distinct_sizes.unwrap_or_else(default_sizes),
            )
        };
        Ok(Self {
            samples: samples.unwrap_or(3),
            same_sizes,
            distinct_sizes,
            mixed_claim_counts: mixed_claim_counts.unwrap_or_default(),
            recursive_fan_ins: recursive_fan_ins.unwrap_or_default(),
            json_path,
            action_json_path,
            warmup_proofs,
        })
    }
}

/// Rejects full and action report paths that currently resolve to the same file.
pub fn ensure_distinct_output_paths(
    json_path: Option<&Path>,
    action_json_path: Option<&Path>,
) -> Result<()> {
    if let (Some(json_path), Some(action_json_path)) = (json_path, action_json_path) {
        ensure!(
            !output_paths_collide(json_path, action_json_path)?,
            "--json and --action-json must write to different paths"
        );
    }
    Ok(())
}

fn output_paths_collide(left: &Path, right: &Path) -> Result<bool> {
    let left = lexically_normalize_absolute(left)?;
    let right = lexically_normalize_absolute(right)?;
    if left == right {
        return Ok(true);
    }

    match same_file::is_same_file(&left, &right) {
        Ok(same) => Ok(same),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to compare output path identity for {} and {}",
                left.display(),
                right.display()
            )
        }),
    }
}

fn lexically_normalize_absolute(path: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(path)
        .with_context(|| format!("failed to resolve output path {}", path.display()))?;
    let mut normalized = PathBuf::new();

    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                // Popping a root or platform prefix returns false and leaves it intact,
                // matching filesystem resolution of an absolute path above its root.
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }

    Ok(normalized)
}

fn option_value(option: &str, value: Option<OsString>) -> Result<OsString> {
    let value = value.with_context(|| format!("{option} requires a value"))?;
    ensure!(
        !value.to_string_lossy().starts_with("--"),
        "{option} requires a value"
    );
    Ok(value)
}

fn parse_sizes(option: &str, value: &str, maximum: usize) -> Result<Vec<usize>> {
    ensure!(
        !value.is_empty(),
        "{option} requires a comma-separated list"
    );

    let mut sizes = Vec::new();
    for component in value.split(',') {
        ensure!(
            !component.is_empty(),
            "{option} contains an empty list entry"
        );
        let size = component
            .parse::<usize>()
            .with_context(|| format!("invalid {option} entry `{component}`"))?;
        ensure!(size > 0, "{option} entries must be greater than zero");
        ensure!(
            size <= maximum,
            "{option} entry {size} exceeds maximum {maximum}"
        );
        ensure!(
            !sizes.contains(&size),
            "{option} contains duplicate entry {size}"
        );
        sizes.push(size);
    }

    Ok(sizes)
}

pub struct BlsFixtureSet {
    claim_count: usize,
    messages: Vec<lighthouse_bls::Hash256>,
    keys: Vec<lighthouse_bls::SecretKey>,
    signatures: Vec<lighthouse_bls::Signature>,
    public_keys: Vec<lighthouse_bls::PublicKey>,
}

impl BlsFixtureSet {
    pub fn same_claim(count: usize) -> Result<Self> {
        let fixtures = Self::build(count, 1)?;
        fixtures.validate_raw()?;
        Ok(fixtures)
    }

    pub fn distinct_claims(count: usize) -> Result<Self> {
        let fixtures = Self::build(count, count)?;
        fixtures.validate_raw()?;
        Ok(fixtures)
    }

    pub fn mixed_claims(signature_count: usize, claim_count: usize) -> Result<Self> {
        let fixtures = Self::build(signature_count, claim_count)?;
        fixtures.validate_raw()?;
        Ok(fixtures)
    }

    fn build(count: usize, claim_count: usize) -> Result<Self> {
        ensure!(count > 0, "BLS fixture count must be greater than zero");
        ensure!(claim_count > 0, "BLS claim count must be greater than zero");
        ensure!(
            claim_count <= count,
            "BLS claim count must not exceed the signature count"
        );
        ensure!(
            count.is_multiple_of(claim_count),
            "BLS signature count must be divisible by claim count"
        );

        let mut messages = Vec::with_capacity(count);
        let mut keys = Vec::with_capacity(count);
        let mut signatures = Vec::with_capacity(count);
        let mut public_keys = Vec::with_capacity(count);

        for index in 0..count {
            let message_index = index % claim_count;
            let message = lighthouse_bls::Hash256::from(indexed_bytes(message_index)?);
            let secret = indexed_bytes(index)
                .with_context(|| format!("failed to create BLS secret for signer {index}"))?;
            let key = lighthouse_bls::SecretKey::deserialize(&secret).map_err(|error| {
                anyhow!("failed to deserialize BLS key for signer {index}: {error:?}")
            })?;
            let public_key = key.public_key();
            let signature = key.sign(message);

            messages.push(message);
            keys.push(key);
            signatures.push(signature);
            public_keys.push(public_key);
        }

        Ok(Self {
            claim_count,
            messages,
            keys,
            signatures,
            public_keys,
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    #[must_use]
    pub fn claim_count(&self) -> usize {
        self.claim_count
    }

    #[must_use]
    pub fn messages(&self) -> &[lighthouse_bls::Hash256] {
        &self.messages
    }

    #[must_use]
    pub fn keys(&self) -> &[lighthouse_bls::SecretKey] {
        &self.keys
    }

    #[must_use]
    pub fn signatures(&self) -> &[lighthouse_bls::Signature] {
        &self.signatures
    }

    #[must_use]
    pub fn public_keys(&self) -> &[lighthouse_bls::PublicKey] {
        &self.public_keys
    }

    #[must_use]
    pub fn aggregate(&self) -> lighthouse_bls::AggregateSignature {
        let mut aggregate = lighthouse_bls::AggregateSignature::infinity();
        for signature in &self.signatures {
            aggregate.add_assign(signature);
        }
        aggregate
    }

    #[must_use]
    pub fn verify_same_claim_aggregate(
        &self,
        aggregate: &lighthouse_bls::AggregateSignature,
    ) -> bool {
        let public_keys = self.public_keys.iter().collect::<Vec<_>>();
        aggregate.fast_aggregate_verify(self.messages[0], &public_keys)
    }

    #[must_use]
    pub fn verify_distinct_claim_aggregate(
        &self,
        aggregate: &lighthouse_bls::AggregateSignature,
    ) -> bool {
        let public_keys = self.public_keys.iter().collect::<Vec<_>>();
        aggregate.aggregate_verify(&self.messages, &public_keys)
    }

    pub fn grouped_claim_context(
        &self,
    ) -> Result<(Vec<lighthouse_bls::Hash256>, Vec<lighthouse_bls::PublicKey>)> {
        let mut messages = Vec::with_capacity(self.claim_count);
        let mut public_keys = Vec::with_capacity(self.claim_count);
        for claim_index in 0..self.claim_count {
            let group = self
                .public_keys
                .iter()
                .skip(claim_index)
                .step_by(self.claim_count)
                .cloned()
                .collect::<Vec<_>>();
            let public_key = lighthouse_bls::AggregatePublicKey::aggregate(&group)
                .map_err(|error| anyhow!("failed to aggregate BLS claim {claim_index}: {error:?}"))?
                .to_public_key();
            messages.push(self.messages[claim_index]);
            public_keys.push(public_key);
        }
        Ok((messages, public_keys))
    }

    pub fn verify_grouped_claim_aggregate(
        &self,
        aggregate: &lighthouse_bls::AggregateSignature,
    ) -> Result<bool> {
        let (messages, public_keys) = self.grouped_claim_context()?;
        let public_keys = public_keys.iter().collect::<Vec<_>>();
        Ok(aggregate.aggregate_verify(&messages, &public_keys))
    }

    #[must_use]
    pub fn signature_sets(&self) -> Vec<lighthouse_bls::SignatureSet<'_>> {
        self.signatures
            .iter()
            .zip(&self.public_keys)
            .zip(&self.messages)
            .map(|((signature, public_key), message)| {
                lighthouse_bls::SignatureSet::single_pubkey(
                    signature,
                    Cow::Borrowed(public_key),
                    *message,
                )
            })
            .collect()
    }

    fn validate_raw(&self) -> Result<()> {
        let expected_len = self.len();
        ensure!(
            [
                self.messages.len(),
                self.keys.len(),
                self.signatures.len(),
                self.public_keys.len(),
            ]
            .into_iter()
            .all(|len| len == expected_len),
            "BLS fixture vectors have inconsistent lengths"
        );

        for index in 0..expected_len {
            ensure!(
                self.signatures[index].verify(&self.public_keys[index], self.messages[index]),
                "BLS signature failed validation for signer {index}"
            );
        }

        Ok(())
    }
}

pub struct FixtureSet {
    lean_claims: Vec<lean_multisig::Claim>,
    lean_keys: Vec<lean_multisig::SecretKey>,
    lean_signatures: Vec<lean_multisig::Signature>,
    lean_public_keys: Vec<lean_multisig::PublicKey>,
    bls: BlsFixtureSet,
}

impl FixtureSet {
    pub fn same_claim(count: usize) -> Result<Self> {
        Self::build(count, 1)
    }

    pub fn distinct_claims(count: usize) -> Result<Self> {
        if count > MAX_DISTINCT_CLAIMS {
            return Err(anyhow!(
                "distinct-claim fixture count {count} exceeds maximum {MAX_DISTINCT_CLAIMS}"
            ));
        }

        Self::build(count, count)
    }

    pub fn mixed_claims(signature_count: usize, claim_count: usize) -> Result<Self> {
        ensure!(
            signature_count <= MAX_SAME_CLAIM_SIGNERS,
            "mixed-claim signature count {signature_count} exceeds maximum {MAX_SAME_CLAIM_SIGNERS}"
        );
        ensure!(
            claim_count <= MAX_DISTINCT_CLAIMS,
            "mixed-claim count {claim_count} exceeds maximum {MAX_DISTINCT_CLAIMS}"
        );
        Self::build(signature_count, claim_count)
    }

    fn build(count: usize, claim_count: usize) -> Result<Self> {
        let bls = BlsFixtureSet::build(count, claim_count)?;

        let mut lean_claims = Vec::with_capacity(count);
        let mut lean_keys = Vec::with_capacity(count);
        let mut lean_signatures = Vec::with_capacity(count);
        let mut lean_public_keys = Vec::with_capacity(count);
        let last_slot = MAX_DISTINCT_CLAIMS
            .checked_sub(1)
            .context("the supported claim count must be nonzero")
            .and_then(|slot| {
                u32::try_from(slot).context("the supported claim slots must fit in u32")
            })?;

        for index in 0..count {
            let message_index = index % claim_count;
            let message = indexed_bytes(message_index)
                .with_context(|| format!("failed to create message for signer {index}"))?;
            let slot = u32::try_from(message_index)
                .with_context(|| format!("slot does not fit in u32 for signer {index}"))?;
            let claim = lean_multisig::Claim::new(message, slot);

            let lean_seed = indexed_bytes(index)
                .with_context(|| format!("failed to create XMSS seed for signer {index}"))?;
            let lean_key = lean_multisig::SecretKey::from_seed(lean_seed, 0..=last_slot)
                .with_context(|| format!("failed to create XMSS key for signer {index}"))?;
            let lean_public_key = lean_key.public_key();
            let lean_signature = lean_key
                .sign(&claim)
                .with_context(|| format!("failed to create XMSS signature for signer {index}"))?;

            lean_claims.push(claim);
            lean_keys.push(lean_key);
            lean_signatures.push(lean_signature);
            lean_public_keys.push(lean_public_key);
        }

        let fixtures = Self {
            lean_claims,
            lean_keys,
            lean_signatures,
            lean_public_keys,
            bls,
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
    pub fn claim_count(&self) -> usize {
        self.bls.claim_count()
    }

    #[must_use]
    pub fn lean_claim_groups(&self) -> Vec<lean_multisig::ClaimSigners> {
        (0..self.claim_count())
            .map(|claim_index| lean_multisig::ClaimSigners {
                claim: self.lean_claims[claim_index],
                signers: self
                    .lean_public_keys
                    .iter()
                    .skip(claim_index)
                    .step_by(self.claim_count())
                    .copied()
                    .collect(),
            })
            .collect()
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
        self.bls.messages()
    }

    #[must_use]
    pub fn bls_keys(&self) -> &[lighthouse_bls::SecretKey] {
        self.bls.keys()
    }

    #[must_use]
    pub fn bls_signatures(&self) -> &[lighthouse_bls::Signature] {
        self.bls.signatures()
    }

    #[must_use]
    pub fn bls_public_keys(&self) -> &[lighthouse_bls::PublicKey] {
        self.bls.public_keys()
    }

    #[must_use]
    pub fn bls_aggregate(&self) -> lighthouse_bls::AggregateSignature {
        self.bls.aggregate()
    }

    #[must_use]
    pub fn verify_bls_same_claim_aggregate(
        &self,
        aggregate: &lighthouse_bls::AggregateSignature,
    ) -> bool {
        self.bls.verify_same_claim_aggregate(aggregate)
    }

    #[must_use]
    pub fn verify_bls_distinct_claim_aggregate(
        &self,
        aggregate: &lighthouse_bls::AggregateSignature,
    ) -> bool {
        self.bls.verify_distinct_claim_aggregate(aggregate)
    }

    pub fn bls_grouped_claim_context(
        &self,
    ) -> Result<(Vec<lighthouse_bls::Hash256>, Vec<lighthouse_bls::PublicKey>)> {
        self.bls.grouped_claim_context()
    }

    pub fn verify_bls_grouped_claim_aggregate(
        &self,
        aggregate: &lighthouse_bls::AggregateSignature,
    ) -> Result<bool> {
        self.bls.verify_grouped_claim_aggregate(aggregate)
    }

    #[must_use]
    pub fn bls_signature_sets(&self) -> Vec<lighthouse_bls::SignatureSet<'_>> {
        self.bls.signature_sets()
    }

    pub fn validate_raw(&self) -> Result<()> {
        let expected_len = self.len();
        ensure!(
            [
                self.lean_claims.len(),
                self.lean_signatures.len(),
                self.lean_public_keys.len(),
                self.bls.len(),
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
        }

        self.bls.validate_raw()?;

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
